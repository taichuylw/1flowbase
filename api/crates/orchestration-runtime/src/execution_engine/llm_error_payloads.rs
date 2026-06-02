use super::*;

pub(super) fn build_empty_prompt_messages_error_payload(runtime: &CompiledLlmRuntime) -> Value {
    json!({
        "provider_instance_id": runtime.provider_instance_id,
        "provider_code": runtime.provider_code,
        "protocol": runtime.protocol,
        "error_code": "prompt_messages_empty",
        "message": "LLM node requires at least one non-empty user or assistant prompt message",
    })
}

pub(super) fn build_binding_resolution_error_payload(error: &anyhow::Error) -> Value {
    let message = error.to_string();
    let error_code = if message.contains("unresolved template selector") {
        "prompt_template_unresolved"
    } else {
        "binding_resolution_failed"
    };

    json!({
        "error_code": error_code,
        "message": message,
    })
}

pub(super) fn build_answer_binding_resolution_error_payload(
    node: &CompiledNode,
    issues: &[BindingResolutionIssue],
) -> Value {
    let error_code = if issues
        .iter()
        .any(|issue| issue.selector.is_some() || issue.message.contains("selector"))
    {
        "prompt_template_unresolved"
    } else {
        "binding_resolution_failed"
    };
    let message = if issues.len() == 1 {
        let issue = &issues[0];
        format!(
            "failed to resolve binding {} for {}: {}",
            issue.binding_key, node.node_id, issue.message
        )
    } else {
        format!(
            "failed to resolve {} bindings for {}",
            issues.len(),
            node.node_id
        )
    };
    let details = issues
        .iter()
        .map(|issue| {
            json!({
                "binding_key": issue.binding_key,
                "selector": issue.selector.as_ref().map(|selector| selector.join(".")),
                "selector_path": issue.selector,
                "message": issue.message,
            })
        })
        .collect::<Vec<_>>();

    json!({
        "error_code": error_code,
        "message": message,
        "details": details,
    })
}
