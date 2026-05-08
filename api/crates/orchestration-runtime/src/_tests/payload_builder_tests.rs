use orchestration_runtime::{
    compiled_plan::CompiledOutput,
    payload_builder::{PublicOutputContract, RawNodeExecutionResult},
};
use serde_json::{json, Map, Value};

fn output(key: &str) -> CompiledOutput {
    CompiledOutput {
        key: key.to_string(),
        title: key.to_string(),
        value_type: "string".to_string(),
    }
}

fn object(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Map<String, Value> {
    entries
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect()
}

#[test]
fn payload_builder_rejects_unknown_public_output_keys() {
    let contract = PublicOutputContract::from_compiled_outputs(&[output("text")]).unwrap();
    let raw = RawNodeExecutionResult {
        executor_output: object([
            ("text", json!("accepted")),
            ("unexpected", json!("must fail")),
        ]),
        metrics_facts: Map::new(),
        error_facts: Map::new(),
        debug_facts: Map::new(),
        provider_events: Vec::new(),
    };

    let error = contract
        .build_node_payloads(raw)
        .expect_err("unknown public output key must fail the build");

    assert!(error.to_string().contains("unexpected"));
}

#[test]
fn payload_builder_allows_unknown_keys_when_structured_expansion_is_explicit() {
    let contract = PublicOutputContract::from_compiled_outputs(&[output("text")])
        .unwrap()
        .with_structured_expansion(true);
    let raw = RawNodeExecutionResult {
        executor_output: object([
            ("text", json!("accepted")),
            ("dynamic_field", json!("expanded")),
        ]),
        metrics_facts: Map::new(),
        error_facts: Map::new(),
        debug_facts: Map::new(),
        provider_events: Vec::new(),
    };

    let built = contract.build_node_payloads(raw).unwrap();

    assert_eq!(
        built.output_payload,
        json!({ "dynamic_field": "expanded", "text": "accepted" })
    );
}

#[test]
fn payload_builder_keeps_reserved_keys_out_of_structured_expansion() {
    let contract = PublicOutputContract::from_compiled_outputs(&[output("text")])
        .unwrap()
        .with_structured_expansion(true);
    let raw = RawNodeExecutionResult {
        executor_output: object([
            ("text", json!("accepted")),
            ("metadata", json!({ "provider": "internal" })),
            ("provider_code", json!("openai_compatible")),
            ("queue_snapshot_id", json!("queue-snapshot-1")),
            ("raw_response_ref", json!("artifact-1")),
        ]),
        metrics_facts: Map::new(),
        error_facts: Map::new(),
        debug_facts: Map::new(),
        provider_events: Vec::new(),
    };

    let built = contract.build_node_payloads(raw).unwrap();

    assert_eq!(built.output_payload, json!({ "text": "accepted" }));
    assert_eq!(
        built.metrics_payload,
        json!({
            "provider_code": "openai_compatible",
            "queue_snapshot_id": "queue-snapshot-1"
        })
    );
    assert_eq!(
        built.debug_payload,
        json!({
            "metadata": { "provider": "internal" },
            "raw_response_ref": "artifact-1"
        })
    );
}

#[test]
fn payload_builder_allows_context_keys_across_non_public_buckets() {
    let contract = PublicOutputContract::from_compiled_outputs(&[output("text")]).unwrap();
    let raw = RawNodeExecutionResult {
        executor_output: object([("text", json!("visible output"))]),
        metrics_facts: object([("provider_code", json!("openai_compatible"))]),
        error_facts: object([
            ("provider_code", json!("openai_compatible")),
            ("message", json!("provider failed")),
        ]),
        debug_facts: object([("provider_code", json!("openai_compatible"))]),
        provider_events: Vec::new(),
    };

    let built = contract.build_node_payloads(raw).unwrap();

    assert_eq!(built.output_payload, json!({ "text": "visible output" }));
    assert_eq!(
        built.metrics_payload,
        json!({ "provider_code": "openai_compatible" })
    );
    assert_eq!(
        built.error_payload,
        json!({
            "message": "provider failed",
            "provider_code": "openai_compatible"
        })
    );
    assert_eq!(
        built.debug_payload,
        json!({ "provider_code": "openai_compatible" })
    );
}

#[test]
fn payload_builder_keeps_payload_buckets_mutually_exclusive() {
    let contract = PublicOutputContract::from_compiled_outputs(&[output("text")]).unwrap();
    let raw = RawNodeExecutionResult {
        executor_output: object([
            ("text", json!("visible output")),
            ("usage", json!({ "input_tokens": 3, "output_tokens": 5 })),
            ("error", json!({ "message": "provider failed" })),
            ("__raw_response", json!({ "id": "debug-1" })),
        ]),
        metrics_facts: object([("latency_ms", json!(42))]),
        error_facts: object([("retryable", json!(false))]),
        debug_facts: object([("trace_id", json!("trace-1"))]),
        provider_events: Vec::new(),
    };

    let built = contract.build_node_payloads(raw).unwrap();

    assert_eq!(built.output_payload, json!({ "text": "visible output" }));
    assert_eq!(
        built.metrics_payload,
        json!({
            "latency_ms": 42,
            "usage": { "input_tokens": 3, "output_tokens": 5 }
        })
    );
    assert_eq!(
        built.error_payload,
        json!({
            "error": { "message": "provider failed" },
            "retryable": false
        })
    );
    assert_eq!(
        built.debug_payload,
        json!({
            "__raw_response": { "id": "debug-1" },
            "trace_id": "trace-1"
        })
    );

    let output_payload = built.output_payload.as_object().unwrap();
    let metrics_payload = built.metrics_payload.as_object().unwrap();
    let error_payload = built.error_payload.as_object().unwrap();
    let debug_payload = built.debug_payload.as_object().unwrap();
    let bucket_keys: [(&str, &Map<String, Value>); 3] = [
        ("metrics", metrics_payload),
        ("error", error_payload),
        ("debug", debug_payload),
    ];

    for key in output_payload.keys() {
        for (bucket_name, bucket) in &bucket_keys {
            assert!(
                !bucket.contains_key(key),
                "public output key {key} leaked into {bucket_name} payload"
            );
        }
    }
}

#[test]
fn payload_builder_rejects_reserved_declared_public_outputs() {
    let error = PublicOutputContract::from_compiled_outputs(&[output("provider_code")])
        .expect_err("reserved output contract keys must be rejected");

    assert!(error.to_string().contains("provider_code"));
}
