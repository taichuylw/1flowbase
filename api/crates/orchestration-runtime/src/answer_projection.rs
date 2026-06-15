use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const ANSWER_SEGMENTS_KEY: &str = "answer_segments";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerProjectionSegment {
    pub kind: AnswerProjectionSegmentKind,
    pub text: String,
}

impl AnswerProjectionSegment {
    pub fn reasoning(text: impl Into<String>) -> Self {
        Self {
            kind: AnswerProjectionSegmentKind::Reasoning,
            text: text.into(),
        }
    }

    pub fn message(text: impl Into<String>) -> Self {
        Self {
            kind: AnswerProjectionSegmentKind::Message,
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerProjectionSegmentKind {
    Reasoning,
    Message,
}

pub fn answer_segments_from_text(text: &str) -> Vec<AnswerProjectionSegment> {
    let mut remaining = text;
    let mut inside_think = false;
    let mut segments = Vec::new();

    while !remaining.is_empty() {
        let tag = if inside_think { "</think>" } else { "<think>" };
        let Some(tag_index) = remaining.find(tag) else {
            push_answer_segment(&mut segments, inside_think, remaining);
            break;
        };

        push_answer_segment(&mut segments, inside_think, &remaining[..tag_index]);
        remaining = &remaining[tag_index + tag.len()..];
        inside_think = !inside_think;
    }

    segments
}

pub fn answer_segments_value_from_text(text: &str) -> Option<Value> {
    answer_segments_value(&answer_segments_from_text(text))
}

pub fn answer_segments_from_value(value: &Value) -> Vec<AnswerProjectionSegment> {
    let Some(items) = value.as_array() else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(|item| {
            let kind = match item.get("kind").and_then(Value::as_str) {
                Some("reasoning") => AnswerProjectionSegmentKind::Reasoning,
                Some("message") => AnswerProjectionSegmentKind::Message,
                _ => return None,
            };
            let text = item
                .get("text")
                .and_then(Value::as_str)
                .filter(|text| !text.is_empty())?
                .to_string();
            Some(AnswerProjectionSegment { kind, text })
        })
        .collect()
}

pub fn answer_segments_value(segments: &[AnswerProjectionSegment]) -> Option<Value> {
    if segments.is_empty() {
        return None;
    }
    serde_json::to_value(segments).ok()
}

fn push_answer_segment(segments: &mut Vec<AnswerProjectionSegment>, reasoning: bool, text: &str) {
    if text.is_empty() {
        return;
    }
    let kind = if reasoning {
        AnswerProjectionSegmentKind::Reasoning
    } else {
        AnswerProjectionSegmentKind::Message
    };

    if let Some(previous) = segments.last_mut().filter(|segment| segment.kind == kind) {
        previous.text.push_str(text);
        return;
    }

    segments.push(AnswerProjectionSegment {
        kind,
        text: text.to_string(),
    });
}
