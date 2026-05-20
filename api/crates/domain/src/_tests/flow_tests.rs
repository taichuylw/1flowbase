use crate::{
    default_flow_document, DefaultUpgradePolicy, DEFAULT_AUTO_INCLUDE_NEW_PROVIDER_INSTANCES,
    DEFAULT_CODE_ISOLATION_TIMEOUT_MS, FLOW_AUTOSAVE_INTERVAL_SECONDS, FLOW_SCHEMA_VERSION,
};
use serde_json::json;
use uuid::Uuid;

#[test]
fn default_flow_document_uses_v2_prompt_messages_contract() {
    let document = default_flow_document(Uuid::now_v7());

    assert_eq!(document["schemaVersion"], json!(FLOW_SCHEMA_VERSION));
    assert_eq!(FLOW_SCHEMA_VERSION, "1flowbase.flow/v2");
    assert!(document["graph"]["nodes"][1]["bindings"]["prompt_messages"].is_object());
    assert!(document["graph"]["nodes"][1]["bindings"]["user_prompt"].is_null());
}

#[test]
fn default_flow_document_start_config_includes_input_fields_and_model_list() {
    let document = default_flow_document(Uuid::now_v7());
    let start_config = &document["graph"]["nodes"][0]["config"];

    assert_eq!(start_config["input_fields"], json!([]));
    assert_eq!(start_config["model_list"], json!([]));
}

#[test]
fn default_flow_document_llm_outputs_match_frontend_contract() {
    let document = default_flow_document(Uuid::now_v7());

    assert_eq!(
        document["graph"]["nodes"][1]["outputs"],
        json!([
            { "key": "text", "title": "模型输出", "valueType": "string" },
            { "key": "usage", "title": "用量", "valueType": "json" }
        ])
    );
}

#[test]
fn default_upgrade_policy_strings_are_stable() {
    assert_eq!(DefaultUpgradePolicy::CreateOnly.as_str(), "create_only");
    assert_eq!(
        DefaultUpgradePolicy::SystemMaintainedAutoUpdate.as_str(),
        "system_maintained_auto_update"
    );
    assert_eq!(
        DefaultUpgradePolicy::UserConfigurablePreserveHistory.as_str(),
        "user_configurable_preserve_history"
    );
    assert_eq!(
        DefaultUpgradePolicy::ExplicitMigrationOnly.as_str(),
        "explicit_migration_only"
    );
}

#[test]
fn simple_system_default_constants_are_stable() {
    let defaults = (
        FLOW_AUTOSAVE_INTERVAL_SECONDS,
        DEFAULT_AUTO_INCLUDE_NEW_PROVIDER_INSTANCES,
        DEFAULT_CODE_ISOLATION_TIMEOUT_MS,
    );

    assert_eq!(defaults, (30, true, 100));
}
