import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { vi } from 'vitest';

const runtimeApi = vi.hoisted(() => ({
  applicationRunsQueryKey: (
    applicationId: string,
    input?: {
      page?: number;
      pageSize?: number;
      timeRangeDays?: number | null;
      sortBy?: 'started_at' | 'finished_at' | 'created_at' | 'updated_at';
      sortOrder?: 'asc' | 'desc';
      cacheMode?: 'default' | 'refresh';
      titleIncludes?: string;
    }
  ) =>
    [
      'applications',
      applicationId,
      'runtime',
      'runs',
      input?.page ?? 1,
      input?.pageSize ?? 20,
      input?.timeRangeDays ?? 'all',
      input?.sortBy ?? 'started_at',
      input?.sortOrder ?? 'desc',
      input?.titleIncludes ?? ''
    ] as const,
  applicationRunTraceTreeQueryKey: (applicationId: string, runId: string) =>
    [
      'applications',
      applicationId,
      'runtime',
      'runs',
      runId,
      'trace-tree'
    ] as const,
  applicationRunOverviewQueryKey: (applicationId: string, runId: string) =>
    [
      'applications',
      applicationId,
      'runtime',
      'runs',
      runId,
      'overview'
    ] as const,
  applicationRunTraceNodeChildrenQueryKey: (
    applicationId: string,
    runId: string,
    traceNodeId: string
  ) =>
    [
      'applications',
      applicationId,
      'runtime',
      'runs',
      runId,
      'trace-tree',
      traceNodeId,
      'children'
    ] as const,
  applicationRunTraceNodeContentQueryKey: (
    applicationId: string,
    runId: string,
    traceNodeId: string
  ) =>
    [
      'applications',
      applicationId,
      'runtime',
      'runs',
      runId,
      'trace-tree',
      traceNodeId,
      'content'
    ] as const,
  applicationRunResumeTimelineQueryKey: (
    applicationId: string,
    runId: string
  ) =>
    [
      'applications',
      applicationId,
      'runtime',
      'runs',
      runId,
      'resume-timeline'
    ] as const,
  applicationRunConversationMessagesQueryKey: (
    applicationId: string,
    runId: string
  ) =>
    [
      'applications',
      applicationId,
      'runtime',
      'runs',
      runId,
      'conversation-messages'
    ] as const,
  applicationConversationMessagesQueryKey: (
    applicationId: string,
    input: { flowRunId?: string | null }
  ) =>
    [
      'applications',
      applicationId,
      'runtime',
      'conversation-messages',
      input.flowRunId ?? ''
    ] as const,
  fetchApplicationRuns: vi.fn(),
  fetchApplicationRunOverview: vi.fn(),
  fetchApplicationRunTraceTree: vi.fn(),
  fetchApplicationRunTraceNodeChildren: vi.fn(),
  fetchApplicationRunTraceNodeContent: vi.fn(),
  fetchApplicationRunTraceNodeDetail: vi.fn(),
  fetchApplicationRunTraceToolCallbackContent: vi.fn(),
  fetchApplicationRunResumeTimeline: vi.fn(),
  fetchApplicationConversationMessages: vi.fn(),
  fetchApplicationRunConversationMessages: vi.fn(),
  fetchRuntimeDebugArtifact: vi.fn(),
  fetchRuntimeDebugArtifacts: vi.fn(),
  exportApplicationRunTraceDump: vi.fn(),
  exportSelectedApplicationRunsTraceDumpZip: vi.fn(),
  resumeFlowRun: vi.fn(),
  completeCallbackTask: vi.fn()
}));

vi.mock('../../api/runtime', () => runtimeApi);

import { AppProviders } from '../../../../app/AppProviders';
import { appI18n } from '../../../../shared/i18n/app-i18n';
import { resetAuthStore, useAuthStore } from '../../../../state/auth-store';
import { ApplicationLogsPage } from '../../pages/ApplicationLogsPage';

function applicationRunsPage(
  items: Array<Record<string, unknown>>,
  overrides?: Partial<{
    total: number;
    page: number;
    page_size: number;
  }>
) {
  return {
    items,
    total: overrides?.total ?? items.length,
    page: overrides?.page ?? 1,
    page_size: overrides?.page_size ?? 20
  };
}

function runSummary(id: string, title: string) {
  return {
    id,
    application_id: 'app-1',
    scope_id: 'workspace-1',
    run_mode: 'published_api_run',
    status: 'succeeded',
    target_node_id: 'node-llm',
    title,
    expand_id: null,
    external_user: null,
    authorized_account: 'root',
    api_key_id: null,
    api_key_name_snapshot: null,
    publication_version_id: null,
    external_conversation_id: null,
    external_trace_id: null,
    compatibility_mode: 'openai-responses-v1',
    idempotency_key: null,
    total_tokens: 10,
    input_tokens: 8,
    output_tokens: 2,
    input_cache_hit_tokens: 0,
    unique_node_count: 2,
    tool_callback_count: 0,
    started_at: '2026-04-17T09:00:00Z',
    finished_at: '2026-04-17T09:00:01Z',
    created_at: '2026-04-17T09:00:00Z',
    updated_at: '2026-04-17T09:00:01Z'
  };
}

function authenticate() {
  useAuthStore.getState().setAuthenticated({
    csrfToken: 'csrf-123',
    actor: {
      id: 'user-1',
      account: 'root',
      effective_display_role: 'root',
      current_workspace_id: 'workspace-1'
    },
    me: {
      id: 'user-1',
      account: 'root',
      email: 'root@example.com',
      phone: null,
      nickname: 'Root',
      name: 'Root',
      avatar_url: null,
      introduction: '',
      preferred_locale: null,
      effective_display_role: 'root',
      permissions: [],
      meta: {}
    }
  });
}

function renderLogsPage() {
  return render(
    <AppProviders>
      <ApplicationLogsPage applicationId="app-1" />
    </AppProviders>
  );
}

function getRunSelectionCheckbox(title: string) {
  return screen.getByLabelText(`选择导出 ${title}`);
}

describe('ApplicationLogsPage - run export downloads', () => {
  let createObjectUrlSpy: ReturnType<typeof vi.fn>;
  let revokeObjectUrlSpy: ReturnType<typeof vi.fn>;
  let anchorClickSpy: { mockRestore: () => void } | undefined;
  let dateNowSpy: { mockRestore: () => void } | undefined;

  beforeEach(async () => {
    window.localStorage.clear();
    window.localStorage.setItem('1flowbase.ui.locale_preference', 'zh_Hans');
    await appI18n.changeLanguage('zh_Hans');
    authenticate();
    dateNowSpy = vi
      .spyOn(Date, 'now')
      .mockReturnValue(new Date('2026-04-18T00:00:00Z').getTime());
    createObjectUrlSpy = vi.fn(() => 'blob:trace-dump');
    revokeObjectUrlSpy = vi.fn();
    Object.defineProperty(window.URL, 'createObjectURL', {
      configurable: true,
      value: createObjectUrlSpy
    });
    Object.defineProperty(window.URL, 'revokeObjectURL', {
      configurable: true,
      value: revokeObjectUrlSpy
    });
    anchorClickSpy = vi
      .spyOn(HTMLAnchorElement.prototype, 'click')
      .mockImplementation(() => undefined);
    runtimeApi.fetchApplicationRuns.mockReset();
    runtimeApi.fetchApplicationRunOverview.mockReset();
    runtimeApi.fetchApplicationRunTraceTree.mockReset();
    runtimeApi.fetchApplicationRunTraceNodeChildren.mockReset();
    runtimeApi.fetchApplicationRunTraceNodeContent.mockReset();
    runtimeApi.fetchApplicationRunTraceNodeDetail.mockReset();
    runtimeApi.fetchApplicationRunTraceToolCallbackContent.mockReset();
    runtimeApi.fetchApplicationRunResumeTimeline.mockReset();
    runtimeApi.fetchApplicationConversationMessages.mockReset();
    runtimeApi.fetchApplicationRunConversationMessages.mockReset();
    runtimeApi.fetchRuntimeDebugArtifact.mockReset();
    runtimeApi.fetchRuntimeDebugArtifacts.mockReset();
    runtimeApi.exportApplicationRunTraceDump.mockReset();
    runtimeApi.exportSelectedApplicationRunsTraceDumpZip.mockReset();
    runtimeApi.fetchApplicationRuns.mockResolvedValue(
      applicationRunsPage([
        runSummary('run-1', '退款总结'),
        runSummary('run-2', '天气查询')
      ])
    );
    runtimeApi.fetchApplicationRunOverview.mockResolvedValue({
      flow_run: {
        id: 'run-1',
        status: 'succeeded',
        input_payload: { query: '退款' },
        output_payload: { answer: '退款摘要' },
        started_at: '2026-04-17T09:00:00Z',
        finished_at: '2026-04-17T09:00:01Z'
      },
      run: {
        id: 'run-1',
        status: 'succeeded',
        compatibility_mode: 'openai-responses-v1',
        started_at: '2026-04-17T09:00:00Z',
        finished_at: '2026-04-17T09:00:01Z'
      },
      statistics: {
        total_tokens: 10,
        unique_node_count: 2,
        tool_callback_count: 0
      },
      answer_snapshot: null
    });
    runtimeApi.fetchApplicationRunConversationMessages.mockResolvedValue({
      items: [
        {
          run_id: 'run-1',
          detail_run_id: 'run-1',
          can_open_detail: true,
          role: 'assistant',
          content: '退款摘要',
          started_at: '2026-04-17T09:00:00Z',
          finished_at: '2026-04-17T09:00:01Z',
          status: 'succeeded',
          query: '退款',
          model: 'deepseek-chat',
          answer: '退款摘要',
          is_current: true
        }
      ],
      page: {
        has_before: false,
        has_after: false,
        before_cursor: null,
        after_cursor: null
      }
    });
    runtimeApi.exportApplicationRunTraceDump.mockResolvedValue({
      blob: new Blob(['{}'], { type: 'application/json' }),
      filename: 'run-1.json',
      contentType: 'application/json'
    });
    runtimeApi.exportSelectedApplicationRunsTraceDumpZip.mockResolvedValue({
      blob: new Blob(['zip'], { type: 'application/zip' }),
      filename: 'selected-runs.zip',
      contentType: 'application/zip'
    });
  });

  afterEach(() => {
    resetAuthStore();
    anchorClickSpy?.mockRestore();
    anchorClickSpy = undefined;
    dateNowSpy?.mockRestore();
    dateNowSpy = undefined;
  });

  test('exports only selected visible run ids from the current page', async () => {
    renderLogsPage();

    expect(await screen.findByText('退款总结')).toBeInTheDocument();
    const exportButton = screen.getByRole('button', {
      name: '导出已选日志'
    });

    expect(exportButton).toBeDisabled();

    fireEvent.click(getRunSelectionCheckbox('退款总结'));
    fireEvent.click(getRunSelectionCheckbox('天气查询'));

    await waitFor(() => expect(exportButton).toBeEnabled());
    fireEvent.click(exportButton);

    await waitFor(() => {
      expect(
        runtimeApi.exportSelectedApplicationRunsTraceDumpZip
      ).toHaveBeenCalledWith('app-1', ['run-1', 'run-2'], 'csrf-123');
    });
    expect(createObjectUrlSpy).toHaveBeenCalledWith(expect.any(Blob));
    expect(runtimeApi.fetchApplicationRunTraceTree).not.toHaveBeenCalled();
    expect(
      runtimeApi.fetchApplicationRunTraceNodeContent
    ).not.toHaveBeenCalled();
  });

  test('shows loading while selected zip export is pending', async () => {
    let resolveExport:
      | ((download: {
          blob: Blob;
          filename: string;
          contentType: string;
        }) => void)
      | undefined;
    runtimeApi.exportSelectedApplicationRunsTraceDumpZip.mockReturnValueOnce(
      new Promise<{
        blob: Blob;
        filename: string;
        contentType: string;
      }>((resolve) => {
        resolveExport = resolve;
      })
    );

    renderLogsPage();

    expect(await screen.findByText('退款总结')).toBeInTheDocument();
    fireEvent.click(getRunSelectionCheckbox('退款总结'));
    await waitFor(() =>
      expect(screen.getByRole('button', { name: '导出已选日志' })).toBeEnabled()
    );

    const exportButton = screen.getByRole('button', {
      name: '导出已选日志'
    });
    fireEvent.click(exportButton);

    await waitFor(() => {
      expect(exportButton).toHaveClass('ant-btn-loading');
    });

    resolveExport?.({
      blob: new Blob(['zip'], { type: 'application/zip' }),
      filename: 'selected-runs.zip',
      contentType: 'application/zip'
    });

    await waitFor(() => {
      expect(exportButton).not.toHaveClass('ant-btn-loading');
    });
  });

  test('clears selected runs after pagination, search, filters and durable refresh', async () => {
    runtimeApi.fetchApplicationRuns.mockResolvedValue(
      applicationRunsPage(
        [runSummary('run-1', '退款总结'), runSummary('run-2', '天气查询')],
        { total: 42 }
      )
    );

    renderLogsPage();

    expect(await screen.findByText('退款总结')).toBeInTheDocument();
    const exportButton = screen.getByRole('button', {
      name: '导出已选日志'
    });

    fireEvent.click(getRunSelectionCheckbox('退款总结'));
    await waitFor(() => expect(exportButton).toBeEnabled());
    fireEvent.click(screen.getByTitle('2'));
    await waitFor(() => expect(exportButton).toBeDisabled());

    expect(await screen.findByText('退款总结')).toBeInTheDocument();
    fireEvent.click(getRunSelectionCheckbox('退款总结'));
    await waitFor(() => expect(exportButton).toBeEnabled());
    fireEvent.change(screen.getByPlaceholderText('搜索标题'), {
      target: { value: '天气' }
    });
    await waitFor(() => expect(exportButton).toBeDisabled());

    expect(await screen.findByText('天气查询')).toBeInTheDocument();
    fireEvent.click(getRunSelectionCheckbox('天气查询'));
    await waitFor(() => expect(exportButton).toBeEnabled());
    fireEvent.mouseDown(screen.getByRole('combobox', { name: '时间间隔' }));
    fireEvent.click(
      await screen.findByText('所有时间', {
        selector: '.ant-select-item-option-content'
      })
    );
    await waitFor(() => expect(exportButton).toBeDisabled());

    expect(await screen.findByText('天气查询')).toBeInTheDocument();
    fireEvent.click(getRunSelectionCheckbox('天气查询'));
    await waitFor(() => expect(exportButton).toBeEnabled());
    fireEvent.click(screen.getByRole('button', { name: '刷新日志' }));
    await waitFor(() => expect(exportButton).toBeDisabled());
  });

  test('reports export failures without saving an empty file', async () => {
    runtimeApi.exportSelectedApplicationRunsTraceDumpZip.mockRejectedValueOnce(
      new Error('export failed')
    );

    renderLogsPage();

    expect(await screen.findByText('退款总结')).toBeInTheDocument();
    fireEvent.click(getRunSelectionCheckbox('退款总结'));
    await waitFor(() =>
      expect(screen.getByRole('button', { name: '导出已选日志' })).toBeEnabled()
    );
    fireEvent.click(screen.getByRole('button', { name: '导出已选日志' }));

    expect(await screen.findByText('导出日志失败')).toBeInTheDocument();
    expect(createObjectUrlSpy).not.toHaveBeenCalled();
  });

  test('requires csrf token before selected zip export', async () => {
    resetAuthStore();

    renderLogsPage();

    expect(await screen.findByText('退款总结')).toBeInTheDocument();
    fireEvent.click(getRunSelectionCheckbox('退款总结'));
    await waitFor(() =>
      expect(screen.getByRole('button', { name: '导出已选日志' })).toBeEnabled()
    );
    fireEvent.click(screen.getByRole('button', { name: '导出已选日志' }));

    expect(
      await screen.findByText('缺少 CSRF token，无法导出日志')
    ).toBeInTheDocument();
    expect(
      runtimeApi.exportSelectedApplicationRunsTraceDumpZip
    ).not.toHaveBeenCalled();
    expect(createObjectUrlSpy).not.toHaveBeenCalled();
  });

  test('exports a single run from the conversation log floating window without composing trace content', async () => {
    renderLogsPage();

    expect(await screen.findByText('退款总结')).toBeInTheDocument();
    fireEvent.click(screen.getAllByRole('button', { name: '查看运行详情' })[0]);
    expect(
      await screen.findByRole('complementary', { name: '运行详情' })
    ).toBeInTheDocument();
    fireEvent.click(
      (await screen.findAllByRole('button', { name: '查看对话日志' }))[0]
    );
    expect(
      await screen.findByRole('complementary', { name: '对话日志' })
    ).toBeInTheDocument();
    fireEvent.click(
      await screen.findByRole('button', { name: '导出当前运行 JSON' })
    );

    await waitFor(() => {
      expect(runtimeApi.exportApplicationRunTraceDump).toHaveBeenCalledWith(
        'app-1',
        'run-1'
      );
    });
    expect(createObjectUrlSpy).toHaveBeenCalledWith(expect.any(Blob));
    expect(runtimeApi.fetchApplicationRunTraceTree).not.toHaveBeenCalled();
    expect(
      runtimeApi.fetchApplicationRunTraceNodeContent
    ).not.toHaveBeenCalled();
  });
});
