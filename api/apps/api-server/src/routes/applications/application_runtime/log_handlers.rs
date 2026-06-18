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

async fn ensure_application_run_trace_projection_status(
    state: &Arc<ApiState>,
    application_id: Uuid,
    flow_run_id: Uuid,
) -> Result<domain::ApplicationRunTraceProjectionStatusRecord, ApiError> {
    let status =
        <MainDurableStore as OrchestrationRuntimeRepository>::get_application_run_trace_projection_status(
            &state.store,
            flow_run_id,
            APPLICATION_RUN_TRACE_PROJECTION_VERSION,
        )
        .await?;

    if let Some(status) = status.as_ref() {
        match status.status {
            domain::ApplicationRunTraceProjectionStatus::Pending
            | domain::ApplicationRunTraceProjectionStatus::Running
            | domain::ApplicationRunTraceProjectionStatus::Failed => return Ok(status.clone()),
            domain::ApplicationRunTraceProjectionStatus::Succeeded
            | domain::ApplicationRunTraceProjectionStatus::Stale
            | domain::ApplicationRunTraceProjectionStatus::Partial => {}
        }
    }

    let source =
        <MainDurableStore as OrchestrationRuntimeRepository>::get_application_run_trace_projection_source(
            &state.store,
            application_id,
            flow_run_id,
        )
        .await?
        .ok_or(ControlPlaneError::NotFound("flow_run"))?;
    let runtime_events = <MainDurableStore as OrchestrationRuntimeRepository>::list_runtime_events(
        &state.store,
        flow_run_id,
        0,
    )
    .await?;
    let source = enrich_application_run_detail_visible_internal_llm_route_traces(
        source,
        &runtime_events,
    );
    let source_watermark = trace_projection_source_watermark(&source);
    if !projection_status_needs_lazy_rebuild(status.as_ref(), &source_watermark) {
        return status.ok_or_else(|| ControlPlaneError::Conflict("trace_projection_status").into());
    }

    let projection = build_application_run_trace_projection(&source)?;
    <MainDurableStore as OrchestrationRuntimeRepository>::replace_application_run_trace_projection(
        &state.store,
        &projection,
    )
    .await?;

    <MainDurableStore as OrchestrationRuntimeRepository>::get_application_run_trace_projection_status(
        &state.store,
        flow_run_id,
        APPLICATION_RUN_TRACE_PROJECTION_VERSION,
    )
    .await?
    .ok_or_else(|| ControlPlaneError::Conflict("trace_projection_status").into())
}

fn to_trace_projection_status_response(
    status: &domain::ApplicationRunTraceProjectionStatusRecord,
) -> ApplicationRunTraceProjectionStatusResponse {
    ApplicationRunTraceProjectionStatusResponse {
        projection_status: status.status.as_str().to_string(),
        projection_version: status.projection_version,
        source_watermark: status.source_watermark.clone(),
        attempt_count: status.attempt_count,
        last_attempt_at: format_optional_time(status.last_attempt_at),
        last_success_at: format_optional_time(status.last_success_at),
        last_error_code: status.last_error_code.clone(),
        last_error_stage: status.last_error_stage.clone(),
        last_error_source_kind: status.last_error_source_kind.clone(),
        last_error_source_locator: status.last_error_source_locator.clone(),
        last_error_ref: status.last_error_ref.clone(),
        retriable: status.retriable,
    }
}

fn application_run_log_response_for_trace_tree(
    application: &domain::ApplicationRecord,
    flow_run: &domain::FlowRunRecord,
) -> application_logs::ApplicationRunLogResponse {
    let application_type = application.application_type.as_str().to_string();

    application_logs::ApplicationRunLogResponse {
        id: flow_run.id.to_string(),
        application_id: application.id.to_string(),
        application_type: application_type.clone(),
        run_object_kind: application.sections.logs.run_object_kind.clone(),
        run_kind: flow_run.run_mode.as_str().to_string(),
        status: flow_run.status.as_str().to_string(),
        title: flow_run.title.clone(),
        source: application_logs::source_for_run(flow_run.api_key_id),
        compatibility_mode: flow_run.compatibility_mode.clone(),
        subject: application_logs::ApplicationRunSubjectResponse {
            kind: application_type,
            id: Some(flow_run.flow_id.to_string()),
            draft_id: Some(flow_run.draft_id.to_string()),
            target_node_id: flow_run.target_node_id.clone(),
        },
        actor: application_logs::actor_from_console_user(
            Some(flow_run.created_by.to_string()),
            flow_run.authorized_account.clone(),
        ),
        correlation: application_logs::ApplicationRunCorrelationResponse {
            api_key_id: flow_run.api_key_id.map(|value| value.to_string()),
            publication_version_id: flow_run
                .publication_version_id
                .map(|value| value.to_string()),
            external_user: flow_run.external_user.clone(),
            external_conversation_id: flow_run.external_conversation_id.clone(),
            external_trace_id: flow_run.external_trace_id.clone(),
            compatibility_mode: flow_run.compatibility_mode.clone(),
            idempotency_key: flow_run.idempotency_key.clone(),
        },
        started_at: application_logs::format_time(flow_run.started_at),
        finished_at: application_logs::format_optional_time(flow_run.finished_at),
        created_at: application_logs::format_time(flow_run.created_at),
        updated_at: application_logs::format_time(flow_run.updated_at),
    }
}

fn projection_is_succeeded(status: &domain::ApplicationRunTraceProjectionStatusRecord) -> bool {
    status.status == domain::ApplicationRunTraceProjectionStatus::Succeeded
}

fn parse_trace_projection_node_id(value: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(value).map_err(|_| ControlPlaneError::InvalidInput("trace_node_id").into())
}

const APPLICATION_RUN_TRACE_CHILDREN_DEFAULT_PAGE_SIZE: i64 = 20;
const APPLICATION_RUN_TRACE_CHILDREN_MAX_PAGE_SIZE: i64 = 100;

#[derive(Debug, Deserialize, Serialize)]
struct ApplicationRunTraceChildrenCursorPayload {
    parent_trace_node_id: Uuid,
    order_key: String,
    trace_node_id: Uuid,
}

fn application_run_trace_children_page_size(page_size: Option<i64>) -> i64 {
    page_size
        .unwrap_or(APPLICATION_RUN_TRACE_CHILDREN_DEFAULT_PAGE_SIZE)
        .clamp(1, APPLICATION_RUN_TRACE_CHILDREN_MAX_PAGE_SIZE)
}

fn parse_application_run_trace_children_cursor(
    cursor: Option<&str>,
    parent_trace_node_id: Uuid,
) -> Result<Option<ApplicationRunTraceChildrenCursor>, ApiError> {
    let Some(cursor) = cursor else {
        return Ok(None);
    };

    let bytes = URL_SAFE_NO_PAD
        .decode(cursor.as_bytes())
        .map_err(|_| ControlPlaneError::InvalidInput("cursor"))?;
    let payload: ApplicationRunTraceChildrenCursorPayload =
        serde_json::from_slice(&bytes).map_err(|_| ControlPlaneError::InvalidInput("cursor"))?;

    if payload.order_key.is_empty() {
        return Err(ControlPlaneError::InvalidInput("cursor").into());
    }
    if payload.parent_trace_node_id != parent_trace_node_id {
        return Err(ControlPlaneError::InvalidInput("cursor").into());
    }

    Ok(Some(ApplicationRunTraceChildrenCursor {
        order_key: payload.order_key,
        trace_node_id: payload.trace_node_id,
    }))
}

fn encode_application_run_trace_children_cursor(
    cursor: &ApplicationRunTraceChildrenCursor,
    parent_trace_node_id: Uuid,
) -> Result<String, ApiError> {
    let payload = ApplicationRunTraceChildrenCursorPayload {
        parent_trace_node_id,
        order_key: cursor.order_key.clone(),
        trace_node_id: cursor.trace_node_id,
    };
    let bytes = serde_json::to_vec(&payload).map_err(ApiError::from)?;

    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn to_trace_node_summary_from_projection(
    node: domain::ApplicationRunTraceNodeRecord,
) -> ApplicationRunTraceNodeSummaryResponse {
    ApplicationRunTraceNodeSummaryResponse {
        trace_node_id: node.trace_node_id.to_string(),
        stable_locator: node.stable_locator,
        parent_trace_node_id: node.parent_trace_node_id.map(|value| value.to_string()),
        node_kind: node.node_kind,
        flow_run_id: node.flow_run_id.to_string(),
        node_run_id: node
            .owner_kind
            .as_deref()
            .is_some_and(|kind| kind == "node_run")
            .then(|| node.owner_id.clone())
            .flatten(),
        callback_task_id: node
            .owner_kind
            .as_deref()
            .is_some_and(|kind| kind == "callback_task")
            .then(|| node.owner_id.clone())
            .flatten(),
        node_id: node.node_id,
        node_type: node.node_type,
        node_alias: node.node_alias,
        status: node.status,
        started_at: format_time(node.started_at),
        finished_at: format_optional_time(node.finished_at),
        duration_ms: node.duration_ms,
        metrics_payload: node.metrics_payload,
        has_children: node.has_children,
        child_count: node.child_count,
        has_content: node.has_content,
    }
}

fn trace_projection_statistics(
    nodes: &[domain::ApplicationRunTraceNodeRecord],
) -> application_logs::ApplicationRunStatisticsResponse {
    let mut unique_node_ids = HashSet::new();
    let mut total_tokens = None;
    let mut input_tokens = None;
    let mut output_tokens = None;
    let mut input_cache_hit_tokens = None;
    let mut tool_callback_count = 0_i64;

    for node in nodes {
        if let Some(node_id) = node.node_id.as_deref() {
            unique_node_ids.insert(node_id);
        }
        if node.node_kind == "tool_callback" {
            tool_callback_count += 1;
        }
        if let Some(node_tokens) = metrics_payload_total_tokens(&node.metrics_payload) {
            total_tokens = Some(total_tokens.unwrap_or(0) + node_tokens);
        }
        if let Some(node_tokens) =
            metrics_payload_usage_token(&node.metrics_payload, "input_tokens")
        {
            input_tokens = Some(input_tokens.unwrap_or(0) + node_tokens);
        }
        if let Some(node_tokens) =
            metrics_payload_usage_token(&node.metrics_payload, "output_tokens")
        {
            output_tokens = Some(output_tokens.unwrap_or(0) + node_tokens);
        }
        if let Some(node_tokens) = metrics_payload_cache_hit_tokens(&node.metrics_payload) {
            input_cache_hit_tokens = Some(input_cache_hit_tokens.unwrap_or(0) + node_tokens);
        }
    }

    application_logs::ApplicationRunStatisticsResponse {
        total_tokens,
        input_tokens,
        output_tokens,
        input_cache_hit_tokens,
        unique_node_count: unique_node_ids.len() as i64,
        tool_callback_count,
    }
}

fn trace_projection_node_content_response(
    node: domain::ApplicationRunTraceNodeRecord,
    content: domain::ApplicationRunTraceNodeContentRecord,
    projection_status: ApplicationRunTraceProjectionStatusResponse,
) -> Result<ApplicationRunTraceNodeContentResponse, ApiError> {
    let node_run = content
        .payload
        .get("node_run")
        .cloned()
        .map(serde_json::from_value::<domain::NodeRunRecord>)
        .transpose()
        .map_err(|_| ControlPlaneError::Conflict("trace_node_content"))?
        .map(to_node_run_response);
    let callback_task = if content.content_kind == "callback_task" {
        Some(
            serde_json::from_value::<domain::CallbackTaskRecord>(content.payload.clone())
                .map_err(|_| ControlPlaneError::Conflict("trace_node_content"))
                .map(to_callback_task_response)?,
        )
    } else {
        None
    };
    let flow_run = if content.content_kind == "stitched_run" {
        Some(
            serde_json::from_value::<domain::FlowRunRecord>(content.payload.clone())
                .map_err(|_| ControlPlaneError::Conflict("trace_node_content"))
                .map(to_flow_run_response)?,
        )
    } else {
        None
    };
    let checkpoints = content
        .payload
        .get("checkpoints")
        .cloned()
        .map(serde_json::from_value::<Vec<domain::CheckpointRecord>>)
        .transpose()
        .map_err(|_| ControlPlaneError::Conflict("trace_node_content"))?
        .unwrap_or_default()
        .into_iter()
        .map(to_checkpoint_response)
        .collect();
    let events = content
        .payload
        .get("events")
        .cloned()
        .map(serde_json::from_value::<Vec<domain::RunEventRecord>>)
        .transpose()
        .map_err(|_| ControlPlaneError::Conflict("trace_node_content"))?
        .unwrap_or_default()
        .into_iter()
        .map(to_run_event_response)
        .collect();

    Ok(ApplicationRunTraceNodeContentResponse {
        trace_node_id: node.trace_node_id.to_string(),
        node_kind: node.node_kind,
        projection_status,
        node_run,
        callback_task,
        flow_run,
        checkpoints,
        events,
        payload: Some(content.payload),
    })
}

async fn find_trace_projection_tool_callback_node(
    state: &Arc<ApiState>,
    flow_run_id: Uuid,
    owner: &domain::ApplicationRunTraceNodeRecord,
    tool_call_id: &str,
) -> Result<domain::ApplicationRunTraceNodeRecord, ApiError> {
    if owner.node_kind == "tool_callback" && owner.owner_id.as_deref() == Some(tool_call_id) {
        return Ok(owner.clone());
    }

    for stable_locator in [
        format!("{}/tool:{tool_call_id}", owner.stable_locator),
        format!("{}/tools/tool:{tool_call_id}", owner.stable_locator),
    ] {
        if let Some(node) =
            <MainDurableStore as OrchestrationRuntimeRepository>::get_application_run_trace_node_by_locator(
                &state.store,
                flow_run_id,
                &stable_locator,
            )
            .await?
        {
            return Ok(node);
        }
    }

    Err(ControlPlaneError::NotFound("tool_callback").into())
}

fn answer_snapshot_for_log_overview(
    detail: &domain::ApplicationRunDetail,
) -> Option<AnswerSnapshotResponse> {
    let (answer_snapshot_node_run, _) = split_answer_snapshot_node_runs(detail);

    if !flow_run_can_expose_answer_snapshot(&detail.flow_run.status) {
        return None;
    }

    answer_snapshot_node_run
        .as_ref()
        .and_then(|node_run| to_answer_snapshot_response(node_run, detail))
}

fn to_application_run_overview_response(
    application: &domain::ApplicationRecord,
    detail: domain::ApplicationRunDetail,
) -> ApplicationRunOverviewResponse {
    let (_, current_visible_node_runs) = split_answer_snapshot_node_runs(&detail);
    let statistics = application_run_statistics(&domain::ApplicationRunDetail {
        node_runs: current_visible_node_runs,
        ..detail.clone()
    });

    ApplicationRunOverviewResponse {
        run: application_run_log_response_for_trace_tree(application, &detail.flow_run),
        statistics,
        flow_run: to_flow_run_response(detail.flow_run.clone()),
        answer_snapshot: answer_snapshot_for_log_overview(&detail),
    }
}

async fn load_application_run_detail_for_log_overview(
    state: Arc<ApiState>,
    application_id: Uuid,
    flow_run_id: Uuid,
) -> Result<domain::ApplicationRunDetail, ApiError> {
    Ok(
        <MainDurableStore as OrchestrationRuntimeRepository>::get_application_run_detail(
            &state.store,
            application_id,
            flow_run_id,
        )
        .await?
        .ok_or(ControlPlaneError::NotFound("flow_run"))?,
    )
}

#[utoipa::path(
    get,
    path = "/api/console/applications/{id}/logs/runs/{run_id}/overview",
    params(
        ("id" = String, Path, description = "Application id"),
        ("run_id" = String, Path, description = "Flow run id")
    ),
    responses(
        (status = 200, body = ApplicationRunOverviewResponse),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn get_application_run_overview(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiSuccess<ApplicationRunOverviewResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    let application = ensure_application_visible(&state, context.user.id, id).await?;
    let detail = load_application_run_detail_for_log_overview(state, id, run_id).await?;
    let response = to_application_run_overview_response(&application, detail);

    Ok(Json(ApiSuccess::new(response)))
}

#[utoipa::path(
    get,
    path = "/api/console/applications/{id}/logs/runs/{run_id}/trace-tree",
    params(
        ("id" = String, Path, description = "Application id"),
        ("run_id" = String, Path, description = "Flow run id")
    ),
    responses(
        (status = 200, body = ApplicationRunTraceTreeResponse),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn get_application_run_trace_tree(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiSuccess<ApplicationRunTraceTreeResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    let application = ensure_application_visible(&state, context.user.id, id).await?;
    let status = ensure_application_run_trace_projection_status(&state, id, run_id).await?;
    let flow_run = <MainDurableStore as OrchestrationRuntimeRepository>::get_flow_run(
        &state.store,
        id,
        run_id,
    )
    .await?
    .ok_or(ControlPlaneError::NotFound("flow_run"))?;
    let nodes = if projection_is_succeeded(&status) {
        <MainDurableStore as OrchestrationRuntimeRepository>::list_application_run_trace_roots(
            &state.store,
            run_id,
        )
        .await?
    } else {
        Vec::new()
    };
    let statistic_nodes = if projection_is_succeeded(&status) {
        <MainDurableStore as OrchestrationRuntimeRepository>::list_application_run_trace_nodes_for_statistics(
            &state.store,
            run_id,
        )
        .await?
    } else {
        Vec::new()
    };
    let statistics = trace_projection_statistics(&statistic_nodes);
    let response = ApplicationRunTraceTreeResponse {
        run: application_run_log_response_for_trace_tree(&application, &flow_run),
        statistics,
        flow_run: to_flow_run_response(flow_run),
        answer_snapshot: None,
        projection_status: to_trace_projection_status_response(&status),
        nodes: nodes
            .into_iter()
            .map(to_trace_node_summary_from_projection)
            .collect(),
    };

    Ok(Json(ApiSuccess::new(response)))
}

#[utoipa::path(
    get,
    path = "/api/console/applications/{id}/logs/runs/{run_id}/trace-tree/nodes",
    params(
        ("id" = String, Path, description = "Application id"),
        ("run_id" = String, Path, description = "Flow run id"),
        ("parent_trace_node_id" = String, Query, description = "Trace node id to expand"),
        ("page_size" = Option<i64>, Query, description = "Page size, defaults to 20 and maxes at 100"),
        ("cursor" = Option<String>, Query, description = "Opaque cursor for the next children page")
    ),
    responses(
        (status = 200, body = ApplicationRunTraceNodeChildrenResponse),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn get_application_run_trace_node_children(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, run_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<ApplicationRunTraceNodeChildrenQuery>,
) -> Result<Json<ApiSuccess<ApplicationRunTraceNodeChildrenResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_application_visible(&state, context.user.id, id).await?;
    let status = ensure_application_run_trace_projection_status(&state, id, run_id).await?;
    let projection_status = to_trace_projection_status_response(&status);
    let page_size = application_run_trace_children_page_size(query.page_size);
    let parent_trace_node_id = parse_trace_projection_node_id(&query.parent_trace_node_id)?;
    let cursor =
        parse_application_run_trace_children_cursor(query.cursor.as_deref(), parent_trace_node_id)?;
    if !projection_is_succeeded(&status) {
        return Ok(Json(ApiSuccess::new(
            ApplicationRunTraceNodeChildrenResponse {
                projection_status,
                items: Vec::new(),
                page_info: ApplicationRunTraceNodeChildrenPageInfoResponse {
                    has_more: false,
                    next_cursor: None,
                    page_size,
                },
            },
        )));
    }
    <MainDurableStore as OrchestrationRuntimeRepository>::get_application_run_trace_node(
        &state.store,
        run_id,
        parent_trace_node_id,
    )
    .await?
    .ok_or(ControlPlaneError::NotFound("trace_node"))?;
    let page = <MainDurableStore as OrchestrationRuntimeRepository>::list_application_run_trace_children_page(
            &state.store,
            ListApplicationRunTraceChildrenPageInput {
                flow_run_id: run_id,
                parent_trace_node_id,
                page_size,
                cursor,
            },
        )
        .await?;
    let next_cursor = page
        .next_cursor
        .as_ref()
        .map(|cursor| encode_application_run_trace_children_cursor(cursor, parent_trace_node_id))
        .transpose()?;
    let items = page
        .items
        .into_iter()
        .map(to_trace_node_summary_from_projection)
        .collect();
    let response = ApplicationRunTraceNodeChildrenResponse {
        projection_status,
        items,
        page_info: ApplicationRunTraceNodeChildrenPageInfoResponse {
            has_more: page.has_more,
            next_cursor,
            page_size: page.page_size,
        },
    };

    Ok(Json(ApiSuccess::new(response)))
}

#[utoipa::path(
    get,
    path = "/api/console/applications/{id}/logs/runs/{run_id}/trace-tree/nodes/{trace_node_id}/content",
    params(
        ("id" = String, Path, description = "Application id"),
        ("run_id" = String, Path, description = "Flow run id"),
        ("trace_node_id" = String, Path, description = "Trace node id to load")
    ),
    responses(
        (status = 200, body = ApplicationRunTraceNodeContentResponse),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn get_application_run_trace_node_content(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, run_id, trace_node_id)): Path<(Uuid, Uuid, String)>,
) -> Result<Json<ApiSuccess<ApplicationRunTraceNodeContentResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_application_visible(&state, context.user.id, id).await?;
    let status = ensure_application_run_trace_projection_status(&state, id, run_id).await?;
    let projection_status = to_trace_projection_status_response(&status);
    let trace_node_uuid = parse_trace_projection_node_id(&trace_node_id)?;
    if !projection_is_succeeded(&status) {
        return Ok(Json(ApiSuccess::new(ApplicationRunTraceNodeContentResponse {
            trace_node_id,
            node_kind: "trace_projection".to_string(),
            projection_status,
            node_run: None,
            callback_task: None,
            flow_run: None,
            checkpoints: Vec::new(),
            events: Vec::new(),
            payload: None,
        })));
    }
    let node = <MainDurableStore as OrchestrationRuntimeRepository>::get_application_run_trace_node(
        &state.store,
        run_id,
        trace_node_uuid,
    )
    .await?
    .ok_or(ControlPlaneError::NotFound("trace_node"))?;
    let content =
        <MainDurableStore as OrchestrationRuntimeRepository>::get_application_run_trace_node_content(
            &state.store,
            run_id,
            trace_node_uuid,
        )
        .await?
        .ok_or(ControlPlaneError::NotFound("trace_node_content"))?;
    let response = trace_projection_node_content_response(node, content, projection_status)?;

    Ok(Json(ApiSuccess::new(response)))
}

#[utoipa::path(
    get,
    path = "/api/console/applications/{id}/logs/runs/{run_id}/trace-tree/nodes/{trace_node_id}/tool-callbacks/{tool_call_id}/content",
    params(
        ("id" = String, Path, description = "Application id"),
        ("run_id" = String, Path, description = "Flow run id"),
        ("trace_node_id" = String, Path, description = "Trace node id that owns the tool callback"),
        ("tool_call_id" = String, Path, description = "Tool call id to load")
    ),
    responses(
        (status = 200, body = ApplicationRunTraceToolCallbackContentResponse),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn get_application_run_trace_tool_callback_content(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, run_id, trace_node_id, tool_call_id)): Path<(Uuid, Uuid, String, String)>,
) -> Result<Json<ApiSuccess<ApplicationRunTraceToolCallbackContentResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_application_visible(&state, context.user.id, id).await?;
    let status = ensure_application_run_trace_projection_status(&state, id, run_id).await?;
    let projection_status = to_trace_projection_status_response(&status);
    let trace_node_uuid = parse_trace_projection_node_id(&trace_node_id)?;
    if !projection_is_succeeded(&status) {
        return Ok(Json(ApiSuccess::new(
            ApplicationRunTraceToolCallbackContentResponse {
                trace_node_id,
                tool_call_id,
                projection_status,
                payload: serde_json::json!({}),
            },
        )));
    }
    let owner = <MainDurableStore as OrchestrationRuntimeRepository>::get_application_run_trace_node(
        &state.store,
        run_id,
        trace_node_uuid,
    )
    .await?
    .ok_or(ControlPlaneError::NotFound("trace_node"))?;
    let tool_node =
        find_trace_projection_tool_callback_node(&state, run_id, &owner, &tool_call_id).await?;
    let content =
        <MainDurableStore as OrchestrationRuntimeRepository>::get_application_run_trace_node_content(
            &state.store,
            run_id,
            tool_node.trace_node_id,
        )
        .await?
        .ok_or(ControlPlaneError::NotFound("trace_node_content"))?;
    let response = ApplicationRunTraceToolCallbackContentResponse {
        trace_node_id: tool_node.trace_node_id.to_string(),
        tool_call_id,
        projection_status,
        payload: content.payload,
    };

    Ok(Json(ApiSuccess::new(response)))
}

#[utoipa::path(
    get,
    path = "/api/console/applications/{id}/logs/runs/{run_id}/resume-timeline",
    params(
        ("id" = String, Path, description = "Application id"),
        ("run_id" = String, Path, description = "Flow run id")
    ),
    responses(
        (status = 200, body = ApplicationRunResumeTimelineResponse),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn get_application_run_resume_timeline(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiSuccess<ApplicationRunResumeTimelineResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_application_visible(&state, context.user.id, id).await?;
    let detail = <MainDurableStore as OrchestrationRuntimeRepository>::get_application_run_detail(
        &state.store,
        id,
        run_id,
    )
    .await?
    .ok_or(ControlPlaneError::NotFound("flow_run"))?;
    let events = detail
        .events
        .into_iter()
        .filter(|event| {
            matches!(
                event.event_type.as_str(),
                "public_run_resume_requested"
                    | "public_run_resume_succeeded"
                    | "public_run_resume_failed"
                    | "public_run_resume_cancelled"
                    | "flow_run_resumed"
            )
        })
        .map(to_run_event_response)
        .collect();
    let callback_tasks = detail
        .callback_tasks
        .into_iter()
        .map(to_callback_task_response)
        .collect();

    Ok(Json(ApiSuccess::new(
        ApplicationRunResumeTimelineResponse {
            flow_run: to_flow_run_response(detail.flow_run),
            callback_tasks,
            events,
        },
    )))
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
