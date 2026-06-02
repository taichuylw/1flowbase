use super::*;

mod input_cache_usage_tests {
    use super::*;

    #[test]
    fn usage_delta_accumulates_input_cache_hit_and_miss_tokens() {
        let mut usage = ProviderUsage {
            input_tokens: Some(100),
            input_cache_hit_tokens: Some(40),
            input_cache_miss_tokens: Some(60),
            output_tokens: Some(12),
            total_tokens: Some(112),
            ..ProviderUsage::default()
        };

        apply_usage_delta(
            &mut usage,
            &ProviderUsage {
                input_cache_hit_tokens: Some(5),
                input_cache_miss_tokens: Some(7),
                ..ProviderUsage::default()
            },
        );

        assert_eq!(usage.input_cache_hit_tokens, Some(45));
        assert_eq!(usage.input_cache_miss_tokens, Some(67));
        assert_eq!(usage.total_tokens(), Some(112));
    }
}

#[cfg(test)]
mod llm_round_timeline_tests {
    use super::*;

    #[test]
    fn llm_round_timeline_keeps_result_context_usage_without_token_delta() {
        let invocation_messages = vec![
            json!({
                "role": "assistant",
                "content": "need weather",
                "usage": {
                    "total_tokens": 8122
                },
                "tool_calls": [
                    {
                        "id": "call_weather",
                        "name": "lookup_weather"
                    }
                ]
            }),
            json!({
                "role": "tool",
                "tool_call_id": "call_weather",
                "content": "{\"temperature\":21}"
            }),
        ];
        let result = ProviderInvocationResult {
            final_content: Some("continue".into()),
            usage: ProviderUsage {
                total_tokens: Some(8224),
                ..ProviderUsage::default()
            },
            ..ProviderInvocationResult::default()
        };
        let result_usage = json!({
            "total_tokens": 8224
        });

        let rounds =
            build_llm_round_timeline(&invocation_messages, Some(&result), Some(&result_usage));

        assert_eq!(
            rounds[0]["tool_results"][0]["result_context_usage"]["total_tokens"],
            json!(8224)
        );
        assert!(rounds[0]["tool_results"][0].get("token_delta").is_none());
    }
}
