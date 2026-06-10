use super::*;

pub(super) fn lock_provider_worker_registry(
    provider_workers: &ProviderWorkerRegistry,
) -> FrameworkResult<std::sync::MutexGuard<'_, HashMap<String, ProviderWorkerHandle>>> {
    provider_workers.lock().map_err(|_| {
        PluginFrameworkError::invalid_provider_package("provider worker registry is unavailable")
    })
}

pub(super) fn provider_worker_handle(
    provider_workers: &ProviderWorkerRegistry,
    plugin_id: String,
    loaded: &LoadedProviderPackage,
) -> FrameworkResult<ProviderWorkerHandle> {
    let mut workers = lock_provider_worker_registry(provider_workers)?;
    Ok(workers
        .entry(plugin_id)
        .or_insert_with(|| {
            Arc::new(Mutex::new(ProviderWorker::new(
                loaded.runtime_executable.clone(),
                loaded.package.manifest.runtime.limits.clone(),
            )))
        })
        .clone())
}

pub(super) fn provider_invocation_limits(limits: &PluginRuntimeLimits) -> PluginRuntimeLimits {
    let mut invocation_limits = limits.clone();
    invocation_limits.timeout_ms = limits
        .invoke_timeout_ms
        .or(Some(DEFAULT_PROVIDER_INVOCATION_TIMEOUT_MS));
    invocation_limits
}

pub(super) fn provider_pool_key(input: &ProviderInvocationInput) -> String {
    format!(
        "provider_pool:v1:provider_instance={}:provider_code={}:protocol={}:model={}",
        stable_pool_component(&input.provider_instance_id),
        stable_pool_component(&input.provider_code),
        stable_pool_component(&input.protocol),
        stable_pool_component(&input.model),
    )
}

pub(super) fn stable_pool_component(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

pub(super) fn provider_stream_transport(input: &ProviderInvocationInput) -> String {
    if let Some(transport_mode) = provider_config_transport_mode(&input.provider_config) {
        return normalize_transport_mode_hint(&transport_mode);
    }
    if input.protocol == "openai_responses" || input.provider_code == "openai" {
        return "http_sse".to_string();
    }
    "provider_stream".to_string()
}

pub(super) fn provider_config_transport_mode(provider_config: &Value) -> Option<String> {
    let value = provider_config.get("transport_mode")?;
    let text = match value {
        Value::String(text) => text.trim().to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    };
    (!text.is_empty()).then_some(text)
}

pub(super) fn normalize_transport_mode_hint(transport_mode: &str) -> String {
    match transport_mode.trim().to_ascii_lowercase().as_str() {
        "" => "http_sse".to_string(),
        "sse" | "http" | "http_sse" => "http_sse".to_string(),
        "ws" | "websocket" | "responses_websocket" => "responses_websocket".to_string(),
        "auto" => "auto".to_string(),
        other => other.to_string(),
    }
}

pub(super) fn elapsed_milliseconds(started_at: OffsetDateTime, now: OffsetDateTime) -> u64 {
    let milliseconds = (now - started_at).whole_milliseconds();
    u64::try_from(milliseconds).unwrap_or(0)
}

pub(super) fn format_timestamp(value: OffsetDateTime) -> String {
    value.format(&Rfc3339).unwrap_or_else(|_| value.to_string())
}

pub(super) fn normalize_models(raw: Value) -> FrameworkResult<Vec<ProviderModelDescriptor>> {
    serde_json::from_value(raw)
        .map_err(|error| PluginFrameworkError::invalid_provider_contract(error.to_string()))
}

pub(super) fn normalize_balance(raw: Value) -> FrameworkResult<ProviderBalanceResult> {
    serde_json::from_value(raw)
        .map_err(|error| PluginFrameworkError::invalid_provider_contract(error.to_string()))
}

pub(super) fn merge_models(
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
