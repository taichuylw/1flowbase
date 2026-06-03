use super::*;
use plugin_framework::provider_contract::{ProviderFinishReason, ProviderInvocationResult};

#[tokio::test]
async fn live_failed_llm_with_inactive_later_branch_activates_terminal_answer() {
    let service = OrchestrationRuntimeService::for_tests_with_provider_results(vec![
        ProviderInvocationResult {
            finish_reason: Some(ProviderFinishReason::Error),
            ..ProviderInvocationResult::default()
        },
    ]);
    let seeded = service
        .seed_application_with_flow("Branch Failover Agent")
        .await;
    let document = json!({
        "schemaVersion": "1flowbase.flow/v2",
        "meta": {
            "flowId": seeded.flow_id.to_string(),
            "name": "Branch Failover Agent",
            "description": "",
            "tags": []
        },
        "graph": {
            "nodes": [
                {
                    "id": "node-start",
                    "type": "start",
                    "alias": "Start",
                    "description": "",
                    "containerId": null,
                    "position": { "x": 0, "y": 0 },
                    "configVersion": 1,
                    "config": {},
                    "bindings": {},
                    "outputs": []
                },
                {
                    "id": "node-if",
                    "type": "if_else",
                    "alias": "If / Else",
                    "description": "",
                    "containerId": null,
                    "position": { "x": 240, "y": 0 },
                    "configVersion": 1,
                    "config": {},
                    "bindings": {
                        "branches": {
                            "kind": "if_else_branches",
                            "value": {
                                "branches": [
                                    {
                                        "id": "if",
                                        "kind": "if",
                                        "title": "If",
                                        "sourceHandle": "if",
                                        "condition": {
                                            "operator": "and",
                                            "conditions": [{
                                                "kind": "rule",
                                                "left": ["node-start", "query"],
                                                "comparator": "exists"
                                            }]
                                        }
                                    },
                                    {
                                        "id": "else",
                                        "kind": "else",
                                        "title": "Else",
                                        "sourceHandle": "else"
                                    }
                                ]
                            }
                        }
                    },
                    "outputs": []
                },
                {
                    "id": "node-llm",
                    "type": "llm",
                    "alias": "LLM",
                    "description": "",
                    "containerId": null,
                    "position": { "x": 480, "y": 0 },
                    "configVersion": 1,
                    "config": {
                        "model_provider": {
                            "provider_code": "fixture_provider",
                            "model_id": "gpt-5.4-mini"
                        }
                    },
                    "bindings": {
                        "prompt_messages": {
                            "kind": "prompt_messages",
                            "value": [{
                                "id": "user-1",
                                "role": "user",
                                "content": {
                                    "kind": "templated_text",
                                    "value": "{{node-start.query}}"
                                }
                            }]
                        }
                    },
                    "outputs": [{ "key": "text", "title": "模型输出", "valueType": "string" }]
                },
                {
                    "id": "node-answer",
                    "type": "answer",
                    "alias": "Answer",
                    "description": "",
                    "containerId": null,
                    "position": { "x": 720, "y": 0 },
                    "configVersion": 1,
                    "config": {},
                    "bindings": {
                        "answer_template": {
                            "kind": "templated_text",
                            "value": "{{ node-llm.text }}"
                        }
                    },
                    "outputs": [{ "key": "answer", "title": "对话输出", "valueType": "string" }]
                },
                {
                    "id": "node-inactive",
                    "type": "x_unknown",
                    "alias": "Inactive",
                    "description": "",
                    "containerId": null,
                    "position": { "x": 480, "y": 120 },
                    "configVersion": 1,
                    "config": {},
                    "bindings": {},
                    "outputs": []
                }
            ],
            "edges": [
                {
                    "id": "edge-start-if",
                    "source": "node-start",
                    "target": "node-if",
                    "sourceHandle": null,
                    "targetHandle": null,
                    "containerId": null,
                    "points": []
                },
                {
                    "id": "edge-if-llm",
                    "source": "node-if",
                    "target": "node-llm",
                    "sourceHandle": "if",
                    "targetHandle": null,
                    "containerId": null,
                    "points": []
                },
                {
                    "id": "edge-else-inactive",
                    "source": "node-if",
                    "target": "node-inactive",
                    "sourceHandle": "else",
                    "targetHandle": null,
                    "containerId": null,
                    "points": []
                },
                {
                    "id": "edge-llm-answer",
                    "source": "node-llm",
                    "target": "node-answer",
                    "sourceHandle": null,
                    "targetHandle": null,
                    "containerId": null,
                    "points": []
                }
            ]
        },
        "editor": {
            "viewport": { "x": 0, "y": 0, "zoom": 1 },
            "annotations": [],
            "activeContainerPath": []
        }
    });

    let started = service
        .start_flow_debug_run(StartFlowDebugRunCommand {
            actor_user_id: seeded.actor_user_id,
            application_id: seeded.application_id,
            input_payload: json!({ "node-start": { "query": "hi" } }),
            document_snapshot: Some(document),
            debug_session_id: None,
        })
        .await
        .unwrap();

    let failed = service
        .continue_flow_debug_run(ContinueFlowDebugRunCommand {
            application_id: seeded.application_id,
            flow_run_id: started.flow_run.id,
            workspace_id: Uuid::nil(),
        })
        .await
        .unwrap();

    assert_eq!(failed.flow_run.status, domain::FlowRunStatus::Failed);
    assert_eq!(
        node_run(&failed, "node-if").debug_payload["selected_source_handle"],
        json!("if")
    );
    assert_eq!(
        node_run(&failed, "node-llm").status,
        domain::NodeRunStatus::Failed
    );
    assert_eq!(
        node_run(&failed, "node-answer").status,
        domain::NodeRunStatus::Succeeded
    );
    assert_eq!(
        failed.flow_run.output_payload["answer"],
        json!("provider invocation finished with error")
    );
    assert!(failed
        .node_runs
        .iter()
        .all(|node_run| node_run.node_id != "node-inactive"));
}
