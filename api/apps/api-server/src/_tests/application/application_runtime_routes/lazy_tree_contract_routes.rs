use super::*;
use control_plane::ports::{
    AppendRuntimeEventInput, CompleteCallbackTaskInput, CreateCallbackTaskInput,
    CreateNodeRunInput, OrchestrationRuntimeRepository, UpdateNodeRunInput,
};
use storage_durable::MainDurableStore;

async fn start_llm_preview(
    app: &axum::Router,
    cookie: &str,
    csrf: &str,
    application_id: &str,
    query: &str,
) -> Value {
    let preview = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/console/applications/{application_id}/orchestration/nodes/node-llm/debug-runs"
                ))
                .header("cookie", cookie)
                .header("x-csrf-token", csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "input_payload": {
                            "node-start": { "query": query }
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = preview.status();
    let body = to_bytes(preview.into_body(), usize::MAX).await.unwrap();
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );

    serde_json::from_slice(&body).unwrap()
}

#[tokio::test]
async fn application_runtime_routes_run_overview_loads_detail_without_trace_nodes() {
    let (state, _) = test_api_state_with_database_url().await;
    let app = crate::app_with_state_and_config(state, &test_config());
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let provider_instance_id = create_ready_provider_instance(&app, &cookie, &csrf).await;
    let application_id =
        seed_agent_flow_application(&app, &cookie, &csrf, &provider_instance_id).await;

    let preview_payload =
        start_llm_preview(&app, &cookie, &csrf, &application_id, "总结退款政策").await;
    let flow_run_id = preview_payload["data"]["flow_run"]["id"].as_str().unwrap();

    let overview = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/overview"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = overview.status();
    let body = to_bytes(overview.into_body(), usize::MAX).await.unwrap();
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let payload: Value = serde_json::from_slice(&body).unwrap();
    let data = &payload["data"];

    assert_eq!(data["flow_run"]["id"], json!(flow_run_id));
    assert_eq!(
        data["flow_run"]["input_payload"]["node-start"]["query"],
        json!("总结退款政策")
    );
    assert_eq!(data["run"]["id"], json!(flow_run_id));
    assert!(data["statistics"].is_object());
    assert!(
        data.get("nodes").is_none(),
        "overview must not expose trace root nodes"
    );
    assert!(
        data.get("node_runs").is_none(),
        "overview must not materialize node run content"
    );
}

#[tokio::test]
async fn application_runtime_routes_debug_snapshot_uses_orchestration_plane() {
    let (state, _) = test_api_state_with_database_url().await;
    let app = crate::app_with_state_and_config(state, &test_config());
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let provider_instance_id = create_ready_provider_instance(&app, &cookie, &csrf).await;
    let application_id =
        seed_agent_flow_application(&app, &cookie, &csrf, &provider_instance_id).await;

    let preview_payload =
        start_llm_preview(&app, &cookie, &csrf, &application_id, "总结退款政策").await;
    let flow_run_id = preview_payload["data"]["flow_run"]["id"].as_str().unwrap();

    let snapshot = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/orchestration/runs/{flow_run_id}/debug-snapshot"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = snapshot.status();
    let body = to_bytes(snapshot.into_body(), usize::MAX).await.unwrap();
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let payload: Value = serde_json::from_slice(&body).unwrap();
    let data = &payload["data"];
    assert_eq!(data["flow_run"]["id"], json!(flow_run_id));
    assert!(
        data["node_runs"]
            .as_array()
            .is_some_and(|items| !items.is_empty()),
        "debug snapshot should remain a debug-session detail contract"
    );

    let old_logs_detail = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(old_logs_detail.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn application_runtime_routes_trace_tree_excludes_llm_tool_calls_as_trace_children() {
    let (state, _) = test_api_state_with_database_url().await;
    let app = crate::app_with_state_and_config(state.clone(), &test_config());
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let provider_instance_id = create_ready_provider_instance(&app, &cookie, &csrf).await;
    let application_id =
        seed_agent_flow_application(&app, &cookie, &csrf, &provider_instance_id).await;

    let preview_payload =
        start_llm_preview(&app, &cookie, &csrf, &application_id, "总结退款政策").await;
    let flow_run_id = preview_payload["data"]["flow_run"]["id"].as_str().unwrap();
    let node_run_id =
        Uuid::parse_str(preview_payload["data"]["node_run"]["id"].as_str().unwrap()).unwrap();

    <MainDurableStore as OrchestrationRuntimeRepository>::create_callback_task(
        &state.store,
        &CreateCallbackTaskInput {
            flow_run_id: Uuid::parse_str(flow_run_id).unwrap(),
            node_run_id,
            callback_kind: "llm_tool_calls".to_string(),
            request_payload: json!({
                "tool_calls": [
                    {
                        "id": "call-refund-policy",
                        "name": "refund_policy_lookup"
                    }
                ]
            }),
            external_ref_payload: None,
        },
    )
    .await
    .unwrap();

    let trace_tree = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(trace_tree.status(), StatusCode::OK);
    let trace_tree_body = to_bytes(trace_tree.into_body(), usize::MAX).await.unwrap();
    let trace_tree_payload: Value = serde_json::from_slice(&trace_tree_body).unwrap();
    let root_nodes = trace_tree_payload["data"]["nodes"].as_array().unwrap();
    let llm_node = root_nodes
        .iter()
        .find(|node| node["node_id"] == json!("node-llm"))
        .expect("LLM node should be present");
    assert_eq!(llm_node["has_children"].as_bool(), Some(true));
    let llm_trace_node_id = llm_node["trace_node_id"].as_str().unwrap();

    let children = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree/nodes?parent_trace_node_id={llm_trace_node_id}"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(children.status(), StatusCode::OK);
    let children_body = to_bytes(children.into_body(), usize::MAX).await.unwrap();
    let children_payload: Value = serde_json::from_slice(&children_body).unwrap();
    let child_items = children_payload["data"]["items"].as_array().unwrap();

    assert_eq!(
        child_items.len(),
        1,
        "llm_tool_calls should appear through the projection tools group"
    );
    assert_eq!(child_items[0]["node_kind"], json!("tool_group"));
}

#[tokio::test]
async fn application_runtime_routes_trace_node_content_exposes_tool_index_and_lazy_tool_detail() {
    let (state, _) = test_api_state_with_database_url().await;
    let app = crate::app_with_state_and_config(state.clone(), &test_config());
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let provider_instance_id = create_ready_provider_instance(&app, &cookie, &csrf).await;
    let application_id =
        seed_agent_flow_application(&app, &cookie, &csrf, &provider_instance_id).await;

    let preview_payload =
        start_llm_preview(&app, &cookie, &csrf, &application_id, "总结退款政策").await;
    let flow_run_id = preview_payload["data"]["flow_run"]["id"].as_str().unwrap();
    let flow_run_uuid = Uuid::parse_str(flow_run_id).unwrap();
    let node_run_id =
        Uuid::parse_str(preview_payload["data"]["node_run"]["id"].as_str().unwrap()).unwrap();

    <MainDurableStore as OrchestrationRuntimeRepository>::update_node_run(
        &state.store,
        &UpdateNodeRunInput {
            node_run_id,
            status: domain::NodeRunStatus::WaitingCallback,
            output_payload: json!({
                "tool_calls": [
                    {
                        "id": "call-refund-policy",
                        "name": "refund_policy_lookup"
                    }
                ]
            }),
            error_payload: None,
            metrics_payload: json!({
                "usage": {
                    "total_tokens": 42
                }
            }),
            debug_payload: json!({
                "llm_rounds": [
                    {
                        "round_index": 0,
                        "assistant": {
                            "role": "assistant",
                            "tool_calls": [
                                {
                                    "id": "call-refund-policy",
                                    "name": "refund_policy_lookup",
                                    "arguments": {
                                        "topic": "refund"
                                    }
                                }
                            ]
                        }
                    }
                ]
            }),
            finished_at: None,
        },
    )
    .await
    .unwrap();

    let callback_task = <MainDurableStore as OrchestrationRuntimeRepository>::create_callback_task(
        &state.store,
        &CreateCallbackTaskInput {
            flow_run_id: flow_run_uuid,
            node_run_id,
            callback_kind: "llm_tool_calls".to_string(),
            request_payload: json!({
                "tool_calls": [
                    {
                        "id": "call-refund-policy",
                        "name": "refund_policy_lookup",
                        "arguments": {
                            "topic": "refund"
                        }
                    }
                ]
            }),
            external_ref_payload: None,
        },
    )
    .await
    .unwrap();
    <MainDurableStore as OrchestrationRuntimeRepository>::complete_callback_task(
        &state.store,
        &CompleteCallbackTaskInput {
            callback_task_id: callback_task.id,
            response_payload: json!({
                "tool_results": [
                    {
                        "tool_call_id": "call-refund-policy",
                        "name": "refund_policy_lookup",
                        "content": "30 days refund window",
                        "execution_status": "succeeded"
                    }
                ]
            }),
            completed_at: callback_task.created_at + time::Duration::milliseconds(1234),
        },
    )
    .await
    .unwrap();
    <MainDurableStore as OrchestrationRuntimeRepository>::append_runtime_events(
        &state.store,
        &[
            AppendRuntimeEventInput {
                flow_run_id: flow_run_uuid,
                node_run_id: Some(node_run_id),
                span_id: None,
                parent_span_id: None,
                event_type: "visible_internal_llm_tool_started".to_string(),
                layer: domain::RuntimeEventLayer::AgentTransition,
                source: domain::RuntimeEventSource::Host,
                trust_level: domain::RuntimeTrustLevel::HostFact,
                item_id: None,
                ledger_ref: None,
                payload: json!({
                    "tool_call_id": "call-refund-policy",
                    "tool_name": "refund_policy_lookup",
                    "main_node_id": "node-llm",
                    "target_node_id": "node-refund-panel",
                    "route_kind": "fusion",
                    "execution_mode": "bounded_parallel_panel",
                    "arguments": {
                        "topic": "refund"
                    }
                }),
                visibility: domain::RuntimeEventVisibility::Workspace,
                durability: domain::RuntimeEventDurability::Durable,
            },
            AppendRuntimeEventInput {
                flow_run_id: flow_run_uuid,
                node_run_id: Some(node_run_id),
                span_id: None,
                parent_span_id: None,
                event_type: "visible_internal_llm_tool_completed".to_string(),
                layer: domain::RuntimeEventLayer::AgentTransition,
                source: domain::RuntimeEventSource::Host,
                trust_level: domain::RuntimeTrustLevel::HostFact,
                item_id: None,
                ledger_ref: None,
                payload: json!({
                    "tool_call_id": "call-refund-policy",
                    "tool_name": "refund_policy_lookup",
                    "main_node_id": "node-llm",
                    "target_node_id": "node-refund-panel",
                    "route_kind": "fusion",
                    "execution_mode": "bounded_parallel_panel",
                    "node_id": "node-refund-panel",
                    "node_alias": "Refund Panel",
                    "node_type": "llm",
                    "provider_route": {
                        "model": "refund-review-v1"
                    },
                    "content": "refund panel says 30 days"
                }),
                visibility: domain::RuntimeEventVisibility::Workspace,
                durability: domain::RuntimeEventDurability::Durable,
            },
        ],
    )
    .await
    .unwrap();

    let trace_tree = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(trace_tree.status(), StatusCode::OK);
    let trace_tree_body = to_bytes(trace_tree.into_body(), usize::MAX).await.unwrap();
    let trace_tree_payload: Value = serde_json::from_slice(&trace_tree_body).unwrap();
    let root_nodes = trace_tree_payload["data"]["nodes"].as_array().unwrap();
    let llm_trace_node_id = root_nodes
        .iter()
        .find(|node| node["node_id"] == json!("node-llm"))
        .expect("LLM node should be present")["trace_node_id"]
        .as_str()
        .unwrap();

    let children = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree/nodes?parent_trace_node_id={llm_trace_node_id}"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(children.status(), StatusCode::OK);
    let children_body = to_bytes(children.into_body(), usize::MAX).await.unwrap();
    let children_payload: Value = serde_json::from_slice(&children_body).unwrap();
    let children_items = children_payload["data"]["items"].as_array().unwrap();
    assert_eq!(
        children_items.len(),
        1,
        "llm_tool_calls must come back through the projection tools group"
    );
    assert_eq!(children_items[0]["node_kind"], json!("tool_group"));
    let tools_trace_node_id = children_items[0]["trace_node_id"].as_str().unwrap();

    let tool_children = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree/nodes?parent_trace_node_id={tools_trace_node_id}"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(tool_children.status(), StatusCode::OK);
    let tool_children_body = to_bytes(tool_children.into_body(), usize::MAX)
        .await
        .unwrap();
    let tool_children_payload: Value = serde_json::from_slice(&tool_children_body).unwrap();
    let tool_child_items = tool_children_payload["data"]["items"].as_array().unwrap();
    assert_eq!(tool_child_items.len(), 1);
    assert_eq!(tool_child_items[0]["node_kind"], json!("tool_callback"));
    assert_eq!(
        tool_child_items[0]["node_alias"],
        json!("refund_policy_lookup")
    );
    assert_eq!(tool_child_items[0]["has_content"], json!(true));

    let content = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree/nodes/{llm_trace_node_id}/content"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(content.status(), StatusCode::OK);
    let content_body = to_bytes(content.into_body(), usize::MAX).await.unwrap();
    let content_payload: Value = serde_json::from_slice(&content_body).unwrap();
    assert!(
        content_payload["data"]["node_run"]["debug_payload"]
            .get("tool_callbacks")
            .is_none(),
        "tool callback summaries should be loaded through projection children"
    );

    let detail = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree/nodes/{llm_trace_node_id}/tool-callbacks/call-refund-policy/content"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_body = to_bytes(detail.into_body(), usize::MAX).await.unwrap();
    let detail_payload: Value = serde_json::from_slice(&detail_body).unwrap();
    let payload = &detail_payload["data"]["payload"];
    assert_eq!(payload["id"], json!("call-refund-policy"));
    assert_eq!(
        payload["request_payload"]["arguments"]["topic"],
        json!("refund")
    );
    assert_eq!(
        payload["callback_payload"]["content"],
        json!("30 days refund window")
    );
    assert_eq!(
        payload["parsed_result"]["content"],
        json!("30 days refund window")
    );
    assert_eq!(payload["duration_ms"], json!(1234));
    assert_eq!(payload["route_trace"]["route_kind"], json!("fusion"));
    assert_eq!(
        payload["route_trace"]["branch_traces"][0]["node_alias"],
        json!("Refund Panel")
    );
    assert_eq!(
        payload["route_trace"]["branch_traces"][0]["output_payload"]["text"],
        json!("refund panel says 30 days")
    );
}

#[tokio::test]
async fn application_runtime_routes_trace_tree_groups_repeated_llm_node_runs_at_root() {
    let (state, _) = test_api_state_with_database_url().await;
    let app = crate::app_with_state_and_config(state.clone(), &test_config());
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let provider_instance_id = create_ready_provider_instance(&app, &cookie, &csrf).await;
    let application_id =
        seed_agent_flow_application(&app, &cookie, &csrf, &provider_instance_id).await;

    let preview_payload =
        start_llm_preview(&app, &cookie, &csrf, &application_id, "总结退款政策").await;
    let flow_run_id = preview_payload["data"]["flow_run"]["id"].as_str().unwrap();
    let flow_run_uuid = Uuid::parse_str(flow_run_id).unwrap();
    let first_node_run_id =
        Uuid::parse_str(preview_payload["data"]["node_run"]["id"].as_str().unwrap()).unwrap();

    <MainDurableStore as OrchestrationRuntimeRepository>::update_node_run(
        &state.store,
        &UpdateNodeRunInput {
            node_run_id: first_node_run_id,
            status: domain::NodeRunStatus::Succeeded,
            output_payload: json!({
                "tool_calls": [
                    {
                        "id": "call_weather",
                        "name": "lookup_weather"
                    }
                ]
            }),
            error_payload: None,
            metrics_payload: json!({
                "usage": {
                    "total_tokens": 14
                }
            }),
            debug_payload: json!({
                "llm_rounds": [
                    {
                        "round_index": 0,
                        "assistant": {
                            "role": "assistant",
                            "tool_calls": [
                                {
                                    "id": "call_weather",
                                    "name": "lookup_weather"
                                }
                            ]
                        }
                    }
                ]
            }),
            finished_at: Some(time::OffsetDateTime::now_utc()),
        },
    )
    .await
    .unwrap();

    let second_node_run = <MainDurableStore as OrchestrationRuntimeRepository>::create_node_run(
        &state.store,
        &CreateNodeRunInput {
            flow_run_id: flow_run_uuid,
            node_id: "node-llm".to_string(),
            node_type: "llm".to_string(),
            node_alias: "LLM".to_string(),
            status: domain::NodeRunStatus::WaitingCallback,
            input_payload: json!({
                "prompt": "continue refund policy"
            }),
            debug_payload: json!({
                "llm_rounds": [
                    {
                        "round_index": 1,
                        "assistant": {
                            "role": "assistant",
                            "tool_calls": [
                                {
                                    "id": "call_policy",
                                    "name": "read_policy"
                                }
                            ]
                        }
                    }
                ]
            }),
            started_at: time::OffsetDateTime::now_utc(),
        },
    )
    .await
    .unwrap();
    <MainDurableStore as OrchestrationRuntimeRepository>::update_node_run(
        &state.store,
        &UpdateNodeRunInput {
            node_run_id: second_node_run.id,
            status: domain::NodeRunStatus::WaitingCallback,
            output_payload: json!({
                "tool_calls": [
                    {
                        "id": "call_policy",
                        "name": "read_policy"
                    }
                ]
            }),
            error_payload: None,
            metrics_payload: json!({
                "usage": {
                    "total_tokens": 24
                }
            }),
            debug_payload: json!({
                "llm_rounds": [
                    {
                        "round_index": 1,
                        "assistant": {
                            "role": "assistant",
                            "tool_calls": [
                                {
                                    "id": "call_policy",
                                    "name": "read_policy"
                                }
                            ]
                        }
                    }
                ]
            }),
            finished_at: None,
        },
    )
    .await
    .unwrap();

    let trace_tree = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(trace_tree.status(), StatusCode::OK);
    let trace_tree_body = to_bytes(trace_tree.into_body(), usize::MAX).await.unwrap();
    let trace_tree_payload: Value = serde_json::from_slice(&trace_tree_body).unwrap();
    let root_nodes = trace_tree_payload["data"]["nodes"].as_array().unwrap();

    assert_eq!(
        root_nodes.len(),
        1,
        "trace root should expose one display node for repeated LLM node runs"
    );
    assert_eq!(root_nodes[0]["node_id"], json!("node-llm"));
    assert_eq!(root_nodes[0]["status"], json!("waiting_callback"));
    let trace_node_id = root_nodes[0]["trace_node_id"].as_str().unwrap();
    Uuid::parse_str(trace_node_id).expect("trace_node_id is deterministic UUID");
    assert!(root_nodes[0]["stable_locator"]
        .as_str()
        .unwrap()
        .contains("/node_group:"));

    let content = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree/nodes/{trace_node_id}/content"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(content.status(), StatusCode::OK);
    let content_body = to_bytes(content.into_body(), usize::MAX).await.unwrap();
    let content_payload: Value = serde_json::from_slice(&content_body).unwrap();
    let rounds = content_payload["data"]["node_run"]["debug_payload"]["llm_rounds"]
        .as_array()
        .unwrap();

    assert_eq!(rounds.len(), 2);
    assert!(
        content_payload["data"]["node_run"]["debug_payload"]
            .get("tool_callbacks")
            .is_none(),
        "grouped LLM tool callbacks are loaded through projection children"
    );
    assert_eq!(
        content_payload["data"]["node_run"]["output_payload"]["tool_calls"][0]["id"],
        json!("call_policy")
    );

    let tools = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree/nodes?parent_trace_node_id={trace_node_id}"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(tools.status(), StatusCode::OK);
    let tools_body = to_bytes(tools.into_body(), usize::MAX).await.unwrap();
    let tools_payload: Value = serde_json::from_slice(&tools_body).unwrap();
    let tool_group_id = tools_payload["data"]["items"][0]["trace_node_id"]
        .as_str()
        .unwrap();
    let tool_callbacks = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree/nodes?parent_trace_node_id={tool_group_id}"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(tool_callbacks.status(), StatusCode::OK);
    let tool_callbacks_body = to_bytes(tool_callbacks.into_body(), usize::MAX)
        .await
        .unwrap();
    let tool_callbacks_payload: Value = serde_json::from_slice(&tool_callbacks_body).unwrap();
    let tool_callback_aliases = tool_callbacks_payload["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|callback| callback["node_alias"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(tool_callback_aliases, vec!["lookup_weather", "read_policy"]);
}
