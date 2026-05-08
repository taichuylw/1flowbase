use crate::{default_flow_document, FLOW_SCHEMA_VERSION};
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
