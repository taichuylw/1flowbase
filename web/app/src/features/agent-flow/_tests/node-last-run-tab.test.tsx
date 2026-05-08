import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { AppProviders } from '../../../app/AppProviders';
import * as runtimeApi from '../api/runtime';
import { NodeLastRunTab } from '../components/detail/tabs/NodeLastRunTab';

describe('NodeLastRunTab', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  test('renders empty state when the selected node has not run yet', async () => {
    vi.spyOn(runtimeApi, 'fetchNodeLastRun').mockResolvedValue(null);

    render(
      <AppProviders>
        <NodeLastRunTab applicationId="app-1" nodeId="node-llm" />
      </AppProviders>
    );

    expect(await screen.findByText('当前节点还没有运行记录')).toBeInTheDocument();
  });

  test('renders runtime-backed summary, io and metadata cards', async () => {
    vi.spyOn(runtimeApi, 'fetchNodeLastRun').mockResolvedValue({
      flow_run: {
        id: 'run-1',
        application_id: 'app-1',
        flow_id: 'flow-1',
        draft_id: 'draft-1',
        compiled_plan_id: 'plan-1',
        run_mode: 'debug_node_preview',
        status: 'succeeded',
        target_node_id: 'node-llm',
        input_payload: {
          'node-start.query': '总结退款政策'
        },
        output_payload: {
          resolved_inputs: {
            user_prompt: '总结退款政策'
          }
        },
        error_payload: null,
        created_by: 'user-1',
        started_at: '2026-04-17T09:00:00Z',
        finished_at: '2026-04-17T09:00:01Z',
        created_at: '2026-04-17T09:00:00Z'
      },
      node_run: {
        id: 'node-run-1',
        flow_run_id: 'run-1',
        node_id: 'node-llm',
        node_type: 'llm',
        node_alias: 'LLM',
        status: 'succeeded',
        input_payload: {
          user_prompt: '总结退款政策'
        },
        output_payload: {
          rendered_templates: {}
        },
        error_payload: null,
        metrics_payload: {
          output_contract_count: 1,
          total_tokens: 72,
          provider_instance_id: 'provider-openai-prod',
          provider_code: 'openai_compatible',
          protocol: 'openai_responses',
          finish_reason: 'stop',
          route: 'primary',
          attempt: 2
        },
        debug_payload: {
          response_ref: 'runtime_artifact:inline:response-1',
          artifact_metadata: {
            content_type: 'application/json',
            original_size_bytes: 2048
          }
        },
        started_at: '2026-04-17T09:00:00Z',
        finished_at: '2026-04-17T09:00:01Z'
      },
      checkpoints: [],
      events: []
    });

    render(
      <AppProviders>
        <NodeLastRunTab applicationId="app-1" nodeId="node-llm" />
      </AppProviders>
    );

    expect(await screen.findByText('运行摘要')).toBeInTheDocument();
    expect(screen.queryByText('运行模式')).not.toBeInTheDocument();
    expect(screen.queryByText('目标节点')).not.toBeInTheDocument();
    expect(screen.getByText('token')).toBeInTheDocument();
    expect(screen.getByText('耗时(ms)')).toBeInTheDocument();
    expect(screen.getByText('72')).toBeInTheDocument();
    expect(screen.getByLabelText('输入 JSON')).toHaveTextContent('user_prompt');
    expect(screen.getByLabelText('输入 JSON')).toHaveTextContent('总结退款政策');
    const outputJson = screen.getByLabelText('输出 JSON');
    expect(outputJson).not.toHaveTextContent('event_details');
    expect(outputJson).not.toHaveTextContent('run_metadata');
    expect(outputJson).not.toHaveTextContent('provider-openai-prod');
    const metricsJson = screen.getByLabelText('指标 JSON');
    expect(metricsJson).toHaveTextContent('provider-openai-prod');
    expect(metricsJson).toHaveTextContent('openai_compatible');
    expect(metricsJson).toHaveTextContent('primary');
    expect(metricsJson).toHaveTextContent('attempt');
    expect(metricsJson).toHaveTextContent('2');
    expect(metricsJson).toHaveTextContent('finish_reason');
    expect(metricsJson).toHaveTextContent('stop');
    expect(metricsJson).toHaveTextContent('duration_ms');
    const debugJson = screen.getByLabelText('Debug JSON');
    expect(debugJson).toHaveTextContent('response_ref');
    expect(debugJson).toHaveTextContent('artifact_metadata');
    expect(debugJson).toHaveTextContent('2048');
    expect(screen.queryByText('执行人')).not.toBeInTheDocument();
    expect(screen.queryByText('Compiled Plan')).not.toBeInTheDocument();
    expect(screen.queryByText('输出契约数')).not.toBeInTheDocument();

    const inputToggle = screen.getByRole('button', { name: '输入' });
    expect(inputToggle).toHaveAttribute('aria-expanded', 'true');

    fireEvent.click(inputToggle);

    expect(inputToggle).toHaveAttribute('aria-expanded', 'false');
    expect(screen.getByRole('button', { name: '放大查看输入 JSON' })).toBeDisabled();
  });

  test('loads truncated last-run payload artifact on explicit action', async () => {
    vi.spyOn(runtimeApi, 'fetchRuntimeDebugArtifact').mockResolvedValue({
      text: '完整 Last Run 内容'
    });
    vi.spyOn(runtimeApi, 'fetchNodeLastRun').mockResolvedValue({
      flow_run: {
        id: 'run-1',
        application_id: 'app-1',
        flow_id: 'flow-1',
        draft_id: 'draft-1',
        compiled_plan_id: 'plan-1',
        run_mode: 'debug_node_preview',
        status: 'succeeded',
        target_node_id: 'node-llm',
        input_payload: {},
        output_payload: {},
        error_payload: null,
        created_by: 'user-1',
        started_at: '2026-04-17T09:00:00Z',
        finished_at: '2026-04-17T09:00:01Z',
        created_at: '2026-04-17T09:00:00Z'
      },
      node_run: {
        id: 'node-run-1',
        flow_run_id: 'run-1',
        node_id: 'node-llm',
        node_type: 'llm',
        node_alias: 'LLM',
        status: 'succeeded',
        input_payload: {},
        output_payload: {
          text: {
            __runtime_debug_artifact: true,
            is_truncated: true,
            original_size_bytes: 4096,
            preview_size_bytes: 128,
            content_type: 'application/json',
            artifact_ref: 'artifact-1',
            preview: '{"text":"preview'
          }
        },
        error_payload: null,
        metrics_payload: {},
        debug_payload: {},
        started_at: '2026-04-17T09:00:00Z',
        finished_at: '2026-04-17T09:00:01Z'
      },
      checkpoints: [],
      events: []
    });

    render(
      <AppProviders>
        <NodeLastRunTab applicationId="app-1" nodeId="node-llm" />
      </AppProviders>
    );

    fireEvent.click(await screen.findByRole('button', { name: '加载完整值' }));

    expect(runtimeApi.fetchRuntimeDebugArtifact).toHaveBeenCalledWith(
      'app-1',
      'artifact-1'
    );
    expect(await screen.findByLabelText('输出 JSON')).toHaveTextContent(
      '完整 Last Run 内容'
    );
  });

  test('renders warning state when runtime payload is malformed', async () => {
    vi
      .spyOn(runtimeApi, 'fetchNodeLastRun')
      .mockResolvedValue({ node_run: null } as never);

    render(
      <AppProviders>
        <NodeLastRunTab applicationId="app-1" nodeId="node-llm" />
      </AppProviders>
    );

    expect(await screen.findByText('上次运行数据异常')).toBeInTheDocument();
  });
});
