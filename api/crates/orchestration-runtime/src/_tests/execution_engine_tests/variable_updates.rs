use super::*;

#[tokio::test]
async fn variable_assigner_updates_env_for_current_run_only() {
    let mut plan = base_plan();

    plan.topological_order = vec![
        "node-start".to_string(),
        "node-env-update".to_string(),
        "node-answer".to_string(),
    ];
    plan.edges = vec![
        CompiledEdge {
            edge_id: "edge-start-env-update".to_string(),
            source: "node-start".to_string(),
            target: "node-env-update".to_string(),
            source_handle: None,
            target_handle: None,
        },
        CompiledEdge {
            edge_id: "edge-env-update-answer".to_string(),
            source: "node-env-update".to_string(),
            target: "node-answer".to_string(),
            source_handle: None,
            target_handle: None,
        },
    ];

    let start = plan
        .nodes
        .get_mut("node-start")
        .expect("start node should exist");
    start.downstream_node_ids = vec!["node-env-update".to_string()];

    plan.nodes.insert(
        "node-env-update".to_string(),
        CompiledNode {
            node_id: "node-env-update".to_string(),
            node_type: "variable_assigner".to_string(),
            alias: "Environment Variable Update".to_string(),
            container_id: None,
            dependency_node_ids: vec!["node-start".to_string()],
            downstream_node_ids: vec!["node-answer".to_string()],
            bindings: BTreeMap::from([(
                "operations".to_string(),
                CompiledBinding {
                    kind: "state_write".to_string(),
                    selector_paths: vec![vec!["node-start".to_string(), "query".to_string()]],
                    raw_value: json!([
                        {
                            "path": ["env", "ApiBaseUrl"],
                            "operator": "set",
                            "source": ["node-start", "query"]
                        }
                    ]),
                },
            )]),
            outputs: vec![CompiledOutput {
                key: "env".to_string(),
                title: "Environment Variables".to_string(),
                value_type: "json".to_string(),
                selector: Vec::new(),
                json_schema: None,
            }],
            config: json!({}),
            plugin_runtime: None,
            llm_runtime: None,
            code_runtime: None,
        },
    );

    let answer = plan
        .nodes
        .get_mut("node-answer")
        .expect("answer node should exist");
    answer.dependency_node_ids = vec!["node-env-update".to_string()];
    answer.bindings = BTreeMap::from([(
        "answer_template".to_string(),
        CompiledBinding {
            kind: "templated_text".to_string(),
            selector_paths: vec![vec!["env".to_string(), "ApiBaseUrl".to_string()]],
            raw_value: json!("{{env.ApiBaseUrl}}"),
        },
    )]);

    let outcome = start_flow_debug_run(
        &plan,
        &json!({
            "env": {
                "ApiBaseUrl": "https://old.example.com"
            },
            "node-start": {
                "query": "https://new.example.com"
            }
        }),
        &successful_invoker(),
    )
    .await
    .unwrap();

    assert_eq!(outcome.stop_reason, ExecutionStopReason::Completed);
    assert_eq!(
        outcome.variable_pool["env"]["ApiBaseUrl"],
        json!("https://new.example.com")
    );
    assert_eq!(
        outcome
            .node_traces
            .iter()
            .find(|trace| trace.node_id == "node-answer")
            .expect("answer trace should exist")
            .output_payload["answer"],
        json!("https://new.example.com")
    );
}
