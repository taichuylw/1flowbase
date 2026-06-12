use super::super::protocol_mappers::{
    anthropic_tool_use_blocks_from_waiting_payload, AnthropicStreamMapper, OpenAiChatStreamMapper,
    OpenAiResponseStreamMapper,
};
use super::super::*;
use super::support::*;
use crate::routes::application_public_api::stream_terminal_fallback::recover_terminal_answer_deltas_from_durable_runtime_events;
use control_plane::{
    application_public_api::native::{NativeRequiredAction, NativeRunStatus},
    ports::{RuntimeEventDurability, RuntimeEventPayload, RuntimeEventSource},
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

#[tokio::test]
async fn openai_chat_durable_waiting_callback_fallback_drains_text_delta_first() {
    let mut run = native_run();
    let node_run_id = Uuid::from_u128(0x55555555555555555555555555555555);
    let callback_task_id = Uuid::from_u128(0x66666666666666666666666666666666);
    run.status = NativeRunStatus::Waiting;
    run.tool_calls = Some(json!([
        {
            "id": "call_next",
            "name": "lookup_next",
            "arguments": { "query": "next" }
        }
    ]));
    run.required_action = Some(NativeRequiredAction {
        action_type: "submit_tool_outputs".to_string(),
        payload: json!({
            "callback_task_id": callback_task_id,
            "callback_kind": "llm_tool_calls",
            "node_run_id": node_run_id,
            "tool_calls": run.tool_calls.clone().unwrap()
        }),
    });

    let stream_events = vec![
        RuntimeEventEnvelope::new(
            run.id,
            1,
            debug_stream_events::answer_text_delta(
                "node-answer",
                "prior node answer".to_string(),
                0,
                Some("node-llm"),
                Some(node_run_id),
                Some("text"),
            ),
        ),
        RuntimeEventEnvelope::new(
            run.id,
            2,
            RuntimeEventPayload {
                event_type: "waiting_callback".to_string(),
                source: RuntimeEventSource::Runtime,
                durability: RuntimeEventDurability::DurableRequired,
                persist_required: true,
                trace_visible: true,
                payload: json!({
                    "type": "waiting_callback",
                    "run_id": run.id,
                    "status": "waiting_callback",
                    "callback_task_id": callback_task_id,
                    "callback_kind": "llm_tool_calls",
                    "node_run_id": node_run_id,
                    "tool_calls": run.tool_calls.clone().unwrap()
                }),
            },
        ),
    ];
    let runtime_event_stream = Arc::new(ReplayBeforeFallbackRuntimeEventStream::new(stream_events));
    let (base_state, _) = crate::_tests::support::test_api_state_with_database_url().await;
    let state = Arc::new(ApiState {
        store: base_state.store.clone(),
        infrastructure: base_state.infrastructure.clone(),
        file_storage_registry: base_state.file_storage_registry.clone(),
        runtime_engine: base_state.runtime_engine.clone(),
        provider_runtime: base_state.provider_runtime.clone(),
        process_started_at: base_state.process_started_at,
        runtime_activity: base_state.runtime_activity.clone(),
        api_runtime_profile: base_state.api_runtime_profile.clone(),
        plugin_runner_system: base_state.plugin_runner_system.clone(),
        official_plugin_source: base_state.official_plugin_source.clone(),
        provider_install_root: base_state.provider_install_root.clone(),
        provider_secret_master_key: base_state.provider_secret_master_key.clone(),
        host_extension_dropin_root: base_state.host_extension_dropin_root.clone(),
        allow_unverified_filesystem_dropins: base_state.allow_unverified_filesystem_dropins,
        allow_uploaded_host_extensions: base_state.allow_uploaded_host_extensions,
        session_store: base_state.session_store.clone(),
        runtime_event_stream,
        api_docs: base_state.api_docs.clone(),
        cookie_name: base_state.cookie_name.clone(),
        cookie_secure: base_state.cookie_secure,
        session_ttl_days: base_state.session_ttl_days,
        bootstrap_workspace_name: base_state.bootstrap_workspace_name.clone(),
    });
    let (sender, mut receiver) = mpsc::channel(32);
    let mut mapper =
        OpenAiChatStreamMapper::new("1flowbase".to_string(), "chatcmpl-test".to_string(), true);

    tokio::time::timeout(
        Duration::from_secs(2),
        send_compatible_runtime_event_stream(
            state,
            run.clone(),
            OPENAI_CHAT_SSE_PROJECTION,
            Some(0),
            None,
            sender,
            move |run, envelope| mapper.runtime_event_to_sse(run, envelope),
        ),
    )
    .await
    .expect("compatible stream should stop at replayed waiting callback");

    let mut events = Vec::new();
    while let Some(event) = receiver.recv().await {
        events.push(event);
    }
    let response = completed_compatible_stream(events);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    assert!(body.contains("prior node answer"), "{body}");
    assert!(body.contains("lookup_next"), "{body}");
    assert!(body.contains("\"finish_reason\":\"tool_calls\""), "{body}");
    assert!(body.contains("[DONE]"), "{body}");
}

#[tokio::test]
async fn openai_chat_waiting_internal_llm_tool_callback_finishes_without_tool_calls() {
    let mut run = native_run();
    let callback_task_id = Uuid::from_u128(0x12121212121212121212121212121212);
    run.status = NativeRunStatus::Waiting;
    run.answer = Some("visible internal LLM output".to_string());
    run.tool_calls = Some(json!([
        {
            "id": "call_internal",
            "type": "visible_internal_llm_tool",
            "name": "inspect_visible_context",
            "arguments": { "query": "visible" }
        }
    ]));

    let mut mapper =
        OpenAiChatStreamMapper::new("1flowbase".to_string(), "chatcmpl-test".to_string(), true);
    let mut events = mapper.runtime_event_to_sse(
        &run,
        RuntimeEventEnvelope::new(run.id, 1, debug_stream_events::flow_started(run.id)),
    );
    events.extend(mapper.runtime_event_to_sse(
        &run,
        RuntimeEventEnvelope::new(
            run.id,
            2,
            RuntimeEventPayload {
                event_type: "waiting_callback".to_string(),
                source: RuntimeEventSource::Runtime,
                durability: RuntimeEventDurability::DurableRequired,
                persist_required: true,
                trace_visible: true,
                payload: json!({
                    "type": "waiting_callback",
                    "run_id": run.id,
                    "status": "waiting_callback",
                    "callback_task_id": callback_task_id,
                    "callback_kind": "llm_tool_calls",
                    "tool_calls": run.tool_calls.clone().unwrap()
                }),
            },
        ),
    ));

    let response = completed_compatible_stream(events);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    assert!(body.contains("\"finish_reason\":\"stop\""), "{body}");
    assert!(!body.contains("\"tool_calls\""), "{body}");
    assert!(!body.contains("required_action_not_supported"), "{body}");
    assert!(body.contains("[DONE]"), "{body}");
}

#[tokio::test]
async fn terminal_answer_recovery_prefers_durable_answer_presentation() {
    let run = native_run();
    let (base_state, _) = crate::_tests::support::test_api_state_with_database_url().await;
    seed_flow_run_for_compat_sse_test(&base_state, &run).await;
    append_compat_sse_runtime_event(
        &base_state,
        run.id,
        "text_delta",
        json!({
            "type": "text_delta",
            "event_type": "text_delta",
            "node_id": "node-answer",
            "text": "durable presentation answer",
            "presentation": {
                "kind": "answer",
                "answer_node_id": "node-answer",
                "source_node_id": "node-llm",
                "source_output_key": "text",
                "segment_index": 0
            }
        }),
    )
    .await;
    append_compat_sse_runtime_event(
        &base_state,
        run.id,
        "flow_finished",
        json!({
            "type": "flow_finished",
            "run_id": run.id,
            "status": "succeeded",
            "output": { "answer": "terminal output answer" }
        }),
    )
    .await;

    let deltas =
        recover_terminal_answer_deltas_from_durable_runtime_events(&base_state, &run).await;

    assert_eq!(deltas.len(), 1);
    assert_eq!(deltas[0].kind, TerminalAnswerDeltaKind::Text);
    assert_eq!(deltas[0].text, "durable presentation answer");
}

#[tokio::test]
async fn openai_chat_resume_replay_terminal_drains_durable_text_before_tool_call() {
    let mut run = native_run();
    let node_run_id = Uuid::from_u128(0x77777777777777777777777777777777);
    let previous_callback_task_id = Uuid::from_u128(0x88888888888888888888888888888888);
    let next_callback_task_id = Uuid::from_u128(0x99999999999999999999999999999999);
    run.status = NativeRunStatus::Waiting;
    run.tool_calls = Some(json!([
        {
            "id": "call_next",
            "name": "lookup_next",
            "arguments": { "query": "next" }
        }
    ]));
    run.required_action = Some(NativeRequiredAction {
        action_type: "submit_tool_outputs".to_string(),
        payload: json!({
            "callback_task_id": next_callback_task_id,
            "callback_kind": "llm_tool_calls",
            "node_run_id": node_run_id,
            "tool_calls": run.tool_calls.clone().unwrap()
        }),
    });

    let (base_state, _) = crate::_tests::support::test_api_state_with_database_url().await;
    seed_flow_run_for_compat_sse_test(&base_state, &run).await;
    append_compat_sse_runtime_event(
        &base_state,
        run.id,
        "waiting_callback",
        json!({
            "type": "waiting_callback",
            "run_id": run.id,
            "status": "waiting_callback",
            "callback_task_id": previous_callback_task_id,
            "callback_kind": "llm_tool_calls",
            "node_run_id": node_run_id,
            "tool_calls": [
                {
                    "id": "call_previous",
                    "name": "lookup_previous",
                    "arguments": { "query": "previous" }
                }
            ]
        }),
    )
    .await;
    append_compat_sse_runtime_event(
        &base_state,
        run.id,
        "text_delta",
        json!({
            "type": "text_delta",
            "event_type": "text_delta",
            "node_id": "node-answer",
            "text": "prior node answer",
            "presentation": {
                "kind": "answer",
                "answer_node_id": "node-answer",
                "source_node_id": "node-llm",
                "source_node_run_id": node_run_id,
                "source_output_key": "text",
                "segment_index": 0
            },
            "stream_sequence": 2,
            "sequence_start": 2,
            "sequence_end": 2
        }),
    )
    .await;
    append_compat_sse_runtime_event(
        &base_state,
        run.id,
        "waiting_callback",
        json!({
            "type": "waiting_callback",
            "run_id": run.id,
            "status": "waiting_callback",
            "callback_task_id": next_callback_task_id,
            "callback_kind": "llm_tool_calls",
            "node_run_id": node_run_id,
            "tool_calls": run.tool_calls.clone().unwrap()
        }),
    )
    .await;

    let subscription_replay = vec![
        RuntimeEventEnvelope::new(run.id, 1, debug_stream_events::flow_started(run.id)),
        RuntimeEventEnvelope::new(
            run.id,
            2,
            RuntimeEventPayload {
                event_type: "waiting_callback".to_string(),
                source: RuntimeEventSource::Runtime,
                durability: RuntimeEventDurability::DurableRequired,
                persist_required: true,
                trace_visible: true,
                payload: json!({
                    "type": "waiting_callback",
                    "run_id": run.id,
                    "status": "waiting_callback",
                    "callback_task_id": next_callback_task_id,
                    "callback_kind": "llm_tool_calls",
                    "node_run_id": node_run_id,
                    "tool_calls": run.tool_calls.clone().unwrap()
                }),
            },
        ),
    ];
    let runtime_event_stream = Arc::new(
        ReplayBeforeFallbackRuntimeEventStream::with_subscription_replay(
            subscription_replay,
            Vec::new(),
        ),
    );
    let state = Arc::new(ApiState {
        store: base_state.store.clone(),
        infrastructure: base_state.infrastructure.clone(),
        file_storage_registry: base_state.file_storage_registry.clone(),
        runtime_engine: base_state.runtime_engine.clone(),
        provider_runtime: base_state.provider_runtime.clone(),
        process_started_at: base_state.process_started_at,
        runtime_activity: base_state.runtime_activity.clone(),
        api_runtime_profile: base_state.api_runtime_profile.clone(),
        plugin_runner_system: base_state.plugin_runner_system.clone(),
        official_plugin_source: base_state.official_plugin_source.clone(),
        provider_install_root: base_state.provider_install_root.clone(),
        provider_secret_master_key: base_state.provider_secret_master_key.clone(),
        host_extension_dropin_root: base_state.host_extension_dropin_root.clone(),
        allow_unverified_filesystem_dropins: base_state.allow_unverified_filesystem_dropins,
        allow_uploaded_host_extensions: base_state.allow_uploaded_host_extensions,
        session_store: base_state.session_store.clone(),
        runtime_event_stream,
        api_docs: base_state.api_docs.clone(),
        cookie_name: base_state.cookie_name.clone(),
        cookie_secure: base_state.cookie_secure,
        session_ttl_days: base_state.session_ttl_days,
        bootstrap_workspace_name: base_state.bootstrap_workspace_name.clone(),
    });
    let (sender, mut receiver) = mpsc::channel(32);
    let mut mapper =
        OpenAiChatStreamMapper::new("1flowbase".to_string(), "chatcmpl-test".to_string(), true);

    tokio::time::timeout(
        Duration::from_secs(2),
        send_compatible_runtime_event_stream(
            state,
            run.clone(),
            OPENAI_CHAT_SSE_PROJECTION,
            Some(0),
            Some(previous_callback_task_id),
            sender,
            move |run, envelope| mapper.runtime_event_to_sse(run, envelope),
        ),
    )
    .await
    .expect("compatible stream should stop at replayed waiting callback");

    let mut events = Vec::new();
    while let Some(event) = receiver.recv().await {
        events.push(event);
    }
    let response = completed_compatible_stream(events);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    let text_index = body.find("prior node answer").unwrap_or_else(|| {
        panic!("resume stream should include prior LLM text before tool call: {body}")
    });
    let tool_index = body
        .find("lookup_next")
        .unwrap_or_else(|| panic!("resume stream should include next tool call: {body}"));
    assert!(
        text_index < tool_index,
        "prior LLM text should be projected before the next tool call: {body}"
    );
    assert!(body.contains("\"finish_reason\":\"tool_calls\""), "{body}");
    assert!(body.contains("[DONE]"), "{body}");
}

#[tokio::test]
async fn anthropic_resume_replay_terminal_drains_route_callback_as_tool_use() {
    let mut run = native_run();
    let route_node_run_id = Uuid::from_u128(0x77777777777777777777777777777777);
    let previous_callback_task_id = Uuid::from_u128(0x88888888888888888888888888888888);
    let route_callback_task_id = Uuid::from_u128(0x99999999999999999999999999999999);
    run.status = NativeRunStatus::Waiting;
    run.tool_calls = Some(json!([
        {
            "id": "call_route_read",
            "name": "Read",
            "arguments": {
                "file_path": "uploads/agent-flow-node-detail-icon-aligned.png"
            }
        }
    ]));
    run.required_action = Some(NativeRequiredAction {
        action_type: "submit_tool_outputs".to_string(),
        payload: json!({
            "callback_task_id": route_callback_task_id,
            "callback_kind": "llm_tool_calls",
            "node_run_id": route_node_run_id,
            "tool_calls": run.tool_calls.clone().unwrap()
        }),
    });

    let (base_state, _) = crate::_tests::support::test_api_state_with_database_url().await;
    seed_flow_run_for_compat_sse_test(&base_state, &run).await;
    append_compat_sse_runtime_event(
        &base_state,
        run.id,
        "waiting_callback",
        json!({
            "type": "waiting_callback",
            "run_id": run.id,
            "status": "waiting_callback",
            "callback_task_id": previous_callback_task_id,
            "callback_kind": "llm_tool_calls",
            "node_run_id": route_node_run_id,
            "tool_calls": [
                {
                    "id": "call_previous",
                    "name": "Read",
                    "arguments": { "file_path": "uploads/old.png" }
                }
            ]
        }),
    )
    .await;
    append_compat_sse_runtime_event(
        &base_state,
        run.id,
        "waiting_callback",
        json!({
            "type": "waiting_callback",
            "run_id": run.id,
            "status": "waiting_callback",
            "callback_task_id": route_callback_task_id,
            "callback_kind": "llm_tool_calls",
            "node_id": "node-llm-1",
            "node_run_id": route_node_run_id,
            "tool_calls": run.tool_calls.clone().unwrap()
        }),
    )
    .await;

    let subscription_replay = vec![
        RuntimeEventEnvelope::new(run.id, 1, debug_stream_events::flow_started(run.id)),
        RuntimeEventEnvelope::new(
            run.id,
            2,
            RuntimeEventPayload {
                event_type: "waiting_callback".to_string(),
                source: RuntimeEventSource::Runtime,
                durability: RuntimeEventDurability::DurableRequired,
                persist_required: true,
                trace_visible: true,
                payload: json!({
                    "type": "waiting_callback",
                    "run_id": run.id,
                    "status": "waiting_callback",
                    "callback_task_id": route_callback_task_id,
                    "callback_kind": "llm_tool_calls",
                    "node_id": "node-llm-1",
                    "node_run_id": route_node_run_id,
                    "tool_calls": run.tool_calls.clone().unwrap()
                }),
            },
        ),
    ];
    let runtime_event_stream = Arc::new(
        ReplayBeforeFallbackRuntimeEventStream::with_subscription_replay(
            subscription_replay,
            Vec::new(),
        ),
    );
    let state = Arc::new(ApiState {
        store: base_state.store.clone(),
        infrastructure: base_state.infrastructure.clone(),
        file_storage_registry: base_state.file_storage_registry.clone(),
        runtime_engine: base_state.runtime_engine.clone(),
        provider_runtime: base_state.provider_runtime.clone(),
        process_started_at: base_state.process_started_at,
        runtime_activity: base_state.runtime_activity.clone(),
        api_runtime_profile: base_state.api_runtime_profile.clone(),
        plugin_runner_system: base_state.plugin_runner_system.clone(),
        official_plugin_source: base_state.official_plugin_source.clone(),
        provider_install_root: base_state.provider_install_root.clone(),
        provider_secret_master_key: base_state.provider_secret_master_key.clone(),
        host_extension_dropin_root: base_state.host_extension_dropin_root.clone(),
        allow_unverified_filesystem_dropins: base_state.allow_unverified_filesystem_dropins,
        allow_uploaded_host_extensions: base_state.allow_uploaded_host_extensions,
        session_store: base_state.session_store.clone(),
        runtime_event_stream,
        api_docs: base_state.api_docs.clone(),
        cookie_name: base_state.cookie_name.clone(),
        cookie_secure: base_state.cookie_secure,
        session_ttl_days: base_state.session_ttl_days,
        bootstrap_workspace_name: base_state.bootstrap_workspace_name.clone(),
    });
    let (sender, mut receiver) = mpsc::channel(32);
    let mut mapper = AnthropicStreamMapper::new("1flowbase".to_string());

    tokio::time::timeout(
        Duration::from_secs(2),
        send_compatible_runtime_event_stream(
            state,
            run.clone(),
            ANTHROPIC_SSE_PROJECTION,
            Some(0),
            Some(previous_callback_task_id),
            sender,
            move |run, envelope| mapper.runtime_event_to_sse(run, envelope),
        ),
    )
    .await
    .expect("Anthropic stream should stop at replayed route callback");

    let mut events = Vec::new();
    while let Some(event) = receiver.recv().await {
        events.push(event);
    }
    let response = completed_compatible_stream(events);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    assert!(body.contains("event: message_start"), "{body}");
    assert!(body.contains("\"type\":\"tool_use\""), "{body}");
    assert!(body.contains("\"name\":\"Read\""), "{body}");
    assert!(
        body.contains("agent-flow-node-detail-icon-aligned.png"),
        "{body}"
    );
    assert!(body.contains("\"stop_reason\":\"tool_use\""), "{body}");
    assert!(body.contains("event: message_stop"), "{body}");
    assert!(!body.contains("uploads/old.png"), "{body}");
}

#[tokio::test]
async fn openai_chat_live_answer_delta_is_not_duplicated_by_durable_drain() {
    let mut run = native_run();
    let node_run_id = Uuid::from_u128(0x77777777777777777777777777777777);
    let callback_task_id = Uuid::from_u128(0x99999999999999999999999999999999);
    run.status = NativeRunStatus::Waiting;
    run.tool_calls = Some(json!([
        {
            "id": "call_next",
            "name": "lookup_next",
            "arguments": { "query": "next" }
        }
    ]));
    run.required_action = Some(NativeRequiredAction {
        action_type: "submit_tool_outputs".to_string(),
        payload: json!({
            "callback_task_id": callback_task_id,
            "callback_kind": "llm_tool_calls",
            "node_run_id": node_run_id,
            "tool_calls": run.tool_calls.clone().unwrap()
        }),
    });

    let (base_state, _) = crate::_tests::support::test_api_state_with_database_url().await;
    seed_flow_run_for_compat_sse_test(&base_state, &run).await;
    append_compat_sse_runtime_event(
        &base_state,
        run.id,
        "text_delta",
        json!({
            "type": "text_delta",
            "event_type": "text_delta",
            "node_id": "node-answer",
            "text": "prior node answer",
            "presentation": {
                "kind": "answer",
                "answer_node_id": "node-answer",
                "source_node_id": "node-llm",
                "source_node_run_id": node_run_id,
                "source_output_key": "text",
                "segment_index": 0
            }
        }),
    )
    .await;
    append_compat_sse_runtime_event(
        &base_state,
        run.id,
        "waiting_callback",
        json!({
            "type": "waiting_callback",
            "run_id": run.id,
            "status": "waiting_callback",
            "callback_task_id": callback_task_id,
            "callback_kind": "llm_tool_calls",
            "node_run_id": node_run_id,
            "tool_calls": run.tool_calls.clone().unwrap()
        }),
    )
    .await;

    let subscription_replay = vec![
        RuntimeEventEnvelope::new(run.id, 1, debug_stream_events::flow_started(run.id)),
        RuntimeEventEnvelope::new(
            run.id,
            2,
            debug_stream_events::answer_text_delta(
                "node-answer",
                "prior node answer".to_string(),
                0,
                Some("node-llm"),
                Some(node_run_id),
                Some("text"),
            ),
        ),
        RuntimeEventEnvelope::new(
            run.id,
            3,
            RuntimeEventPayload {
                event_type: "waiting_callback".to_string(),
                source: RuntimeEventSource::Runtime,
                durability: RuntimeEventDurability::DurableRequired,
                persist_required: true,
                trace_visible: true,
                payload: json!({
                    "type": "waiting_callback",
                    "run_id": run.id,
                    "status": "waiting_callback",
                    "callback_task_id": callback_task_id,
                    "callback_kind": "llm_tool_calls",
                    "node_run_id": node_run_id,
                    "tool_calls": run.tool_calls.clone().unwrap()
                }),
            },
        ),
    ];
    let runtime_event_stream = Arc::new(
        ReplayBeforeFallbackRuntimeEventStream::with_subscription_replay(
            subscription_replay,
            Vec::new(),
        ),
    );
    let state = Arc::new(ApiState {
        store: base_state.store.clone(),
        infrastructure: base_state.infrastructure.clone(),
        file_storage_registry: base_state.file_storage_registry.clone(),
        runtime_engine: base_state.runtime_engine.clone(),
        provider_runtime: base_state.provider_runtime.clone(),
        process_started_at: base_state.process_started_at,
        runtime_activity: base_state.runtime_activity.clone(),
        api_runtime_profile: base_state.api_runtime_profile.clone(),
        plugin_runner_system: base_state.plugin_runner_system.clone(),
        official_plugin_source: base_state.official_plugin_source.clone(),
        provider_install_root: base_state.provider_install_root.clone(),
        provider_secret_master_key: base_state.provider_secret_master_key.clone(),
        host_extension_dropin_root: base_state.host_extension_dropin_root.clone(),
        allow_unverified_filesystem_dropins: base_state.allow_unverified_filesystem_dropins,
        allow_uploaded_host_extensions: base_state.allow_uploaded_host_extensions,
        session_store: base_state.session_store.clone(),
        runtime_event_stream,
        api_docs: base_state.api_docs.clone(),
        cookie_name: base_state.cookie_name.clone(),
        cookie_secure: base_state.cookie_secure,
        session_ttl_days: base_state.session_ttl_days,
        bootstrap_workspace_name: base_state.bootstrap_workspace_name.clone(),
    });
    let (sender, mut receiver) = mpsc::channel(32);
    let mut mapper =
        OpenAiChatStreamMapper::new("1flowbase".to_string(), "chatcmpl-test".to_string(), true);

    tokio::time::timeout(
        Duration::from_secs(2),
        send_compatible_runtime_event_stream(
            state,
            run.clone(),
            OPENAI_CHAT_SSE_PROJECTION,
            Some(0),
            None,
            sender,
            move |run, envelope| mapper.runtime_event_to_sse(run, envelope),
        ),
    )
    .await
    .expect("compatible stream should stop at replayed waiting callback");

    let mut events = Vec::new();
    while let Some(event) = receiver.recv().await {
        events.push(event);
    }
    let response = completed_compatible_stream(events);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    assert_eq!(body.matches("prior node answer").count(), 1, "{body}");
    assert!(body.contains("lookup_next"), "{body}");
    assert!(body.contains("\"finish_reason\":\"tool_calls\""), "{body}");
    assert!(body.contains("[DONE]"), "{body}");
}

#[test]
fn openai_responses_resume_terminal_answer_fallback_emits_output_delta() {
    let run = native_run();
    let mut mapper = OpenAiResponseStreamMapper::new("1flowbase".to_string(), None, true);
    let events = mapper.runtime_event_to_sse(
        &run,
        RuntimeEventEnvelope::new(
            run.id,
            1,
            debug_stream_events::flow_finished(run.id, json!({ "answer": "最终回答" })),
        ),
    );

    // output_item.added + output_text.delta + output_item.done + response.completed
    assert_eq!(events.len(), 4);
}

#[tokio::test]
async fn openai_chat_failed_terminal_with_answer_finishes_without_error_event() {
    let mut run = native_run();
    run.status = NativeRunStatus::Failed;
    run.answer = Some("工具失败后的回答".to_string());
    let mut mapper =
        OpenAiChatStreamMapper::new("1flowbase".to_string(), "chatcmpl-test".to_string(), true);
    let events = mapper.runtime_event_to_sse(
        &run,
        RuntimeEventEnvelope::new(
            run.id,
            1,
            debug_stream_events::flow_failed(run.id, json!({ "message": "tool callback failed" })),
        ),
    );

    let response = completed_compatible_stream(events);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    assert!(body.contains("工具失败后的回答"), "{body}");
    assert!(body.contains("\"finish_reason\":\"stop\""), "{body}");
    assert!(!body.contains("\"error\""), "{body}");
    assert!(body.contains("[DONE]"), "{body}");
}

#[tokio::test]
async fn openai_responses_failed_terminal_with_answer_completes_without_failed_event() {
    let mut run = native_run();
    run.status = NativeRunStatus::Failed;
    run.answer = Some("工具失败后的回答".to_string());
    let mut mapper = OpenAiResponseStreamMapper::new("1flowbase".to_string(), None, true);
    let events = mapper.runtime_event_to_sse(
        &run,
        RuntimeEventEnvelope::new(
            run.id,
            1,
            debug_stream_events::flow_failed(run.id, json!({ "message": "tool callback failed" })),
        ),
    );

    let response = completed_compatible_stream(events);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    assert!(body.contains("event: response.output_text.delta"), "{body}");
    assert!(body.contains("工具失败后的回答"), "{body}");
    assert!(body.contains("event: response.completed"), "{body}");
    assert!(!body.contains("event: response.failed"), "{body}");
}

#[tokio::test]
async fn openai_responses_waiting_callback_streams_function_call_added_done_and_completed() {
    let run = native_run();
    let callback_task_id = Uuid::from_u128(0xcccccccccccccccccccccccccccccccc);
    let mut mapper = OpenAiResponseStreamMapper::new("1flowbase".to_string(), None, false);
    let events = mapper.runtime_event_to_sse(
        &run,
        RuntimeEventEnvelope::new(
            run.id,
            1,
            RuntimeEventPayload {
                event_type: "waiting_callback".to_string(),
                source: RuntimeEventSource::Runtime,
                durability: RuntimeEventDurability::DurableRequired,
                persist_required: true,
                trace_visible: true,
                payload: json!({
                    "type": "waiting_callback",
                    "run_id": run.id,
                    "status": "waiting_callback",
                    "callback_task_id": callback_task_id,
                    "callback_kind": "llm_tool_calls",
                    "tool_calls": [
                        {
                            "id": "call_inventory",
                            "name": "lookup_inventory",
                            "arguments": {"sku": "sku_123"}
                        }
                    ]
                }),
            },
        ),
    );

    let response = completed_compatible_stream(events);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    let added_index = body
        .find("event: response.output_item.added")
        .unwrap_or_else(|| panic!("Responses function_call should emit output_item.added: {body}"));
    let done_index = body
        .find("event: response.output_item.done")
        .unwrap_or_else(|| panic!("Responses function_call should emit output_item.done: {body}"));
    let completed_index = body
        .find("event: response.completed")
        .unwrap_or_else(|| panic!("Responses function_call should complete the response: {body}"));
    assert!(
        added_index < done_index && done_index < completed_index,
        "Responses function_call events should follow added -> done -> completed: {body}"
    );
    assert!(body.contains("\"type\":\"function_call\""), "{body}");
    assert!(body.contains("\"name\":\"lookup_inventory\""), "{body}");
    assert!(
        body.contains("\"arguments\":\"{\\\"sku\\\":\\\"sku_123\\\"}\""),
        "{body}"
    );
}

#[tokio::test]
async fn openai_responses_waiting_internal_llm_tool_callback_completes_without_function_call() {
    let mut run = native_run();
    let callback_task_id = Uuid::from_u128(0x34343434343434343434343434343434);
    run.status = NativeRunStatus::Waiting;
    run.answer = Some("visible internal LLM output".to_string());
    run.tool_calls = Some(json!([
        {
            "id": "call_internal",
            "visibility": "internal",
            "origin": "visible_internal_llm_tool",
            "name": "inspect_visible_context",
            "arguments": { "query": "visible" }
        }
    ]));

    let mut mapper = OpenAiResponseStreamMapper::new("1flowbase".to_string(), None, true);
    let mut events = mapper.runtime_event_to_sse(
        &run,
        RuntimeEventEnvelope::new(run.id, 1, debug_stream_events::flow_started(run.id)),
    );
    events.extend(mapper.runtime_event_to_sse(
        &run,
        RuntimeEventEnvelope::new(
            run.id,
            2,
            RuntimeEventPayload {
                event_type: "waiting_callback".to_string(),
                source: RuntimeEventSource::Runtime,
                durability: RuntimeEventDurability::DurableRequired,
                persist_required: true,
                trace_visible: true,
                payload: json!({
                    "type": "waiting_callback",
                    "run_id": run.id,
                    "status": "waiting_callback",
                    "callback_task_id": callback_task_id,
                    "callback_kind": "llm_tool_calls",
                    "tool_calls": run.tool_calls.clone().unwrap()
                }),
            },
        ),
    ));

    let response = completed_compatible_stream(events);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    assert!(body.contains("event: response.completed"), "{body}");
    assert!(!body.contains("\"type\":\"function_call\""), "{body}");
    assert!(!body.contains("required_action_not_supported"), "{body}");
}

#[test]
fn anthropic_waiting_callback_maps_to_tool_use_block() {
    let callback_task_id = Uuid::from_u128(0xcccccccccccccccccccccccccccccccc);
    let blocks = anthropic_tool_use_blocks_from_waiting_payload(&json!({
        "callback_kind": "llm_tool_calls",
        "callback_task_id": callback_task_id,
        "tool_calls": [
            {
                "id": "toolu_weather",
                "name": "lookup_weather",
                "arguments": {"city": "Hangzhou"}
            }
        ]
    }))
    .expect("LLM callback should map to Anthropic tool_use blocks");

    assert_eq!(blocks[0]["type"], json!("tool_use"));
    assert_eq!(blocks[0]["name"], json!("lookup_weather"));
    assert_eq!(blocks[0]["input"]["city"], json!("Hangzhou"));
    assert!(blocks[0]["id"]
        .as_str()
        .expect("tool_use id should be encoded")
        .contains("toolu_weather"));
}

#[tokio::test]
async fn anthropic_waiting_internal_llm_tool_callback_streams_text_without_tool_use() {
    let mut run = native_run();
    let callback_task_id = Uuid::from_u128(0x56565656565656565656565656565656);
    run.status = NativeRunStatus::Waiting;
    run.answer = Some("visible internal LLM output".to_string());
    run.tool_calls = Some(json!([
        {
            "id": "toolu_internal",
            "metadata": {
                "visibility": "internal",
                "origin": "visible_internal_llm_tool"
            },
            "name": "inspect_visible_context",
            "arguments": { "query": "visible" }
        }
    ]));

    let mut mapper = AnthropicStreamMapper::new("1flowbase".to_string());
    let mut events = mapper.runtime_event_to_sse(
        &run,
        RuntimeEventEnvelope::new(run.id, 1, debug_stream_events::flow_started(run.id)),
    );
    events.extend(mapper.runtime_event_to_sse(
        &run,
        RuntimeEventEnvelope::new(
            run.id,
            2,
            RuntimeEventPayload {
                event_type: "waiting_callback".to_string(),
                source: RuntimeEventSource::Runtime,
                durability: RuntimeEventDurability::DurableRequired,
                persist_required: true,
                trace_visible: true,
                payload: json!({
                    "type": "waiting_callback",
                    "run_id": run.id,
                    "status": "waiting_callback",
                    "callback_task_id": callback_task_id,
                    "callback_kind": "llm_tool_calls",
                    "tool_calls": run.tool_calls.clone().unwrap()
                }),
            },
        ),
    ));

    let response = completed_compatible_stream(events);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    assert!(body.contains("visible internal LLM output"), "{body}");
    assert!(body.contains("\"type\":\"text_delta\""), "{body}");
    assert!(body.contains("\"stop_reason\":\"end_turn\""), "{body}");
    assert!(!body.contains("\"type\":\"tool_use\""), "{body}");
    assert!(!body.contains("required_action_not_supported"), "{body}");
}

#[tokio::test]
async fn anthropic_text_stream_follows_claude_messages_event_order() {
    let run = native_run();
    let mut mapper = AnthropicStreamMapper::new("1flowbase".to_string());
    let mut events = mapper.runtime_event_to_sse(
        &run,
        RuntimeEventEnvelope::new(run.id, 1, debug_stream_events::flow_started(run.id)),
    );
    events.extend(mapper.runtime_event_to_sse(
        &run,
        RuntimeEventEnvelope::new(
            run.id,
            2,
            debug_stream_events::answer_text_delta(
                "node-answer",
                "hello ClaudeCode".to_string(),
                0,
                Some("node-llm"),
                None,
                Some("text"),
            ),
        ),
    ));
    events.extend(mapper.runtime_event_to_sse(
        &run,
        RuntimeEventEnvelope::new(
            run.id,
            3,
            debug_stream_events::flow_finished(run.id, json!({ "answer": "hello ClaudeCode" })),
        ),
    ));

    let response = completed_compatible_stream(events);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    let message_start = body
        .find("event: message_start")
        .unwrap_or_else(|| panic!("Anthropic stream should start with message_start: {body}"));
    let block_start = body
        .find("event: content_block_start")
        .unwrap_or_else(|| panic!("Anthropic stream should open a content block: {body}"));
    let text_delta = body
        .find("\"type\":\"text_delta\"")
        .unwrap_or_else(|| panic!("Anthropic stream should emit text_delta: {body}"));
    let block_stop = body
        .find("event: content_block_stop")
        .unwrap_or_else(|| panic!("Anthropic stream should close the content block: {body}"));
    let message_delta = body
        .find("event: message_delta")
        .unwrap_or_else(|| panic!("Anthropic stream should emit message_delta: {body}"));
    let message_stop = body
        .find("event: message_stop")
        .unwrap_or_else(|| panic!("Anthropic stream should stop with message_stop: {body}"));
    assert!(
        message_start < block_start
            && block_start < text_delta
            && text_delta < block_stop
            && block_stop < message_delta
            && message_delta < message_stop,
        "Anthropic event order should match Claude Messages streaming: {body}"
    );
    assert!(body.contains("hello ClaudeCode"), "{body}");
    assert!(body.contains("\"stop_reason\":\"end_turn\""), "{body}");
}

#[tokio::test]
async fn anthropic_waiting_callback_streams_tool_input_json_delta() {
    let callback_task_id = Uuid::from_u128(0xcccccccccccccccccccccccccccccccc);
    let mut mapper = AnthropicStreamMapper::new("1flowbase".to_string());
    let events = mapper
        .anthropic_tool_use_events(
            &json!({
                "callback_kind": "llm_tool_calls",
                "callback_task_id": callback_task_id,
                "tool_calls": [
                    {
                        "id": "toolu_bash",
                        "name": "Bash",
                        "arguments": {
                            "command": "pwd && ls -la",
                            "description": "List files"
                        }
                    }
                ]
            }),
            None,
        )
        .expect("LLM callback should map to Anthropic tool_use stream events");
    let response = completed_compatible_stream(events);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    assert!(body.contains("\"input\":{}"), "{body}");
    assert!(body.contains("\"type\":\"input_json_delta\""), "{body}");
    assert!(
        body.contains("\\\"command\\\":\\\"pwd && ls -la\\\""),
        "{body}"
    );
}

#[tokio::test]
async fn openai_responses_live_text_stream_wraps_deltas_in_output_items() {
    let run = native_run();
    let mut mapper = OpenAiResponseStreamMapper::new("1flowbase".to_string(), None, false);
    let mut events = Vec::new();
    events.extend(mapper.runtime_event_to_sse(
        &run,
        RuntimeEventEnvelope::new(run.id, 1, debug_stream_events::flow_started(run.id)),
    ));
    for (index, (event_type, text)) in [
        ("reasoning_delta", "thinking"),
        ("text_delta", "你好"),
        ("text_delta", "，OK"),
    ]
    .into_iter()
    .enumerate()
    {
        events.extend(mapper.runtime_event_to_sse(
            &run,
            RuntimeEventEnvelope::new(
                run.id,
                index as i64 + 2,
                RuntimeEventPayload {
                    event_type: event_type.to_string(),
                    source: RuntimeEventSource::Runtime,
                    durability: RuntimeEventDurability::DurableRequired,
                    persist_required: true,
                    trace_visible: true,
                    payload: json!({
                        "type": event_type,
                        "event_type": event_type,
                        "node_id": "node-answer",
                        "text": text,
                        "presentation": {
                            "kind": "answer",
                            "answer_node_id": "node-answer",
                            "source_node_id": "node-llm",
                            "source_output_key": "text",
                            "segment_index": 0
                        }
                    }),
                },
            ),
        ));
    }
    events.extend(mapper.runtime_event_to_sse(
        &run,
        RuntimeEventEnvelope::new(
            run.id,
            5,
            debug_stream_events::flow_finished(run.id, json!({ "answer": "你好，OK" })),
        ),
    ));

    let response = completed_compatible_stream(events);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    let reasoning_added = body
        .find("\"type\":\"reasoning\"")
        .unwrap_or_else(|| panic!("reasoning item should be added before its deltas: {body}"));
    let reasoning_delta = body
        .find("event: response.reasoning_text.delta")
        .unwrap_or_else(|| panic!("reasoning delta should stream: {body}"));
    let message_added = body
        .find("\"type\":\"message\"")
        .unwrap_or_else(|| panic!("message item should be added before text deltas: {body}"));
    let text_delta = body
        .find("event: response.output_text.delta")
        .unwrap_or_else(|| panic!("text delta should stream: {body}"));
    let completed = body
        .find("event: response.completed")
        .unwrap_or_else(|| panic!("stream should complete: {body}"));
    assert!(
        reasoning_added < reasoning_delta && reasoning_delta < message_added,
        "reasoning item lifecycle should precede the message item: {body}"
    );
    assert!(
        message_added < text_delta && text_delta < completed,
        "message item should open before text deltas and close before completed: {body}"
    );
    let item_done_count = body.matches("event: response.output_item.done").count();
    assert!(
        item_done_count >= 2,
        "both reasoning and message items should be closed, got {item_done_count}: {body}"
    );
    assert!(
        body.contains("\"text\":\"你好，OK\""),
        "message output_item.done should carry the accumulated text: {body}"
    );
}
