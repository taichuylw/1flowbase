#[utoipa::path(
    get,
    path = "/api/console/applications/{id}/logs/runs",
    params(
        ("id" = String, Path, description = "Application id"),
        ("page" = Option<i64>, Query, description = "1-based page number"),
        ("page_size" = Option<i64>, Query, description = "Page size"),
        ("time_range_days" = Option<i64>, Query, description = "Optional created-at day window"),
        ("sort_by" = Option<String>, Query, description = "Sort field: created_at, started_at, finished_at or updated_at"),
        ("sort_order" = Option<String>, Query, description = "Sort direction: asc or desc"),
        ("cache_mode" = Option<String>, Query, description = "Read mode: refresh bypasses application log cache reads")
    ),
    responses(
        (status = 200, body = FlowRunSummaryPageResponse),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn list_application_runs(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Query(query): Query<ApplicationRunsQuery>,
) -> Result<Json<ApiSuccess<FlowRunSummaryPageResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    let application = ensure_application_visible(&state, context.user.id, id).await?;
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    let created_after = application_runs_created_after(&query);
    let sort_by = normalize_application_run_sort_by(query.sort_by.as_deref()).to_string();
    let sort_order = normalize_application_run_sort_order(query.sort_order.as_deref()).to_string();
    let refresh_cache = should_refresh_application_run_logs(query.cache_mode.as_deref());
    let cache = state.infrastructure.cache_store();
    let cache_key = application_log_cache::summary_page_cache_key(
        context.actor.current_workspace_id,
        id,
        &query,
        page,
        page_size,
        &sort_by,
        &sort_order,
    );

    if !refresh_cache {
        if let Some(cached) =
            application_log_cache::read::<FlowRunSummaryPageResponse>(cache.as_ref(), &cache_key)
                .await
        {
            return Ok(Json(ApiSuccess::new(cached)));
        }
    }

    let runs_page =
        <MainDurableStore as OrchestrationRuntimeRepository>::list_application_run_logs_page(
            &state.store,
            id,
            control_plane::ports::ListApplicationRunsPageInput {
                page,
                page_size,
                created_after,
                sort_by: Some(sort_by),
                sort_order: Some(sort_order),
            },
        )
        .await?;

    let mut items = Vec::with_capacity(runs_page.items.len());

    for log_summary in runs_page.items {
        let statistics = application_logs::ApplicationRunStatisticsResponse {
            total_tokens: log_summary.total_tokens,
            input_tokens: log_summary.input_tokens,
            output_tokens: log_summary.output_tokens,
            input_cache_hit_tokens: log_summary.input_cache_hit_tokens,
            unique_node_count: log_summary.unique_node_count,
            tool_callback_count: log_summary.tool_callback_count,
        };
        items.push(to_flow_run_summary_response(
            &application,
            log_summary.run,
            statistics,
        ));
    }

    let response = FlowRunSummaryPageResponse {
        items,
        total: runs_page.total,
        page: runs_page.page,
        page_size: runs_page.page_size,
    };

    if application_log_cache::summary_page_cacheable(&response) {
        application_log_cache::write(
            cache.as_ref(),
            &cache_key,
            &response,
            application_log_cache::summary_page_cache_ttl(page),
        )
        .await;
    }

    Ok(Json(ApiSuccess::new(response)))
}

#[utoipa::path(
    get,
    path = "/api/console/applications/{id}/logs/conversations/{conversation_id}/messages",
    params(
        ("id" = String, Path, description = "Application id"),
        ("conversation_id" = String, Path, description = "External conversation id"),
        ("around_run_id" = Option<String>, Query, description = "Flow run id to center the page around"),
        ("before" = Option<String>, Query, description = "Load runs before this cursor run id"),
        ("after" = Option<String>, Query, description = "Load runs after this cursor run id"),
        ("limit" = Option<i64>, Query, description = "Page size, defaults to 5")
    ),
    responses(
        (status = 200, body = ApplicationConversationMessagesPageResponse),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn list_application_conversation_messages(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, conversation_id)): Path<(Uuid, String)>,
    Query(query): Query<ApplicationConversationMessagesQuery>,
) -> Result<Json<ApiSuccess<ApplicationConversationMessagesPageResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_application_visible(&state, context.user.id, id).await?;

    let page =
        <MainDurableStore as OrchestrationRuntimeRepository>::list_application_conversation_runs_page(
            &state.store,
            id,
            ListApplicationConversationRunsPageInput {
                external_conversation_id: conversation_id,
                around_run_id: query.around_run_id,
                before_run_id: parse_optional_uuid_cursor(query.before.as_deref()),
                after_run_id: parse_optional_uuid_cursor(query.after.as_deref()),
                limit: query.limit.unwrap_or(5),
            },
        )
        .await?;
    let current_run_id = query.around_run_id;
    let workspace_id = context.actor.current_workspace_id;
    let load_debug_artifact = |artifact_id| {
        let state = state.clone();

        async move {
            load_runtime_debug_artifact_json_value(state, workspace_id, id, artifact_id)
                .await
                .ok()
        }
    };
    let mut items = Vec::with_capacity(page.items.len());
    for run in page.items {
        items.push(
            to_application_conversation_message_response(run, current_run_id, &load_debug_artifact)
                .await,
        );
    }

    Ok(Json(ApiSuccess::new(
        ApplicationConversationMessagesPageResponse {
            items,
            page: ApplicationConversationMessagesPageInfoResponse {
                has_before: page.has_before,
                has_after: page.has_after,
                before_cursor: page.before_cursor.map(|value| value.to_string()),
                after_cursor: page.after_cursor.map(|value| value.to_string()),
            },
        },
    )))
}

#[utoipa::path(
    get,
    path = "/api/console/applications/{id}/logs/runs/{run_id}/conversation/messages",
    params(
        ("id" = String, Path, description = "Application id"),
        ("run_id" = String, Path, description = "Flow run id"),
        ("before" = Option<String>, Query, description = "Load messages before this cursor"),
        ("after" = Option<String>, Query, description = "Load messages after this cursor"),
        ("limit" = Option<i64>, Query, description = "Page size, defaults to 5")
    ),
    responses(
        (status = 200, body = ApplicationConversationMessagesPageResponse),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn list_application_run_conversation_messages(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, run_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<ApplicationConversationMessagesQuery>,
) -> Result<Json<ApiSuccess<ApplicationConversationMessagesPageResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_application_visible(&state, context.user.id, id).await?;

    let detail = <MainDurableStore as OrchestrationRuntimeRepository>::get_application_run_detail(
        &state.store,
        id,
        run_id,
    )
    .await?
    .ok_or(ControlPlaneError::NotFound("flow_run"))?;

    let workspace_id = context.actor.current_workspace_id;
    let load_debug_artifact = |artifact_id| {
        let state = state.clone();

        async move {
            load_runtime_debug_artifact_json_value(state, workspace_id, id, artifact_id)
                .await
                .ok()
        }
    };
    let fallback_page =
        conversation_messages_from_run_detail(&detail, &query, &load_debug_artifact).await;

    Ok(Json(ApiSuccess::new(fallback_page)))
}

#[utoipa::path(
    get,
    path = "/api/console/applications/{id}/logs/runs/{run_id}",
    params(
        ("id" = String, Path, description = "Application id"),
        ("run_id" = String, Path, description = "Flow run id")
    ),
    responses(
        (status = 200, body = ApplicationRunDetailResponse),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn get_application_run_detail(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiSuccess<ApplicationRunDetailResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    let application = ensure_application_visible(&state, context.user.id, id).await?;
    let detail = <MainDurableStore as OrchestrationRuntimeRepository>::get_application_run_detail(
        &state.store,
        id,
        run_id,
    )
    .await?
    .ok_or(ControlPlaneError::NotFound("flow_run"))?;
    let runtime_events = <MainDurableStore as OrchestrationRuntimeRepository>::list_runtime_events(
        &state.store,
        run_id,
        0,
    )
    .await?;
    let detail =
        enrich_application_run_detail_visible_internal_llm_route_traces(detail, &runtime_events);
    let response = to_application_run_detail_response(&application, detail);

    Ok(Json(ApiSuccess::new(response)))
}

#[utoipa::path(
    get,
    path = "/api/console/applications/{id}/logs/runs/{run_id}/nodes/{node_id}",
    params(
        ("id" = String, Path, description = "Application id"),
        ("run_id" = String, Path, description = "Flow run id"),
        ("node_id" = String, Path, description = "Flow node id")
    ),
    responses(
        (status = 200, body = NodeLastRunResponse),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn get_application_run_node_last_run(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, run_id, node_id)): Path<(Uuid, Uuid, String)>,
) -> Result<Json<ApiSuccess<Option<NodeLastRunResponse>>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_application_visible(&state, context.user.id, id).await?;

    let detail = <MainDurableStore as OrchestrationRuntimeRepository>::get_application_run_detail(
        &state.store,
        id,
        run_id,
    )
    .await?
    .ok_or(ControlPlaneError::NotFound("flow_run"))?;
    let runtime_events = <MainDurableStore as OrchestrationRuntimeRepository>::list_runtime_events(
        &state.store,
        run_id,
        0,
    )
    .await?;
    let detail =
        enrich_application_run_detail_visible_internal_llm_route_traces(detail, &runtime_events);

    let Some(node_run) = detail
        .node_runs
        .into_iter()
        .rev()
        .find(|candidate| candidate.node_id == node_id)
    else {
        return Ok(Json(ApiSuccess::new(None)));
    };

    let node_run_id = node_run.id;
    let checkpoints = detail
        .checkpoints
        .into_iter()
        .filter(|checkpoint| checkpoint.node_run_id == Some(node_run_id))
        .collect();
    let events = detail
        .events
        .into_iter()
        .filter(|event| event.node_run_id == Some(node_run_id))
        .collect();

    Ok(Json(ApiSuccess::new(Some(to_node_last_run_response(
        domain::NodeLastRun {
            flow_run: detail.flow_run,
            node_run,
            checkpoints,
            events,
        },
    )))))
}

#[utoipa::path(
    get,
    path = "/api/console/applications/{id}/logs/runs/{run_id}/debug-stream",
    params(
        ("id" = String, Path, description = "Application id"),
        ("run_id" = String, Path, description = "Flow run id")
    ),
    responses(
        (status = 200, body = RuntimeDebugStreamResponse),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn get_runtime_debug_stream(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiSuccess<RuntimeDebugStreamResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_application_visible(&state, context.user.id, id).await?;

    <MainDurableStore as OrchestrationRuntimeRepository>::get_flow_run(&state.store, id, run_id)
        .await?
        .ok_or(ControlPlaneError::NotFound("flow_run"))?;

    let parts = <MainDurableStore as OrchestrationRuntimeRepository>::list_runtime_events(
        &state.store,
        run_id,
        0,
    )
    .await?
    .iter()
    .filter_map(|event| {
        control_plane::runtime_observability::debug_read_model::fold_event_to_debug_part(
            run_id, event,
        )
    })
    .map(to_runtime_debug_stream_part_response)
    .collect();

    Ok(Json(ApiSuccess::new(RuntimeDebugStreamResponse { parts })))
}

#[utoipa::path(
    get,
    path = "/api/console/applications/{id}/orchestration/nodes/{node_id}/last-run",
    params(
        ("id" = String, Path, description = "Application id"),
        ("node_id" = String, Path, description = "Node id")
    ),
    responses(
        (status = 200, body = NodeLastRunResponse),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn get_node_last_run(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, node_id)): Path<(Uuid, String)>,
) -> Result<Json<ApiSuccess<Option<NodeLastRunResponse>>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_application_visible(&state, context.user.id, id).await?;

    let last_run = <MainDurableStore as OrchestrationRuntimeRepository>::get_latest_node_run(
        &state.store,
        id,
        &node_id,
    )
    .await?;
    let last_run = match last_run {
        Some(last_run) => {
            let runtime_events =
                <MainDurableStore as OrchestrationRuntimeRepository>::list_runtime_events(
                    &state.store,
                    last_run.flow_run.id,
                    0,
                )
                .await?;
            Some(to_node_last_run_response(
                enrich_node_last_run_visible_internal_llm_route_traces(last_run, &runtime_events),
            ))
        }
        None => None,
    };

    Ok(Json(ApiSuccess::new(last_run)))
}
