#[derive(Debug, Deserialize, ToSchema)]
pub struct StartNodeDebugPreviewBody {
    pub input_payload: serde_json::Value,
    pub document: Option<serde_json::Value>,
    pub debug_session_id: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct StartFlowDebugRunBody {
    pub input_payload: serde_json::Value,
    pub document: Option<serde_json::Value>,
    pub debug_session_id: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DebugRunStreamQuery {
    pub from_sequence: Option<i64>,
    pub last_event_id: Option<String>,
}

#[derive(Debug, Deserialize, Default, ToSchema)]
pub struct ApplicationRunsQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub time_range_days: Option<i64>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub cache_mode: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResumeFlowRunBody {
    pub checkpoint_id: String,
    pub input_payload: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CompleteCallbackTaskBody {
    pub response_payload: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct FlowRunSummaryResponse {
    pub id: String,
    pub application_id: String,
    pub application_type: String,
    pub run_object_kind: String,
    pub run_kind: String,
    pub run_mode: String,
    pub status: String,
    pub target_node_id: Option<String>,
    pub title: String,
    pub expand_id: Option<String>,
    pub authorized_account: Option<String>,
    pub source: String,
    pub compatibility_mode: Option<String>,
    pub subject: application_logs::ApplicationRunSubjectResponse,
    pub actor: application_logs::ApplicationRunActorResponse,
    pub correlation: application_logs::ApplicationRunCorrelationResponse,
    pub statistics: application_logs::ApplicationRunStatisticsResponse,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct FlowRunSummaryPageResponse {
    pub items: Vec<FlowRunSummaryResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct FlowRunResponse {
    pub id: String,
    pub application_id: String,
    pub flow_id: String,
    pub draft_id: String,
    pub compiled_plan_id: Option<String>,
    pub run_mode: String,
    pub status: String,
    pub target_node_id: Option<String>,
    pub title: String,
    pub expand_id: Option<String>,
    pub authorized_account: Option<String>,
    pub external_conversation_id: Option<String>,
    pub query: Option<String>,
    pub model: Option<String>,
    pub input_payload: serde_json::Value,
    pub output_payload: serde_json::Value,
    pub error_payload: Option<serde_json::Value>,
    pub created_by: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ApplicationConversationMessagesQuery {
    pub around_run_id: Option<Uuid>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApplicationConversationMessageResponse {
    pub run_id: String,
    pub detail_run_id: Option<String>,
    pub can_open_detail: bool,
    pub role: Option<String>,
    pub content: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: String,
    pub query: Option<String>,
    pub model: Option<String>,
    pub answer: Option<String>,
    pub is_current: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApplicationConversationMessagesPageInfoResponse {
    pub has_before: bool,
    pub has_after: bool,
    pub before_cursor: Option<String>,
    pub after_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApplicationConversationMessagesPageResponse {
    pub items: Vec<ApplicationConversationMessageResponse>,
    pub page: ApplicationConversationMessagesPageInfoResponse,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct NodeRunResponse {
    pub id: String,
    pub flow_run_id: String,
    pub node_id: String,
    pub node_type: String,
    pub node_alias: String,
    pub status: String,
    pub input_payload: serde_json::Value,
    pub input_payload_view: serde_json::Value,
    pub output_payload: serde_json::Value,
    pub error_payload: Option<serde_json::Value>,
    pub metrics_payload: serde_json::Value,
    pub debug_payload: serde_json::Value,
    pub started_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CheckpointResponse {
    pub id: String,
    pub flow_run_id: String,
    pub node_run_id: Option<String>,
    pub status: String,
    pub reason: String,
    pub locator_payload: serde_json::Value,
    pub variable_snapshot: serde_json::Value,
    pub external_ref_payload: Option<serde_json::Value>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CallbackTaskResponse {
    pub id: String,
    pub flow_run_id: String,
    pub node_run_id: String,
    pub callback_kind: String,
    pub status: String,
    pub request_payload: serde_json::Value,
    pub response_payload: Option<serde_json::Value>,
    pub external_ref_payload: Option<serde_json::Value>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct RunEventResponse {
    pub id: String,
    pub flow_run_id: String,
    pub node_run_id: Option<String>,
    pub sequence: i64,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct AnswerSnapshotResponse {
    pub kind: String,
    pub text: String,
    pub output_payload: serde_json::Value,
    pub complete: bool,
    pub materialized_from: String,
    pub answer_node_id: String,
    pub answer_node_run_id: String,
    pub waiting_node_id: Option<String>,
    pub waiting_node_run_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ApplicationRunStitchedTraceResponse {
    pub source_flow_run: FlowRunResponse,
    pub node_runs: Vec<NodeRunResponse>,
    pub callback_tasks: Vec<CallbackTaskResponse>,
    pub events: Vec<RunEventResponse>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ApplicationRunDetailResponse {
    pub run: application_logs::ApplicationRunLogResponse,
    pub statistics: application_logs::ApplicationRunStatisticsResponse,
    pub detail: application_logs::ApplicationRunTypedDetailResponse,
    pub flow_run: FlowRunResponse,
    pub answer_snapshot: Option<AnswerSnapshotResponse>,
    pub node_runs: Vec<NodeRunResponse>,
    pub checkpoints: Vec<CheckpointResponse>,
    pub callback_tasks: Vec<CallbackTaskResponse>,
    pub events: Vec<RunEventResponse>,
    pub stitched_trace: Vec<ApplicationRunStitchedTraceResponse>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApplicationRunOverviewResponse {
    pub run: application_logs::ApplicationRunLogResponse,
    pub statistics: application_logs::ApplicationRunStatisticsResponse,
    pub flow_run: FlowRunResponse,
    pub answer_snapshot: Option<AnswerSnapshotResponse>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApplicationRunTraceNodeSummaryResponse {
    pub trace_node_id: String,
    pub stable_locator: String,
    pub parent_trace_node_id: Option<String>,
    pub node_kind: String,
    pub flow_run_id: String,
    pub node_run_id: Option<String>,
    pub callback_task_id: Option<String>,
    pub node_id: Option<String>,
    pub node_type: Option<String>,
    pub node_alias: String,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub metrics_payload: serde_json::Value,
    pub has_children: bool,
    pub child_count: i64,
    pub has_content: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApplicationRunTraceProjectionStatusResponse {
    pub projection_status: String,
    pub projection_version: i32,
    pub source_watermark: String,
    pub attempt_count: i32,
    pub last_attempt_at: Option<String>,
    pub last_success_at: Option<String>,
    pub last_error_code: Option<String>,
    pub last_error_stage: Option<String>,
    pub last_error_source_kind: Option<String>,
    pub last_error_source_locator: Option<String>,
    pub last_error_ref: Option<String>,
    pub retriable: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApplicationRunTraceTreeResponse {
    pub run: application_logs::ApplicationRunLogResponse,
    pub statistics: application_logs::ApplicationRunStatisticsResponse,
    pub flow_run: FlowRunResponse,
    pub answer_snapshot: Option<AnswerSnapshotResponse>,
    pub projection_status: ApplicationRunTraceProjectionStatusResponse,
    pub nodes: Vec<ApplicationRunTraceNodeSummaryResponse>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ApplicationRunTraceNodeChildrenQuery {
    pub parent_trace_node_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApplicationRunTraceNodeChildrenResponse {
    pub projection_status: ApplicationRunTraceProjectionStatusResponse,
    pub items: Vec<ApplicationRunTraceNodeSummaryResponse>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApplicationRunTraceNodeContentResponse {
    pub trace_node_id: String,
    pub node_kind: String,
    pub projection_status: ApplicationRunTraceProjectionStatusResponse,
    pub node_run: Option<NodeRunResponse>,
    pub callback_task: Option<CallbackTaskResponse>,
    pub flow_run: Option<FlowRunResponse>,
    pub checkpoints: Vec<CheckpointResponse>,
    pub events: Vec<RunEventResponse>,
    pub payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApplicationRunTraceToolCallbackContentResponse {
    pub trace_node_id: String,
    pub tool_call_id: String,
    pub projection_status: ApplicationRunTraceProjectionStatusResponse,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApplicationRunResumeTimelineResponse {
    pub flow_run: FlowRunResponse,
    pub callback_tasks: Vec<CallbackTaskResponse>,
    pub events: Vec<RunEventResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RuntimeDebugStreamResponse {
    pub parts: Vec<RuntimeDebugStreamPartResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RuntimeDebugStreamPartResponse {
    pub id: String,
    pub flow_run_id: String,
    pub item_id: Option<String>,
    pub span_id: Option<String>,
    pub part_type: String,
    pub status: String,
    pub trust_level: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NodeLastRunResponse {
    pub flow_run: FlowRunResponse,
    pub node_run: NodeRunResponse,
    pub checkpoints: Vec<CheckpointResponse>,
    pub events: Vec<RunEventResponse>,
}
