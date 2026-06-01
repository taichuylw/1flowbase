import type { FlowAuthoringDocument } from '@1flowbase/flow-schema';

import { ApiClientError } from './errors';
import { apiFetch } from './transport';

export { getConsoleApplicationRunConversationMessages } from './console-application-run-conversation';

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

export type ConsoleApplicationRuntimeTrend =
  | 'rising'
  | 'steady'
  | 'falling';

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

export function startConsoleNodeDebugPreview(
  applicationId: string,
  nodeId: string,
  input: {
    input_payload: Record<string, unknown>;
    document?: FlowAuthoringDocument;
    debug_session_id?: string;
  },
  csrfToken: string,
  baseUrl?: string
) {
  return apiFetch<ConsoleNodeLastRun>({
    path: `/api/console/applications/${applicationId}/orchestration/nodes/${nodeId}/debug-runs`,
    method: 'POST',
    body: input,
    csrfToken,
    baseUrl
  });
}

export function startConsoleFlowDebugRun(
  applicationId: string,
  input: {
    input_payload: Record<string, unknown>;
    document?: FlowAuthoringDocument;
    debug_session_id?: string;
  },
  csrfToken: string,
  baseUrl?: string
) {
  return apiFetch<ConsoleApplicationRunDetail>({
    path: `/api/console/applications/${applicationId}/orchestration/debug-runs`,
    method: 'POST',
    body: input,
    csrfToken,
    baseUrl
  });
}

function dispatchSseEvent(
  eventBuffer: string,
  handlers: ConsoleFlowDebugStreamHandlers
) {
  let eventId: string | undefined;
  let eventType: string | undefined;
  const dataLines: string[] = [];

  for (const line of eventBuffer.split(/\r?\n/)) {
    if (line.startsWith('data:')) {
      dataLines.push(line.slice(5).trimStart());
      continue;
    }

    if (line.startsWith('id:')) {
      eventId = line.slice(3).trim();
      continue;
    }

    if (line.startsWith('event:')) {
      eventType = line.slice(6).trim();
    }
  }

  if (dataLines.length === 0) {
    return;
  }

  const rawPayload = JSON.parse(dataLines.join('\n'));
  const parsedPayload = normalizeStreamPayload(rawPayload, eventType, eventId);

  if (!parsedPayload) {
    return;
  }

  handlers.onEvent(parsedPayload);
}

function normalizeStreamPayload(
  raw: unknown,
  fallbackEventType?: string,
  fallbackEventId?: string
): ConsoleFlowDebugStreamEvent | null {
  if (!isRecord(raw)) {
    return null;
  }

  const explicitType = isNonEmptyString(raw.event_type)
    ? raw.event_type
    : isNonEmptyString(raw.type)
      ? raw.type
      : undefined;
  const payloadType = isRecord(raw.payload)
    ? isNonEmptyString(raw.payload.type)
      ? String(raw.payload.type)
      : undefined
    : undefined;
  const sseEventType = isNonEmptyString(fallbackEventType)
    ? fallbackEventType
    : undefined;
  const eventType = explicitType ?? payloadType ?? sseEventType;

  if (!eventType || !isKnownStreamEventType(eventType)) {
    return null;
  }

  if (isRecord(raw.payload) && isConsoleFlowDebugEventEnvelope(raw)) {
    return normalizeFromEnvelope(raw, eventType, fallbackEventId);
  }

  if (!isNonEmptyString(raw.type)) {
    return null;
  }

  const legacyEvent = raw as ConsoleFlowDebugStreamEvent;
  if (fallbackEventId && !isNonEmptyString(raw.event_id)) {
    return {
      ...legacyEvent,
      event_id: fallbackEventId
    };
  }

  return legacyEvent;
}

function normalizeFromEnvelope(
  raw: Record<string, unknown>,
  eventType: ConsoleFlowDebugStreamEvent['type'],
  fallbackEventId?: string
): ConsoleFlowDebugStreamEvent | null {
  const payload = isRecord(raw.payload) ? raw.payload : {};
  const contentType: 'text' | 'reasoning' | null | undefined =
    raw.content_type === 'text' || raw.content_type === 'reasoning'
      ? raw.content_type
      : raw.content_type === null
        ? null
        : undefined;
  const base: {
    event_id?: string;
    run_id?: string;
    sequence?: number;
    created_at?: string;
    delta_index?: number | null;
    content_type?: 'text' | 'reasoning' | null;
    node_run_id?: string | null;
  } = {
    event_id: toOptionalString(raw.event_id) ?? fallbackEventId,
    run_id: toOptionalString(raw.run_id),
    sequence: typeof raw.sequence === 'number' ? raw.sequence : undefined,
    created_at: toOptionalString(raw.created_at),
    delta_index:
      raw.delta_index === null || typeof raw.delta_index === 'number'
        ? raw.delta_index
        : undefined,
    content_type: contentType,
    node_run_id: normalizeNullableString(raw.node_run_id)
  };

  const nodeId = toOptionalString(payload.node_id) ?? '';
  const output = isRecord(payload.output) ? payload.output : {};
  const errorPayload = isRecord(payload.error_payload)
    ? payload.error_payload
    : payload.error_payload === null
      ? null
      : undefined;

  if (eventType === 'flow_accepted') {
    return {
      ...base,
      type: 'flow_accepted',
      run_id: base.run_id ?? '',
      status: isNonEmptyString(payload.status)
        ? String(payload.status)
        : 'running'
    };
  }

  if (eventType === 'flow_started') {
    return {
      ...base,
      type: 'flow_started',
      run_id: base.run_id ?? '',
      status: isNonEmptyString(payload.status)
        ? String(payload.status)
        : 'running'
    };
  }

  if (eventType === 'node_started') {
    return {
      ...base,
      type: 'node_started',
      run_id: base.run_id,
      node_run_id: base.node_run_id ?? '',
      node_id: nodeId,
      node_type: isNonEmptyString(payload.node_type)
        ? String(payload.node_type)
        : 'node',
      title: isNonEmptyString(payload.title) ? String(payload.title) : 'node',
      input_payload: isRecord(payload.input_payload)
        ? payload.input_payload
        : {},
      started_at: toOptionalString(payload.started_at)
    };
  }

  if (eventType === 'node_finished') {
    return {
      ...base,
      type: 'node_finished',
      run_id: base.run_id,
      node_run_id: base.node_run_id ?? '',
      node_id: nodeId,
      status: isNonEmptyString(payload.status)
        ? String(payload.status)
        : 'succeeded',
      output_payload: isRecord(payload.output_payload)
        ? payload.output_payload
        : {},
      error_payload: errorPayload,
      metrics_payload: isRecord(payload.metrics_payload)
        ? payload.metrics_payload
        : {},
      debug_payload: isRecord(payload.debug_payload)
        ? payload.debug_payload
        : {},
      started_at: toOptionalString(payload.started_at),
      finished_at:
        payload.finished_at === null
          ? null
          : toOptionalString(payload.finished_at)
    };
  }

  if (eventType === 'text_delta') {
    return {
      ...base,
      type: 'text_delta',
      run_id: base.run_id,
      node_run_id: base.node_run_id,
      node_id: nodeId,
      text:
        toOptionalString(raw.text) ??
        toOptionalString(payload.text) ??
        toOptionalString(payload.delta) ??
        ''
    };
  }

  if (eventType === 'reasoning_delta') {
    return {
      ...base,
      type: 'reasoning_delta',
      run_id: base.run_id,
      node_run_id: base.node_run_id,
      node_id: nodeId,
      text:
        toOptionalString(raw.text) ??
        toOptionalString(payload.text) ??
        toOptionalString(payload.delta) ??
        ''
    };
  }

  if (eventType === 'usage_snapshot') {
    return {
      ...base,
      type: 'usage_snapshot',
      run_id: base.run_id,
      node_run_id: base.node_run_id,
      node_id: nodeId,
      usage: payload.usage
    };
  }

  if (eventType === 'flow_finished') {
    return {
      ...base,
      type: 'flow_finished',
      run_id: base.run_id ?? '',
      status: isNonEmptyString(payload.status)
        ? String(payload.status)
        : 'completed',
      output
    };
  }

  if (eventType === 'flow_failed') {
    return {
      ...base,
      type: 'flow_failed',
      run_id: base.run_id ?? '',
      error: isNonEmptyString(payload.error)
        ? String(payload.error)
        : 'stream error',
      error_payload: errorPayload
    };
  }

  if (eventType === 'flow_cancelled') {
    return {
      ...base,
      type: 'flow_cancelled',
      run_id: base.run_id ?? '',
      status: isNonEmptyString(payload.status)
        ? String(payload.status)
        : 'cancelled',
      reason: toOptionalString(payload.reason),
      manual_stop: payload.manual_stop === true
    };
  }

  if (eventType === 'waiting_human') {
    return {
      ...base,
      type: 'waiting_human',
      run_id: base.run_id ?? '',
      node_run_id: base.node_run_id,
      node_id: nodeId || undefined,
      status: 'waiting_human'
    };
  }

  if (eventType === 'waiting_callback') {
    return {
      ...base,
      type: 'waiting_callback',
      run_id: base.run_id ?? '',
      node_run_id: base.node_run_id,
      node_id: nodeId || undefined,
      status: 'waiting_callback'
    };
  }

  if (eventType === 'heartbeat') {
    return {
      ...base,
      type: 'heartbeat'
    };
  }

  if (eventType === 'replay_expired') {
    return {
      ...base,
      type: 'replay_expired',
      run_id: base.run_id ?? toOptionalString(payload.run_id) ?? '',
      from_sequence:
        typeof payload.from_sequence === 'number'
          ? payload.from_sequence
          : typeof raw.from_sequence === 'number'
            ? raw.from_sequence
            : undefined,
      reason:
        toOptionalString(payload.reason) ??
        toOptionalString(raw.reason) ??
        'cursor_expired'
    };
  }

  return null;
}

function isKnownStreamEventType(
  type: string
): type is ConsoleFlowDebugStreamEvent['type'] {
  return (
    type === 'flow_accepted' ||
    type === 'flow_started' ||
    type === 'node_started' ||
    type === 'node_finished' ||
    type === 'text_delta' ||
    type === 'reasoning_delta' ||
    type === 'usage_snapshot' ||
    type === 'flow_finished' ||
    type === 'flow_failed' ||
    type === 'flow_cancelled' ||
    type === 'waiting_human' ||
    type === 'waiting_callback' ||
    type === 'heartbeat' ||
    type === 'replay_expired'
  );
}

function isConsoleFlowDebugEventEnvelope(raw: Record<string, unknown>) {
  return typeof raw.event_type === 'string' && isRecord(raw.payload);
}

function normalizeNullableString(value: unknown): string | null | undefined {
  if (value === null) {
    return null;
  }

  return toOptionalString(value);
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === 'string' && value.trim().length > 0;
}

function toOptionalString(value: unknown): string | undefined {
  return isNonEmptyString(value) ? value : undefined;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

async function readSseStream(
  response: Response,
  handlers: ConsoleFlowDebugStreamHandlers
) {
  const reader = response.body?.getReader();

  if (!reader) {
    handlers.onCompleted?.();
    return;
  }

  const decoder = new TextDecoder();
  let buffer = '';

  while (true) {
    const { value, done } = await reader.read();

    if (done) {
      if (buffer.trim().length > 0) {
        dispatchSseEvent(buffer, handlers);
      }
      handlers.onCompleted?.();
      return;
    }

    buffer += decoder.decode(value, { stream: true });
    const eventFrames = buffer.split(/\r?\n\r?\n/);
    buffer = eventFrames.pop() ?? '';

    for (const eventFrame of eventFrames) {
      dispatchSseEvent(eventFrame, handlers);
    }
  }
}

export async function startConsoleFlowDebugRunStream(
  applicationId: string,
  input: {
    input_payload: Record<string, unknown>;
    document?: FlowAuthoringDocument;
    debug_session_id?: string;
  },
  csrfToken: string,
  handlers: ConsoleFlowDebugStreamHandlers,
  options?: {
    cursor?: ConsoleFlowDebugStreamCursor;
    baseUrl?: string;
  }
) {
  const abortController = new AbortController();
  handlers.getAbortController?.(abortController);
  const baseUrl = options?.baseUrl;
  const query = buildStreamCursorQuery(options?.cursor);

  const response = await fetch(
    `${baseUrl ?? ''}/api/console/applications/${applicationId}/orchestration/debug-runs/stream${query}`,
    {
      method: 'POST',
      credentials: 'include',
      signal: abortController.signal,
      headers: {
        accept: 'text/event-stream',
        'content-type': 'application/json',
        'x-csrf-token': csrfToken
      },
      body: JSON.stringify(input)
    }
  );

  if (!response.ok) {
    throw await ApiClientError.fromResponse(response);
  }

  await readSseStream(response, handlers);
}

export async function subscribeConsoleFlowDebugRunStream(
  applicationId: string,
  runId: string,
  csrfToken: string,
  handlers: ConsoleFlowDebugStreamHandlers,
  options?: {
    cursor?: ConsoleFlowDebugStreamCursor;
    baseUrl?: string;
  }
) {
  const abortController = new AbortController();
  handlers.getAbortController?.(abortController);
  const baseUrl = options?.baseUrl;
  const query = buildStreamCursorQuery(options?.cursor);

  const response = await fetch(
    `${baseUrl ?? ''}/api/console/applications/${applicationId}/orchestration/runs/${runId}/debug-stream${query}`,
    {
      method: 'GET',
      credentials: 'include',
      signal: abortController.signal,
      headers: {
        accept: 'text/event-stream',
        'x-csrf-token': csrfToken
      }
    }
  );

  if (!response.ok) {
    throw await ApiClientError.fromResponse(response);
  }

  await readSseStream(response, handlers);
}

function buildStreamCursorQuery(cursor?: ConsoleFlowDebugStreamCursor) {
  if (!cursor) {
    return '';
  }

  const params = new URLSearchParams();
  if (typeof cursor.from_sequence === 'number') {
    params.set('from_sequence', String(cursor.from_sequence));
  }
  if (cursor.last_event_id) {
    params.set('last_event_id', cursor.last_event_id);
  }

  const query = params.toString();
  return query ? `?${query}` : '';
}

export function resumeConsoleFlowRun(
  applicationId: string,
  runId: string,
  input: { checkpoint_id: string; input_payload: Record<string, unknown> },
  csrfToken: string,
  baseUrl?: string
) {
  return apiFetch<ConsoleApplicationRunDetail>({
    path: `/api/console/applications/${applicationId}/orchestration/runs/${runId}/resume`,
    method: 'POST',
    body: input,
    csrfToken,
    baseUrl
  });
}

export function cancelConsoleFlowRun(
  applicationId: string,
  runId: string,
  csrfToken: string,
  baseUrl?: string
) {
  return apiFetch<ConsoleApplicationRunDetail>({
    path: `/api/console/applications/${applicationId}/orchestration/runs/${runId}/cancel`,
    method: 'POST',
    csrfToken,
    baseUrl
  });
}

export function completeConsoleCallbackTask(
  applicationId: string,
  callbackTaskId: string,
  input: { response_payload: Record<string, unknown> },
  csrfToken: string,
  baseUrl?: string
) {
  return apiFetch<ConsoleApplicationRunDetail>({
    path: `/api/console/applications/${applicationId}/orchestration/callback-tasks/${callbackTaskId}/complete`,
    method: 'POST',
    body: input,
    csrfToken,
    baseUrl
  });
}

export function getConsoleApplicationRuns(
  applicationId: string,
  input: GetConsoleApplicationRunsInput = {},
  baseUrl?: string
) {
  const page = input.page ?? 1;
  const pageSize = input.page_size ?? 20;
  const searchParams = new URLSearchParams({
    page: String(page),
    page_size: String(pageSize)
  });
  if (input.time_range_days !== undefined) {
    searchParams.set('time_range_days', String(input.time_range_days));
  }
  if (input.sort_by !== undefined) {
    searchParams.set('sort_by', input.sort_by);
  }
  if (input.sort_order !== undefined) {
    searchParams.set('sort_order', input.sort_order);
  }
  if (input.cache_mode !== undefined && input.cache_mode !== 'default') {
    searchParams.set('cache_mode', input.cache_mode);
  }
  return apiFetch<ConsoleApplicationRunsPage>({
    path:
      `/api/console/applications/${applicationId}/logs/runs?` +
      searchParams.toString(),
    baseUrl
  });
}

export function getConsoleApplicationRunMonitoringReport(
  applicationId: string,
  input: GetConsoleApplicationRunMonitoringReportInput = {},
  baseUrl?: string
) {
  const searchParams = new URLSearchParams();
  if (input.from !== undefined) {
    searchParams.set('from', input.from);
  }
  if (input.to !== undefined) {
    searchParams.set('to', input.to);
  }
  if (input.time_range_days !== undefined) {
    searchParams.set('time_range_days', String(input.time_range_days));
  }
  if (input.bucket !== undefined) {
    searchParams.set('bucket', input.bucket);
  }
  const query = searchParams.toString();

  return apiFetch<ConsoleApplicationRunMonitoringReport>({
    path:
      `/api/console/applications/${applicationId}/monitoring/run-metrics` +
      (query ? `?${query}` : ''),
    baseUrl
  });
}

export function getConsoleApplicationRuntimeActivity(
  applicationId: string,
  baseUrl?: string
) {
  return apiFetch<ConsoleApplicationRuntimeActivity>({
    path: `/api/console/applications/${applicationId}/monitoring/runtime-activity`,
    method: 'GET',
    baseUrl
  });
}

export function getConsoleApplicationRunDetail(
  applicationId: string,
  runId: string,
  baseUrl?: string
) {
  return apiFetch<ConsoleApplicationRunDetail>({
    path: `/api/console/applications/${applicationId}/logs/runs/${runId}`,
    baseUrl
  });
}

export function getConsoleApplicationRunNodeLastRun(
  applicationId: string,
  runId: string,
  nodeId: string,
  baseUrl?: string
) {
  return apiFetch<ConsoleNodeLastRun | null>({
    path: `/api/console/applications/${applicationId}/logs/runs/${runId}/nodes/${nodeId}`,
    baseUrl
  });
}

export function getConsoleRuntimeDebugStream(
  applicationId: string,
  runId: string,
  baseUrl?: string
) {
  return apiFetch<RuntimeDebugStreamResponse>({
    path: `/api/console/applications/${applicationId}/logs/runs/${runId}/debug-stream`,
    baseUrl
  });
}

export function getConsoleDebugVariableSnapshot(
  applicationId: string,
  baseUrl?: string
) {
  return apiFetch<ConsoleDebugVariableSnapshot>({
    path: `/api/console/applications/${applicationId}/orchestration/debug-variable-snapshot`,
    baseUrl
  });
}

export function upsertConsoleDebugVariableCacheEntry(
  applicationId: string,
  input: UpsertConsoleDebugVariableCacheEntryInput,
  csrfToken: string,
  baseUrl?: string
) {
  return apiFetch<{ ok: boolean }>({
    path: `/api/console/applications/${applicationId}/orchestration/debug-variable-cache`,
    method: 'PUT',
    body: input,
    csrfToken,
    baseUrl
  });
}

export function deleteConsoleDebugVariableCacheEntries(
  applicationId: string,
  input: DeleteConsoleDebugVariableCacheEntriesInput,
  csrfToken: string,
  baseUrl?: string
) {
  return apiFetch<{ ok: boolean }>({
    path: `/api/console/applications/${applicationId}/orchestration/debug-variable-cache`,
    method: 'DELETE',
    body: input,
    csrfToken,
    baseUrl
  });
}

export async function getConsoleRuntimeDebugArtifact(
  applicationId: string,
  artifactId: string,
  baseUrl?: string
) {
  const response = await fetch(
    `${baseUrl ?? ''}/api/console/applications/${applicationId}/orchestration/debug-artifacts/${artifactId}`,
    {
      method: 'GET',
      credentials: 'include',
      headers: {
        accept: 'application/json, text/plain;q=0.9, */*;q=0.1'
      }
    }
  );

  if (!response.ok) {
    throw await ApiClientError.fromResponse(response);
  }

  const contentType = response.headers.get('content-type') ?? '';
  if (contentType.includes('application/json')) {
    return response.json() as Promise<unknown>;
  }

  return response.text();
}

export function getConsoleNodeLastRun(
  applicationId: string,
  nodeId: string,
  baseUrl?: string
) {
  return apiFetch<ConsoleNodeLastRun | null>({
    path: `/api/console/applications/${applicationId}/orchestration/nodes/${nodeId}/last-run`,
    baseUrl
  });
}
