use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheInspectionCapabilities {
    pub list_domains: bool,
    pub list_entries: bool,
    pub reveal_value: bool,
    pub clear_entry: bool,
    pub clear_domain: bool,
}

impl CacheInspectionCapabilities {
    pub const fn unsupported() -> Self {
        Self {
            list_domains: false,
            list_entries: false,
            reveal_value: false,
            clear_entry: false,
            clear_domain: false,
        }
    }

    pub const fn supported() -> Self {
        Self {
            list_domains: true,
            list_entries: true,
            reveal_value: true,
            clear_entry: true,
            clear_domain: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheDomainSnapshot {
    pub domain_code: String,
    pub entry_count: u64,
    pub total_value_size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheEntrySnapshot {
    pub domain_code: String,
    pub key: String,
    pub value_size_bytes: u64,
    pub ttl_seconds: Option<i64>,
    pub created_at_unix: Option<i64>,
    pub expires_at_unix: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheEntryValueSnapshot {
    pub metadata: CacheEntrySnapshot,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EphemeralInspectionCapabilities {
    pub list_entries: bool,
    pub reveal_value: bool,
}

impl EphemeralInspectionCapabilities {
    pub const fn unsupported() -> Self {
        Self {
            list_entries: false,
            reveal_value: false,
        }
    }

    pub const fn supported() -> Self {
        Self {
            list_entries: true,
            reveal_value: true,
        }
    }

    pub const fn metadata_only() -> Self {
        Self {
            list_entries: true,
            reveal_value: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EphemeralEntrySnapshot {
    pub contract_code: String,
    pub group_code: Option<String>,
    pub key: String,
    pub entry_kind: String,
    pub status: String,
    pub owner: Option<String>,
    pub value_size_bytes: u64,
    pub ttl_seconds: Option<i64>,
    pub created_at_unix: Option<i64>,
    pub expires_at_unix: Option<i64>,
    pub sensitive: bool,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EphemeralEntryValueSnapshot {
    pub metadata: EphemeralEntrySnapshot,
    pub value: serde_json::Value,
}

#[async_trait]
pub trait CacheStore: Send + Sync {
    async fn get_json(&self, key: &str) -> anyhow::Result<Option<serde_json::Value>>;

    async fn set_json(
        &self,
        key: &str,
        value: serde_json::Value,
        ttl: Option<time::Duration>,
    ) -> anyhow::Result<()>;

    async fn set_if_absent_json(
        &self,
        key: &str,
        value: serde_json::Value,
        ttl: Option<time::Duration>,
    ) -> anyhow::Result<bool>;

    async fn delete(&self, key: &str) -> anyhow::Result<()>;

    async fn touch(&self, key: &str, ttl: time::Duration) -> anyhow::Result<bool>;

    fn inspection_capabilities(&self) -> CacheInspectionCapabilities {
        CacheInspectionCapabilities::unsupported()
    }

    async fn list_cache_domains(&self) -> anyhow::Result<Vec<CacheDomainSnapshot>> {
        Ok(Vec::new())
    }

    async fn list_cache_entries(
        &self,
        _domain_code: &str,
    ) -> anyhow::Result<Vec<CacheEntrySnapshot>> {
        Ok(Vec::new())
    }

    async fn reveal_cache_entry(
        &self,
        _domain_code: &str,
        _key: &str,
    ) -> anyhow::Result<Option<CacheEntryValueSnapshot>> {
        Ok(None)
    }

    async fn clear_cache_entry(&self, _domain_code: &str, _key: &str) -> anyhow::Result<bool> {
        Ok(false)
    }

    async fn clear_cache_domain(&self, _domain_code: &str) -> anyhow::Result<u64> {
        Ok(0)
    }

    fn ephemeral_inspection_capabilities(&self) -> EphemeralInspectionCapabilities {
        EphemeralInspectionCapabilities::unsupported()
    }

    async fn list_ephemeral_entries(&self) -> anyhow::Result<Vec<EphemeralEntrySnapshot>> {
        Ok(Vec::new())
    }

    async fn reveal_ephemeral_entry(
        &self,
        _key: &str,
    ) -> anyhow::Result<Option<EphemeralEntryValueSnapshot>> {
        Ok(None)
    }
}

#[async_trait]
pub trait DistributedLock: Send + Sync {
    async fn acquire(&self, key: &str, owner: &str, ttl: time::Duration) -> anyhow::Result<bool>;

    async fn renew(&self, key: &str, owner: &str, ttl: time::Duration) -> anyhow::Result<bool>;

    async fn release(&self, key: &str, owner: &str) -> anyhow::Result<bool>;

    fn ephemeral_inspection_capabilities(&self) -> EphemeralInspectionCapabilities {
        EphemeralInspectionCapabilities::unsupported()
    }

    async fn list_ephemeral_entries(&self) -> anyhow::Result<Vec<EphemeralEntrySnapshot>> {
        Ok(Vec::new())
    }

    async fn reveal_ephemeral_entry(
        &self,
        _key: &str,
    ) -> anyhow::Result<Option<EphemeralEntryValueSnapshot>> {
        Ok(None)
    }
}

#[async_trait]
pub trait EventBus: Send + Sync {
    async fn publish(&self, topic: &str, payload: serde_json::Value) -> anyhow::Result<()>;

    async fn poll(&self, topic: &str) -> anyhow::Result<Option<serde_json::Value>>;

    fn ephemeral_inspection_capabilities(&self) -> EphemeralInspectionCapabilities {
        EphemeralInspectionCapabilities::unsupported()
    }

    async fn list_ephemeral_entries(&self) -> anyhow::Result<Vec<EphemeralEntrySnapshot>> {
        Ok(Vec::new())
    }

    async fn reveal_ephemeral_entry(
        &self,
        _key: &str,
    ) -> anyhow::Result<Option<EphemeralEntryValueSnapshot>> {
        Ok(None)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClaimedTask {
    pub task_id: String,
    pub payload: serde_json::Value,
    pub claimed_by: String,
    pub idempotency_key: Option<String>,
    pub claim_expires_at_unix: i64,
}

#[async_trait]
pub trait TaskQueue: Send + Sync {
    async fn enqueue(
        &self,
        queue: &str,
        payload: serde_json::Value,
        idempotency_key: Option<&str>,
    ) -> anyhow::Result<String>;

    async fn claim(
        &self,
        queue: &str,
        worker: &str,
        visibility_timeout: time::Duration,
    ) -> anyhow::Result<Option<ClaimedTask>>;

    async fn ack(&self, queue: &str, task_id: &str, worker: &str) -> anyhow::Result<bool>;

    async fn fail(
        &self,
        queue: &str,
        task_id: &str,
        worker: &str,
        reason: &str,
    ) -> anyhow::Result<bool>;

    fn ephemeral_inspection_capabilities(&self) -> EphemeralInspectionCapabilities {
        EphemeralInspectionCapabilities::unsupported()
    }

    async fn list_ephemeral_entries(&self) -> anyhow::Result<Vec<EphemeralEntrySnapshot>> {
        Ok(Vec::new())
    }

    async fn reveal_ephemeral_entry(
        &self,
        _key: &str,
    ) -> anyhow::Result<Option<EphemeralEntryValueSnapshot>> {
        Ok(None)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimitDecision {
    pub allowed: bool,
    pub remaining: u64,
    pub reset_after_ms: u64,
}

#[async_trait]
pub trait RateLimitStore: Send + Sync {
    async fn consume(
        &self,
        key: &str,
        limit: u64,
        window: time::Duration,
    ) -> anyhow::Result<RateLimitDecision>;

    async fn reset(&self, key: &str) -> anyhow::Result<()>;

    fn ephemeral_inspection_capabilities(&self) -> EphemeralInspectionCapabilities {
        EphemeralInspectionCapabilities::unsupported()
    }

    async fn list_ephemeral_entries(&self) -> anyhow::Result<Vec<EphemeralEntrySnapshot>> {
        Ok(Vec::new())
    }

    async fn reveal_ephemeral_entry(
        &self,
        _key: &str,
    ) -> anyhow::Result<Option<EphemeralEntryValueSnapshot>> {
        Ok(None)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeEventSource {
    Runtime,
    Provider,
    Persister,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeEventDurability {
    Ephemeral,
    DurableRequired,
    AuditRequired,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEventPayload {
    pub event_type: String,
    pub source: RuntimeEventSource,
    pub durability: RuntimeEventDurability,
    pub persist_required: bool,
    pub trace_visible: bool,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEventEnvelope {
    pub run_id: Uuid,
    pub node_run_id: Option<Uuid>,
    pub sequence: i64,
    pub event_id: String,
    pub event_type: String,
    pub occurred_at: time::OffsetDateTime,
    pub delta_index: Option<i64>,
    pub content_type: Option<String>,
    pub text: Option<String>,
    pub source: RuntimeEventSource,
    pub durability: RuntimeEventDurability,
    pub persist_required: bool,
    pub trace_visible: bool,
    pub payload: serde_json::Value,
}

impl RuntimeEventEnvelope {
    pub fn new(run_id: Uuid, sequence: i64, event: RuntimeEventPayload) -> Self {
        let node_run_id = event
            .payload
            .get("node_run_id")
            .and_then(serde_json::Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok());
        let text = event
            .payload
            .get("text")
            .or_else(|| event.payload.get("delta"))
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string);
        let (delta_index, content_type) = match event.event_type.as_str() {
            "text_delta" => (
                Some(
                    event
                        .payload
                        .get("delta_index")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(sequence),
                ),
                Some("text".to_string()),
            ),
            "reasoning_delta" => (
                Some(
                    event
                        .payload
                        .get("delta_index")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(sequence),
                ),
                Some("reasoning".to_string()),
            ),
            _ => (None, None),
        };

        Self {
            run_id,
            node_run_id,
            sequence,
            event_id: format!("{run_id}:{sequence}"),
            event_type: event.event_type,
            occurred_at: time::OffsetDateTime::now_utc(),
            delta_index,
            content_type,
            text,
            source: event.source,
            durability: event.durability,
            persist_required: event.persist_required,
            trace_visible: event.trace_visible,
            payload: event.payload,
        }
    }
}

pub struct RuntimeEventSubscription {
    pub replay: Vec<RuntimeEventEnvelope>,
    pub live_events: mpsc::UnboundedReceiver<RuntimeEventEnvelope>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeEventOverflowBehavior {
    DropOldEphemeralKeepRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeEventStreamPolicy {
    pub ttl: time::Duration,
    pub max_events: usize,
    pub max_bytes: usize,
    pub overflow_behavior: RuntimeEventOverflowBehavior,
}

impl RuntimeEventStreamPolicy {
    pub fn debug_default() -> Self {
        Self {
            ttl: time::Duration::minutes(30),
            max_events: 20_000,
            max_bytes: 16 * 1024 * 1024,
            overflow_behavior: RuntimeEventOverflowBehavior::DropOldEphemeralKeepRequired,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeEventCloseReason {
    Finished,
    Failed,
    Cancelled,
    WaitingHuman,
    WaitingCallback,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeEventTrimPolicy {
    pub before_sequence: Option<i64>,
    pub keep_required: bool,
}

#[async_trait]
pub trait RuntimeEventStream: Send + Sync {
    async fn open_run(&self, run_id: Uuid, policy: RuntimeEventStreamPolicy) -> anyhow::Result<()>;

    async fn append(
        &self,
        run_id: Uuid,
        event: RuntimeEventPayload,
    ) -> anyhow::Result<RuntimeEventEnvelope>;

    async fn subscribe(
        &self,
        run_id: Uuid,
        from_sequence: Option<i64>,
    ) -> anyhow::Result<RuntimeEventSubscription>;

    async fn replay(
        &self,
        run_id: Uuid,
        from_sequence: Option<i64>,
        limit: usize,
    ) -> anyhow::Result<Vec<RuntimeEventEnvelope>>;

    async fn close_run(&self, run_id: Uuid, reason: RuntimeEventCloseReason) -> anyhow::Result<()>;

    async fn trim(&self, run_id: Uuid, policy: RuntimeEventTrimPolicy) -> anyhow::Result<()>;

    fn ephemeral_inspection_capabilities(&self) -> EphemeralInspectionCapabilities {
        EphemeralInspectionCapabilities::unsupported()
    }

    async fn list_ephemeral_entries(&self) -> anyhow::Result<Vec<EphemeralEntrySnapshot>> {
        Ok(Vec::new())
    }

    async fn reveal_ephemeral_entry(
        &self,
        _key: &str,
    ) -> anyhow::Result<Option<EphemeralEntryValueSnapshot>> {
        Ok(None)
    }
}
