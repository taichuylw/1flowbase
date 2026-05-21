use uuid::Uuid;

pub(crate) const OPENAI_CALLBACK_TOOL_CALL_PREFIX: &str = "calltask_";
pub(crate) const ANTHROPIC_CALLBACK_TOOL_USE_PREFIX: &str = "toolu_task_";

pub(crate) fn encode_openai_callback_tool_call_id(
    callback_task_id: Uuid,
    original_tool_call_id: &str,
) -> String {
    format!(
        "{OPENAI_CALLBACK_TOOL_CALL_PREFIX}{}_{}",
        callback_task_id.simple(),
        original_tool_call_id
    )
}

pub(crate) fn decode_openai_callback_tool_call_id(value: &str) -> Option<(Uuid, String)> {
    let rest = value.strip_prefix(OPENAI_CALLBACK_TOOL_CALL_PREFIX)?;
    decode_callback_bound_id(rest)
}

pub(crate) fn encode_anthropic_callback_tool_use_id(
    callback_task_id: Uuid,
    original_tool_use_id: &str,
) -> String {
    format!(
        "{ANTHROPIC_CALLBACK_TOOL_USE_PREFIX}{}_{}",
        callback_task_id.simple(),
        original_tool_use_id
    )
}

pub(crate) fn decode_anthropic_callback_tool_use_id(value: &str) -> Option<(Uuid, String)> {
    let rest = value.strip_prefix(ANTHROPIC_CALLBACK_TOOL_USE_PREFIX)?;
    decode_callback_bound_id(rest)
}

fn decode_callback_bound_id(value: &str) -> Option<(Uuid, String)> {
    let (callback_task_id, original_id) = value.split_once('_')?;
    if callback_task_id.len() != 32 || original_id.is_empty() {
        return None;
    }
    Some((
        Uuid::parse_str(callback_task_id).ok()?,
        original_id.to_string(),
    ))
}
