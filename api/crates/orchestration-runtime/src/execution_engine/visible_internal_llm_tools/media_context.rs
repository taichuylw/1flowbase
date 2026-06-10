use super::*;

pub(super) fn visible_internal_llm_tool_llm_resolved_inputs(
    resolved_inputs: &Map<String, Value>,
    variable_pool: &Map<String, Value>,
) -> Map<String, Value> {
    let mut inputs = resolved_inputs.clone();
    let Some(context) = variable_pool
        .get(VISIBLE_INTERNAL_LLM_TOOL_VARIABLE)
        .and_then(|value| value.get("context"))
        .and_then(Value::as_object)
    else {
        return inputs;
    };

    if !inputs.contains_key("history") {
        if let Some(history) = context
            .get("history")
            .and_then(Value::as_array)
            .filter(|history| !history.is_empty())
        {
            inputs.insert("history".to_string(), Value::Array(history.clone()));
        }
    }
    if !inputs.contains_key("files") {
        if let Some(files) = context
            .get("files")
            .and_then(Value::as_array)
            .filter(|files| !files.is_empty())
        {
            inputs.insert("files".to_string(), Value::Array(files.clone()));
        }
    }
    let inherited_tools = (!inputs.contains_key("tools")
        && !visible_internal_llm_tool_has_media_argument(variable_pool))
    .then(|| {
        context
            .get("tools")
            .and_then(Value::as_array)
            .filter(|tools| !tools.is_empty())
    })
    .flatten();
    if let Some(tools) = inherited_tools {
        inputs.insert("tools".to_string(), Value::Array(tools.clone()));
    }

    inputs
}

pub(in crate::execution_engine) async fn inject_visible_internal_llm_tool_media_content_blocks(
    input: &mut ProviderInvocationInput,
    variable_pool: &Map<String, Value>,
) {
    let media_items = visible_internal_llm_tool_media_argument(variable_pool);
    if media_items.is_empty() {
        return;
    }

    let mut injected_blocks = Vec::new();
    for media in media_items {
        if let Some(block) = image_content_block_from_workspace_media(&media).await {
            injected_blocks.push(block);
        }
    }
    if injected_blocks.is_empty() {
        return;
    }

    let Some(message) = input
        .messages
        .iter_mut()
        .rev()
        .find(|message| message.role == ProviderMessageRole::User)
    else {
        return;
    };

    let mut content_blocks = message
        .content_blocks
        .take()
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_else(|| {
            let content = message.content.trim();
            if content.is_empty() {
                Vec::new()
            } else {
                vec![json!({ "type": "text", "text": content })]
            }
        });
    content_blocks.extend(injected_blocks);
    message.content_blocks = Some(Value::Array(content_blocks));
}

pub(in crate::execution_engine) fn visible_internal_llm_tool_has_media_argument(
    variable_pool: &Map<String, Value>,
) -> bool {
    !visible_internal_llm_tool_media_argument(variable_pool).is_empty()
}

fn visible_internal_llm_tool_media_argument(variable_pool: &Map<String, Value>) -> Vec<Value> {
    variable_pool
        .get(VISIBLE_INTERNAL_LLM_TOOL_VARIABLE)
        .and_then(|value| value.get("arguments"))
        .and_then(|arguments| arguments.get("media"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

async fn image_content_block_from_workspace_media(media: &Value) -> Option<Value> {
    if media.get("kind").and_then(Value::as_str) != Some("image")
        || media.get("source").and_then(Value::as_str) != Some("workspace_path")
    {
        return None;
    }
    let raw_path = media.get("path").and_then(Value::as_str)?.trim();
    if raw_path.is_empty() || !looks_like_image_path(raw_path) {
        return None;
    }
    let resolved_path = resolve_workspace_media_path(raw_path).await?;
    let bytes = tokio::fs::read(&resolved_path).await.ok()?;
    let media_type = image_media_type_from_path(&resolved_path)?;
    use base64::Engine as _;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(json!({
        "type": "image_url",
        "image_url": {
            "url": format!("data:{media_type};base64,{encoded}")
        }
    }))
}

async fn resolve_workspace_media_path(raw_path: &str) -> Option<std::path::PathBuf> {
    let normalized = raw_path.replace('\\', "/");
    let requested_path = std::path::Path::new(&normalized);
    if !workspace_media_path_shape_allowed(requested_path) {
        return None;
    }

    let current_dir = std::env::current_dir().ok()?;
    let mut roots = vec![current_dir.clone()];
    if let Some(parent) = current_dir.parent() {
        roots.push(parent.to_path_buf());
    }
    let canonical_roots = roots
        .into_iter()
        .filter_map(|root| std::fs::canonicalize(root).ok())
        .collect::<Vec<_>>();

    if requested_path.is_absolute() {
        let canonical = tokio::fs::canonicalize(requested_path).await.ok()?;
        return canonical_roots
            .iter()
            .any(|root| canonical.starts_with(root))
            .then_some(canonical);
    }

    for root in canonical_roots {
        let candidate = root.join(requested_path);
        let Ok(canonical) = tokio::fs::canonicalize(candidate).await else {
            continue;
        };
        if canonical.starts_with(&root) {
            return Some(canonical);
        }
    }
    None
}

fn workspace_media_path_shape_allowed(path: &std::path::Path) -> bool {
    path.components().all(|component| {
        matches!(
            component,
            std::path::Component::Normal(_)
                | std::path::Component::CurDir
                | std::path::Component::RootDir
        )
    })
}

fn looks_like_image_path(path: &str) -> bool {
    let path = path.trim().to_ascii_lowercase();
    matches!(
        std::path::Path::new(&path)
            .extension()
            .and_then(|extension| extension.to_str()),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp")
    )
}

fn image_media_type_from_path(path: &std::path::Path) -> Option<String> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("png") => Some("image/png".to_string()),
        Some("jpg" | "jpeg") => Some("image/jpeg".to_string()),
        Some("gif") => Some("image/gif".to_string()),
        Some("webp") => Some("image/webp".to_string()),
        Some("bmp") => Some("image/bmp".to_string()),
        _ => None,
    }
}

pub(super) fn visible_internal_llm_tool_inherited_context(
    variable_pool: &Map<String, Value>,
    main_node_id: &str,
) -> Value {
    let history = inherited_main_llm_history(variable_pool, main_node_id)
        .or_else(|| synthesized_run_context_history(variable_pool))
        .unwrap_or_default();
    json!({
        "history": history,
        "query": find_run_context_value(variable_pool, "query").unwrap_or(Value::Null),
        "files": find_run_context_array(variable_pool, "files"),
        "tools": find_run_context_array(variable_pool, "tools"),
    })
}

fn inherited_main_llm_history(
    variable_pool: &Map<String, Value>,
    main_node_id: &str,
) -> Option<Vec<Value>> {
    let mut history = variable_pool
        .get(main_node_id)?
        .get(LLM_TOOL_CALLBACK_STATE_KEY)?
        .get("history")?
        .as_array()?
        .clone();
    if history
        .last()
        .and_then(Value::as_object)
        .is_some_and(|message| {
            message.get("role").and_then(Value::as_str) == Some("assistant")
                && message
                    .get("tool_calls")
                    .and_then(Value::as_array)
                    .is_some_and(|tool_calls| !tool_calls.is_empty())
        })
    {
        history.pop();
    }
    (!history.is_empty()).then_some(history)
}

fn synthesized_run_context_history(variable_pool: &Map<String, Value>) -> Option<Vec<Value>> {
    let mut content_parts = Vec::new();
    if let Some(query) =
        find_run_context_value(variable_pool, "query").and_then(|value| value_to_text(&value))
    {
        if !query.trim().is_empty() {
            content_parts.push(query);
        }
    }
    let files = find_run_context_array(variable_pool, "files");
    if !files.is_empty() {
        content_parts.push(format!("Files: {}", Value::Array(files)));
    }

    let content = content_parts.join("\n\n");
    (!content.trim().is_empty()).then(|| {
        vec![json!({
            "role": "user",
            "content": content,
        })]
    })
}

fn find_run_context_value(variable_pool: &Map<String, Value>, key: &str) -> Option<Value> {
    variable_pool
        .get("node-start")
        .and_then(|payload| payload.get(key))
        .cloned()
        .or_else(|| {
            variable_pool
                .values()
                .find_map(|payload| payload.as_object()?.get(key).cloned())
        })
}

fn find_run_context_array(variable_pool: &Map<String, Value>, key: &str) -> Vec<Value> {
    find_run_context_value(variable_pool, key)
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
}
