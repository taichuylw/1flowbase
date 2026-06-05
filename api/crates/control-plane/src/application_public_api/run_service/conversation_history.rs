use serde_json::{json, Value};

use super::super::conversations::ApplicationPublicConversationMessageRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ApplicationPublicConversationTurn {
    user_content: String,
    assistant_parts: Vec<String>,
}

pub(super) fn application_public_conversation_messages_to_native_history(
    messages: Vec<ApplicationPublicConversationMessageRecord>,
) -> Vec<Value> {
    let mut turns = Vec::<ApplicationPublicConversationTurn>::new();

    for message in messages {
        match message.role.as_str() {
            "user" => {
                if let Some(user_content) = normalize_conversation_user_content(&message.content) {
                    turns.push(ApplicationPublicConversationTurn {
                        user_content,
                        assistant_parts: Vec::new(),
                    });
                }
            }
            "assistant" => {
                let Some(turn) = turns.last_mut() else {
                    continue;
                };
                if let Some(assistant_content) =
                    normalize_conversation_assistant_content(&message.content)
                {
                    turn.assistant_parts.push(assistant_content);
                }
            }
            _ => {}
        }
    }

    let mut deduped = Vec::<ApplicationPublicConversationTurn>::new();
    for turn in turns {
        if deduped
            .last()
            .is_some_and(|last| last.user_content == turn.user_content)
        {
            if let Some(last) = deduped.last_mut() {
                *last = turn;
            }
            continue;
        }
        deduped.push(turn);
    }

    let mut history = Vec::new();
    for turn in deduped {
        history.push(json!({
            "role": "user",
            "content": turn.user_content,
        }));
        let assistant_content = turn
            .assistant_parts
            .into_iter()
            .filter_map(|content| trimmed_history_text(&content))
            .collect::<Vec<_>>()
            .join("\n\n");
        if let Some(assistant_content) = trimmed_history_text(&assistant_content) {
            history.push(json!({
                "role": "assistant",
                "content": assistant_content,
            }));
        }
    }

    history
}

fn normalize_conversation_user_content(content: &str) -> Option<String> {
    trimmed_history_text(&strip_tag_blocks(content, "system-reminder"))
}

fn normalize_conversation_assistant_content(content: &str) -> Option<String> {
    let without_thinking = strip_tag_blocks(content, "think");
    let without_tool_calls = strip_tag_blocks(&without_thinking, "tool_call");
    let visible_content =
        content_after_beautified_marker(&without_tool_calls).unwrap_or(without_tool_calls.as_str());
    trimmed_history_text(visible_content)
}

fn strip_tag_blocks(content: &str, tag: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut output = content.to_string();

    while let Some(start) = output.find(&open) {
        let search_start = start + open.len();
        let Some(end) = output[search_start..].find(&close) else {
            break;
        };
        let end = search_start + end + close.len();
        output.replace_range(start..end, "");
    }

    output
}

fn content_after_beautified_marker(content: &str) -> Option<&str> {
    let marker = "下面是美化后内容";
    let marker_start = content.find(marker)?;
    Some(
        content[marker_start + marker.len()..].trim_start_matches(|value: char| {
            value.is_whitespace() || value == '-' || value == '—'
        }),
    )
}

fn trimmed_history_text(content: &str) -> Option<String> {
    let trimmed = content.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}
