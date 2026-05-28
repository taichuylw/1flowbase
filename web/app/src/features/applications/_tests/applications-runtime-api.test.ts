import { afterEach, describe, expect, test, vi } from 'vitest';

vi.mock('@1flowbase/api-client', () => ({
  completeConsoleCallbackTask: vi.fn().mockResolvedValue(undefined),
  getConsoleApplicationRunDetail: vi.fn().mockResolvedValue({
    flow_run: { id: 'run-1' },
    node_runs: [],
    checkpoints: [],
    callback_tasks: [],
    events: []
  }),
  getConsoleApplicationRunMonitoringReport: vi.fn().mockResolvedValue({
    meta: {
      started_from: null,
      started_to: null,
      bucket: 'day',
      slow_run_threshold_ms: 30000
    },
    overview: {
      total_count: 0,
      success_count: 0,
      failed_count: 0,
      cancelled_count: 0,
      success_rate: 0,
      failed_rate: 0,
      running_count_included: false
    },
    duration: {
      duration_recorded_count: 0,
      avg_duration_ms: 0,
      percentile_fifty_duration_ms: 0,
      percentile_ninety_five_duration_ms: 0,
      slow_run_rate: 0
    },
    tokens: {
      total_tokens_sum: 0,
      avg_tokens_per_run: 0,
      token_recorded_count: 0
    },
    tool_callbacks: {
      total_tool_callback_count: 0,
      avg_tool_callback_count: 0,
      runs_with_tool_callback: 0
    },
    nodes: {
      avg_unique_node_count: 0,
      max_unique_node_count: 0
    },
    concurrency: {
      peak_concurrency: 0
    },
    tokens_trend: [],
    protocols: [],
    sources: [],
    authorized_accounts: [],
    external_users: [],
    api_keys: [],
    external_conversations: [],
    slowest_runs: [],
    high_token_runs: []
  }),
  getConsoleApplicationRuns: vi.fn().mockResolvedValue([]),
  getConsoleRuntimeDebugStream: vi.fn().mockResolvedValue({ parts: [] }),
  resumeConsoleFlowRun: vi.fn().mockResolvedValue(undefined),
  getDefaultApiBaseUrl: vi.fn().mockReturnValue('http://127.0.0.1:7800')
}));

import {
  completeConsoleCallbackTask,
  getConsoleApplicationRunDetail,
  getConsoleApplicationRunMonitoringReport,
  getConsoleApplicationRuns,
  getConsoleRuntimeDebugStream,
  resumeConsoleFlowRun
} from '@1flowbase/api-client';

import {
  applicationRunDetailQueryKey,
  applicationRunMonitoringReportQueryKey,
  applicationRunsQueryKey,
  applicationRuntimeDebugStreamQueryKey,
  completeCallbackTask,
  fetchApplicationRunDetail,
  fetchApplicationRunMonitoringReport,
  fetchApplicationRuns,
  fetchRuntimeDebugStream,
  resumeFlowRun
} from '../api/runtime';

afterEach(() => {
  vi.clearAllMocks();
});

describe('applications runtime api', () => {
  test('builds stable runtime query keys', () => {
    expect(applicationRunsQueryKey('app-1')).toEqual([
      'applications',
      'app-1',
      'runtime',
      'runs',
      1,
      20,
      'all',
      'started_at',
      'desc'
    ]);
    expect(applicationRunDetailQueryKey('app-1', 'run-1')).toEqual([
      'applications',
      'app-1',
      'runtime',
      'runs',
      'run-1'
    ]);
    expect(applicationRuntimeDebugStreamQueryKey('app-1', 'run-1')).toEqual([
      'applications',
      'app-1',
      'runtime',
      'runs',
      'run-1',
      'debug-stream'
    ]);
    expect(applicationRunMonitoringReportQueryKey('app-1')).toEqual([
      'applications',
      'app-1',
      'runtime',
      'monitoring',
      'run-metrics',
      7,
      'day'
    ]);
  });

  test('passes the resolved base url to runtime read requests', async () => {
    await fetchApplicationRuns('app-1', { cacheMode: 'refresh' });
    await fetchApplicationRunDetail('app-1', 'run-1');
    await fetchApplicationRunMonitoringReport('app-1', {
      timeRangeDays: 28,
      bucket: 'week'
    });
    await fetchRuntimeDebugStream('app-1', 'run-1');

    expect(getConsoleApplicationRuns).toHaveBeenCalledWith(
      'app-1',
      expect.objectContaining({
        page: 1,
        page_size: 20,
        cache_mode: 'refresh',
        sort_by: 'started_at',
        sort_order: 'desc'
      }),
      'http://127.0.0.1:7800'
    );
    expect(getConsoleApplicationRunDetail).toHaveBeenCalledWith(
      'app-1',
      'run-1',
      'http://127.0.0.1:7800'
    );
    expect(getConsoleApplicationRunMonitoringReport).toHaveBeenCalledWith(
      'app-1',
      {
        time_range_days: 28,
        bucket: 'week'
      },
      'http://127.0.0.1:7800'
    );
    expect(getConsoleRuntimeDebugStream).toHaveBeenCalledWith(
      'app-1',
      'run-1',
      'http://127.0.0.1:7800'
    );
  });

  test('maps runtime mutation payloads before calling the console client', async () => {
    await resumeFlowRun(
      'app-1',
      'run-1',
      'checkpoint-1',
      { approved: true },
      'csrf-123'
    );
    await completeCallbackTask(
      'app-1',
      'callback-1',
      { decision: 'approve' },
      'csrf-123'
    );

    expect(resumeConsoleFlowRun).toHaveBeenCalledWith(
      'app-1',
      'run-1',
      {
        checkpoint_id: 'checkpoint-1',
        input_payload: { approved: true }
      },
      'csrf-123',
      'http://127.0.0.1:7800'
    );
    expect(completeConsoleCallbackTask).toHaveBeenCalledWith(
      'app-1',
      'callback-1',
      {
        response_payload: { decision: 'approve' }
      },
      'csrf-123',
      'http://127.0.0.1:7800'
    );
  });
});
