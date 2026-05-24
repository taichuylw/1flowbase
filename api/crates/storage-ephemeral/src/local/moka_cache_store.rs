use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration as StdDuration, Instant},
};

use async_trait::async_trait;
use control_plane::ports::{
    CacheDomainSnapshot, CacheEntrySnapshot, CacheEntryValueSnapshot, CacheInspectionCapabilities,
    CacheStore,
};
use moka::{future::Cache, Expiry};
use time::OffsetDateTime;
use tokio::sync::Mutex;

use crate::EphemeralKvStore;

#[derive(Clone)]
struct CacheEntry {
    value: serde_json::Value,
    ttl: Option<time::Duration>,
    created_at: OffsetDateTime,
    expires_at: Option<OffsetDateTime>,
}

struct CacheEntryExpiry;

impl Expiry<String, CacheEntry> for CacheEntryExpiry {
    fn expire_after_create(
        &self,
        _key: &String,
        value: &CacheEntry,
        _created_at: Instant,
    ) -> Option<StdDuration> {
        MokaCacheStore::ttl_to_std(value.ttl)
    }

    fn expire_after_update(
        &self,
        _key: &String,
        value: &CacheEntry,
        _updated_at: Instant,
        _duration_until_expiry: Option<StdDuration>,
    ) -> Option<StdDuration> {
        MokaCacheStore::ttl_to_std(value.ttl)
    }
}

#[derive(Clone)]
pub struct MokaCacheStore {
    namespace: String,
    cache: Cache<String, CacheEntry>,
    set_if_absent_guard: Arc<Mutex<()>>,
}

impl MokaCacheStore {
    pub fn new(namespace: impl Into<String>, max_capacity: u64) -> Self {
        Self {
            namespace: namespace.into(),
            cache: Cache::builder()
                .max_capacity(max_capacity)
                .expire_after(CacheEntryExpiry)
                .build(),
            set_if_absent_guard: Arc::new(Mutex::new(())),
        }
    }

    fn namespaced_key(&self, key: &str) -> String {
        format!("{}:{}", self.namespace, key)
    }

    fn key_without_namespace(&self, key: &str) -> Option<String> {
        key.strip_prefix(&format!("{}:", self.namespace))
            .map(ToString::to_string)
    }

    fn domain_code(key: &str) -> &str {
        key.split_once(':')
            .map(|(domain, _)| domain)
            .unwrap_or("default")
    }

    fn key_matches_domain(key: &str, domain_code: &str) -> bool {
        Self::domain_code(key) == domain_code
    }

    fn ttl_to_std(ttl: Option<time::Duration>) -> Option<StdDuration> {
        ttl.map(|value| {
            if value <= time::Duration::ZERO {
                StdDuration::ZERO
            } else {
                value.try_into().unwrap_or(StdDuration::MAX)
            }
        })
    }

    fn ttl_is_non_positive(ttl: Option<time::Duration>) -> bool {
        ttl.is_some_and(|value| value <= time::Duration::ZERO)
    }

    async fn get_entry(&self, key: &str) -> Option<CacheEntry> {
        self.cache.get(&self.namespaced_key(key)).await
    }

    fn entry_expires_at(
        now: OffsetDateTime,
        ttl: Option<time::Duration>,
    ) -> Option<OffsetDateTime> {
        ttl.filter(|value| *value > time::Duration::ZERO)
            .map(|value| now + value)
    }

    async fn set_entry(&self, key: &str, value: serde_json::Value, ttl: Option<time::Duration>) {
        let namespaced_key = self.namespaced_key(key);
        if Self::ttl_is_non_positive(ttl) {
            self.cache.invalidate(&namespaced_key).await;
            return;
        }

        let now = OffsetDateTime::now_utc();
        self.cache
            .insert(
                namespaced_key,
                CacheEntry {
                    value,
                    ttl,
                    created_at: now,
                    expires_at: Self::entry_expires_at(now, ttl),
                },
            )
            .await;
    }

    fn entry_size_bytes(entry: &CacheEntry) -> u64 {
        serde_json::to_vec(&entry.value)
            .map(|bytes| bytes.len() as u64)
            .unwrap_or(0)
    }

    fn remaining_ttl_seconds(entry: &CacheEntry, now: OffsetDateTime) -> Option<i64> {
        match entry.expires_at {
            Some(expires_at) => Some((expires_at - now).whole_seconds().max(0)),
            None => entry.ttl.map(|ttl| ttl.whole_seconds().max(0)),
        }
    }

    fn entry_snapshot(key: String, entry: &CacheEntry) -> CacheEntrySnapshot {
        let now = OffsetDateTime::now_utc();
        CacheEntrySnapshot {
            domain_code: Self::domain_code(&key).to_string(),
            key,
            value_size_bytes: Self::entry_size_bytes(entry),
            ttl_seconds: Self::remaining_ttl_seconds(entry, now),
            created_at_unix: Some(entry.created_at.unix_timestamp()),
            expires_at_unix: entry.expires_at.map(|value| value.unix_timestamp()),
        }
    }

    async fn visible_entries(&self) -> Vec<(String, CacheEntry)> {
        self.cache.run_pending_tasks().await;
        self.cache
            .iter()
            .filter_map(|(key, entry)| self.key_without_namespace(&key).map(|key| (key, entry)))
            .collect()
    }
}

#[async_trait]
impl CacheStore for MokaCacheStore {
    async fn get_json(&self, key: &str) -> anyhow::Result<Option<serde_json::Value>> {
        Ok(self.get_entry(key).await.map(|entry| entry.value))
    }

    async fn set_json(
        &self,
        key: &str,
        value: serde_json::Value,
        ttl: Option<time::Duration>,
    ) -> anyhow::Result<()> {
        self.set_entry(key, value, ttl).await;
        Ok(())
    }

    async fn set_if_absent_json(
        &self,
        key: &str,
        value: serde_json::Value,
        ttl: Option<time::Duration>,
    ) -> anyhow::Result<bool> {
        EphemeralKvStore::set_if_absent_json(self, key, value, ttl).await
    }

    async fn delete(&self, key: &str) -> anyhow::Result<()> {
        self.cache.invalidate(&self.namespaced_key(key)).await;
        Ok(())
    }

    async fn touch(&self, key: &str, ttl: time::Duration) -> anyhow::Result<bool> {
        let namespaced_key = self.namespaced_key(key);
        if ttl <= time::Duration::ZERO {
            self.cache.invalidate(&namespaced_key).await;
            return Ok(false);
        }

        let Some(entry) = self.cache.get(&namespaced_key).await else {
            return Ok(false);
        };

        let now = OffsetDateTime::now_utc();
        self.cache
            .insert(
                namespaced_key,
                CacheEntry {
                    value: entry.value,
                    ttl: Some(ttl),
                    created_at: entry.created_at,
                    expires_at: Self::entry_expires_at(now, Some(ttl)),
                },
            )
            .await;
        Ok(true)
    }

    fn inspection_capabilities(&self) -> CacheInspectionCapabilities {
        CacheInspectionCapabilities::supported()
    }

    async fn list_cache_domains(&self) -> anyhow::Result<Vec<CacheDomainSnapshot>> {
        let mut domains = BTreeMap::<String, CacheDomainSnapshot>::new();
        for (key, entry) in self.visible_entries().await {
            let domain_code = Self::domain_code(&key).to_string();
            let entry_size = Self::entry_size_bytes(&entry);
            let domain = domains
                .entry(domain_code.clone())
                .or_insert(CacheDomainSnapshot {
                    domain_code,
                    entry_count: 0,
                    total_value_size_bytes: 0,
                });
            domain.entry_count += 1;
            domain.total_value_size_bytes += entry_size;
        }

        Ok(domains.into_values().collect())
    }

    async fn list_cache_entries(
        &self,
        domain_code: &str,
    ) -> anyhow::Result<Vec<CacheEntrySnapshot>> {
        let mut entries = self
            .visible_entries()
            .await
            .into_iter()
            .filter(|(key, _)| Self::key_matches_domain(key, domain_code))
            .map(|(key, entry)| Self::entry_snapshot(key, &entry))
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| left.key.cmp(&right.key));
        Ok(entries)
    }

    async fn reveal_cache_entry(
        &self,
        domain_code: &str,
        key: &str,
    ) -> anyhow::Result<Option<CacheEntryValueSnapshot>> {
        if !Self::key_matches_domain(key, domain_code) {
            return Ok(None);
        }

        let Some(entry) = self.get_entry(key).await else {
            return Ok(None);
        };
        let metadata = Self::entry_snapshot(key.to_string(), &entry);
        Ok(Some(CacheEntryValueSnapshot {
            metadata,
            value: entry.value,
        }))
    }

    async fn clear_cache_entry(&self, domain_code: &str, key: &str) -> anyhow::Result<bool> {
        if !Self::key_matches_domain(key, domain_code) {
            return Ok(false);
        }

        let existed = self.get_entry(key).await.is_some();
        self.cache.invalidate(&self.namespaced_key(key)).await;
        Ok(existed)
    }

    async fn clear_cache_domain(&self, domain_code: &str) -> anyhow::Result<u64> {
        let keys = self
            .visible_entries()
            .await
            .into_iter()
            .filter_map(|(key, _)| Self::key_matches_domain(&key, domain_code).then_some(key))
            .collect::<Vec<_>>();
        let count = keys.len() as u64;
        for key in keys {
            self.cache.invalidate(&self.namespaced_key(&key)).await;
        }
        Ok(count)
    }
}

#[async_trait]
impl EphemeralKvStore for MokaCacheStore {
    async fn set_json(
        &self,
        key: &str,
        value: serde_json::Value,
        ttl: Option<time::Duration>,
    ) -> anyhow::Result<()> {
        self.set_entry(key, value, ttl).await;
        Ok(())
    }

    async fn get_json(&self, key: &str) -> anyhow::Result<Option<serde_json::Value>> {
        Ok(self.get_entry(key).await.map(|entry| entry.value))
    }

    async fn delete(&self, key: &str) -> anyhow::Result<()> {
        self.cache.invalidate(&self.namespaced_key(key)).await;
        Ok(())
    }

    async fn touch(&self, key: &str, ttl: time::Duration) -> anyhow::Result<bool> {
        CacheStore::touch(self, key, ttl).await
    }

    async fn set_if_absent_json(
        &self,
        key: &str,
        value: serde_json::Value,
        ttl: Option<time::Duration>,
    ) -> anyhow::Result<bool> {
        let _guard = self.set_if_absent_guard.lock().await;
        let namespaced_key = self.namespaced_key(key);
        if self.cache.get(&namespaced_key).await.is_some() {
            return Ok(false);
        }

        if Self::ttl_is_non_positive(ttl) {
            self.cache.invalidate(&namespaced_key).await;
            return Ok(true);
        }

        let now = OffsetDateTime::now_utc();
        self.cache
            .insert(
                namespaced_key,
                CacheEntry {
                    value,
                    ttl,
                    created_at: now,
                    expires_at: Self::entry_expires_at(now, ttl),
                },
            )
            .await;
        Ok(true)
    }
}
