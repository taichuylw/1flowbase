import type { FlowAuthoringDocument } from '@1flowbase/flow-schema';

import { ApiClientError } from '../../errors';
import { apiFetch } from '../../transport';
import type {
  ConsoleApplicationRunDetail,
  ConsoleApplicationRunMonitoringReport,
  ConsoleApplicationRunsPage,
  ConsoleApplicationRuntimeActivity,
  ConsoleDebugVariableSnapshot,
  ConsoleNodeLastRun,
  DeleteConsoleDebugVariableCacheEntriesInput,
  GetConsoleApplicationRunMonitoringReportInput,
  GetConsoleApplicationRunsInput,
  RuntimeDebugStreamResponse,
  UpsertConsoleDebugVariableCacheEntryInput
} from './types';

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
