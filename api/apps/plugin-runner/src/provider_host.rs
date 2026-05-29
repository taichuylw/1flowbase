use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use plugin_framework::{
    error::{FrameworkResult, PluginFrameworkError},
    manifest_v1::PluginExecutionMode,
    provider_contract::{
        ModelDiscoveryMode, ProviderBalanceResult, ProviderInvocationInput,
        ProviderInvocationResult, ProviderModelDescriptor, ProviderStdioMethod,
        ProviderStdioRequest, ProviderStreamEvent,
    },
};
use serde::Serialize;
use serde_json::Value;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tokio::sync::Mutex;

use crate::package_loader::{LoadedProviderPackage, PackageLoader};
use crate::stdio_runtime::{call_executable, call_executable_streaming, ProviderWorker};

#[derive(Debug, Clone, Serialize)]
pub struct LoadedProviderSummary {
    pub plugin_id: String,
    pub provider_code: String,
    pub plugin_version: String,
    pub protocol: String,
    pub model_discovery_mode: ModelDiscoveryMode,
}

impl LoadedProviderSummary {
    fn from_loaded(loaded: &LoadedProviderPackage) -> Self {
        Self {
            plugin_id: loaded.package.identifier(),
            provider_code: loaded.package.provider.provider_code.clone(),
            plugin_version: loaded.package.manifest.version.clone(),
            protocol: loaded.package.provider.protocol.clone(),
            model_discovery_mode: loaded.package.provider.model_discovery_mode,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LoadedProviderSource {
    package_root: PathBuf,
    source_identity: Option<String>,
}

impl LoadedProviderSource {
    fn resolve(package_root: &str, source_identity: Option<&str>) -> FrameworkResult<Self> {
        let package_root = fs::canonicalize(package_root).map_err(|error| {
            PluginFrameworkError::invalid_provider_package(format!(
                "cannot resolve package root: {error}"
            ))
        })?;
        Ok(Self {
            package_root,
            source_identity: source_identity.map(ToOwned::to_owned),
        })
    }

    fn can_skip_reload(&self, requested: &Self) -> bool {
        self.source_identity.is_some() && self == requested
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderValidationOutput {
    pub output: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderModelsOutput {
    pub models: Vec<ProviderModelDescriptor>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderBalanceOutput {
    pub balance: ProviderBalanceResult,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderInvokeStreamOutput {
    pub events: Vec<ProviderStreamEvent>,
    pub result: ProviderInvocationResult,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProviderActiveStreamsOutput {
    pub streams: Vec<ProviderActiveStreamSnapshot>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProviderActiveStreamSnapshot {
    pub invocation_id: String,
    pub plugin_id: String,
    pub provider_instance_id: String,
    pub provider_code: String,
    pub protocol: String,
    pub model: String,
    pub transport: String,
    pub status: String,
    pub started_at: String,
    pub last_event_at: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone)]
struct ActiveProviderStreamRecord {
    invocation_id: String,
    plugin_id: String,
    provider_instance_id: String,
    provider_code: String,
    protocol: String,
    model: String,
    transport: String,
    status: String,
    started_at: OffsetDateTime,
    last_event_at: OffsetDateTime,
}

impl ActiveProviderStreamRecord {
    fn new(invocation_id: String, plugin_id: &str, input: &ProviderInvocationInput) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            invocation_id,
            plugin_id: plugin_id.to_string(),
            provider_instance_id: input.provider_instance_id.clone(),
            provider_code: input.provider_code.clone(),
            protocol: input.protocol.clone(),
            model: input.model.clone(),
            transport: provider_stream_transport(input),
            status: "running".to_string(),
            started_at: now,
            last_event_at: now,
        }
    }

    fn snapshot(&self, now: OffsetDateTime) -> ProviderActiveStreamSnapshot {
        ProviderActiveStreamSnapshot {
            invocation_id: self.invocation_id.clone(),
            plugin_id: self.plugin_id.clone(),
            provider_instance_id: self.provider_instance_id.clone(),
            provider_code: self.provider_code.clone(),
            protocol: self.protocol.clone(),
            model: self.model.clone(),
            transport: self.transport.clone(),
            status: self.status.clone(),
            started_at: format_timestamp(self.started_at),
            last_event_at: format_timestamp(self.last_event_at),
            duration_ms: elapsed_milliseconds(self.started_at, now),
        }
    }
}

#[derive(Debug)]
pub struct ProviderHost {
    loaded_packages: HashMap<String, LoadedProviderPackage>,
    loaded_sources: HashMap<String, LoadedProviderSource>,
    provider_workers: Mutex<HashMap<String, ProviderWorker>>,
    active_streams: Arc<Mutex<HashMap<String, ActiveProviderStreamRecord>>>,
    next_invocation_sequence: AtomicU64,
}

impl Default for ProviderHost {
    fn default() -> Self {
        Self {
            loaded_packages: HashMap::new(),
            loaded_sources: HashMap::new(),
            provider_workers: Mutex::new(HashMap::new()),
            active_streams: Arc::new(Mutex::new(HashMap::new())),
            next_invocation_sequence: AtomicU64::new(1),
        }
    }
}

impl ProviderHost {
    pub fn load(&mut self, package_root: &str) -> FrameworkResult<LoadedProviderSummary> {
        self.load_with_source_identity(package_root, None)
    }

    fn load_with_source_identity(
        &mut self,
        package_root: &str,
        source_identity: Option<&str>,
    ) -> FrameworkResult<LoadedProviderSummary> {
        let source = LoadedProviderSource::resolve(package_root, source_identity)?;
        self.load_source(source, None)
    }

    fn load_source(
        &mut self,
        source: LoadedProviderSource,
        expected_plugin_id: Option<&str>,
    ) -> FrameworkResult<LoadedProviderSummary> {
        let loaded = PackageLoader::load(&source.package_root)?;
        let summary = LoadedProviderSummary::from_loaded(&loaded);
        if let Some(expected_plugin_id) = expected_plugin_id {
            if summary.plugin_id != expected_plugin_id {
                return Err(PluginFrameworkError::invalid_provider_package(format!(
                    "loaded provider package id {} does not match requested {expected_plugin_id}",
                    summary.plugin_id
                )));
            }
        }
        self.loaded_packages
            .insert(summary.plugin_id.clone(), loaded);
        self.loaded_sources
            .insert(summary.plugin_id.clone(), source);
        self.provider_workers.get_mut().remove(&summary.plugin_id);
        Ok(summary)
    }

    pub fn is_loaded(&self, plugin_id: &str) -> bool {
        self.loaded_packages.contains_key(plugin_id)
    }

    pub fn load_if_needed(
        &mut self,
        plugin_id: &str,
        package_root: &str,
        source_identity: Option<&str>,
    ) -> FrameworkResult<()> {
        let requested_source = LoadedProviderSource::resolve(package_root, source_identity)?;
        if self
            .loaded_sources
            .get(plugin_id)
            .is_some_and(|loaded_source| loaded_source.can_skip_reload(&requested_source))
        {
            return Ok(());
        }
        self.load_source(requested_source, Some(plugin_id))
            .map(|_| ())
    }

    pub fn reload(&mut self, plugin_id: &str) -> FrameworkResult<LoadedProviderSummary> {
        let source = match self.loaded_sources.get(plugin_id).cloned() {
            Some(source) => source,
            None => {
                let package_root = self
                    .loaded_packages
                    .get(plugin_id)
                    .ok_or_else(|| {
                        PluginFrameworkError::invalid_provider_package(format!(
                            "provider package is not loaded: {plugin_id}"
                        ))
                    })?
                    .package_root
                    .clone();
                LoadedProviderSource {
                    package_root,
                    source_identity: None,
                }
            }
        };
        if !self.loaded_packages.contains_key(plugin_id) {
            return Err(PluginFrameworkError::invalid_provider_package(format!(
                "provider package is not loaded: {plugin_id}"
            )));
        }
        self.load_source(source, Some(plugin_id))
    }

    pub async fn validate(
        &self,
        plugin_id: &str,
        provider_config: Value,
    ) -> FrameworkResult<ProviderValidationOutput> {
        let loaded = self.loaded_package(plugin_id)?;
        let output = self
            .call_runtime(loaded, ProviderStdioMethod::Validate, provider_config)
            .await?;
        Ok(ProviderValidationOutput { output })
    }

    pub async fn list_models(
        &self,
        plugin_id: &str,
        provider_config: Value,
    ) -> FrameworkResult<ProviderModelsOutput> {
        let loaded = self.loaded_package(plugin_id)?;
        let models = match loaded.package.provider.model_discovery_mode {
            ModelDiscoveryMode::Static => loaded.package.predefined_models.clone(),
            ModelDiscoveryMode::Dynamic => {
                let dynamic = self
                    .call_runtime(loaded, ProviderStdioMethod::ListModels, provider_config)
                    .await?;
                normalize_models(dynamic)?
            }
            ModelDiscoveryMode::Hybrid => {
                let dynamic = self
                    .call_runtime(loaded, ProviderStdioMethod::ListModels, provider_config)
                    .await?;
                merge_models(
                    &loaded.package.predefined_models,
                    normalize_models(dynamic)?,
                )
            }
        };
        Ok(ProviderModelsOutput { models })
    }

    pub async fn get_balance(
        &self,
        plugin_id: &str,
        provider_config: Value,
    ) -> FrameworkResult<ProviderBalanceOutput> {
        let loaded = self.loaded_package(plugin_id)?;
        let raw_balance = self
            .call_runtime(loaded, ProviderStdioMethod::Balance, provider_config)
            .await?;
        Ok(ProviderBalanceOutput {
            balance: normalize_balance(raw_balance)?,
        })
    }

    pub async fn invoke_stream(
        &self,
        plugin_id: &str,
        input: ProviderInvocationInput,
    ) -> FrameworkResult<ProviderInvokeStreamOutput> {
        self.invoke_stream_with_live_events(plugin_id, input, None)
            .await
    }

    pub async fn invoke_stream_with_live_events(
        &self,
        plugin_id: &str,
        input: ProviderInvocationInput,
        live_events: Option<tokio::sync::mpsc::UnboundedSender<ProviderStreamEvent>>,
    ) -> FrameworkResult<ProviderInvokeStreamOutput> {
        let loaded = self.loaded_package(plugin_id)?;
        let invocation_id = self.register_active_stream(plugin_id, &input).await;
        let event_observer = Some(self.active_stream_event_observer(invocation_id.clone()));
        let request = ProviderStdioRequest {
            method: ProviderStdioMethod::Invoke,
            input: serde_json::to_value(input).unwrap(),
        };
        let output = match loaded.package.manifest.execution_mode {
            PluginExecutionMode::ProcessPerCall => {
                call_executable_streaming(
                    &loaded.runtime_executable,
                    &request,
                    &loaded.package.manifest.runtime.limits,
                    live_events,
                    event_observer,
                )
                .await
            }
            PluginExecutionMode::StatefulProviderWorker => {
                let mut workers = self.provider_workers.lock().await;
                let worker = workers.entry(plugin_id.to_string()).or_insert_with(|| {
                    ProviderWorker::new(
                        loaded.runtime_executable.clone(),
                        loaded.package.manifest.runtime.limits.clone(),
                    )
                });
                worker
                    .call_streaming(&request, live_events, event_observer)
                    .await
            }
            _ => Err(PluginFrameworkError::invalid_provider_package(
                "model provider package declares unsupported execution_mode",
            )),
        };
        self.remove_active_stream(&invocation_id).await;
        let output = output?;
        Ok(ProviderInvokeStreamOutput {
            events: output.events,
            result: output.result,
        })
    }

    pub async fn active_stream_snapshot(&self) -> ProviderActiveStreamsOutput {
        let now = OffsetDateTime::now_utc();
        let mut streams = self
            .active_streams
            .lock()
            .await
            .values()
            .map(|record| record.snapshot(now))
            .collect::<Vec<_>>();
        streams.sort_by(|left, right| left.started_at.cmp(&right.started_at));
        ProviderActiveStreamsOutput { streams }
    }

    async fn register_active_stream(
        &self,
        plugin_id: &str,
        input: &ProviderInvocationInput,
    ) -> String {
        let sequence = self
            .next_invocation_sequence
            .fetch_add(1, Ordering::Relaxed);
        let invocation_id = format!("{plugin_id}:{sequence}");
        let record = ActiveProviderStreamRecord::new(invocation_id.clone(), plugin_id, input);
        self.active_streams
            .lock()
            .await
            .insert(invocation_id.clone(), record);
        invocation_id
    }

    fn active_stream_event_observer(
        &self,
        invocation_id: String,
    ) -> tokio::sync::mpsc::UnboundedSender<()> {
        let active_streams = Arc::clone(&self.active_streams);
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            while receiver.recv().await.is_some() {
                if let Some(record) = active_streams.lock().await.get_mut(&invocation_id) {
                    record.last_event_at = OffsetDateTime::now_utc();
                }
            }
        });
        sender
    }

    async fn remove_active_stream(&self, invocation_id: &str) {
        self.active_streams.lock().await.remove(invocation_id);
    }

    fn loaded_package(&self, plugin_id: &str) -> FrameworkResult<&LoadedProviderPackage> {
        self.loaded_packages.get(plugin_id).ok_or_else(|| {
            PluginFrameworkError::invalid_provider_package(format!(
                "provider package is not loaded: {plugin_id}"
            ))
        })
    }

    async fn call_runtime(
        &self,
        loaded: &LoadedProviderPackage,
        method: ProviderStdioMethod,
        input: Value,
    ) -> FrameworkResult<Value> {
        let request = ProviderStdioRequest { method, input };
        match loaded.package.manifest.execution_mode {
            PluginExecutionMode::ProcessPerCall => {
                call_executable(
                    &loaded.runtime_executable,
                    &request,
                    &loaded.package.manifest.runtime.limits,
                )
                .await
            }
            PluginExecutionMode::StatefulProviderWorker => {
                let plugin_id = loaded.package.identifier();
                let mut workers = self.provider_workers.lock().await;
                let worker = workers.entry(plugin_id).or_insert_with(|| {
                    ProviderWorker::new(
                        loaded.runtime_executable.clone(),
                        loaded.package.manifest.runtime.limits.clone(),
                    )
                });
                worker.call(&request).await
            }
            _ => Err(PluginFrameworkError::invalid_provider_package(
                "model provider package declares unsupported execution_mode",
            )),
        }
    }
}

fn provider_stream_transport(input: &ProviderInvocationInput) -> String {
    if let Some(transport_mode) = provider_config_transport_mode(&input.provider_config) {
        return normalize_transport_mode_hint(&transport_mode);
    }
    if input.protocol == "openai_responses" || input.provider_code == "openai" {
        return "http_sse".to_string();
    }
    "provider_stream".to_string()
}

fn provider_config_transport_mode(provider_config: &Value) -> Option<String> {
    let value = provider_config.get("transport_mode")?;
    let text = match value {
        Value::String(text) => text.trim().to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    };
    (!text.is_empty()).then_some(text)
}

fn normalize_transport_mode_hint(transport_mode: &str) -> String {
    match transport_mode.trim().to_ascii_lowercase().as_str() {
        "" => "http_sse".to_string(),
        "sse" | "http" | "http_sse" => "http_sse".to_string(),
        "ws" | "websocket" | "responses_websocket" => "responses_websocket".to_string(),
        "auto" => "auto".to_string(),
        other => other.to_string(),
    }
}

fn elapsed_milliseconds(started_at: OffsetDateTime, now: OffsetDateTime) -> u64 {
    let milliseconds = (now - started_at).whole_milliseconds();
    u64::try_from(milliseconds).unwrap_or(0)
}

fn format_timestamp(value: OffsetDateTime) -> String {
    value.format(&Rfc3339).unwrap_or_else(|_| value.to_string())
}

fn normalize_models(raw: Value) -> FrameworkResult<Vec<ProviderModelDescriptor>> {
    serde_json::from_value(raw)
        .map_err(|error| PluginFrameworkError::invalid_provider_contract(error.to_string()))
}

fn normalize_balance(raw: Value) -> FrameworkResult<ProviderBalanceResult> {
    serde_json::from_value(raw)
        .map_err(|error| PluginFrameworkError::invalid_provider_contract(error.to_string()))
}

fn merge_models(
    static_models: &[ProviderModelDescriptor],
    dynamic_models: Vec<ProviderModelDescriptor>,
) -> Vec<ProviderModelDescriptor> {
    let mut merged = BTreeMap::new();
    for model in static_models {
        merged.insert(model.model_id.clone(), model.clone());
    }
    for model in dynamic_models {
        merged.insert(model.model_id.clone(), model);
    }
    merged.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use serde_json::json;

    struct TempProviderPackage {
        root: PathBuf,
    }

    impl TempProviderPackage {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!("provider-host-test-{nonce}"));
            fs::create_dir_all(&root).unwrap();
            let package = Self { root };
            package.write_provider_package("Fixture Provider");
            package
        }

        fn path(&self) -> &Path {
            &self.root
        }

        fn write(&self, relative_path: &str, content: &str) {
            let path = self.root.join(relative_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, content).unwrap();
        }

        fn write_provider_package(&self, display_name: &str) {
            self.write(
                "manifest.yaml",
                &format!(
                    r#"manifest_version: 1
plugin_id: fixture_provider
version: 0.1.0
vendor: 1flowbase
display_name: {display_name}
description: Fixture provider
source_kind: uploaded
trust_level: checksum_only
consumption_kind: runtime_extension
execution_mode: process_per_call
slot_codes:
  - model_provider
binding_targets:
  - workspace
selection_mode: assignment_then_select
minimum_host_version: 0.1.0
contract_version: 1flowbase.provider/v1
schema_version: 1flowbase.plugin.manifest/v1
permissions:
  network: none
  secrets: provider_instance_only
  storage: none
  mcp: none
  subprocess: deny
runtime:
  protocol: stdio_json
  entry: bin/fixture_provider
  limits:
    timeout_ms: 30000
node_contributions: []
"#
                ),
            );
            self.write(
                "provider/fixture_provider.yaml",
                r#"provider_code: fixture_provider
display_name: Fixture Provider
protocol: openai_compatible
model_discovery: static
config_schema: []
"#,
            );
            self.write(
                "i18n/en_US.json",
                r#"{ "plugin": { "label": "Fixture Provider" } }"#,
            );
            self.write("bin/fixture_provider", "#!/usr/bin/env bash\n");
        }
    }

    impl Drop for TempProviderPackage {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn normalize_models_accepts_current_provider_descriptor_shape() {
        let models = normalize_models(json!([{
            "model_id": "gpt-4o-mini",
            "display_name": "GPT-4o mini",
            "source": "dynamic",
            "supports_streaming": true,
            "supports_tool_call": false,
            "supports_multimodal": false,
            "context_window": null,
            "max_output_tokens": null,
            "parameter_form": null,
            "provider_metadata": {}
        }]))
        .expect("current provider descriptor shape should stay supported");

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].model_id, "gpt-4o-mini");
    }

    #[test]
    fn normalize_models_rejects_legacy_provider_descriptor_shape() {
        assert!(
            normalize_models(json!([{
                "code": "gpt-4o-mini",
                "label": "GPT-4o mini",
                "family": "llm",
                "mode": "chat"
            }]))
            .is_err(),
            "legacy code/label model descriptors should be rejected once current contract is the only supported shape"
        );
    }

    #[test]
    fn load_if_needed_skips_reloading_matching_loaded_provider_source() {
        let package = TempProviderPackage::new();
        let mut host = ProviderHost::default();
        let summary = host
            .load_with_source_identity(package.path().to_str().unwrap(), Some("gen-1"))
            .unwrap();
        assert!(host.is_loaded(&summary.plugin_id));

        package.write_provider_package("Mutated Provider");
        host.load_if_needed(
            &summary.plugin_id,
            package.path().to_str().unwrap(),
            Some("gen-1"),
        )
        .unwrap();

        let loaded = host.loaded_packages.get(&summary.plugin_id).unwrap();
        assert_eq!(loaded.package.manifest.display_name, "Fixture Provider");
    }

    #[test]
    fn load_if_needed_reloads_when_provider_source_identity_changes() {
        let package = TempProviderPackage::new();
        let mut host = ProviderHost::default();
        let summary = host
            .load_with_source_identity(package.path().to_str().unwrap(), Some("gen-1"))
            .unwrap();
        assert!(host.is_loaded(&summary.plugin_id));

        package.write_provider_package("Mutated Provider");
        host.load_if_needed(
            &summary.plugin_id,
            package.path().to_str().unwrap(),
            Some("gen-2"),
        )
        .unwrap();

        let loaded = host.loaded_packages.get(&summary.plugin_id).unwrap();
        assert_eq!(loaded.package.manifest.display_name, "Mutated Provider");
    }
}
