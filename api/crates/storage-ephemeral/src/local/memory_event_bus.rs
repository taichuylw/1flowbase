use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use async_trait::async_trait;
use control_plane::ports::{
    EphemeralEntrySnapshot, EphemeralEntryValueSnapshot, EphemeralInspectionCapabilities, EventBus,
};
use tokio::sync::Mutex;

#[derive(Clone, Default)]
pub struct MemoryEventBus {
    topics: Arc<Mutex<HashMap<String, VecDeque<serde_json::Value>>>>,
}

impl MemoryEventBus {
    pub fn new() -> Self {
        Self::default()
    }

    fn entry_key(topic: &str, index: usize) -> String {
        format!("{topic}#{index}")
    }

    fn parse_entry_key(key: &str) -> Option<(&str, usize)> {
        let (topic, index) = key.rsplit_once('#')?;
        Some((topic, index.parse().ok()?))
    }

    fn value_size_bytes(value: &serde_json::Value) -> u64 {
        serde_json::to_vec(value)
            .map(|bytes| bytes.len() as u64)
            .unwrap_or(0)
    }

    fn entry_snapshot(
        topic: &str,
        index: usize,
        value: &serde_json::Value,
    ) -> EphemeralEntrySnapshot {
        EphemeralEntrySnapshot {
            contract_code: "event-bus".to_string(),
            group_code: Some(topic.to_string()),
            key: Self::entry_key(topic, index),
            entry_kind: "event".to_string(),
            status: "buffered".to_string(),
            owner: None,
            value_size_bytes: Self::value_size_bytes(value),
            ttl_seconds: None,
            created_at_unix: None,
            expires_at_unix: None,
            sensitive: true,
            metadata: serde_json::json!({
                "topic": topic,
                "index": index,
            }),
        }
    }
}

#[async_trait]
impl EventBus for MemoryEventBus {
    async fn publish(&self, topic: &str, payload: serde_json::Value) -> anyhow::Result<()> {
        self.topics
            .lock()
            .await
            .entry(topic.to_string())
            .or_default()
            .push_back(payload);
        Ok(())
    }

    async fn poll(&self, topic: &str) -> anyhow::Result<Option<serde_json::Value>> {
        Ok(self
            .topics
            .lock()
            .await
            .get_mut(topic)
            .and_then(VecDeque::pop_front))
    }

    fn ephemeral_inspection_capabilities(&self) -> EphemeralInspectionCapabilities {
        EphemeralInspectionCapabilities::supported()
    }

    async fn list_ephemeral_entries(&self) -> anyhow::Result<Vec<EphemeralEntrySnapshot>> {
        let topics = self.topics.lock().await;
        let mut entries = topics
            .iter()
            .flat_map(|(topic, values)| {
                values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| Self::entry_snapshot(topic, index, value))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| {
            left.group_code
                .cmp(&right.group_code)
                .then(left.key.cmp(&right.key))
        });
        Ok(entries)
    }

    async fn reveal_ephemeral_entry(
        &self,
        key: &str,
    ) -> anyhow::Result<Option<EphemeralEntryValueSnapshot>> {
        let Some((topic, index)) = Self::parse_entry_key(key) else {
            return Ok(None);
        };
        let topics = self.topics.lock().await;
        let Some(value) = topics
            .get(topic)
            .and_then(|values| values.get(index))
            .cloned()
        else {
            return Ok(None);
        };
        Ok(Some(EphemeralEntryValueSnapshot {
            metadata: Self::entry_snapshot(topic, index, &value),
            value,
        }))
    }
}
