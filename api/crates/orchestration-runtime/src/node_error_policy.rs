use serde_json::Value;

use crate::compiled_plan::CompiledNode;

pub const ERROR_BRANCH_SOURCE_HANDLE: &str = "error";

const ERROR_POLICY_CONFIG_KEY: &str = "error_policy";
const ERROR_DEFAULT_OUTPUT_CONFIG_KEY: &str = "error_default_output";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeErrorPolicy {
    None,
    DefaultValue,
    ErrorBranch,
}

pub fn node_error_policy(node: &CompiledNode) -> NodeErrorPolicy {
    match node
        .config
        .get(ERROR_POLICY_CONFIG_KEY)
        .and_then(Value::as_str)
    {
        Some("default_value") => NodeErrorPolicy::DefaultValue,
        Some("error_branch") => NodeErrorPolicy::ErrorBranch,
        _ => NodeErrorPolicy::None,
    }
}

pub fn node_supports_error_policy(node: &CompiledNode) -> bool {
    node.node_type != "start"
}

pub fn node_uses_error_branch(node: &CompiledNode) -> bool {
    node_supports_error_policy(node) && node_error_policy(node) == NodeErrorPolicy::ErrorBranch
}

pub fn error_default_output(node: &CompiledNode) -> Option<Value> {
    node.config.get(ERROR_DEFAULT_OUTPUT_CONFIG_KEY).cloned()
}
