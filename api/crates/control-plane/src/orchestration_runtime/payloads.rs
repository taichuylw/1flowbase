use serde_json::Value;

pub(super) fn persisted_node_output_payload(
    output_payload: &Value,
    _metrics_payload: &Value,
    _error_payload: Option<&Value>,
    _debug_payload: &Value,
) -> Value {
    output_payload.clone()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::persisted_node_output_payload;

    #[test]
    fn persisted_output_preserves_executor_output_fields_even_when_payload_names_overlap() {
        let output_payload = json!({
            "text": "<think>先分析</think>正式回答",
            "finish_reason": "stop",
            "provider_route": { "provider_code": "openai_compatible" },
            "response_id": "resp_1",
            "provider_metadata": { "raw_id": "chatcmpl-1" },
        });
        let metrics_payload = json!({
            "finish_reason": "stop",
            "provider_code": "openai_compatible",
        });
        let debug_payload = json!({
            "provider_events": [{ "type": "text_delta", "delta": "正式回答" }],
        });

        let persisted =
            persisted_node_output_payload(&output_payload, &metrics_payload, None, &debug_payload);

        assert_eq!(
            persisted,
            json!({
                "text": "<think>先分析</think>正式回答",
                "finish_reason": "stop",
                "provider_route": { "provider_code": "openai_compatible" },
                "response_id": "resp_1",
                "provider_metadata": { "raw_id": "chatcmpl-1" },
            })
        );
    }
}
