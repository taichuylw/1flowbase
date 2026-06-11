use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum VisibleInternalLlmToolExternalToolPolicy {
    #[default]
    Forbidden,
    Inherited,
}

impl VisibleInternalLlmToolExternalToolPolicy {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Forbidden => EXTERNAL_TOOL_POLICY_FORBIDDEN,
            Self::Inherited => EXTERNAL_TOOL_POLICY_INHERITED,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct VisibleInternalLlmTool {
    pub(super) name: String,
    pub(super) description: Option<String>,
    pub(super) target_node_id: String,
    pub(super) input_schema: Option<Value>,
    pub(super) external_tool_policy: VisibleInternalLlmToolExternalToolPolicy,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct VisibleInternalLlmToolOutput {
    pub(super) text: String,
    pub(super) provider_events: Vec<ProviderStreamEvent>,
    pub(super) route_events: Vec<Value>,
}

pub(super) enum VisibleInternalLlmToolBranchExecution {
    Completed(VisibleInternalLlmToolOutput),
    Waiting {
        wait: Box<LlmToolCallbackWait>,
        branch_text: String,
        route_events: Vec<Value>,
    },
    Failed {
        error_payload: Value,
        route_events: Vec<Value>,
    },
}

pub(super) enum VisibleInternalLlmToolNodeExecution {
    Completed(Value),
    Waiting(Box<LlmToolCallbackWait>),
    Failed(Value),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct VisibleInternalLlmToolPendingCall {
    pub(super) tool_call: Value,
    pub(super) tool: VisibleInternalLlmTool,
}

pub(super) enum VisibleInternalLlmToolRemainingExecution {
    Completed {
        tool_results: Vec<Value>,
        visible_transcript: String,
        provider_events: Vec<ProviderStreamEvent>,
        route_events: Vec<Value>,
    },
    Waiting(Box<LlmToolCallbackWait>),
    Failed {
        error_payload: Value,
        provider_events: Vec<ProviderStreamEvent>,
        route_events: Vec<Value>,
    },
}
