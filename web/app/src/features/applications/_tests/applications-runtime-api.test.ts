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
  fetchConsoleRuntimeModelRecords: vi.fn().mockResolvedValue({
    items: [],
    total: 0
  }),
  getConsoleRuntimeDebugStream: vi.fn().mockResolvedValue({ parts: [] }),
  resumeConsoleFlowRun: vi.fn().mockResolvedValue(undefined),
  getDefaultApiBaseUrl: vi.fn().mockReturnValue('http://127.0.0.1:7800')
}));

import {
  completeConsoleCallbackTask,
  fetchConsoleRuntimeModelRecords,
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
  fetchApplicationConversationMessages,
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
      'desc',
      ''
    ]);
    expect(
      applicationRunsQueryKey('app-1', { titleIncludes: '退款' })
    ).toEqual([
      'applications',
      'app-1',
      'runtime',
      'runs',
      1,
      20,
      'all',
      'started_at',
      'desc',
      '退款'
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

  test('reads run log summaries from Runtime Data Model records', async () => {
    vi.mocked(fetchConsoleRuntimeModelRecords).mockResolvedValueOnce({
      items: [
        {
          id: 'run-1',
          flow_run_id: 'run-1',
          application_id: 'app-1',
          scope_id: 'workspace-1',
          run_mode: 'published_api_run',
          status: 'succeeded',
          target_node_id: null,
          title: '退款总结',
          external_conversation_id: 'conversation-1',
          total_tokens: 120,
          unique_node_count: 3,
          tool_callback_count: 1,
          started_at: '2026-05-08T00:00:00Z',
          finished_at: '2026-05-08T00:00:01Z',
          created_at: '2026-05-08T00:00:00Z',
          updated_at: '2026-05-08T00:00:01Z'
        }
      ],
      total: 1
    });

    await expect(
      fetchApplicationRuns('app-1', { titleIncludes: '退款' })
    ).resolves.toMatchObject({
      items: [
        {
          flow_run_id: 'run-1',
          application_id: 'app-1',
          scope_id: 'workspace-1',
          title: '退款总结'
        }
      ],
      total: 1,
      page: 1,
      page_size: 20
    });

    expect(fetchConsoleRuntimeModelRecords).toHaveBeenCalledWith(
      'application_run_log_summaries',
      {
        page: 1,
        page_size: 20,
        filter: {
          application_id: { $eq: 'app-1' },
          title: { $includes: '退款' }
        },
        sort: {
          field: 'started_at',
          direction: 'desc'
        }
      },
      'http://127.0.0.1:7800'
    );
    expect(getConsoleApplicationRuns).not.toHaveBeenCalled();
  });

  test('reads conversation messages from Runtime Data Model records', async () => {
    vi.mocked(fetchConsoleRuntimeModelRecords).mockResolvedValueOnce({
      items: [
        {
          id: 'message-1',
          application_id: 'app-1',
          scope_id: 'workspace-1',
          conversation_id: 'conversation-record-1',
          flow_run_id: 'run-1',
          role: 'assistant',
          content: '退款政策摘要',
          sequence: 2,
          status: 'succeeded',
          created_at: '2026-05-08T00:00:01Z'
        }
      ],
      total: 1
    });

    await expect(
      fetchApplicationConversationMessages('app-1', {
        conversationId: 'conversation-record-1',
        page: 1,
        pageSize: 5
      })
    ).resolves.toMatchObject({
      items: [
        {
          id: 'message-1',
          conversation_id: 'conversation-record-1',
          flow_run_id: 'run-1',
          role: 'assistant',
          content: '退款政策摘要'
        }
      ],
      total: 1,
      page: 1,
      page_size: 5
    });

    expect(fetchConsoleRuntimeModelRecords).toHaveBeenCalledWith(
      'application_conversation_messages',
      {
        page: 1,
        page_size: 5,
        filter: {
          application_id: { $eq: 'app-1' },
          conversation_id: { $eq: 'conversation-record-1' }
        },
        sort: {
          field: 'sequence',
          direction: 'desc'
        }
      },
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
