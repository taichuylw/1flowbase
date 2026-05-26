import {
  completeConsoleCallbackTask,
  getConsoleApplicationRunConversationMessages,
  getConsoleApplicationRunDetail,
  getConsoleApplicationRunMonitoringReport,
  getConsoleApplicationRuns,
  getConsoleRuntimeDebugArtifact,
  getConsoleRuntimeDebugStream,
  type ConsoleApplicationRunsPage,
  type ConsoleApplicationRunMonitoringApiKeyUsage,
  type ConsoleApplicationRunMonitoringAuthorizedAccountUsage,
  type ConsoleApplicationRunMonitoringBucket,
  type ConsoleApplicationRunMonitoringExternalConversationUsage,
  type ConsoleApplicationRunMonitoringExternalUserUsage,
  type ConsoleApplicationRunMonitoringProtocolBreakdown,
  type ConsoleApplicationRunMonitoringReport,
  type ConsoleApplicationRunMonitoringRunRank,
  type ConsoleApplicationRunMonitoringSourceBreakdown,
  type ConsoleApplicationConversationMessagesPage,
  resumeConsoleFlowRun,
  type ConsoleApplicationRunDetail,
  type ConsoleApplicationRunSummary,
  type RuntimeDebugStreamPart
} from '@1flowbase/api-client';

import { getApplicationsApiBaseUrl } from './applications';

export type ApplicationRunSummary = ConsoleApplicationRunSummary;
export type ApplicationRunsPage = ConsoleApplicationRunsPage;
export type ApplicationRunDetail = ConsoleApplicationRunDetail;
export type ApplicationRunMonitoringBucket =
  ConsoleApplicationRunMonitoringBucket;
export type ApplicationRunMonitoringReport =
  ConsoleApplicationRunMonitoringReport;
export type ApplicationRunMonitoringApiKeyUsage =
  ConsoleApplicationRunMonitoringApiKeyUsage;
export type ApplicationRunMonitoringAuthorizedAccountUsage =
  ConsoleApplicationRunMonitoringAuthorizedAccountUsage;
export type ApplicationRunMonitoringExternalConversationUsage =
  ConsoleApplicationRunMonitoringExternalConversationUsage;
export type ApplicationRunMonitoringExternalUserUsage =
  ConsoleApplicationRunMonitoringExternalUserUsage;
export type ApplicationRunMonitoringProtocolBreakdown =
  ConsoleApplicationRunMonitoringProtocolBreakdown;
export type ApplicationRunMonitoringRunRank =
  ConsoleApplicationRunMonitoringRunRank;
export type ApplicationRunMonitoringSourceBreakdown =
  ConsoleApplicationRunMonitoringSourceBreakdown;
export type ApplicationConversationMessagesPage =
  ConsoleApplicationConversationMessagesPage;
export type ApplicationRuntimeDebugStreamPart = RuntimeDebugStreamPart;
export type { RuntimeDebugStreamPart };
export type ApplicationRunSortField =
  | 'created_at'
  | 'started_at'
  | 'finished_at'
  | 'updated_at';
export type ApplicationRunSortOrder = 'asc' | 'desc';
export type ApplicationRunCacheMode = 'default' | 'refresh';

export interface FetchApplicationRunMonitoringReportInput {
  timeRangeDays?: number | null;
  bucket?: ApplicationRunMonitoringBucket;
}

export interface FetchApplicationRunsInput {
  page?: number;
  pageSize?: number;
  timeRangeDays?: number | null;
  sortBy?: ApplicationRunSortField;
  sortOrder?: ApplicationRunSortOrder;
  cacheMode?: ApplicationRunCacheMode;
}

export const applicationRunsQueryKey = (
  applicationId: string,
  input: FetchApplicationRunsInput = {}
) =>
  [
    'applications',
    applicationId,
    'runtime',
    'runs',
    input.page ?? 1,
    input.pageSize ?? 20,
    input.timeRangeDays ?? 'all',
    input.sortBy ?? 'started_at',
    input.sortOrder ?? 'desc'
  ] as const;

export const applicationRunDetailQueryKey = (
  applicationId: string,
  runId: string
) => ['applications', applicationId, 'runtime', 'runs', runId] as const;

export const applicationConversationMessagesQueryKey = (
  applicationId: string,
  runId: string
) =>
  [
    'applications',
    applicationId,
    'runtime',
    'runs',
    runId,
    'conversation',
    'around',
    runId
  ] as const;

export const applicationRuntimeDebugStreamQueryKey = (
  applicationId: string,
  runId: string
) =>
  [
    'applications',
    applicationId,
    'runtime',
    'runs',
    runId,
    'debug-stream'
  ] as const;

export const applicationRunMonitoringReportQueryKey = (
  applicationId: string,
  input: FetchApplicationRunMonitoringReportInput = {}
) =>
  [
    'applications',
    applicationId,
    'runtime',
    'monitoring',
    'run-metrics',
    input.timeRangeDays ?? 7,
    input.bucket ?? 'day'
  ] as const;

export function fetchApplicationRuns(
  applicationId: string,
  input: FetchApplicationRunsInput = {}
) {
  return getConsoleApplicationRuns(
    applicationId,
    {
      page: input.page ?? 1,
      page_size: input.pageSize ?? 20,
      time_range_days: input.timeRangeDays ?? undefined,
      sort_by: input.sortBy ?? 'started_at',
      sort_order: input.sortOrder ?? 'desc',
      cache_mode: input.cacheMode ?? 'default'
    },
    getApplicationsApiBaseUrl()
  );
}

export function fetchApplicationRunMonitoringReport(
  applicationId: string,
  input: FetchApplicationRunMonitoringReportInput = {}
) {
  return getConsoleApplicationRunMonitoringReport(
    applicationId,
    {
      time_range_days: input.timeRangeDays ?? 7,
      bucket: input.bucket ?? 'day'
    },
    getApplicationsApiBaseUrl()
  );
}

export function fetchApplicationRunDetail(
  applicationId: string,
  runId: string
) {
  return getConsoleApplicationRunDetail(
    applicationId,
    runId,
    getApplicationsApiBaseUrl()
  );
}

export function fetchApplicationConversationMessages(
  applicationId: string,
  runId: string,
  input: {
    before?: string;
    after?: string;
    limit?: number;
  } = {}
) {
  return getConsoleApplicationRunConversationMessages(
    applicationId,
    runId,
    {
      before: input.before,
      after: input.after,
      limit: input.limit
    },
    getApplicationsApiBaseUrl()
  );
}

export function fetchRuntimeDebugStream(applicationId: string, runId: string) {
  return getConsoleRuntimeDebugStream(
    applicationId,
    runId,
    getApplicationsApiBaseUrl()
  );
}

export { getConsoleRuntimeDebugStream };

export function fetchRuntimeDebugArtifact(
  applicationId: string,
  artifactId: string
) {
  return getConsoleRuntimeDebugArtifact(
    applicationId,
    artifactId,
    getApplicationsApiBaseUrl()
  );
}

export function resumeFlowRun(
  applicationId: string,
  runId: string,
  checkpointId: string,
  inputPayload: Record<string, unknown>,
  csrfToken: string
) {
  return resumeConsoleFlowRun(
    applicationId,
    runId,
    {
      checkpoint_id: checkpointId,
      input_payload: inputPayload
    },
    csrfToken,
    getApplicationsApiBaseUrl()
  );
}

export function completeCallbackTask(
  applicationId: string,
  callbackTaskId: string,
  responsePayload: Record<string, unknown>,
  csrfToken: string
) {
  return completeConsoleCallbackTask(
    applicationId,
    callbackTaskId,
    {
      response_payload: responsePayload
    },
    csrfToken,
    getApplicationsApiBaseUrl()
  );
}
