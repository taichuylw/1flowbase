use std::collections::BTreeMap;

use orchestration_runtime::answer_presentation::{
    AnswerPresentationPlan, AnswerPresentationSegment,
};
use plugin_framework::provider_contract::ProviderStreamEvent;
use serde_json::Value;
use uuid::Uuid;

use crate::ports::RuntimeEventPayload;

use super::{debug_stream_events, DebugDeltaKind, ThinkTagStreamSplitter};

#[derive(Debug)]
pub(super) struct AnswerPresentationCursor {
    plan: AnswerPresentationPlan,
    next_segment_index: usize,
    emitted_text: BTreeMap<usize, String>,
    completed_outputs: BTreeMap<(String, String), CompletedOutput>,
}

#[derive(Debug, Clone)]
struct CompletedOutput {
    value: String,
    node_run_id: Option<Uuid>,
}

impl AnswerPresentationCursor {
    pub(super) fn from_plan(
        plan: &orchestration_runtime::compiled_plan::CompiledPlan,
    ) -> Option<Self> {
        AnswerPresentationPlan::from_plan(plan).map(|plan| Self {
            plan,
            next_segment_index: 0,
            emitted_text: BTreeMap::new(),
            completed_outputs: BTreeMap::new(),
        })
    }

    pub(super) fn push_provider_event(
        &mut self,
        source_node_id: &str,
        source_node_run_id: Uuid,
        event: &ProviderStreamEvent,
    ) -> Vec<RuntimeEventPayload> {
        let (reasoning, text) = match event {
            ProviderStreamEvent::ReasoningDelta { delta } => (true, delta.as_str()),
            ProviderStreamEvent::TextDelta { delta } => (false, delta.as_str()),
            _ => return Vec::new(),
        };

        self.push_delta(source_node_id, source_node_run_id, reasoning, text)
    }

    pub(super) fn complete_node(
        &mut self,
        node_id: &str,
        node_run_id: Uuid,
        output_payload: &Value,
    ) -> Vec<RuntimeEventPayload> {
        self.complete_node_with_run_id(node_id, Some(node_run_id), output_payload)
    }

    pub(super) fn complete_node_with_run_id(
        &mut self,
        node_id: &str,
        node_run_id: Option<Uuid>,
        output_payload: &Value,
    ) -> Vec<RuntimeEventPayload> {
        if let Some(output) = output_payload.as_object() {
            for segment in &self.plan.segments {
                let AnswerPresentationSegment::NodeOutput {
                    node_id: source_node_id,
                    output_key,
                } = segment
                else {
                    continue;
                };
                if source_node_id != node_id {
                    continue;
                }
                let Some(value) = output.get(output_key).and_then(Value::as_str) else {
                    continue;
                };
                self.completed_outputs.insert(
                    (source_node_id.clone(), output_key.clone()),
                    CompletedOutput {
                        value: value.to_string(),
                        node_run_id,
                    },
                );
            }
        }

        self.drain_ready_segments()
    }

    fn push_delta(
        &mut self,
        source_node_id: &str,
        source_node_run_id: Uuid,
        reasoning: bool,
        text: &str,
    ) -> Vec<RuntimeEventPayload> {
        let mut events = self.drain_ready_segments();
        if text.is_empty() {
            return events;
        }

        let Some((segment_index, output_key)) = self.current_node_output_segment(source_node_id)
        else {
            return events;
        };
        let output_key = output_key.to_string();
        if !reasoning {
            self.emitted_text
                .entry(segment_index)
                .or_default()
                .push_str(text);
        }

        events.push(self.answer_delta(
            segment_index,
            reasoning,
            text.to_string(),
            Some(source_node_id),
            Some(source_node_run_id),
            Some(&output_key),
        ));
        events
    }

    fn current_node_output_segment(&self, source_node_id: &str) -> Option<(usize, &str)> {
        let segment_index = self.next_segment_index;
        let segment = self.plan.segments.get(segment_index)?;
        match segment {
            AnswerPresentationSegment::NodeOutput {
                node_id,
                output_key,
            } if node_id == source_node_id => Some((segment_index, output_key.as_str())),
            _ => None,
        }
    }

    fn drain_ready_segments(&mut self) -> Vec<RuntimeEventPayload> {
        let mut events = Vec::new();

        while let Some(segment) = self.plan.segments.get(self.next_segment_index) {
            match segment {
                AnswerPresentationSegment::StaticText(text) => {
                    if !text.is_empty() {
                        events.push(self.answer_delta(
                            self.next_segment_index,
                            false,
                            text.clone(),
                            None,
                            None,
                            None,
                        ));
                    }
                    self.next_segment_index += 1;
                }
                AnswerPresentationSegment::NodeOutput {
                    node_id,
                    output_key,
                } => {
                    let key = (node_id.clone(), output_key.clone());
                    let Some(completed) = self.completed_outputs.get(&key).cloned() else {
                        break;
                    };
                    let segment_index = self.next_segment_index;
                    let already = self
                        .emitted_text
                        .get(&segment_index)
                        .map(String::as_str)
                        .unwrap_or("");
                    if already.is_empty() {
                        events.extend(self.answer_deltas_from_final_text(
                            segment_index,
                            &completed.value,
                            Some(node_id),
                            completed.node_run_id,
                            Some(output_key),
                        ));
                    } else {
                        let final_visible_text = visible_answer_text(&completed.value);
                        if let Some(suffix) = final_visible_text.strip_prefix(already) {
                            if !suffix.is_empty() {
                                events.push(self.answer_delta(
                                    segment_index,
                                    false,
                                    suffix.to_string(),
                                    Some(node_id),
                                    completed.node_run_id,
                                    Some(output_key),
                                ));
                            }
                        }
                    }
                    self.next_segment_index += 1;
                }
            }
        }

        events
    }

    fn answer_deltas_from_final_text(
        &self,
        segment_index: usize,
        text: &str,
        source_node_id: Option<&str>,
        source_node_run_id: Option<Uuid>,
        source_output_key: Option<&str>,
    ) -> Vec<RuntimeEventPayload> {
        let mut splitter = ThinkTagStreamSplitter::default();
        splitter
            .split(text)
            .into_iter()
            .chain(splitter.finish())
            .map(|part| {
                self.answer_delta(
                    segment_index,
                    part.kind == DebugDeltaKind::Reasoning,
                    part.text,
                    source_node_id,
                    source_node_run_id,
                    source_output_key,
                )
            })
            .collect()
    }

    fn answer_delta(
        &self,
        segment_index: usize,
        reasoning: bool,
        text: String,
        source_node_id: Option<&str>,
        source_node_run_id: Option<Uuid>,
        source_output_key: Option<&str>,
    ) -> RuntimeEventPayload {
        if reasoning {
            debug_stream_events::answer_reasoning_delta(
                &self.plan.answer_node_id,
                text,
                segment_index,
                source_node_id,
                source_node_run_id,
                source_output_key,
            )
        } else {
            debug_stream_events::answer_text_delta(
                &self.plan.answer_node_id,
                text,
                segment_index,
                source_node_id,
                source_node_run_id,
                source_output_key,
            )
        }
    }
}

fn visible_answer_text(text: &str) -> String {
    let mut splitter = ThinkTagStreamSplitter::default();
    splitter
        .split(text)
        .into_iter()
        .chain(splitter.finish())
        .filter(|part| part.kind == DebugDeltaKind::Text)
        .map(|part| part.text)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cursor_with_segments(segments: Vec<AnswerPresentationSegment>) -> AnswerPresentationCursor {
        AnswerPresentationCursor {
            plan: AnswerPresentationPlan {
                answer_node_id: "node-answer".to_string(),
                answer_output_key: "answer".to_string(),
                segments,
            },
            next_segment_index: 0,
            emitted_text: BTreeMap::new(),
            completed_outputs: BTreeMap::new(),
        }
    }

    #[test]
    fn leading_static_text_is_emitted_before_live_node_delta() {
        let mut cursor = cursor_with_segments(vec![
            AnswerPresentationSegment::StaticText("回答：".to_string()),
            AnswerPresentationSegment::NodeOutput {
                node_id: "node-llm".to_string(),
                output_key: "text".to_string(),
            },
        ]);
        let node_run_id = Uuid::now_v7();

        let events = cursor.push_provider_event(
            "node-llm",
            node_run_id,
            &ProviderStreamEvent::TextDelta {
                delta: "he".to_string(),
            },
        );

        let text_deltas = events
            .iter()
            .filter_map(|event| event.payload["text"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(text_deltas, vec!["回答：", "he"]);
    }

    #[test]
    fn final_suffix_uses_visible_text_when_completed_output_contains_think_tags() {
        let mut cursor = cursor_with_segments(vec![AnswerPresentationSegment::NodeOutput {
            node_id: "node-llm".to_string(),
            output_key: "text".to_string(),
        }]);
        let node_run_id = Uuid::now_v7();
        cursor.push_provider_event(
            "node-llm",
            node_run_id,
            &ProviderStreamEvent::TextDelta {
                delta: "he".to_string(),
            },
        );

        let events = cursor.complete_node(
            "node-llm",
            node_run_id,
            &json!({ "text": "<think>reason</think>hello" }),
        );

        let text_deltas = events
            .iter()
            .filter(|event| event.event_type == "text_delta")
            .filter_map(|event| event.payload["text"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(text_deltas, vec!["llo"]);
    }
}
