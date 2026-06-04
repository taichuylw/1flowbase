use plugin_framework::provider_contract::ProviderInvocationInput;
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SchedulerAdmissionMetadata {
    pub compatibility_mode: String,
    pub client_session_key: Option<String>,
    pub run_admission_key: String,
    pub stateless: bool,
}

pub fn derive_scheduler_admission_metadata(
    run: &domain::FlowRunRecord,
) -> SchedulerAdmissionMetadata {
    let compatibility_mode = normalized_compatibility_mode(run.compatibility_mode.as_deref());
    let client_session_key = derive_client_session_key(run, &compatibility_mode);
    let run_admission_key = client_session_key
        .as_ref()
        .map(|key| format!("run_admission:v1:{key}"))
        .unwrap_or_else(|| format!("run_admission:v1:run:{}", run.id));

    SchedulerAdmissionMetadata {
        compatibility_mode,
        stateless: client_session_key.is_none(),
        client_session_key,
        run_admission_key,
    }
}

pub fn derive_provider_pool_key(input: &ProviderInvocationInput) -> String {
    format!(
        "provider_pool:v1:provider_instance={}:provider_code={}:protocol={}:model={}",
        stable_component(&input.provider_instance_id),
        stable_component(&input.provider_code),
        stable_component(&input.protocol),
        stable_component(&input.model),
    )
}

fn derive_client_session_key(
    run: &domain::FlowRunRecord,
    compatibility_mode: &str,
) -> Option<String> {
    if let Some(external_user) = run
        .external_user
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        if client_protocol_uses_external_user(compatibility_mode) {
            return Some(format!(
                "client_session:v1:application={}:api_key={}:mode={}:external_user_fp={}",
                run.application_id,
                run.api_key_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "none".to_string()),
                stable_component(compatibility_mode),
                short_fingerprint(external_user),
            ));
        }
    }

    if compatibility_mode == "native" && !run.debug_session_id.trim().is_empty() {
        return Some(format!(
            "client_session:v1:application={}:run_mode={}:debug_session_fp={}:actor={}",
            run.application_id,
            run.run_mode.as_str(),
            short_fingerprint(&run.debug_session_id),
            run.created_by,
        ));
    }

    None
}

fn normalized_compatibility_mode(value: Option<&str>) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| "native".to_string())
}

fn client_protocol_uses_external_user(compatibility_mode: &str) -> bool {
    compatibility_mode.contains("anthropic")
        || compatibility_mode.contains("claude")
        || compatibility_mode.contains("responses")
        || compatibility_mode.contains("codex")
}

fn stable_component(value: &str) -> String {
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

fn short_fingerprint(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
