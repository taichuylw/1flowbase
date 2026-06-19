export type ConsoleFlowRunMode =
  | 'debug_node_preview'
  | 'debug_flow_run'
  | 'published_api_run';

export interface ConsoleApplicationRunSubject {
  kind: string;
  id?: string | null;
  draft_id?: string | null;
  target_node_id?: string | null;
}

export interface ConsoleApplicationRunActor {
  kind: string;
  id?: string | null;
  display_name?: string | null;
}

export interface ConsoleApplicationRunCorrelation {
  api_key_id?: string | null;
  publication_version_id?: string | null;
  external_user?: string | null;
  external_conversation_id?: string | null;
  external_trace_id?: string | null;
  compatibility_mode?: string | null;
  idempotency_key?: string | null;
}

export interface ConsoleApplicationRunLog {
  id: string;
  application_id: string;
  application_type: string;
  run_object_kind: string;
  run_kind: string;
  status: string;
  title: string;
  source: string;
  compatibility_mode?: string | null;
  subject: ConsoleApplicationRunSubject;
  actor: ConsoleApplicationRunActor;
  correlation: ConsoleApplicationRunCorrelation;
  started_at: string;
  finished_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface ConsoleApplicationRunSummary {
  id: string;
  application_id?: string;
  application_type?: string;
  run_object_kind?: string;
  run_kind?: string;
  run_mode: ConsoleFlowRunMode;
  status: string;
  target_node_id: string | null;
  title?: string;
  expand_id?: string | null;
  authorized_account?: string | null;
  source?: string;
  compatibility_mode?: string | null;
  subject?: ConsoleApplicationRunSubject;
  actor?: ConsoleApplicationRunActor;
  correlation?: ConsoleApplicationRunCorrelation;
  statistics?: ConsoleApplicationRunStatistics;
  started_at: string;
  finished_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface ConsoleApplicationRunStatistics {
  total_tokens: number | null;
  input_tokens: number | null;
  output_tokens: number | null;
  input_cache_hit_tokens: number | null;
  unique_node_count: number;
  tool_callback_count: number;
}

export interface ConsoleApplicationRunsPage {
  items: ConsoleApplicationRunSummary[];
  total: number;
  page: number;
  page_size: number;
}

export type ConsoleApplicationRunMonitoringBucket =
  | 'hour'
  | 'day'
  | 'week'
  | 'month';

export interface GetConsoleApplicationRunMonitoringReportInput {
  from?: string;
  to?: string;
  time_range_days?: number;
  bucket?: ConsoleApplicationRunMonitoringBucket;
}

export interface ConsoleApplicationRunMonitoringMeta {
  started_from: string | null;
  started_to: string | null;
  bucket: ConsoleApplicationRunMonitoringBucket;
  slow_run_threshold_ms: number;
}

export interface ConsoleApplicationRunMonitoringOverview {
  total_count: number;
  success_count: number;
  failed_count: number;
  cancelled_count: number;
  success_rate: number;
  failed_rate: number;
  running_count_included: boolean;
}

export interface ConsoleApplicationRunMonitoringDuration {
  duration_recorded_count: number;
  avg_duration_ms: number;
  p50_duration_ms: number;
  p95_duration_ms: number;
  slow_run_rate: number;
}

export interface ConsoleApplicationRunMonitoringTokens {
  total_tokens_sum: number;
  input_tokens_sum: number;
  output_tokens_sum: number;
  input_cache_hit_tokens_sum: number;
  avg_tokens_per_run: number;
  token_recorded_count: number;
}

export interface ConsoleApplicationRunMonitoringToolCallbacks {
  total_tool_callback_count: number;
  avg_tool_callback_count: number;
  runs_with_tool_callback: number;
}

export interface ConsoleApplicationRunMonitoringNodes {
  avg_unique_node_count: number;
  max_unique_node_count: number;
}

export interface ConsoleApplicationRunMonitoringConcurrency {
  peak_concurrency: number;
}

export interface ConsoleApplicationRunMonitoringTokenTrendPoint {
  bucket_start: string;
  run_count: number;
  total_tokens: number;
  input_tokens: number;
  output_tokens: number;
  input_cache_hit_tokens: number;
}

export interface ConsoleApplicationRunMonitoringProtocolBreakdown {
  protocol: string;
  request_count: number;
  success_rate: number;
  avg_duration_ms: number;
  total_tokens: number;
}

export interface ConsoleApplicationRunMonitoringSourceBreakdown {
  source: string;
  request_count: number;
  success_rate: number;
  total_tokens: number;
}

export interface ConsoleApplicationRunMonitoringAuthorizedAccountUsage {
  authorized_account: string | null;
  request_count: number;
  total_tokens: number;
  avg_duration_ms: number;
  failed_count: number;
}

export interface ConsoleApplicationRunMonitoringExternalUserUsage {
  external_user: string | null;
  request_count: number;
  total_tokens: number;
  avg_duration_ms: number;
  failed_count: number;
}

export interface ConsoleApplicationRunMonitoringApiKeyUsage {
  api_key_id: string;
  api_key_name_snapshot: string | null;
  request_count: number;
  total_tokens: number;
  avg_duration_ms: number;
  failed_count: number;
}

export interface ConsoleApplicationRunMonitoringExternalConversationUsage {
  external_conversation_id: string | null;
  request_count: number;
  total_tokens: number;
  avg_duration_ms: number;
  failed_count: number;
}

export interface ConsoleApplicationRunMonitoringRunRank {
  flow_run_id: string;
  title: string;
  status: string;
  started_at: string;
  finished_at: string | null;
  duration_ms: number | null;
  total_tokens: number | null;
}

export interface ConsoleApplicationRunMonitoringReport {
  meta: ConsoleApplicationRunMonitoringMeta;
  overview: ConsoleApplicationRunMonitoringOverview;
  duration: ConsoleApplicationRunMonitoringDuration;
  tokens: ConsoleApplicationRunMonitoringTokens;
  tokens_comparison: ConsoleApplicationRunMonitoringTokensComparison;
  tool_callbacks: ConsoleApplicationRunMonitoringToolCallbacks;
  nodes: ConsoleApplicationRunMonitoringNodes;
  concurrency: ConsoleApplicationRunMonitoringConcurrency;
  tokens_trend: ConsoleApplicationRunMonitoringTokenTrendPoint[];
  protocols: ConsoleApplicationRunMonitoringProtocolBreakdown[];
  sources: ConsoleApplicationRunMonitoringSourceBreakdown[];
  authorized_accounts: ConsoleApplicationRunMonitoringAuthorizedAccountUsage[];
  external_users: ConsoleApplicationRunMonitoringExternalUserUsage[];
  api_keys: ConsoleApplicationRunMonitoringApiKeyUsage[];
  external_conversations: ConsoleApplicationRunMonitoringExternalConversationUsage[];
  slowest_runs: ConsoleApplicationRunMonitoringRunRank[];
  high_token_runs: ConsoleApplicationRunMonitoringRunRank[];
}

export interface ConsoleApplicationRunMonitoringTokensComparison {
  previous_total_tokens_sum: number;
  previous_run_count: number;
  previous_avg_tokens_per_run: number;
  token_change_rate: number;
  run_count_change_rate: number;
  avg_tokens_per_run_change_rate: number;
  traffic_effect: number;
  cost_per_run_effect: number;
}

export interface ConsoleApplicationRuntimeActivity {
  meta: {
    application_id: string;
    scope: 'current_instance' | string;
    storage: 'memory' | string;
    instance_started_at: string;
    snapshot_at: string;
  };
  active: {
    total: number;
    http_requests: number;
    sse_connections: number;
    websocket_connections: number;
    application_executions: number;
    tool_calls: number;
    model_requests: number;
    waiting: number | null;
  };
  peaks: {
    process_peak_concurrency: number;
    recent_peak_concurrency: number;
  };
  rolling_minute: {
    completed: number;
    failed: number;
    cancelled: number;
    disconnected: number;
  };
  windows: ConsoleApplicationRuntimeActivityWindows;
  health: ConsoleApplicationRuntimeActivityHealth;
  age_distribution: ConsoleApplicationRuntimeActivityAgeDistribution;
  long_connection_age_distribution: ConsoleApplicationRuntimeActivityAgeDistribution;
  pressure: {
    slow_active_executions: number;
    execution_slots_used: number | null;
    execution_slots_limit: number | null;
  };
  resources: {
    process_rss_bytes: number | null;
  };
}

export interface ConsoleApplicationRuntimeActivityWindows {
  one_minute: ConsoleApplicationRuntimeActivityWindow;
  five_minutes: ConsoleApplicationRuntimeActivityWindow;
  fifteen_minutes: ConsoleApplicationRuntimeActivityWindow;
}

export interface ConsoleApplicationRuntimeActivityWindow {
  window_seconds: number;
  completed: number;
  failed: number;
  cancelled: number;
  disconnected: number;
  peak_concurrency: number;
  failure_rate: number;
  disconnect_rate: number;
  throughput_per_minute: number;
}

export type ConsoleApplicationRuntimeHealthState =
  | 'healthy'
  | 'busy'
  | 'slow'
  | 'unstable'
  | 'failing'
  | 'failing_now';

export type ConsoleApplicationRuntimeTrend = 'rising' | 'steady' | 'falling';

export interface ConsoleApplicationRuntimeActivityHealth {
  state: ConsoleApplicationRuntimeHealthState;
  failure_rate_1m: number;
  failure_rate_5m: number;
  failure_rate_15m: number;
  disconnect_rate_5m: number;
  slow_ratio: number;
  active_pressure: number;
  throughput_5m_per_minute: number;
  throughput_15m_per_minute: number;
  throughput_trend: ConsoleApplicationRuntimeTrend;
  failure_trend: number;
}

export interface ConsoleApplicationRuntimeActivityAgeDistribution {
  under_5s: number;
  from_5s_to_30s: number;
  from_30s_to_120s: number;
  over_120s: number;
}

export interface GetConsoleApplicationRunsInput {
  page?: number;
  page_size?: number;
  time_range_days?: number;
  sort_by?: 'created_at' | 'started_at' | 'finished_at' | 'updated_at';
  sort_order?: 'asc' | 'desc';
  cache_mode?: 'default' | 'refresh';
}

export interface ConsoleFlowRunDetail {
  id: string;
  application_id: string;
  flow_id: string;
  draft_id: string;
  compiled_plan_id: string | null;
  debug_session_id?: string;
  run_mode: ConsoleFlowRunMode;
  status: string;
  target_node_id: string | null;
  title?: string;
  expand_id?: string | null;
  authorized_account?: string | null;
  external_conversation_id?: string | null;
  query?: string | null;
  model?: string | null;
  input_payload: Record<string, unknown>;
  output_payload: Record<string, unknown>;
  error_payload: Record<string, unknown> | null;
  created_by: string;
  started_at: string;
  finished_at: string | null;
  created_at: string;
  updated_at?: string;
}

export interface ConsoleNodeRunDetail {
  id: string;
  flow_run_id: string;
  node_id: string;
  node_type: string;
  node_alias: string;
  status: string;
  input_payload: Record<string, unknown>;
  input_payload_view?: Record<string, unknown>;
  output_payload: Record<string, unknown>;
  error_payload: Record<string, unknown> | null;
  metrics_payload: Record<string, unknown>;
  debug_payload?: Record<string, unknown>;
  started_at: string;
  finished_at: string | null;
}

export interface ConsoleRunCheckpoint {
  id: string;
  flow_run_id: string;
  node_run_id: string | null;
  status: string;
  reason: string;
  locator_payload: Record<string, unknown>;
  variable_snapshot: Record<string, unknown>;
  external_ref_payload: Record<string, unknown> | null;
  created_at: string;
}

export interface ConsoleRunEvent {
  id: string;
  flow_run_id: string;
  node_run_id: string | null;
  sequence: number;
  event_type: string;
  payload: Record<string, unknown>;
  created_at: string;
}

export interface ConsoleCallbackTask {
  id: string;
  flow_run_id: string;
  node_run_id: string;
  callback_kind: string;
  status: 'pending' | 'completed' | 'cancelled';
  request_payload: Record<string, unknown>;
  response_payload: Record<string, unknown> | null;
  external_ref_payload: Record<string, unknown> | null;
  created_at: string;
  completed_at: string | null;
}

export interface ConsoleAnswerSnapshot {
  kind: string;
  text: string;
  output_payload: Record<string, unknown>;
  complete: boolean;
  materialized_from: string;
  answer_node_id: string;
  answer_node_run_id: string;
  waiting_node_id?: string | null;
  waiting_node_run_id?: string | null;
}

export interface ConsoleApplicationRunStitchedTrace {
  source_flow_run: ConsoleFlowRunDetail;
  node_runs: ConsoleNodeRunDetail[];
  callback_tasks: ConsoleCallbackTask[];
  events: ConsoleRunEvent[];
}

export interface ConsoleApplicationRunDetail {
  run?: ConsoleApplicationRunLog;
  statistics?: ConsoleApplicationRunStatistics;
  detail?: ConsoleApplicationRunTypedDetail;
  flow_run: ConsoleFlowRunDetail;
  answer_snapshot?: ConsoleAnswerSnapshot | null;
  node_runs: ConsoleNodeRunDetail[];
  checkpoints: ConsoleRunCheckpoint[];
  callback_tasks: ConsoleCallbackTask[];
  events: ConsoleRunEvent[];
  stitched_trace?: ConsoleApplicationRunStitchedTrace[];
}

export interface ConsoleApplicationRunOverview {
  run: ConsoleApplicationRunLog;
  statistics: ConsoleApplicationRunStatistics;
  flow_run: ConsoleFlowRunDetail;
  answer_snapshot?: ConsoleAnswerSnapshot | null;
}

export type ConsoleApplicationRunTraceNodeKind =
  | 'node_run'
  | 'callback_task'
  | 'tool_group'
  | 'tool_callback'
  | 'stitched_context'
  | 'stitched_run'
  | 'route'
  | 'fusion'
  | 'branch'
  | 'event';

export interface ConsoleApplicationRunTraceNodeSummary {
  trace_node_id: string;
  stable_locator: string;
  parent_trace_node_id?: string | null;
  node_kind: ConsoleApplicationRunTraceNodeKind;
  flow_run_id: string;
  node_run_id?: string | null;
  callback_task_id?: string | null;
  node_id?: string | null;
  node_type?: string | null;
  node_mode?: string | null;
  node_alias: string;
  status: string;
  started_at: string;
  finished_at?: string | null;
  duration_ms?: number | null;
  metrics_payload: Record<string, unknown>;
  has_children: boolean;
  child_count: number;
  has_content: boolean;
}

export interface ConsoleApplicationRunTraceProjectionStatus {
  projection_status:
    | 'pending'
    | 'running'
    | 'succeeded'
    | 'failed'
    | 'stale'
    | 'partial';
  projection_version: number;
  source_watermark: string;
  attempt_count: number;
  last_attempt_at?: string | null;
  last_success_at?: string | null;
  last_error_code?: string | null;
  last_error_stage?: string | null;
  last_error_source_kind?: string | null;
  last_error_source_locator?: string | null;
  last_error_ref?: string | null;
  retriable: boolean;
}

export interface ConsoleApplicationRunTraceTree {
  run: ConsoleApplicationRunLog;
  statistics: ConsoleApplicationRunStatistics;
  flow_run: ConsoleFlowRunDetail;
  answer_snapshot?: ConsoleAnswerSnapshot | null;
  projection_status: ConsoleApplicationRunTraceProjectionStatus;
  nodes: ConsoleApplicationRunTraceNodeSummary[];
}

export interface ConsoleApplicationRunTraceNodeChildrenPageInfo {
  has_more: boolean;
  next_cursor?: string | null;
  page_size: number;
}

export interface ConsoleApplicationRunTraceNodeChildren {
  projection_status: ConsoleApplicationRunTraceProjectionStatus;
  items: ConsoleApplicationRunTraceNodeSummary[];
  page_info: ConsoleApplicationRunTraceNodeChildrenPageInfo;
}

export interface ConsoleApplicationRunTraceNodeContent {
  trace_node_id: string;
  node_kind: ConsoleApplicationRunTraceNodeKind;
  projection_status: ConsoleApplicationRunTraceProjectionStatus;
  content_kind: string;
  source_refs: unknown;
  detail_refs: unknown;
  payload: Record<string, unknown>;
}

export interface ConsoleApplicationRunTraceNodeDetail {
  trace_node_id: string;
  node_kind: ConsoleApplicationRunTraceNodeKind;
  projection_status: ConsoleApplicationRunTraceProjectionStatus;
  detail_ref_id: string;
  detail_kind: string;
  source_refs: unknown;
  payload: Record<string, unknown>;
}

export interface ConsoleApplicationRunTraceToolCallbackContent {
  trace_node_id: string;
  tool_call_id: string;
  projection_status: ConsoleApplicationRunTraceProjectionStatus;
  payload: Record<string, unknown>;
}

export interface ConsoleApplicationRunResumeTimeline {
  flow_run: ConsoleFlowRunDetail;
  callback_tasks: ConsoleCallbackTask[];
  events: ConsoleRunEvent[];
}

export interface ConsoleApplicationConversationMessage {
  run_id: string;
  detail_run_id?: string | null;
  can_open_detail?: boolean;
  role?: 'system' | 'user' | 'assistant' | null;
  content?: string | null;
  started_at: string;
  finished_at: string | null;
  status: string;
  query?: string | null;
  model?: string | null;
  answer?: string | null;
  is_current: boolean;
}

export interface ConsoleApplicationConversationMessagesPage {
  items: ConsoleApplicationConversationMessage[];
  page: {
    has_before: boolean;
    has_after: boolean;
    before_cursor?: string | null;
    after_cursor?: string | null;
  };
}

export interface GetConsoleApplicationConversationMessagesInput {
  around_run_id?: string;
  before?: string;
  after?: string;
  limit?: number;
}

export interface ConsoleApplicationRunTypedDetail {
  kind: string;
  flow_run: ConsoleFlowRunDetail;
  answer_snapshot?: ConsoleAnswerSnapshot | null;
  node_runs: ConsoleNodeRunDetail[];
  checkpoints: ConsoleRunCheckpoint[];
  callback_tasks: ConsoleCallbackTask[];
  events: ConsoleRunEvent[];
  stitched_trace?: ConsoleApplicationRunStitchedTrace[];
}

export interface ConsoleDebugVariableCacheKey {
  node_id: string;
  variable_key: string;
}

export interface UpsertConsoleDebugVariableCacheEntryInput extends ConsoleDebugVariableCacheKey {
  value: unknown;
}

export interface DeleteConsoleDebugVariableCacheEntriesInput {
  keys?: ConsoleDebugVariableCacheKey[];
}

export interface RuntimeDebugStreamPart {
  id: string;
  flow_run_id: string;
  item_id?: string | null;
  span_id?: string | null;
  part_type: string;
  status: string;
  trust_level: string;
  payload: unknown;
}

export interface RuntimeDebugStreamResponse {
  parts: RuntimeDebugStreamPart[];
}

export interface ConsoleFlowDebugStreamCursor {
  from_sequence?: number;
  last_event_id?: string;
}

export type ConsoleFlowDebugStreamEvent =
  | {
      type: 'flow_accepted';
      run_id: string;
      status: 'queued' | 'starting' | string;
      event_id?: string;
      sequence?: number;
      created_at?: string;
      delta_index?: number | null;
      content_type?: 'text' | 'reasoning' | null;
    }
  | {
      type: 'flow_started';
      run_id: string;
      status: string;
      event_id?: string;
      sequence?: number;
      created_at?: string;
      delta_index?: number | null;
      content_type?: 'text' | 'reasoning' | null;
    }
  | {
      type: 'node_started';
      node_run_id: string;
      node_id: string;
      node_type: string;
      title: string;
      input_payload?: Record<string, unknown>;
      started_at?: string;
      run_id?: string;
      event_id?: string;
      sequence?: number;
      created_at?: string;
      delta_index?: number | null;
      content_type?: 'text' | 'reasoning' | null;
    }
  | {
      type: 'node_finished';
      node_run_id: string;
      node_id: string;
      status: string;
      output_payload?: Record<string, unknown>;
      error_payload?: Record<string, unknown> | null;
      metrics_payload?: Record<string, unknown>;
      debug_payload?: Record<string, unknown>;
      started_at?: string;
      finished_at?: string | null;
      run_id?: string;
      event_id?: string;
      sequence?: number;
      created_at?: string;
      delta_index?: number | null;
      content_type?: 'text' | 'reasoning' | null;
    }
  | {
      type: 'text_delta';
      node_run_id?: string | null;
      node_id: string;
      text: string;
      run_id?: string;
      event_id?: string;
      sequence?: number;
      created_at?: string;
      delta_index?: number | null;
      content_type?: 'text' | 'reasoning' | null;
    }
  | {
      type: 'reasoning_delta';
      node_run_id?: string | null;
      node_id: string;
      text: string;
      run_id?: string;
      event_id?: string;
      sequence?: number;
      created_at?: string;
      delta_index?: number | null;
      content_type?: 'text' | 'reasoning' | null;
    }
  | {
      type: 'usage_snapshot';
      node_run_id?: string | null;
      node_id: string;
      usage: unknown;
      run_id?: string;
      event_id?: string;
      sequence?: number;
      created_at?: string;
      delta_index?: number | null;
      content_type?: 'text' | 'reasoning' | null;
    }
  | {
      type: 'flow_finished';
      run_id: string;
      status: string;
      output: Record<string, unknown>;
      event_id?: string;
      sequence?: number;
      created_at?: string;
      delta_index?: number | null;
      content_type?: 'text' | 'reasoning' | null;
    }
  | {
      type: 'flow_failed';
      run_id: string;
      error: string;
      error_payload?: Record<string, unknown> | null;
      event_id?: string;
      sequence?: number;
      created_at?: string;
      delta_index?: number | null;
      content_type?: 'text' | 'reasoning' | null;
    }
  | {
      type: 'flow_cancelled';
      run_id: string;
      status: 'cancelled' | string;
      reason?: string;
      manual_stop?: boolean;
      event_id?: string;
      sequence?: number;
      created_at?: string;
      delta_index?: number | null;
      content_type?: 'text' | 'reasoning' | null;
    }
  | {
      type: 'waiting_human';
      run_id: string;
      node_run_id?: string | null;
      node_id?: string;
      status: 'waiting_human' | string;
      event_id?: string;
      sequence?: number;
      created_at?: string;
      delta_index?: number | null;
      content_type?: 'text' | 'reasoning' | null;
    }
  | {
      type: 'waiting_callback';
      run_id: string;
      node_run_id?: string | null;
      node_id?: string;
      status: 'waiting_callback' | string;
      event_id?: string;
      sequence?: number;
      created_at?: string;
      delta_index?: number | null;
      content_type?: 'text' | 'reasoning' | null;
    }
  | {
      type: 'heartbeat';
      event_id?: string;
      sequence?: number;
      created_at?: string;
      delta_index?: number | null;
      content_type?: 'text' | 'reasoning' | null;
    }
  | {
      type: 'replay_expired';
      run_id: string;
      from_sequence?: number | null;
      reason?: 'cursor_expired' | string;
      event_id?: string;
      sequence?: number;
      created_at?: string;
      delta_index?: number | null;
      content_type?: 'text' | 'reasoning' | null;
    };

export interface ConsoleFlowDebugStreamHandlers {
  onEvent: (event: ConsoleFlowDebugStreamEvent) => void;
  onCompleted?: () => void;
  getAbortController?: (abortController: AbortController) => void;
}

export interface ConsoleNodeLastRun {
  flow_run: ConsoleFlowRunDetail;
  node_run: ConsoleNodeRunDetail;
  checkpoints: ConsoleRunCheckpoint[];
  events: ConsoleRunEvent[];
}

export interface ConsoleDebugVariableSnapshot {
  snapshot_schema_version?: string;
  workspace_id?: string;
  actor_user_id?: string;
  draft_id?: string;
  flow_schema_version?: string;
  document_hash?: string;
  debug_session_id?: string;
  latest_run_scope?: {
    flow_run_id: string;
    run_mode: string;
    status: string;
    target_node_id: string | null;
  } | null;
  snapshot_completeness?: string;
  source_flow_run_ids?: Record<string, unknown>;
  source_node_run_ids?: Record<string, unknown>;
  variable_cache: Record<string, Record<string, unknown>>;
}

export interface ConsoleRuntimeDebugArtifactPreview {
  __runtime_debug_artifact: true;
  artifact_scope?: string;
  field_path?: string[];
  is_truncated: boolean;
  original_size_bytes: number;
  preview_size_bytes: number;
  content_type: string;
  artifact_ref: string;
  preview: string;
}
