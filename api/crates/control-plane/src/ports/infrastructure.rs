use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

pub const EPHEMERAL_INSPECTION_DEFAULT_LIMIT: usize = 50;
pub const EPHEMERAL_INSPECTION_MAX_LIMIT: usize = 200;
pub const EPHEMERAL_INSPECTION_DEFAULT_BYTE_LIMIT: usize = 64 * 1024;
pub const EPHEMERAL_INSPECTION_MAX_BYTE_LIMIT: usize = 256 * 1024;
pub const EPHEMERAL_VALUE_PREVIEW_BYTES: usize = 8 * 1024;
pub const EPHEMERAL_VALUE_FULL_BYTES: usize = 256 * 1024;
pub const EPHEMERAL_VALUE_MAX_BYTES: usize = 1024 * 1024;
pub const EPHEMERAL_PAYLOAD_MAX_BYTES: usize = 1024 * 1024;

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
    pub list_tree: bool,
    pub search_entries: bool,
    pub reveal_value: bool,
    pub default_page_size: u64,
    pub max_page_size: u64,
    pub default_byte_limit: u64,
    pub max_byte_limit: u64,
    pub default_preview_size_bytes: u64,
    pub max_full_value_size_bytes: u64,
    pub max_value_size_bytes: u64,
    pub max_payload_size_bytes: u64,
}

impl EphemeralInspectionCapabilities {
    pub const fn unsupported() -> Self {
        Self {
            list_entries: false,
            list_tree: false,
            search_entries: false,
            reveal_value: false,
            default_page_size: EPHEMERAL_INSPECTION_DEFAULT_LIMIT as u64,
            max_page_size: EPHEMERAL_INSPECTION_MAX_LIMIT as u64,
            default_byte_limit: EPHEMERAL_INSPECTION_DEFAULT_BYTE_LIMIT as u64,
            max_byte_limit: EPHEMERAL_INSPECTION_MAX_BYTE_LIMIT as u64,
            default_preview_size_bytes: EPHEMERAL_VALUE_PREVIEW_BYTES as u64,
            max_full_value_size_bytes: EPHEMERAL_VALUE_FULL_BYTES as u64,
            max_value_size_bytes: EPHEMERAL_VALUE_MAX_BYTES as u64,
            max_payload_size_bytes: EPHEMERAL_PAYLOAD_MAX_BYTES as u64,
        }
    }

    pub const fn supported() -> Self {
        Self {
            list_entries: true,
            list_tree: true,
            search_entries: true,
            reveal_value: true,
            default_page_size: EPHEMERAL_INSPECTION_DEFAULT_LIMIT as u64,
            max_page_size: EPHEMERAL_INSPECTION_MAX_LIMIT as u64,
            default_byte_limit: EPHEMERAL_INSPECTION_DEFAULT_BYTE_LIMIT as u64,
            max_byte_limit: EPHEMERAL_INSPECTION_MAX_BYTE_LIMIT as u64,
            default_preview_size_bytes: EPHEMERAL_VALUE_PREVIEW_BYTES as u64,
            max_full_value_size_bytes: EPHEMERAL_VALUE_FULL_BYTES as u64,
            max_value_size_bytes: EPHEMERAL_VALUE_MAX_BYTES as u64,
            max_payload_size_bytes: EPHEMERAL_PAYLOAD_MAX_BYTES as u64,
        }
    }

    pub const fn metadata_only() -> Self {
        Self {
            list_entries: true,
            list_tree: true,
            search_entries: true,
            reveal_value: false,
            default_page_size: EPHEMERAL_INSPECTION_DEFAULT_LIMIT as u64,
            max_page_size: EPHEMERAL_INSPECTION_MAX_LIMIT as u64,
            default_byte_limit: EPHEMERAL_INSPECTION_DEFAULT_BYTE_LIMIT as u64,
            max_byte_limit: EPHEMERAL_INSPECTION_MAX_BYTE_LIMIT as u64,
            default_preview_size_bytes: EPHEMERAL_VALUE_PREVIEW_BYTES as u64,
            max_full_value_size_bytes: EPHEMERAL_VALUE_FULL_BYTES as u64,
            max_value_size_bytes: EPHEMERAL_VALUE_MAX_BYTES as u64,
            max_payload_size_bytes: EPHEMERAL_PAYLOAD_MAX_BYTES as u64,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EphemeralInspectionPageRequest {
    pub inspection_path: Vec<String>,
    pub cursor: Option<String>,
    pub limit: usize,
    pub byte_limit: usize,
}

impl EphemeralInspectionPageRequest {
    pub fn new(
        inspection_path: Vec<String>,
        cursor: Option<String>,
        limit: Option<usize>,
        byte_limit: Option<usize>,
    ) -> Self {
        Self {
            inspection_path,
            cursor,
            limit: limit
                .unwrap_or(EPHEMERAL_INSPECTION_DEFAULT_LIMIT)
                .clamp(1, EPHEMERAL_INSPECTION_MAX_LIMIT),
            byte_limit: byte_limit
                .unwrap_or(EPHEMERAL_INSPECTION_DEFAULT_BYTE_LIMIT)
                .clamp(1, EPHEMERAL_INSPECTION_MAX_BYTE_LIMIT),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EphemeralInspectionSummarySnapshot {
    pub entry_count: u64,
    pub sensitive_entry_count: u64,
    pub total_value_size_bytes: u64,
}

impl EphemeralInspectionSummarySnapshot {
    pub const fn empty() -> Self {
        Self {
            entry_count: 0,
            sensitive_entry_count: 0,
            total_value_size_bytes: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EphemeralValueRevealMode {
    Metadata,
    Preview,
    Full,
}

impl Default for EphemeralValueRevealMode {
    fn default() -> Self {
        Self::Preview
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EphemeralEntryValueState {
    Hidden,
    Available,
    Preview,
    ValueTooLarge,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EphemeralInspectionTreeNodeSnapshot {
    pub node_ref: String,
    pub label: String,
    pub inspection_path: Vec<String>,
    pub depth: u64,
    pub entry_count: u64,
    pub sensitive_entry_count: u64,
    pub total_value_size_bytes: u64,
    pub has_children: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EphemeralInspectionTreePage {
    pub inspection_path: Vec<String>,
    pub nodes: Vec<EphemeralInspectionTreeNodeSnapshot>,
    pub next_cursor: Option<String>,
    pub limit: u64,
    pub byte_limit: u64,
    pub emitted_bytes: u64,
    pub truncated_by_byte_limit: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EphemeralInspectionEntryPage {
    pub inspection_path: Vec<String>,
    pub entries: Vec<EphemeralEntrySnapshot>,
    pub next_cursor: Option<String>,
    pub limit: u64,
    pub byte_limit: u64,
    pub emitted_bytes: u64,
    pub truncated_by_byte_limit: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EphemeralEntrySnapshot {
    pub contract_code: String,
    pub group_code: Option<String>,
    pub entry_ref: String,
    pub key: String,
    pub inspection_path: Vec<String>,
    pub entry_kind: String,
    pub status: String,
    pub owner: Option<String>,
    pub value_size_bytes: u64,
    pub metadata_size_bytes: u64,
    pub ttl_seconds: Option<i64>,
    pub created_at_unix: Option<i64>,
    pub expires_at_unix: Option<i64>,
    pub sensitive: bool,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EphemeralEntryValueSnapshot {
    pub metadata: EphemeralEntrySnapshot,
    pub reveal_mode: EphemeralValueRevealMode,
    pub value_state: EphemeralEntryValueState,
    pub value: Option<serde_json::Value>,
    pub value_preview: Option<String>,
    pub preview_size_bytes: u64,
    pub full_value_size_bytes: u64,
}

impl EphemeralEntryValueSnapshot {
    pub fn from_value(
        metadata: EphemeralEntrySnapshot,
        value: serde_json::Value,
        reveal_mode: EphemeralValueRevealMode,
    ) -> Self {
        let serialized = serde_json::to_string(&value).unwrap_or_default();
        let full_value_size_bytes = serialized.len() as u64;
        match reveal_mode {
            EphemeralValueRevealMode::Metadata => Self {
                metadata,
                reveal_mode,
                value_state: EphemeralEntryValueState::Hidden,
                value: None,
                value_preview: None,
                preview_size_bytes: 0,
                full_value_size_bytes,
            },
            EphemeralValueRevealMode::Preview
                if serialized.len() <= EPHEMERAL_VALUE_PREVIEW_BYTES =>
            {
                Self {
                    metadata,
                    reveal_mode,
                    value_state: EphemeralEntryValueState::Available,
                    value: Some(value),
                    value_preview: None,
                    preview_size_bytes: full_value_size_bytes,
                    full_value_size_bytes,
                }
            }
            EphemeralValueRevealMode::Preview => {
                let preview = truncate_json_preview(&serialized, EPHEMERAL_VALUE_PREVIEW_BYTES);
                Self {
                    metadata,
                    reveal_mode,
                    value_state: EphemeralEntryValueState::Preview,
                    value: None,
                    preview_size_bytes: preview.len() as u64,
                    value_preview: Some(preview),
                    full_value_size_bytes,
                }
            }
            EphemeralValueRevealMode::Full if serialized.len() <= EPHEMERAL_VALUE_FULL_BYTES => {
                Self {
                    metadata,
                    reveal_mode,
                    value_state: EphemeralEntryValueState::Available,
                    value: Some(value),
                    value_preview: None,
                    preview_size_bytes: full_value_size_bytes,
                    full_value_size_bytes,
                }
            }
            EphemeralValueRevealMode::Full => Self {
                metadata,
                reveal_mode,
                value_state: EphemeralEntryValueState::ValueTooLarge,
                value: None,
                value_preview: None,
                preview_size_bytes: 0,
                full_value_size_bytes,
            },
        }
    }
}

pub fn ephemeral_metadata_size_bytes(metadata: &serde_json::Value) -> u64 {
    serde_json::to_vec(metadata)
        .map(|bytes| bytes.len() as u64)
        .unwrap_or(0)
}

pub fn ensure_ephemeral_value_size(value: &serde_json::Value) -> anyhow::Result<()> {
    ensure_ephemeral_json_size(
        value,
        EPHEMERAL_VALUE_MAX_BYTES,
        "ephemeral_value_too_large",
    )
}

pub fn ensure_ephemeral_payload_size(value: &serde_json::Value) -> anyhow::Result<()> {
    ensure_ephemeral_json_size(
        value,
        EPHEMERAL_PAYLOAD_MAX_BYTES,
        "ephemeral_payload_too_large",
    )
}

pub fn summarize_ephemeral_entries(
    entries: &[EphemeralEntrySnapshot],
) -> EphemeralInspectionSummarySnapshot {
    EphemeralInspectionSummarySnapshot {
        entry_count: entries.len() as u64,
        sensitive_entry_count: entries.iter().filter(|entry| entry.sensitive).count() as u64,
        total_value_size_bytes: entries
            .iter()
            .map(|entry| entry.value_size_bytes)
            .sum::<u64>(),
    }
}

pub fn paginate_ephemeral_entries(
    mut entries: Vec<EphemeralEntrySnapshot>,
    request: EphemeralInspectionPageRequest,
) -> EphemeralInspectionEntryPage {
    entries.sort_by_key(entry_cursor_token);
    let mut page_entries = Vec::new();
    let mut emitted_bytes = 0usize;
    let mut truncated_by_byte_limit = false;
    let filtered_entries = entries
        .into_iter()
        .filter(|entry| path_has_prefix(&entry.inspection_path, &request.inspection_path))
        .filter(|entry| {
            request
                .cursor
                .as_ref()
                .is_none_or(|cursor| entry_cursor_token(entry).as_str() > cursor.as_str())
        })
        .collect::<Vec<_>>();

    for entry in filtered_entries.iter().cloned() {
        let item_size = serialized_size_bytes(&entry);
        if !page_entries.is_empty()
            && (page_entries.len() >= request.limit
                || emitted_bytes.saturating_add(item_size) > request.byte_limit)
        {
            truncated_by_byte_limit = emitted_bytes.saturating_add(item_size) > request.byte_limit;
            break;
        }
        emitted_bytes = emitted_bytes.saturating_add(item_size);
        page_entries.push(entry);
        if page_entries.len() >= request.limit {
            break;
        }
    }

    let next_cursor = page_entries
        .last()
        .filter(|last| {
            let last_cursor = entry_cursor_token(last);
            filtered_entries
                .iter()
                .any(|entry| entry_cursor_token(entry) > last_cursor)
        })
        .map(entry_cursor_token);
    EphemeralInspectionEntryPage {
        inspection_path: request.inspection_path,
        entries: page_entries,
        next_cursor,
        limit: request.limit as u64,
        byte_limit: request.byte_limit as u64,
        emitted_bytes: emitted_bytes as u64,
        truncated_by_byte_limit,
    }
}

pub fn paginate_ephemeral_tree(
    entries: Vec<EphemeralEntrySnapshot>,
    request: EphemeralInspectionPageRequest,
) -> EphemeralInspectionTreePage {
    let mut nodes = BTreeMap::<String, EphemeralInspectionTreeNodeSnapshot>::new();
    for entry in entries {
        if !path_has_prefix(&entry.inspection_path, &request.inspection_path)
            || entry.inspection_path.len() <= request.inspection_path.len()
        {
            continue;
        }
        let child_depth = request.inspection_path.len();
        let label = entry.inspection_path[child_depth].clone();
        let inspection_path = entry.inspection_path[..=child_depth].to_vec();
        let node_ref = inspection_path.join("/");
        let node = nodes
            .entry(node_ref.clone())
            .or_insert(EphemeralInspectionTreeNodeSnapshot {
                node_ref,
                label,
                inspection_path,
                depth: (child_depth + 1) as u64,
                entry_count: 0,
                sensitive_entry_count: 0,
                total_value_size_bytes: 0,
                has_children: false,
            });
        node.entry_count += 1;
        node.sensitive_entry_count += u64::from(entry.sensitive);
        node.total_value_size_bytes += entry.value_size_bytes;
        node.has_children |= entry.inspection_path.len() > child_depth + 1;
    }

    let mut page_nodes = Vec::new();
    let mut emitted_bytes = 0usize;
    let mut truncated_by_byte_limit = false;
    let mut filtered_nodes = nodes
        .into_values()
        .filter(|node| {
            request
                .cursor
                .as_ref()
                .is_none_or(|cursor| tree_cursor_token(node).as_str() > cursor.as_str())
        })
        .collect::<Vec<_>>();
    filtered_nodes.sort_by_key(tree_cursor_token);
    for node in filtered_nodes.iter().cloned() {
        let item_size = serialized_size_bytes(&node);
        if !page_nodes.is_empty()
            && (page_nodes.len() >= request.limit
                || emitted_bytes.saturating_add(item_size) > request.byte_limit)
        {
            truncated_by_byte_limit = emitted_bytes.saturating_add(item_size) > request.byte_limit;
            break;
        }
        emitted_bytes = emitted_bytes.saturating_add(item_size);
        page_nodes.push(node);
        if page_nodes.len() >= request.limit {
            break;
        }
    }

    let next_cursor = page_nodes
        .last()
        .filter(|last| {
            let last_cursor = tree_cursor_token(last);
            filtered_nodes
                .iter()
                .any(|node| tree_cursor_token(node) > last_cursor)
        })
        .map(tree_cursor_token);
    EphemeralInspectionTreePage {
        inspection_path: request.inspection_path,
        nodes: page_nodes,
        next_cursor,
        limit: request.limit as u64,
        byte_limit: request.byte_limit as u64,
        emitted_bytes: emitted_bytes as u64,
        truncated_by_byte_limit,
    }
}

pub fn search_ephemeral_entries(
    entries: Vec<EphemeralEntrySnapshot>,
    query: &str,
    request: EphemeralInspectionPageRequest,
) -> EphemeralInspectionEntryPage {
    let normalized_query = query.trim().to_lowercase();
    if normalized_query.is_empty() {
        return paginate_ephemeral_entries(entries, request);
    }
    let matched_entries = entries
        .into_iter()
        .filter(|entry| ephemeral_entry_matches_query(entry, &normalized_query))
        .collect::<Vec<_>>();
    paginate_ephemeral_entries(matched_entries, request)
}

fn ephemeral_entry_matches_query(entry: &EphemeralEntrySnapshot, query: &str) -> bool {
    let mut searchable = vec![
        entry.entry_ref.as_str(),
        entry.key.as_str(),
        entry.contract_code.as_str(),
        entry.entry_kind.as_str(),
        entry.status.as_str(),
    ]
    .join(" ");
    if let Some(group_code) = &entry.group_code {
        searchable.push(' ');
        searchable.push_str(group_code);
    }
    if let Some(owner) = &entry.owner {
        searchable.push(' ');
        searchable.push_str(owner);
    }
    searchable.push(' ');
    searchable.push_str(&entry.inspection_path.join("/"));
    searchable.push(' ');
    searchable.push_str(&entry.metadata.to_string());
    searchable.to_lowercase().contains(query)
}

fn path_has_prefix(path: &[String], prefix: &[String]) -> bool {
    prefix.len() <= path.len()
        && prefix
            .iter()
            .zip(path.iter())
            .all(|(prefix, segment)| prefix == segment)
}

fn serialized_size_bytes<T: Serialize>(value: &T) -> usize {
    serde_json::to_vec(value)
        .map(|bytes| bytes.len())
        .unwrap_or(0)
}

fn entry_cursor_token(entry: &EphemeralEntrySnapshot) -> String {
    format!(
        "{}\u{1f}{}",
        path_cursor_token(&entry.inspection_path),
        normalized_cursor_segment(&entry.entry_ref)
    )
}

fn tree_cursor_token(node: &EphemeralInspectionTreeNodeSnapshot) -> String {
    path_cursor_token(&node.inspection_path)
}

fn path_cursor_token(path: &[String]) -> String {
    path.iter()
        .map(|segment| normalized_cursor_segment(segment))
        .collect::<Vec<_>>()
        .join("\u{1f}")
}

fn normalized_cursor_segment(segment: &str) -> String {
    segment
        .parse::<u128>()
        .map(|value| format!("n{value:039}"))
        .unwrap_or_else(|_| format!("s{segment}"))
}

fn truncate_json_preview(value: &str, byte_limit: usize) -> String {
    let mut end = byte_limit.min(value.len());
    while !value.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    value[..end].to_string()
}

fn ensure_ephemeral_json_size(
    value: &serde_json::Value,
    max_bytes: usize,
    error_code: &'static str,
) -> anyhow::Result<()> {
    let size = serde_json::to_vec(value).map(|bytes| bytes.len())?;
    anyhow::ensure!(size <= max_bytes, error_code);
    Ok(())
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

    async fn summarize_ephemeral_entries(
        &self,
    ) -> anyhow::Result<EphemeralInspectionSummarySnapshot> {
        Ok(summarize_ephemeral_entries(
            &self.list_ephemeral_entries().await?,
        ))
    }

    async fn list_ephemeral_tree(
        &self,
        request: EphemeralInspectionPageRequest,
    ) -> anyhow::Result<EphemeralInspectionTreePage> {
        Ok(paginate_ephemeral_tree(
            self.list_ephemeral_entries().await?,
            request,
        ))
    }

    async fn list_ephemeral_entry_page(
        &self,
        request: EphemeralInspectionPageRequest,
    ) -> anyhow::Result<EphemeralInspectionEntryPage> {
        Ok(paginate_ephemeral_entries(
            self.list_ephemeral_entries().await?,
            request,
        ))
    }

    async fn search_ephemeral_entry_page(
        &self,
        query: &str,
        request: EphemeralInspectionPageRequest,
    ) -> anyhow::Result<EphemeralInspectionEntryPage> {
        Ok(search_ephemeral_entries(
            self.list_ephemeral_entries().await?,
            query,
            request,
        ))
    }

    async fn reveal_ephemeral_entry(
        &self,
        _entry_ref: &str,
        _reveal_mode: EphemeralValueRevealMode,
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

    async fn summarize_ephemeral_entries(
        &self,
    ) -> anyhow::Result<EphemeralInspectionSummarySnapshot> {
        Ok(summarize_ephemeral_entries(
            &self.list_ephemeral_entries().await?,
        ))
    }

    async fn list_ephemeral_tree(
        &self,
        request: EphemeralInspectionPageRequest,
    ) -> anyhow::Result<EphemeralInspectionTreePage> {
        Ok(paginate_ephemeral_tree(
            self.list_ephemeral_entries().await?,
            request,
        ))
    }

    async fn list_ephemeral_entry_page(
        &self,
        request: EphemeralInspectionPageRequest,
    ) -> anyhow::Result<EphemeralInspectionEntryPage> {
        Ok(paginate_ephemeral_entries(
            self.list_ephemeral_entries().await?,
            request,
        ))
    }

    async fn search_ephemeral_entry_page(
        &self,
        query: &str,
        request: EphemeralInspectionPageRequest,
    ) -> anyhow::Result<EphemeralInspectionEntryPage> {
        Ok(search_ephemeral_entries(
            self.list_ephemeral_entries().await?,
            query,
            request,
        ))
    }

    async fn reveal_ephemeral_entry(
        &self,
        _entry_ref: &str,
        _reveal_mode: EphemeralValueRevealMode,
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

    async fn summarize_ephemeral_entries(
        &self,
    ) -> anyhow::Result<EphemeralInspectionSummarySnapshot> {
        Ok(summarize_ephemeral_entries(
            &self.list_ephemeral_entries().await?,
        ))
    }

    async fn list_ephemeral_tree(
        &self,
        request: EphemeralInspectionPageRequest,
    ) -> anyhow::Result<EphemeralInspectionTreePage> {
        Ok(paginate_ephemeral_tree(
            self.list_ephemeral_entries().await?,
            request,
        ))
    }

    async fn list_ephemeral_entry_page(
        &self,
        request: EphemeralInspectionPageRequest,
    ) -> anyhow::Result<EphemeralInspectionEntryPage> {
        Ok(paginate_ephemeral_entries(
            self.list_ephemeral_entries().await?,
            request,
        ))
    }

    async fn search_ephemeral_entry_page(
        &self,
        query: &str,
        request: EphemeralInspectionPageRequest,
    ) -> anyhow::Result<EphemeralInspectionEntryPage> {
        Ok(search_ephemeral_entries(
            self.list_ephemeral_entries().await?,
            query,
            request,
        ))
    }

    async fn reveal_ephemeral_entry(
        &self,
        _entry_ref: &str,
        _reveal_mode: EphemeralValueRevealMode,
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

    async fn summarize_ephemeral_entries(
        &self,
    ) -> anyhow::Result<EphemeralInspectionSummarySnapshot> {
        Ok(summarize_ephemeral_entries(
            &self.list_ephemeral_entries().await?,
        ))
    }

    async fn list_ephemeral_tree(
        &self,
        request: EphemeralInspectionPageRequest,
    ) -> anyhow::Result<EphemeralInspectionTreePage> {
        Ok(paginate_ephemeral_tree(
            self.list_ephemeral_entries().await?,
            request,
        ))
    }

    async fn list_ephemeral_entry_page(
        &self,
        request: EphemeralInspectionPageRequest,
    ) -> anyhow::Result<EphemeralInspectionEntryPage> {
        Ok(paginate_ephemeral_entries(
            self.list_ephemeral_entries().await?,
            request,
        ))
    }

    async fn search_ephemeral_entry_page(
        &self,
        query: &str,
        request: EphemeralInspectionPageRequest,
    ) -> anyhow::Result<EphemeralInspectionEntryPage> {
        Ok(search_ephemeral_entries(
            self.list_ephemeral_entries().await?,
            query,
            request,
        ))
    }

    async fn reveal_ephemeral_entry(
        &self,
        _entry_ref: &str,
        _reveal_mode: EphemeralValueRevealMode,
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

    async fn summarize_ephemeral_entries(
        &self,
    ) -> anyhow::Result<EphemeralInspectionSummarySnapshot> {
        Ok(summarize_ephemeral_entries(
            &self.list_ephemeral_entries().await?,
        ))
    }

    async fn list_ephemeral_tree(
        &self,
        request: EphemeralInspectionPageRequest,
    ) -> anyhow::Result<EphemeralInspectionTreePage> {
        Ok(paginate_ephemeral_tree(
            self.list_ephemeral_entries().await?,
            request,
        ))
    }

    async fn list_ephemeral_entry_page(
        &self,
        request: EphemeralInspectionPageRequest,
    ) -> anyhow::Result<EphemeralInspectionEntryPage> {
        Ok(paginate_ephemeral_entries(
            self.list_ephemeral_entries().await?,
            request,
        ))
    }

    async fn search_ephemeral_entry_page(
        &self,
        query: &str,
        request: EphemeralInspectionPageRequest,
    ) -> anyhow::Result<EphemeralInspectionEntryPage> {
        Ok(search_ephemeral_entries(
            self.list_ephemeral_entries().await?,
            query,
            request,
        ))
    }

    async fn reveal_ephemeral_entry(
        &self,
        _entry_ref: &str,
        _reveal_mode: EphemeralValueRevealMode,
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

    async fn summarize_ephemeral_entries(
        &self,
    ) -> anyhow::Result<EphemeralInspectionSummarySnapshot> {
        Ok(summarize_ephemeral_entries(
            &self.list_ephemeral_entries().await?,
        ))
    }

    async fn list_ephemeral_tree(
        &self,
        request: EphemeralInspectionPageRequest,
    ) -> anyhow::Result<EphemeralInspectionTreePage> {
        Ok(paginate_ephemeral_tree(
            self.list_ephemeral_entries().await?,
            request,
        ))
    }

    async fn list_ephemeral_entry_page(
        &self,
        request: EphemeralInspectionPageRequest,
    ) -> anyhow::Result<EphemeralInspectionEntryPage> {
        Ok(paginate_ephemeral_entries(
            self.list_ephemeral_entries().await?,
            request,
        ))
    }

    async fn search_ephemeral_entry_page(
        &self,
        query: &str,
        request: EphemeralInspectionPageRequest,
    ) -> anyhow::Result<EphemeralInspectionEntryPage> {
        Ok(search_ephemeral_entries(
            self.list_ephemeral_entries().await?,
            query,
            request,
        ))
    }

    async fn reveal_ephemeral_entry(
        &self,
        _entry_ref: &str,
        _reveal_mode: EphemeralValueRevealMode,
    ) -> anyhow::Result<Option<EphemeralEntryValueSnapshot>> {
        Ok(None)
    }
}
