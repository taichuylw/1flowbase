use super::*;
use axum::http::HeaderValue;

#[test]
fn runtime_event_cursor_accepts_numeric_and_run_scoped_event_ids() {
    let run_id = Uuid::now_v7();

    assert_eq!(parse_runtime_event_cursor(run_id, "7"), Some(7));
    assert_eq!(
        parse_runtime_event_cursor(run_id, &format!("{run_id}:8")),
        Some(8)
    );
    assert_eq!(
        parse_runtime_event_cursor(Uuid::now_v7(), &format!("{run_id}:8")),
        None
    );
    assert_eq!(parse_runtime_event_cursor(run_id, "not-a-cursor"), None);
}

#[test]
fn metrics_payload_cache_hit_tokens_accepts_cache_read_tokens() {
    assert_eq!(
        metrics_payload_cache_hit_tokens(&serde_json::json!({
            "usage": {
                "input_cache_hit_tokens": null,
                "cache_read_tokens": 29_504
            }
        })),
        Some(29_504)
    );
    assert_eq!(
        metrics_payload_cache_hit_tokens(&serde_json::json!({
            "usage": {
                "input_cache_hit_tokens": 11,
                "cache_read_tokens": 29_504
            }
        })),
        Some(11)
    );
}

#[test]
fn debug_run_stream_cursor_prefers_query_before_last_event_id_header() {
    let run_id = Uuid::now_v7();
    let mut headers = HeaderMap::new();
    headers.insert(
        "last-event-id",
        HeaderValue::from_str(&format!("{run_id}:11")).unwrap(),
    );

    assert_eq!(
        debug_run_stream_from_sequence(
            run_id,
            &DebugRunStreamQuery {
                from_sequence: Some(9),
                last_event_id: Some(format!("{run_id}:10")),
            },
            &headers,
        ),
        Some(9)
    );
    assert_eq!(
        debug_run_stream_from_sequence(
            run_id,
            &DebugRunStreamQuery {
                from_sequence: None,
                last_event_id: Some(format!("{run_id}:10")),
            },
            &headers,
        ),
        Some(10)
    );
    assert_eq!(
        debug_run_stream_from_sequence(
            run_id,
            &DebugRunStreamQuery {
                from_sequence: None,
                last_event_id: None,
            },
            &headers,
        ),
        Some(11)
    );
}

#[test]
fn usage_total_tokens_uses_total_or_known_segments() {
    assert_eq!(
        usage_total_tokens(&serde_json::json!({
            "total_tokens": 128,
            "input_tokens": 40,
            "output_tokens": 12
        })),
        Some(128)
    );
    assert_eq!(
        usage_total_tokens(&serde_json::json!({
            "input_tokens": 40,
            "output_tokens": 12,
            "reasoning_tokens": 6
        })),
        Some(58)
    );
    assert_eq!(usage_total_tokens(&serde_json::json!({})), None);
}

#[test]
fn callback_task_tool_callback_count_reads_offloaded_tool_call_count() {
    let base_task = domain::CallbackTaskRecord {
        id: Uuid::now_v7(),
        flow_run_id: Uuid::now_v7(),
        node_run_id: Uuid::now_v7(),
        callback_kind: "llm_tool_calls".to_string(),
        status: domain::CallbackTaskStatus::Completed,
        request_payload: serde_json::json!({
            "tool_calls": [
                { "id": "call-1" },
                { "id": "call-2" }
            ]
        }),
        response_payload: None,
        external_ref_payload: None,
        created_at: OffsetDateTime::UNIX_EPOCH,
        completed_at: Some(OffsetDateTime::UNIX_EPOCH),
    };
    assert_eq!(callback_task_tool_callback_count(&base_task), 2);

    let offloaded_task = domain::CallbackTaskRecord {
        request_payload: serde_json::json!({
            "tool_calls": {
                "__runtime_debug_artifact": true,
                "artifact_ref": Uuid::now_v7().to_string(),
                "tool_call_count": 3
            }
        }),
        ..base_task
    };
    assert_eq!(callback_task_tool_callback_count(&offloaded_task), 3);
}

#[test]
fn application_run_statistics_counts_indexed_llm_tool_callbacks() {
    let application = test_application_record();
    let flow_run_id = Uuid::now_v7();
    let node_run_id = Uuid::now_v7();
    let callback_task = domain::CallbackTaskRecord {
        id: Uuid::now_v7(),
        flow_run_id,
        node_run_id,
        callback_kind: "llm_tool_calls".to_string(),
        status: domain::CallbackTaskStatus::Completed,
        request_payload: serde_json::json!({
            "tool_calls": [
                { "id": "call-1", "name": "Read" }
            ]
        }),
        response_payload: None,
        external_ref_payload: None,
        created_at: OffsetDateTime::UNIX_EPOCH,
        completed_at: Some(OffsetDateTime::UNIX_EPOCH),
    };
    let detail = domain::ApplicationRunDetail {
        flow_run: test_flow_run_record(
            application.id,
            flow_run_id,
            domain::FlowRunStatus::Succeeded,
            serde_json::json!({}),
        ),
        node_runs: vec![domain::NodeRunRecord {
            id: node_run_id,
            flow_run_id,
            node_id: "node-llm".to_string(),
            node_type: "llm".to_string(),
            node_alias: "LLM".to_string(),
            status: domain::NodeRunStatus::Succeeded,
            input_payload: serde_json::json!({}),
            output_payload: serde_json::json!({}),
            error_payload: None,
            metrics_payload: serde_json::json!({}),
            debug_payload: serde_json::json!({
                "llm_rounds": [
                    {
                        "round_index": 0,
                        "assistant": {
                            "tool_calls": [
                                { "id": "call-1", "name": "Read" },
                                { "id": "call-2", "name": "problem_review" }
                            ]
                        }
                    }
                ]
            }),
            started_at: OffsetDateTime::UNIX_EPOCH,
            finished_at: Some(OffsetDateTime::UNIX_EPOCH),
        }],
        checkpoints: Vec::new(),
        callback_tasks: vec![callback_task],
        events: Vec::new(),
        stitched_trace: Vec::new(),
    };

    let statistics = application_run_statistics(&detail);

    assert_eq!(statistics.tool_callback_count, 2);
}

#[test]
fn trace_tree_endpoints_read_projection_without_full_detail_fallback() {
    let log_endpoint_source = include_str!("log_handlers.rs");

    for function_name in [
        "get_application_run_trace_tree",
        "get_application_run_trace_node_children",
        "get_application_run_trace_node_content",
        "get_application_run_trace_tool_callback_content",
    ] {
        let function_source =
            application_trace_tree_endpoint_source(log_endpoint_source, function_name);

        assert!(
            function_source.contains("ensure_application_run_trace_projection_status"),
            "{function_name} must enter through projection status"
        );
        assert!(
            !function_source.contains("get_application_run_detail"),
            "{function_name} must not fallback to full run detail reads"
        );
    }

    assert!(application_trace_tree_endpoint_source(
        log_endpoint_source,
        "get_application_run_trace_tree"
    )
    .contains("list_application_run_trace_roots"));
    assert!(application_trace_tree_endpoint_source(
        log_endpoint_source,
        "get_application_run_trace_tree"
    )
    .contains("get_application_run_trace_statistics"));
    assert!(!application_trace_tree_endpoint_source(
        log_endpoint_source,
        "get_application_run_trace_tree"
    )
    .contains("list_application_run_trace_nodes_for_statistics"));
    assert!(application_trace_tree_endpoint_source(
        log_endpoint_source,
        "get_application_run_trace_node_children"
    )
    .contains("list_application_run_trace_children"));
    assert!(
        application_trace_tree_endpoint_source(
            log_endpoint_source,
            "get_application_run_trace_node_content"
        )
        .contains(
            "<MainDurableStore as OrchestrationRuntimeRepository>::get_application_run_trace_node_content"
        )
    );
    assert!(
        application_trace_tree_endpoint_source(
            log_endpoint_source,
            "get_application_run_trace_tool_callback_content"
        )
        .contains(
            "<MainDurableStore as OrchestrationRuntimeRepository>::get_application_run_trace_node_content"
        )
    );
}

#[test]
fn trace_projection_status_ensure_checks_lightweight_watermark_before_full_source() {
    let function_source = application_runtime_function_source(
        include_str!("log_handlers.rs"),
        "async fn ensure_application_run_trace_projection_status",
    );
    let watermark_query = function_source
        .find("get_application_run_trace_projection_source_watermark")
        .expect("trace projection ensure must read a lightweight source watermark");
    let full_source_query = function_source
        .find("get_application_run_trace_projection_source(")
        .expect("trace projection ensure may read the full source only for rebuild");

    assert!(
        watermark_query < full_source_query,
        "trace projection ensure must decide unchanged succeeded projections before loading the full detail source"
    );
}

#[test]
fn trace_node_content_response_serializes_refs_without_heavy_containers() {
    let trace_node_id = Uuid::now_v7();
    let flow_run_id = Uuid::now_v7();
    let now = OffsetDateTime::UNIX_EPOCH;
    let response = trace_projection_node_content_response(
        domain::ApplicationRunTraceNodeRecord {
            trace_node_id,
            flow_run_id,
            parent_trace_node_id: None,
            stable_locator: format!("run:{flow_run_id}/node:{}", Uuid::now_v7()),
            node_kind: "node_run".to_string(),
            owner_kind: Some("node_run".to_string()),
            owner_id: Some(Uuid::now_v7().to_string()),
            order_key: "000001".to_string(),
            node_id: Some("node-llm".to_string()),
            node_type: Some("llm".to_string()),
            node_alias: "LLM".to_string(),
            status: "succeeded".to_string(),
            started_at: now,
            finished_at: Some(now),
            duration_ms: Some(0),
            metrics_payload: serde_json::json!({}),
            has_children: false,
            child_count: 0,
            has_content: true,
            content_ref: None,
            projection_version: APPLICATION_RUN_TRACE_PROJECTION_VERSION,
            source_watermark: "source:1".to_string(),
            created_at: now,
            updated_at: now,
        },
        domain::ApplicationRunTraceNodeContentRecord {
            trace_node_id,
            content_kind: "node_run".to_string(),
            payload: serde_json::json!({
                "payload_index": {
                    "node_run_count": 1,
                    "checkpoint_count": 1,
                    "event_count": 1
                },
                "detail_refs": [
                    {
                        "detail_kind": "node_run",
                        "source_kind": "node_run",
                        "source_locator": "node-run-1",
                        "count": 1
                    }
                ]
            }),
            source_refs: serde_json::json!([
                {
                    "source_kind": "node_run",
                    "source_locator": "node-run-1"
                }
            ]),
            created_at: now,
            updated_at: now,
        },
        ApplicationRunTraceProjectionStatusResponse {
            projection_status: "succeeded".to_string(),
            projection_version: APPLICATION_RUN_TRACE_PROJECTION_VERSION,
            source_watermark: "source:1".to_string(),
            attempt_count: 1,
            last_attempt_at: Some(format_time(now)),
            last_success_at: Some(format_time(now)),
            last_error_code: None,
            last_error_stage: None,
            last_error_source_kind: None,
            last_error_source_locator: None,
            last_error_ref: None,
            retriable: false,
        },
    )
    .expect("trace node content response should serialize");
    let serialized = serde_json::to_value(response).expect("response should be JSON");
    let data = serialized
        .as_object()
        .expect("trace node content response should be an object");

    assert!(!data.contains_key("node_run"));
    assert!(!data.contains_key("checkpoints"));
    assert!(!data.contains_key("events"));
    assert_eq!(serialized["content_kind"], serde_json::json!("node_run"));
    assert_eq!(
        serialized["payload"]["payload_index"]["node_run_count"],
        serde_json::json!(1)
    );
    assert_eq!(
        serialized["detail_refs"][0]["detail_kind"],
        serde_json::json!("node_run")
    );
    assert_eq!(
        serialized["source_refs"][0]["source_kind"],
        serde_json::json!("node_run")
    );
}

#[test]
fn trace_node_content_raw_payload_keeps_empty_payload_as_object() {
    let payload = trace_node_content_raw_payload_response(serde_json::json!({
        "node_run": {
            "id": "node-run-1"
        },
        "checkpoints": [],
        "events": [],
        "detail_refs": [],
        "source_refs": []
    }));

    assert_eq!(payload, serde_json::json!({}));
}

fn application_trace_tree_endpoint_source<'a>(
    log_endpoint_source: &'a str,
    function_name: &str,
) -> &'a str {
    application_runtime_function_source(
        log_endpoint_source,
        &format!("pub async fn {function_name}"),
    )
}

fn application_runtime_function_source<'a>(
    log_endpoint_source: &'a str,
    function_marker: &str,
) -> &'a str {
    let start = log_endpoint_source
        .find(function_marker)
        .unwrap_or_else(|| panic!("{function_marker} source exists"));
    let remaining_source = &log_endpoint_source[start..];
    let end = remaining_source
        .find("\n#[utoipa::path(")
        .unwrap_or(remaining_source.len());

    &remaining_source[..end]
}

#[test]
fn start_node_response_moves_legacy_output_payload_into_input() {
    let run = domain::NodeRunRecord {
        id: Uuid::now_v7(),
        flow_run_id: Uuid::now_v7(),
        node_id: "node-start".to_string(),
        node_type: "start".to_string(),
        node_alias: "Start".to_string(),
        status: domain::NodeRunStatus::Succeeded,
        input_payload: serde_json::json!({}),
        output_payload: serde_json::json!({
            "query": "ping",
            "tools": [
                {
                    "name": "read_file",
                    "source": "openai_compatible"
                }
            ]
        }),
        error_payload: None,
        metrics_payload: serde_json::json!({}),
        debug_payload: serde_json::json!({}),
        started_at: OffsetDateTime::UNIX_EPOCH,
        finished_at: Some(OffsetDateTime::UNIX_EPOCH),
    };

    let response = to_node_run_response(run);

    assert_eq!(response.input_payload["query"], serde_json::json!("ping"));
    assert_eq!(
        response.input_payload["tools"][0]["name"],
        serde_json::json!("read_file")
    );
    assert_eq!(response.output_payload, serde_json::json!({}));
}

#[test]
fn start_node_response_exposes_input_payload_truth_view() {
    let artifact_ref = Uuid::now_v7().to_string();
    let run = domain::NodeRunRecord {
        id: Uuid::now_v7(),
        flow_run_id: Uuid::now_v7(),
        node_id: "node-start".to_string(),
        node_type: "start".to_string(),
        node_alias: "Start".to_string(),
        status: domain::NodeRunStatus::Succeeded,
        input_payload: serde_json::json!({
            "query": "say hello",
            "model": "deepseek-chat",
            "files": [{ "name": "brief.md" }],
            "sys": {
                "workflow_run_id": "run-1"
            },
            "env": {
                "ApiBaseUrl": "https://api.example.com"
            },
            "history": {
                "__runtime_debug_artifact": true,
                "artifact_ref": artifact_ref,
                "is_truncated": true,
                "field_path": ["history"],
                "preview": "[{\"role\":\"user\",\"content\":\"old"
            },
            "tools": []
        }),
        output_payload: serde_json::json!({ "query": "say hello" }),
        error_payload: None,
        metrics_payload: serde_json::json!({}),
        debug_payload: serde_json::json!({}),
        started_at: OffsetDateTime::UNIX_EPOCH,
        finished_at: Some(OffsetDateTime::UNIX_EPOCH),
    };

    let response = to_node_run_response(run);

    assert_eq!(response.input_payload["query"], "say hello");
    assert_eq!(response.input_payload["model"], "deepseek-chat");
    assert_eq!(response.input_payload["sys"]["workflow_run_id"], "run-1");
    assert_eq!(
        response.input_payload["env"]["ApiBaseUrl"],
        "https://api.example.com"
    );
    assert_eq!(
        response.input_payload["history"]["field_path"],
        serde_json::json!(["history"])
    );
    assert_eq!(response.input_payload_view, response.input_payload);
}

fn test_application_record() -> domain::ApplicationRecord {
    domain::ApplicationRecord {
        id: Uuid::now_v7(),
        workspace_id: Uuid::now_v7(),
        application_type: domain::ApplicationType::AgentFlow,
        name: "Support Agent".to_string(),
        description: "runtime".to_string(),
        icon: None,
        icon_type: None,
        icon_background: None,
        created_by: Uuid::now_v7(),
        updated_at: OffsetDateTime::UNIX_EPOCH,
        tags: Vec::new(),
        sections: domain::ApplicationSections {
            orchestration: domain::ApplicationOrchestrationSection {
                status: "enabled".to_string(),
                subject_kind: "flow".to_string(),
                subject_status: "draft".to_string(),
                current_subject_id: Some(Uuid::now_v7()),
                current_draft_id: Some(Uuid::now_v7()),
            },
            api: domain::ApplicationApiSection {
                status: "enabled".to_string(),
                credential_kind: "api_key".to_string(),
                invoke_routing_mode: "application".to_string(),
                invoke_path_template: None,
                api_capability_status: "enabled".to_string(),
                credentials_status: "enabled".to_string(),
            },
            logs: domain::ApplicationLogsSection {
                status: "enabled".to_string(),
                runs_capability_status: "enabled".to_string(),
                run_object_kind: "application_run".to_string(),
                log_retention_status: "default".to_string(),
            },
            monitoring: domain::ApplicationMonitoringSection {
                status: "enabled".to_string(),
                metrics_capability_status: "enabled".to_string(),
                metrics_object_kind: "application_run".to_string(),
                tracing_config_status: "default".to_string(),
            },
        },
    }
}

fn test_flow_run_record(
    application_id: Uuid,
    flow_run_id: Uuid,
    status: domain::FlowRunStatus,
    output_payload: serde_json::Value,
) -> domain::FlowRunRecord {
    domain::FlowRunRecord {
        id: flow_run_id,
        application_id,
        flow_id: Uuid::now_v7(),
        draft_id: Uuid::now_v7(),
        compiled_plan_id: Some(Uuid::now_v7()),
        debug_session_id: "debug-session".to_string(),
        flow_schema_version: "1flowbase.flow/v2".to_string(),
        document_hash: "hash".to_string(),
        run_mode: domain::FlowRunMode::DebugFlowRun,
        target_node_id: None,
        title: "天气？".to_string(),
        status,
        input_payload: serde_json::json!({
            "node-start": {
                "query": "天气？"
            }
        }),
        output_payload,
        error_payload: None,
        created_by: Uuid::now_v7(),
        authorized_account: Some("root".to_string()),
        api_key_id: None,
        publication_version_id: None,
        external_user: None,
        external_conversation_id: None,
        external_trace_id: None,
        compatibility_mode: None,
        idempotency_key: None,
        started_at: OffsetDateTime::UNIX_EPOCH,
        finished_at: None,
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    }
}

fn test_runtime_event_record(
    flow_run_id: Uuid,
    node_run_id: Option<Uuid>,
    event_type: &str,
    payload: serde_json::Value,
) -> domain::RuntimeEventRecord {
    domain::RuntimeEventRecord {
        id: Uuid::now_v7(),
        flow_run_id,
        node_run_id,
        span_id: None,
        parent_span_id: None,
        sequence: 1,
        event_type: event_type.to_string(),
        layer: domain::RuntimeEventLayer::RuntimeItem,
        source: domain::RuntimeEventSource::Host,
        trust_level: domain::RuntimeTrustLevel::HostFact,
        item_id: None,
        ledger_ref: None,
        payload,
        visibility: domain::RuntimeEventVisibility::Workspace,
        durability: domain::RuntimeEventDurability::Durable,
        created_at: OffsetDateTime::UNIX_EPOCH,
    }
}

#[test]
fn run_detail_response_moves_waiting_prefix_answer_into_answer_snapshot() {
    let application = test_application_record();
    let flow_run_id = Uuid::now_v7();
    let waiting_node_run_id = Uuid::now_v7();
    let virtual_answer_node_run_id = Uuid::now_v7();
    let detail = domain::ApplicationRunDetail {
        flow_run: test_flow_run_record(
            application.id,
            flow_run_id,
            domain::FlowRunStatus::WaitingCallback,
            serde_json::json!({ "answer": "LLM1 final\n----\n" }),
        ),
        node_runs: vec![
            domain::NodeRunRecord {
                id: waiting_node_run_id,
                flow_run_id,
                node_id: "node-llm-2".to_string(),
                node_type: "llm".to_string(),
                node_alias: "LLM2".to_string(),
                status: domain::NodeRunStatus::WaitingCallback,
                input_payload: serde_json::json!({}),
                output_payload: serde_json::json!({ "tool_calls": [] }),
                error_payload: None,
                metrics_payload: serde_json::json!({}),
                debug_payload: serde_json::json!({}),
                started_at: OffsetDateTime::UNIX_EPOCH,
                finished_at: None,
            },
            domain::NodeRunRecord {
                id: virtual_answer_node_run_id,
                flow_run_id,
                node_id: "node-answer".to_string(),
                node_type: "answer".to_string(),
                node_alias: "Answer".to_string(),
                status: domain::NodeRunStatus::Succeeded,
                input_payload: serde_json::json!({
                    "presentation": {
                        "kind": "answer",
                        "complete": false,
                        "materialized_from": "waiting_prefix"
                    }
                }),
                output_payload: serde_json::json!({
                    "answer": "LLM1 final\n----\n"
                }),
                error_payload: None,
                metrics_payload: serde_json::json!({}),
                debug_payload: serde_json::json!({
                    "answer_presentation": {
                        "partial": true,
                        "materialized_from": "waiting_prefix"
                    }
                }),
                started_at: OffsetDateTime::UNIX_EPOCH,
                finished_at: Some(OffsetDateTime::UNIX_EPOCH),
            },
        ],
        checkpoints: vec![domain::CheckpointRecord {
            id: Uuid::now_v7(),
            flow_run_id,
            node_run_id: Some(waiting_node_run_id),
            status: "waiting_callback".to_string(),
            reason: "等待 callback 回填".to_string(),
            locator_payload: serde_json::json!({
                "node_id": "node-llm-2",
                "next_node_index": 2
            }),
            variable_snapshot: serde_json::json!({}),
            external_ref_payload: None,
            created_at: OffsetDateTime::UNIX_EPOCH,
        }],
        callback_tasks: Vec::new(),
        events: Vec::new(),
        stitched_trace: Vec::new(),
    };

    let response = to_application_run_detail_response(&application, detail);

    assert_eq!(response.node_runs.len(), 1);
    assert_eq!(response.node_runs[0].node_id, "node-llm-2");
    let answer_snapshot = response
        .answer_snapshot
        .expect("waiting_prefix answer should become answer_snapshot");
    assert_eq!(answer_snapshot.text, "LLM1 final\n----\n");
    assert!(!answer_snapshot.complete);
    assert_eq!(answer_snapshot.materialized_from, "waiting_prefix");
    assert_eq!(answer_snapshot.answer_node_id, "node-answer");
    assert_eq!(
        answer_snapshot.answer_node_run_id,
        virtual_answer_node_run_id.to_string()
    );
    assert_eq!(
        answer_snapshot.waiting_node_id.as_deref(),
        Some("node-llm-2")
    );
    assert_eq!(
        answer_snapshot.waiting_node_run_id.as_deref(),
        Some(waiting_node_run_id.to_string().as_str())
    );
    assert!(response
        .node_runs
        .iter()
        .all(|node_run| node_run.node_id != "node-answer"));
}

#[test]
fn run_detail_response_exposes_stitched_trace_sources() {
    let application = test_application_record();
    let current_flow_run_id = Uuid::now_v7();
    let source_flow_run_id = Uuid::now_v7();
    let source_node_run_id = Uuid::now_v7();
    let source_answer_node_run_id = Uuid::now_v7();
    let callback_task_id = Uuid::now_v7();
    let detail = domain::ApplicationRunDetail {
        flow_run: test_flow_run_record(
            application.id,
            current_flow_run_id,
            domain::FlowRunStatus::Succeeded,
            serde_json::json!({ "answer": "done" }),
        ),
        node_runs: Vec::new(),
        checkpoints: Vec::new(),
        callback_tasks: Vec::new(),
        events: Vec::new(),
        stitched_trace: vec![domain::ApplicationRunStitchedTrace {
            source_flow_run: test_flow_run_record(
                application.id,
                source_flow_run_id,
                domain::FlowRunStatus::Cancelled,
                serde_json::json!({}),
            ),
            node_runs: vec![
                domain::NodeRunRecord {
                    id: source_node_run_id,
                    flow_run_id: source_flow_run_id,
                    node_id: "node-llm".to_string(),
                    node_type: "llm".to_string(),
                    node_alias: "LLM".to_string(),
                    status: domain::NodeRunStatus::Succeeded,
                    input_payload: serde_json::json!({}),
                    output_payload: serde_json::json!({ "usage": { "total_tokens": 33520 } }),
                    error_payload: None,
                    metrics_payload: serde_json::json!({}),
                    debug_payload: serde_json::json!({
                        "visible_internal_llm_tool_trace": [
                            {
                                "kind": "visible_internal_llm_tool_trace",
                                "tool_call_id": "call_image",
                                "tool_name": "image_llm",
                                "status": "succeeded"
                            }
                        ]
                    }),
                    started_at: OffsetDateTime::UNIX_EPOCH,
                    finished_at: Some(OffsetDateTime::UNIX_EPOCH),
                },
                domain::NodeRunRecord {
                    id: source_answer_node_run_id,
                    flow_run_id: source_flow_run_id,
                    node_id: "node-answer".to_string(),
                    node_type: "answer".to_string(),
                    node_alias: "Answer".to_string(),
                    status: domain::NodeRunStatus::Succeeded,
                    input_payload: serde_json::json!({
                        "presentation": {
                            "materialized_from": "waiting_prefix"
                        }
                    }),
                    output_payload: serde_json::json!({ "answer": "route prefix" }),
                    error_payload: None,
                    metrics_payload: serde_json::json!({}),
                    debug_payload: serde_json::json!({}),
                    started_at: OffsetDateTime::UNIX_EPOCH,
                    finished_at: Some(OffsetDateTime::UNIX_EPOCH),
                },
            ],
            callback_tasks: vec![domain::CallbackTaskRecord {
                id: callback_task_id,
                flow_run_id: source_flow_run_id,
                node_run_id: source_node_run_id,
                callback_kind: "llm_tool_calls".to_string(),
                status: domain::CallbackTaskStatus::Completed,
                request_payload: serde_json::json!({
                    "tool_calls": [
                        { "id": "call_image", "name": "image_llm" }
                    ]
                }),
                response_payload: None,
                external_ref_payload: None,
                created_at: OffsetDateTime::UNIX_EPOCH,
                completed_at: Some(OffsetDateTime::UNIX_EPOCH),
            }],
            events: Vec::new(),
            runtime_events: Vec::new(),
        }],
    };

    let response = to_application_run_detail_response(&application, detail);

    assert_eq!(response.callback_tasks.len(), 0);
    assert_eq!(response.stitched_trace.len(), 1);
    assert_eq!(
        response.stitched_trace[0].source_flow_run.id,
        source_flow_run_id.to_string()
    );
    assert_eq!(
        response.stitched_trace[0].node_runs[0].id,
        source_node_run_id.to_string()
    );
    assert_eq!(response.stitched_trace[0].node_runs.len(), 1);
    assert!(response.stitched_trace[0]
        .node_runs
        .iter()
        .all(|node_run| node_run.node_type != "answer"));
    assert_eq!(
        response.stitched_trace[0].callback_tasks[0].id,
        callback_task_id.to_string()
    );
    assert_eq!(
        response.detail.stitched_trace[0].callback_tasks[0].flow_run_id,
        source_flow_run_id.to_string()
    );
}

#[test]
fn visible_internal_llm_route_trace_uses_precise_node_run_id_before_reused_node_id() {
    let application = test_application_record();
    let flow_run_id = Uuid::now_v7();
    let routed_node_run_id = Uuid::now_v7();
    let later_node_run_id = Uuid::now_v7();
    let detail = domain::ApplicationRunDetail {
        flow_run: test_flow_run_record(
            application.id,
            flow_run_id,
            domain::FlowRunStatus::Succeeded,
            serde_json::json!({ "answer": "done" }),
        ),
        node_runs: vec![
            domain::NodeRunRecord {
                id: routed_node_run_id,
                flow_run_id,
                node_id: "node-llm".to_string(),
                node_type: "llm".to_string(),
                node_alias: "LLM".to_string(),
                status: domain::NodeRunStatus::Succeeded,
                input_payload: serde_json::json!({}),
                output_payload: serde_json::json!({}),
                error_payload: None,
                metrics_payload: serde_json::json!({}),
                debug_payload: serde_json::json!({
                    "callback_kind": "llm_tool_calls",
                    "callback_task_id": Uuid::now_v7().to_string()
                }),
                started_at: OffsetDateTime::UNIX_EPOCH,
                finished_at: Some(OffsetDateTime::UNIX_EPOCH),
            },
            domain::NodeRunRecord {
                id: later_node_run_id,
                flow_run_id,
                node_id: "node-llm".to_string(),
                node_type: "llm".to_string(),
                node_alias: "LLM".to_string(),
                status: domain::NodeRunStatus::Succeeded,
                input_payload: serde_json::json!({}),
                output_payload: serde_json::json!({}),
                error_payload: None,
                metrics_payload: serde_json::json!({}),
                debug_payload: serde_json::json!({
                    "llm_rounds": [
                        {
                            "round_index": 0,
                            "assistant": {
                                "role": "assistant",
                                "content": "later continuation"
                            }
                        }
                    ]
                }),
                started_at: OffsetDateTime::UNIX_EPOCH + Duration::seconds(1),
                finished_at: Some(OffsetDateTime::UNIX_EPOCH + Duration::seconds(1)),
            },
        ],
        checkpoints: Vec::new(),
        callback_tasks: Vec::new(),
        events: Vec::new(),
        stitched_trace: Vec::new(),
    };
    let runtime_events = vec![test_runtime_event_record(
        flow_run_id,
        Some(routed_node_run_id),
        "visible_internal_llm_tool_completed",
        serde_json::json!({
            "main_node_id": "node-llm",
            "target_node_id": "node-llm-1",
            "tool_name": "image_llm",
            "tool_call_id": "call_image",
            "node_run_id": routed_node_run_id.to_string(),
            "provider_route": {
                "model": "mimo-v2.5",
                "provider_code": "anthropic"
            }
        }),
    )];

    let detail =
        enrich_application_run_detail_visible_internal_llm_route_traces(detail, &runtime_events);

    let routed_node = detail
        .node_runs
        .iter()
        .find(|node_run| node_run.id == routed_node_run_id)
        .expect("routed node run should stay visible");
    assert_eq!(
        routed_node.debug_payload["visible_internal_llm_tool_trace"][0]["tool_name"],
        serde_json::json!("image_llm")
    );
    assert_eq!(
        routed_node.debug_payload["visible_internal_llm_tool_trace"][0]["route_model"],
        serde_json::json!("mimo-v2.5")
    );
    assert_eq!(
        routed_node.debug_payload["llm_rounds"][0]["assistant"]["tool_calls"][0]["id"],
        serde_json::json!("call_image")
    );
    assert_eq!(
        routed_node.debug_payload["llm_rounds"][0]["assistant"]["tool_calls"][0]["name"],
        serde_json::json!("image_llm")
    );
    let later_node = detail
        .node_runs
        .iter()
        .find(|node_run| node_run.id == later_node_run_id)
        .expect("later node run should stay visible");
    assert!(later_node
        .debug_payload
        .get("visible_internal_llm_tool_trace")
        .is_none());
}

#[test]
fn visible_internal_llm_fusion_branch_trace_uses_branch_node_run_payloads() {
    let application = test_application_record();
    let flow_run_id = Uuid::now_v7();
    let main_node_run_id = Uuid::now_v7();
    let branch_node_run_id = Uuid::now_v7();
    let detail = domain::ApplicationRunDetail {
        flow_run: test_flow_run_record(
            application.id,
            flow_run_id,
            domain::FlowRunStatus::Succeeded,
            serde_json::json!({ "answer": "done" }),
        ),
        node_runs: vec![
            domain::NodeRunRecord {
                id: main_node_run_id,
                flow_run_id,
                node_id: "node-main-llm".to_string(),
                node_type: "llm".to_string(),
                node_alias: "LLM".to_string(),
                status: domain::NodeRunStatus::Succeeded,
                input_payload: serde_json::json!({}),
                output_payload: serde_json::json!({}),
                error_payload: None,
                metrics_payload: serde_json::json!({}),
                debug_payload: serde_json::json!({
                    "llm_rounds": [
                        {
                            "round_index": 0,
                            "assistant": {
                                "role": "assistant",
                                "tool_calls": [
                                    {
                                        "id": "call_fusion",
                                        "name": "fusion_review"
                                    }
                                ]
                            }
                        }
                    ]
                }),
                started_at: OffsetDateTime::UNIX_EPOCH,
                finished_at: Some(OffsetDateTime::UNIX_EPOCH),
            },
            domain::NodeRunRecord {
                id: branch_node_run_id,
                flow_run_id,
                node_id: "node-panel-a".to_string(),
                node_type: "llm".to_string(),
                node_alias: "LLM2".to_string(),
                status: domain::NodeRunStatus::Succeeded,
                input_payload: serde_json::json!({
                    "prompt_messages": [
                        {
                            "role": "user",
                            "content": "review refund policy"
                        }
                    ],
                    "model": "risk-v1"
                }),
                output_payload: serde_json::json!({
                    "text": "panel A says strict",
                    "provider_route": {
                        "model": "risk-v1"
                    }
                }),
                error_payload: None,
                metrics_payload: serde_json::json!({
                    "usage": {
                        "total_tokens": 42
                    }
                }),
                debug_payload: serde_json::json!({
                    "llm_rounds": [
                        {
                            "round_index": 0,
                            "assistant": {
                                "content": "risk result"
                            }
                        }
                    ],
                    "provider_debug": "branch debug detail"
                }),
                started_at: OffsetDateTime::UNIX_EPOCH,
                finished_at: Some(OffsetDateTime::UNIX_EPOCH),
            },
        ],
        checkpoints: Vec::new(),
        callback_tasks: Vec::new(),
        events: Vec::new(),
        stitched_trace: Vec::new(),
    };
    let runtime_events = vec![test_runtime_event_record(
        flow_run_id,
        Some(main_node_run_id),
        "visible_internal_llm_tool_completed",
        serde_json::json!({
            "event_type": "visible_internal_llm_tool_completed",
            "main_node_id": "node-main-llm",
            "target_node_id": "node-panel-a",
            "tool_name": "fusion_review",
            "tool_call_id": "call_fusion",
            "tool_mode": "fusion",
            "execution_mode": "bounded_parallel_panel",
            "node_id": "node-panel-a",
            "node_alias": "LLM2",
            "node_type": "llm",
            "provider_route": {
                "model": "risk-v1"
            },
            "content": "panel A says strict"
        }),
    )];

    let detail =
        enrich_application_run_detail_visible_internal_llm_route_traces(detail, &runtime_events);

    let main_node = detail
        .node_runs
        .iter()
        .find(|node_run| node_run.id == main_node_run_id)
        .expect("main node run should stay visible");
    let branch_trace =
        &main_node.debug_payload["visible_internal_llm_tool_trace"][0]["branch_traces"][0];
    assert_eq!(
        branch_trace["input_payload"]["prompt_messages"][0]["content"],
        serde_json::json!("review refund policy")
    );
    assert_eq!(
        branch_trace["debug_payload"]["provider_debug"],
        serde_json::json!("branch debug detail")
    );
    assert_eq!(
        branch_trace["output_payload"]["text"],
        serde_json::json!("panel A says strict")
    );
    assert_eq!(
        branch_trace["metrics_payload"]["usage"]["total_tokens"],
        serde_json::json!(42)
    );
}

#[test]
fn run_detail_response_hides_historical_waiting_prefix_after_run_finishes() {
    let application = test_application_record();
    let flow_run_id = Uuid::now_v7();
    let waiting_node_run_id = Uuid::now_v7();
    let virtual_answer_node_run_id = Uuid::now_v7();
    let final_answer_node_run_id = Uuid::now_v7();
    let detail = domain::ApplicationRunDetail {
        flow_run: test_flow_run_record(
            application.id,
            flow_run_id,
            domain::FlowRunStatus::Succeeded,
            serde_json::json!({ "answer": "final answer" }),
        ),
        node_runs: vec![
            domain::NodeRunRecord {
                id: waiting_node_run_id,
                flow_run_id,
                node_id: "node-llm-2".to_string(),
                node_type: "llm".to_string(),
                node_alias: "LLM2".to_string(),
                status: domain::NodeRunStatus::Succeeded,
                input_payload: serde_json::json!({}),
                output_payload: serde_json::json!({ "text": "final answer" }),
                error_payload: None,
                metrics_payload: serde_json::json!({}),
                debug_payload: serde_json::json!({}),
                started_at: OffsetDateTime::UNIX_EPOCH,
                finished_at: Some(OffsetDateTime::UNIX_EPOCH),
            },
            domain::NodeRunRecord {
                id: virtual_answer_node_run_id,
                flow_run_id,
                node_id: "node-answer".to_string(),
                node_type: "answer".to_string(),
                node_alias: "Answer".to_string(),
                status: domain::NodeRunStatus::Succeeded,
                input_payload: serde_json::json!({
                    "presentation": {
                        "kind": "answer",
                        "complete": false,
                        "materialized_from": "waiting_prefix"
                    }
                }),
                output_payload: serde_json::json!({
                    "answer": "prefix answer"
                }),
                error_payload: None,
                metrics_payload: serde_json::json!({}),
                debug_payload: serde_json::json!({
                    "answer_presentation": {
                        "partial": true,
                        "materialized_from": "waiting_prefix"
                    }
                }),
                started_at: OffsetDateTime::UNIX_EPOCH,
                finished_at: Some(OffsetDateTime::UNIX_EPOCH),
            },
            domain::NodeRunRecord {
                id: final_answer_node_run_id,
                flow_run_id,
                node_id: "node-answer".to_string(),
                node_type: "answer".to_string(),
                node_alias: "Answer".to_string(),
                status: domain::NodeRunStatus::Succeeded,
                input_payload: serde_json::json!({}),
                output_payload: serde_json::json!({
                    "answer": "final answer"
                }),
                error_payload: None,
                metrics_payload: serde_json::json!({}),
                debug_payload: serde_json::json!({}),
                started_at: OffsetDateTime::UNIX_EPOCH,
                finished_at: Some(OffsetDateTime::UNIX_EPOCH),
            },
        ],
        checkpoints: vec![domain::CheckpointRecord {
            id: Uuid::now_v7(),
            flow_run_id,
            node_run_id: Some(waiting_node_run_id),
            status: "waiting_callback".to_string(),
            reason: "历史等待点".to_string(),
            locator_payload: serde_json::json!({
                "node_id": "node-llm-2"
            }),
            variable_snapshot: serde_json::json!({}),
            external_ref_payload: None,
            created_at: OffsetDateTime::UNIX_EPOCH,
        }],
        callback_tasks: Vec::new(),
        events: Vec::new(),
        stitched_trace: Vec::new(),
    };

    let response = to_application_run_detail_response(&application, detail);

    assert!(response.answer_snapshot.is_none());
    assert!(response
        .node_runs
        .iter()
        .all(|node_run| node_run.id != virtual_answer_node_run_id.to_string()));
    assert!(response
        .node_runs
        .iter()
        .any(|node_run| node_run.id == final_answer_node_run_id.to_string()));
}

#[test]
fn flow_run_response_exposes_query_and_model_short_fields() {
    let run = domain::FlowRunRecord {
        id: Uuid::now_v7(),
        application_id: Uuid::now_v7(),
        flow_id: Uuid::now_v7(),
        draft_id: Uuid::now_v7(),
        compiled_plan_id: None,
        debug_session_id: "debug-session".to_string(),
        flow_schema_version: "1flowbase.flow/v2".to_string(),
        document_hash: "hash".to_string(),
        run_mode: domain::FlowRunMode::PublishedApiRun,
        target_node_id: None,
        title: "say hello".to_string(),
        status: domain::FlowRunStatus::Succeeded,
        input_payload: serde_json::json!({
            "node-start": {
                "query": "say hello",
                "model": "deepseek-chat"
            }
        }),
        output_payload: serde_json::json!({ "answer": "hello" }),
        error_payload: None,
        created_by: Uuid::now_v7(),
        authorized_account: Some("root".to_string()),
        api_key_id: None,
        publication_version_id: None,
        external_user: Some("user-1".to_string()),
        external_conversation_id: Some("conversation-1".to_string()),
        external_trace_id: None,
        compatibility_mode: None,
        idempotency_key: None,
        started_at: OffsetDateTime::UNIX_EPOCH,
        finished_at: Some(OffsetDateTime::UNIX_EPOCH),
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    };

    let response = to_flow_run_response(run);

    assert_eq!(response.query.as_deref(), Some("say hello"));
    assert_eq!(response.model.as_deref(), Some("deepseek-chat"));
    assert_eq!(
        response.external_conversation_id.as_deref(),
        Some("conversation-1")
    );
}

#[tokio::test]
async fn run_conversation_without_external_conversation_id_reads_imported_history_and_current_turn()
{
    let run_id = Uuid::now_v7();
    let run = domain::FlowRunRecord {
        id: run_id,
        application_id: Uuid::now_v7(),
        flow_id: Uuid::now_v7(),
        draft_id: Uuid::now_v7(),
        compiled_plan_id: None,
        debug_session_id: "debug-session".to_string(),
        flow_schema_version: "1flowbase.flow/v2".to_string(),
        document_hash: "hash".to_string(),
        run_mode: domain::FlowRunMode::PublishedApiRun,
        target_node_id: None,
        title: "current question".to_string(),
        status: domain::FlowRunStatus::Succeeded,
        input_payload: serde_json::json!({
            "node-start": {
                "query": "current question",
                "model": "deepseek-chat",
                "history": [
                    { "role": "system", "content": "hidden" },
                    { "role": "user", "content": "old question 1" },
                    { "role": "assistant", "content": "old answer 1" },
                    { "role": "tool", "content": "tool payload" },
                    { "role": "user", "content": "old question 2" },
                    { "role": "assistant", "content": "old answer 2" }
                ]
            }
        }),
        output_payload: serde_json::json!({ "answer": "current answer" }),
        error_payload: None,
        created_by: Uuid::now_v7(),
        authorized_account: Some("root".to_string()),
        api_key_id: None,
        publication_version_id: None,
        external_user: None,
        external_conversation_id: None,
        external_trace_id: None,
        compatibility_mode: None,
        idempotency_key: None,
        started_at: OffsetDateTime::UNIX_EPOCH,
        finished_at: Some(OffsetDateTime::UNIX_EPOCH),
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    };

    let load_debug_artifact = |_| async { None::<serde_json::Value> };
    let page = conversation_messages_from_single_run(
        &run,
        &ApplicationConversationMessagesQuery {
            around_run_id: None,
            before: None,
            after: None,
            limit: Some(2),
        },
        &load_debug_artifact,
    )
    .await;

    assert_eq!(page.items.len(), 2);
    assert!(page.page.has_before);
    assert!(!page.page.has_after);
    let before_cursor = page
        .page
        .before_cursor
        .clone()
        .expect("initial page should expose earlier context cursor");
    assert_eq!(page.items[0].role.as_deref(), Some("assistant"));
    assert_eq!(page.items[0].content.as_deref(), Some("old answer 2"));
    assert_eq!(page.items[0].query, None);
    assert_eq!(page.items[0].answer, None);
    assert!(!page.items[0].can_open_detail);
    assert_eq!(page.items[0].detail_run_id, None);
    assert_eq!(page.items[1].run_id, run_id.to_string());
    assert_eq!(page.items[1].role, None);
    assert_eq!(page.items[1].content, None);
    assert_eq!(page.items[1].query.as_deref(), Some("current question"));
    assert_eq!(page.items[1].answer.as_deref(), Some("current answer"));
    assert!(page.items[1].can_open_detail);
    let run_id_string = run_id.to_string();
    assert_eq!(
        page.items[1].detail_run_id.as_deref(),
        Some(run_id_string.as_str())
    );

    let previous_page = conversation_messages_from_single_run(
        &run,
        &ApplicationConversationMessagesQuery {
            around_run_id: None,
            before: Some(before_cursor),
            after: None,
            limit: Some(5),
        },
        &load_debug_artifact,
    )
    .await;
    assert_eq!(previous_page.items.len(), 4);
    assert!(!previous_page.page.has_before);
    assert!(previous_page.page.has_after);
    assert_eq!(previous_page.items[0].role.as_deref(), Some("system"));
    assert_eq!(previous_page.items[0].content.as_deref(), Some("hidden"));
    assert_eq!(previous_page.items[1].role.as_deref(), Some("user"));
    assert_eq!(
        previous_page.items[1].content.as_deref(),
        Some("old question 1")
    );
}

#[tokio::test]
async fn run_conversation_hides_claude_code_control_history_from_imported_context() {
    let run_id = Uuid::now_v7();
    let run = domain::FlowRunRecord {
        id: run_id,
        application_id: Uuid::now_v7(),
        flow_id: Uuid::now_v7(),
        draft_id: Uuid::now_v7(),
        compiled_plan_id: None,
        debug_session_id: "debug-session".to_string(),
        flow_schema_version: "1flowbase.flow/v2".to_string(),
        document_hash: "hash".to_string(),
        run_mode: domain::FlowRunMode::PublishedApiRun,
        target_node_id: None,
        title: "current question".to_string(),
        status: domain::FlowRunStatus::Succeeded,
        input_payload: serde_json::json!({
            "node-start": {
                "query": "那你帮我拉一下最新代码",
                "history": [
                    {
                        "role": "user",
                        "content": "This session is being continued from a previous conversation that ran out of context. The summary below covers the earlier portion of the conversation.\n\nSummary: hi\n\nIf you need specific details from before compaction (like exact code snippets, error messages, or content you generated), read the full transcript at: C:\\Users\\Lw\\.claude\\projects\\repo\\session.jsonl"
                    },
                    {
                        "role": "assistant",
                        "content": "已恢复上下文。"
                    },
                    { "role": "user", "content": "visible old question" },
                    { "role": "assistant", "content": "visible old answer" }
                ]
            }
        }),
        output_payload: serde_json::json!({ "answer": "current answer" }),
        error_payload: None,
        created_by: Uuid::now_v7(),
        authorized_account: Some("root".to_string()),
        api_key_id: None,
        publication_version_id: None,
        external_user: None,
        external_conversation_id: None,
        external_trace_id: None,
        compatibility_mode: Some("anthropic-messages-v1".to_string()),
        idempotency_key: None,
        started_at: OffsetDateTime::UNIX_EPOCH,
        finished_at: Some(OffsetDateTime::UNIX_EPOCH),
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    };

    let load_debug_artifact = |_| async { None::<serde_json::Value> };
    let page = conversation_messages_from_single_run(
        &run,
        &ApplicationConversationMessagesQuery {
            around_run_id: None,
            before: None,
            after: None,
            limit: Some(5),
        },
        &load_debug_artifact,
    )
    .await;

    let visible_text = page
        .items
        .iter()
        .flat_map(|item| {
            [
                item.content.clone(),
                item.query.clone(),
                item.answer.clone(),
            ]
            .into_iter()
            .flatten()
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(!visible_text.contains("This session is being continued"));
    assert!(!visible_text.contains("已恢复上下文"));
    assert!(visible_text.contains("visible old question"));
    assert!(visible_text.contains("visible old answer"));
    assert!(visible_text.contains("那你帮我拉一下最新代码"));
    assert!(visible_text.contains("current answer"));
}

#[tokio::test]
async fn run_conversation_reads_llm_system_when_run_input_system_is_split_from_provider_messages() {
    let run_id = Uuid::now_v7();
    let application_id = Uuid::now_v7();
    let flow_run = domain::FlowRunRecord {
        id: run_id,
        application_id,
        flow_id: Uuid::now_v7(),
        draft_id: Uuid::now_v7(),
        compiled_plan_id: None,
        debug_session_id: "debug-session".to_string(),
        flow_schema_version: "1flowbase.flow/v2".to_string(),
        document_hash: "hash".to_string(),
        run_mode: domain::FlowRunMode::PublishedApiRun,
        target_node_id: None,
        title: "hi ?".to_string(),
        status: domain::FlowRunStatus::Succeeded,
        input_payload: serde_json::json!({
            "node-start": {
                "query": "hi ?",
                "model": "1flowbase"
            }
        }),
        output_payload: serde_json::json!({ "answer": "Hello!" }),
        error_payload: None,
        created_by: Uuid::now_v7(),
        authorized_account: Some("root".to_string()),
        api_key_id: None,
        publication_version_id: None,
        external_user: None,
        external_conversation_id: Some("conversation-1".to_string()),
        external_trace_id: None,
        compatibility_mode: Some("anthropic-compatible".to_string()),
        idempotency_key: None,
        started_at: OffsetDateTime::UNIX_EPOCH,
        finished_at: Some(OffsetDateTime::UNIX_EPOCH),
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    };
    let detail = domain::ApplicationRunDetail {
        flow_run,
        node_runs: vec![domain::NodeRunRecord {
            id: Uuid::now_v7(),
            flow_run_id: run_id,
            node_id: "node-llm".to_string(),
            node_type: "llm".to_string(),
            node_alias: "LLM".to_string(),
            status: domain::NodeRunStatus::Succeeded,
            input_payload: serde_json::json!({
                "prompt_messages": [
                    {
                        "id": "user-1",
                        "role": "user",
                        "content": "hi ?"
                    }
                ]
            }),
            output_payload: serde_json::json!({ "answer": "Hello!" }),
            error_payload: None,
            metrics_payload: serde_json::json!({}),
            debug_payload: serde_json::json!({
                "llm_context": {
                    "effective_system": "Use the image-aware system policy.",
                    "provider_messages": [
                        { "role": "user", "content": "hi ?" }
                    ]
                }
            }),
            started_at: OffsetDateTime::UNIX_EPOCH,
            finished_at: Some(OffsetDateTime::UNIX_EPOCH),
        }],
        checkpoints: Vec::new(),
        callback_tasks: Vec::new(),
        events: Vec::new(),
        stitched_trace: Vec::new(),
    };

    let load_debug_artifact = |_| async { None::<serde_json::Value> };
    let page = conversation_messages_from_run_detail(
        &detail,
        &ApplicationConversationMessagesQuery {
            around_run_id: None,
            before: None,
            after: None,
            limit: Some(5),
        },
        &load_debug_artifact,
    )
    .await;

    assert_eq!(page.items.len(), 2);
    assert!(!page.page.has_before);
    assert!(!page.page.has_after);
    assert_eq!(page.items[0].role.as_deref(), Some("system"));
    assert_eq!(
        page.items[0].content.as_deref(),
        Some("Use the image-aware system policy.")
    );
    assert!(!page.items[0].can_open_detail);
    assert_eq!(page.items[1].run_id, run_id.to_string());
    assert_eq!(page.items[1].query.as_deref(), Some("hi ?"));
    assert_eq!(page.items[1].answer.as_deref(), Some("Hello!"));
}

#[tokio::test]
async fn llm_prompt_messages_system_content_reads_system_prompt_message() {
    let payload = serde_json::json!({
        "prompt_messages": [
            {
                "id": "system-1",
                "role": "system",
                "content": "Use the node policy."
            },
            {
                "id": "user-1",
                "role": "user",
                "content": "hi ?"
            }
        ]
    });
    let load_debug_artifact = |_| async { None::<serde_json::Value> };

    assert_eq!(
        llm_prompt_messages_system_content(&payload, &load_debug_artifact)
            .await
            .as_deref(),
        Some("Use the node policy.")
    );
}

#[tokio::test]
async fn run_conversation_without_external_conversation_id_reads_artifact_backed_imported_history()
{
    let run_id = Uuid::now_v7();
    let artifact_id = Uuid::now_v7();
    let run = domain::FlowRunRecord {
        id: run_id,
        application_id: Uuid::now_v7(),
        flow_id: Uuid::now_v7(),
        draft_id: Uuid::now_v7(),
        compiled_plan_id: None,
        debug_session_id: "debug-session".to_string(),
        flow_schema_version: "1flowbase.flow/v2".to_string(),
        document_hash: "hash".to_string(),
        run_mode: domain::FlowRunMode::PublishedApiRun,
        target_node_id: None,
        title: "current question".to_string(),
        status: domain::FlowRunStatus::Succeeded,
        input_payload: serde_json::json!({
            "__runtime_debug_artifact": true,
            "artifact_ref": artifact_id.to_string(),
            "is_truncated": true,
            "query": "current question",
            "model": "deepseek-chat",
            "preview": "{\"node-start\":{\"query\":\"current question\""
        }),
        output_payload: serde_json::json!({ "answer": "current answer" }),
        error_payload: None,
        created_by: Uuid::now_v7(),
        authorized_account: Some("root".to_string()),
        api_key_id: None,
        publication_version_id: None,
        external_user: None,
        external_conversation_id: None,
        external_trace_id: None,
        compatibility_mode: None,
        idempotency_key: None,
        started_at: OffsetDateTime::UNIX_EPOCH,
        finished_at: Some(OffsetDateTime::UNIX_EPOCH),
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    };

    let load_debug_artifact = move |requested_artifact_id: Uuid| async move {
        (requested_artifact_id == artifact_id).then(|| {
            serde_json::json!({
                "node-start": {
                    "query": "current question",
                    "model": "deepseek-chat",
                    "history": [
                        { "role": "system", "content": "hidden" },
                        { "role": "user", "content": "old question" },
                        { "role": "assistant", "content": "old answer" }
                    ]
                }
            })
        })
    };
    let page = conversation_messages_from_single_run(
        &run,
        &ApplicationConversationMessagesQuery {
            around_run_id: None,
            before: None,
            after: None,
            limit: Some(5),
        },
        &load_debug_artifact,
    )
    .await;

    assert_eq!(page.items.len(), 4);
    assert_eq!(page.items[0].role.as_deref(), Some("system"));
    assert_eq!(page.items[0].content.as_deref(), Some("hidden"));
    assert!(!page.items[0].can_open_detail);
    assert_eq!(page.items[1].role.as_deref(), Some("user"));
    assert_eq!(page.items[1].content.as_deref(), Some("old question"));
    assert_eq!(page.items[2].role.as_deref(), Some("assistant"));
    assert_eq!(page.items[2].content.as_deref(), Some("old answer"));
    assert_eq!(page.items[3].run_id, run_id.to_string());
    assert_eq!(page.items[3].query.as_deref(), Some("current question"));
    assert!(page.items[3].can_open_detail);
}

#[tokio::test]
async fn run_conversation_hydrates_artifact_backed_current_answer() {
    let run_id = Uuid::now_v7();
    let artifact_id = Uuid::now_v7();
    let full_answer = "full final answer from artifact";
    let run = domain::FlowRunRecord {
        id: run_id,
        application_id: Uuid::now_v7(),
        flow_id: Uuid::now_v7(),
        draft_id: Uuid::now_v7(),
        compiled_plan_id: None,
        debug_session_id: "debug-session".to_string(),
        flow_schema_version: "1flowbase.flow/v2".to_string(),
        document_hash: "hash".to_string(),
        run_mode: domain::FlowRunMode::PublishedApiRun,
        target_node_id: None,
        title: "current question".to_string(),
        status: domain::FlowRunStatus::Succeeded,
        input_payload: serde_json::json!({
            "node-start": {
                "query": "current question",
                "model": "deepseek-chat"
            }
        }),
        output_payload: serde_json::json!({
            "answer": {
                "__runtime_debug_artifact": true,
                "artifact_ref": artifact_id.to_string(),
                "field_path": ["answer"],
                "preview": "preview final answer"
            }
        }),
        error_payload: None,
        created_by: Uuid::now_v7(),
        authorized_account: Some("root".to_string()),
        api_key_id: None,
        publication_version_id: None,
        external_user: None,
        external_conversation_id: None,
        external_trace_id: None,
        compatibility_mode: None,
        idempotency_key: None,
        started_at: OffsetDateTime::UNIX_EPOCH,
        finished_at: Some(OffsetDateTime::UNIX_EPOCH),
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    };

    let load_debug_artifact = move |requested_artifact_id: Uuid| async move {
        (requested_artifact_id == artifact_id)
            .then(|| serde_json::Value::String(full_answer.to_string()))
    };
    let page = conversation_messages_from_single_run(
        &run,
        &ApplicationConversationMessagesQuery {
            around_run_id: None,
            before: None,
            after: None,
            limit: Some(5),
        },
        &load_debug_artifact,
    )
    .await;

    let current = page.items.last().expect("current run message exists");
    assert_eq!(current.answer.as_deref(), Some(full_answer));
}
