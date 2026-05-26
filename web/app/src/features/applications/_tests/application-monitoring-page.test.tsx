import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

const echartsMock = vi.hoisted(() => ({
  chart: {
    dispose: vi.fn(),
    resize: vi.fn(),
    setOption: vi.fn()
  },
  init: vi.fn()
}));

const runtimeApi = vi.hoisted(() => ({
  applicationRunMonitoringReportQueryKey: (
    applicationId: string,
    input?: {
      timeRangeDays?: number | null;
      bucket?: 'hour' | 'day' | 'week' | 'month';
    }
  ) =>
    [
      'applications',
      applicationId,
      'runtime',
      'monitoring',
      'run-metrics',
      input?.timeRangeDays ?? 7,
      input?.bucket ?? 'day'
    ] as const,
  fetchApplicationRunMonitoringReport: vi.fn()
}));

vi.mock('echarts/core', () => ({
  init: echartsMock.init,
  use: vi.fn()
}));
vi.mock('echarts/charts', () => ({
  BarChart: {},
  LineChart: {},
  PieChart: {}
}));
vi.mock('echarts/components', () => ({
  GridComponent: {},
  LegendComponent: {},
  TooltipComponent: {}
}));
vi.mock('echarts/renderers', () => ({
  CanvasRenderer: {}
}));
vi.mock('../api/runtime', () => runtimeApi);

import { AppProviders } from '../../../app/AppProviders';
import { resetAuthStore } from '../../../state/auth-store';
import { ApplicationMonitoringPage } from '../pages/ApplicationMonitoringPage';

function monitoringReport() {
  return {
    meta: {
      started_from: '2026-05-01T00:00:00Z',
      started_to: null,
      bucket: 'day',
      slow_run_threshold_ms: 30000
    },
    overview: {
      total_count: 12,
      success_count: 9,
      failed_count: 2,
      cancelled_count: 1,
      success_rate: 0.75,
      failed_rate: 0.1667,
      running_count_included: false
    },
    duration: {
      duration_recorded_count: 12,
      avg_duration_ms: 2400,
      p50_duration_ms: 1800,
      p95_duration_ms: 32000,
      slow_run_rate: 0.08
    },
    tokens: {
      total_tokens_sum: 5600,
      avg_tokens_per_run: 466.7,
      token_recorded_count: 12
    },
    tool_callbacks: {
      total_tool_callback_count: 7,
      avg_tool_callback_count: 0.58,
      runs_with_tool_callback: 3
    },
    nodes: {
      avg_unique_node_count: 4.2,
      max_unique_node_count: 8
    },
    concurrency: {
      peak_concurrency: 3
    },
    tokens_trend: [
      {
        bucket_start: '2026-05-01T00:00:00Z',
        run_count: 4,
        total_tokens: 1200
      },
      {
        bucket_start: '2026-05-02T00:00:00Z',
        run_count: 8,
        total_tokens: 4400
      }
    ],
    protocols: [
      {
        protocol: 'default',
        request_count: 7,
        success_rate: 0.85,
        avg_duration_ms: 1800,
        total_tokens: 2600
      },
      {
        protocol: 'openai-responses-v1',
        request_count: 5,
        success_rate: 0.6,
        avg_duration_ms: 3200,
        total_tokens: 3000
      }
    ],
    sources: [
      {
        source: 'console',
        request_count: 7,
        success_rate: 0.85,
        total_tokens: 2600
      },
      {
        source: 'public_api',
        request_count: 5,
        success_rate: 0.6,
        total_tokens: 3000
      }
    ],
    authorized_accounts: [
      {
        authorized_account: 'root',
        request_count: 7,
        total_tokens: 2600,
        avg_duration_ms: 1800,
        failed_count: 0
      }
    ],
    external_users: [
      {
        external_user: 'customer-1',
        request_count: 5,
        total_tokens: 3000,
        avg_duration_ms: 3200,
        failed_count: 2
      }
    ],
    api_keys: [
      {
        api_key_id: 'key-1',
        request_count: 5,
        total_tokens: 3000,
        avg_duration_ms: 3200,
        failed_count: 2
      }
    ],
    external_conversations: [
      {
        external_conversation_id: 'conversation-1',
        request_count: 5,
        total_tokens: 3000,
        avg_duration_ms: 3200,
        failed_count: 2
      }
    ],
    slowest_runs: [
      {
        flow_run_id: 'run-2',
        title: '最慢运行',
        status: 'failed',
        started_at: '2026-05-02T10:00:00Z',
        finished_at: '2026-05-02T10:00:40Z',
        duration_ms: 40000,
        total_tokens: 3000
      }
    ],
    high_token_runs: [
      {
        flow_run_id: 'run-2',
        title: '最慢运行',
        status: 'failed',
        started_at: '2026-05-02T10:00:00Z',
        finished_at: '2026-05-02T10:00:40Z',
        duration_ms: 40000,
        total_tokens: 3000
      }
    ]
  };
}

describe('ApplicationMonitoringPage', () => {
  beforeEach(() => {
    resetAuthStore();
    vi.clearAllMocks();
    echartsMock.init.mockReturnValue(echartsMock.chart);
    runtimeApi.fetchApplicationRunMonitoringReport.mockResolvedValue(
      monitoringReport()
    );
  });

  test('renders backend aggregated monitoring report and charts', async () => {
    render(
      <AppProviders>
        <ApplicationMonitoringPage applicationId="app-1" />
      </AppProviders>
    );

    expect(await screen.findByText('12')).toBeInTheDocument();
    expect(screen.getByText('75.0%')).toBeInTheDocument();
    expect(screen.getByText('运行中数未包含')).toBeInTheDocument();
    expect(screen.getByText('openai-responses-v1')).toBeInTheDocument();
    expect(screen.getByText('customer-1')).toBeInTheDocument();
    expect(screen.getAllByText('最慢运行').length).toBeGreaterThan(0);
    expect(echartsMock.chart.setOption).toHaveBeenCalled();
  });

  test('refreshes the report when time range changes', async () => {
    render(
      <AppProviders>
        <ApplicationMonitoringPage applicationId="app-1" />
      </AppProviders>
    );

    expect(await screen.findByText('12')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('radio', { name: '过去 4 周' }));

    await waitFor(() => {
      expect(
        runtimeApi.fetchApplicationRunMonitoringReport
      ).toHaveBeenLastCalledWith('app-1', {
        timeRangeDays: 28,
        bucket: 'day'
      });
    });
  });
});
