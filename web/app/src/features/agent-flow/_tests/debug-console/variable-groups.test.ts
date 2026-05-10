import { describe, expect, test } from 'vitest';
import { createDefaultAgentFlowDocument } from '@1flowbase/flow-schema';

import {
  buildRunContextFromDocument,
  mapRunDetailToVariableGroups,
  mapVariableCacheToVariableGroup
} from '../../lib/debug-console/variable-groups';
import type { FlowDebugRunDetail } from '../../api/runtime';

const createRunDetail = (): FlowDebugRunDetail => ({
  flow_run: {
    id: 'flow-run-1',
    application_id: 'app-1',
    flow_id: 'flow-1',
    draft_id: 'draft-1',
    compiled_plan_id: 'plan-1',
    run_mode: 'debug_flow_run' as const,
    status: 'succeeded',
    target_node_id: null,
    input_payload: {
      'node-start': {
        query: '请总结退款政策'
      },
      'node-llm': {
        user_prompt: '请总结退款政策'
      },
      'node-answer': {
        answer_template: '退款政策摘要'
      }
    },
    output_payload: {
      answer: '退款政策摘要'
    },
    error_payload: null,
    created_by: 'user-1',
    started_at: '2026-04-25T10:00:00Z',
    finished_at: '2026-04-25T10:00:02Z',
    created_at: '2026-04-25T10:00:00Z'
  },
  node_runs: [
    {
      id: 'node-run-start',
      flow_run_id: 'flow-run-1',
      node_id: 'node-start',
      node_type: 'start',
      node_alias: 'Start',
      status: 'succeeded',
      input_payload: {},
      output_payload: { query: '请总结退款政策' },
      error_payload: null,
      metrics_payload: {},
      started_at: '2026-04-25T10:00:00Z',
      finished_at: '2026-04-25T10:00:00Z'
    },
    {
      id: 'node-run-llm',
      flow_run_id: 'flow-run-1',
      node_id: 'node-llm',
      node_type: 'llm',
      node_alias: 'LLM',
      status: 'succeeded',
      input_payload: { user_prompt: '请总结退款政策' },
      output_payload: { text: '退款政策摘要' },
      error_payload: null,
      metrics_payload: {},
      started_at: '2026-04-25T10:00:00Z',
      finished_at: '2026-04-25T10:00:01Z'
    },
    {
      id: 'node-run-answer',
      flow_run_id: 'flow-run-1',
      node_id: 'node-answer',
      node_type: 'answer',
      node_alias: 'Answer',
      status: 'succeeded',
      input_payload: { answer_template: '退款政策摘要' },
      output_payload: { answer: '退款政策摘要' },
      error_payload: null,
      metrics_payload: {},
      started_at: '2026-04-25T10:00:01Z',
      finished_at: '2026-04-25T10:00:02Z'
    }
  ],
  checkpoints: [],
  callback_tasks: [],
  events: []
});

describe('debug console variable groups', () => {
  test('maps variable cache entries at public output key level', () => {
    const group = mapVariableCacheToVariableGroup(
      {
        'node-llm': {
          text: '你好?',
          structured_output: { intent: 'refund' }
        }
      },
      {
        'node-llm': {
          label: 'LLM',
          nodeType: 'llm',
          outputs: [
            { key: 'text', title: '模型输出', valueType: 'string' },
            {
              key: 'structured_output',
              title: '结构化输出',
              valueType: 'json'
            }
          ]
        }
      }
    );

    expect(group).toEqual({
      title: 'Variable Cache',
      items: [
        {
          key: 'node-llm.text',
          label: 'LLM/text',
          helperText: '模型输出',
          value: '你好?'
        },
        {
          key: 'node-llm.structured_output',
          label: 'LLM/structured_output',
          helperText: '结构化输出',
          value: { intent: 'refund' }
        }
      ]
    });
  });

  test('maps run detail variables with run context inputs and node outputs', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const runContext = buildRunContextFromDocument(document);

    const variableGroups = mapRunDetailToVariableGroups(createRunDetail(), {
      applicationId: 'app-1',
      draftId: 'draft-1',
      runContext
    });

    const inputGroup = variableGroups.find((group) => group.title === 'Input Variables');
    const inputKeys = (inputGroup?.items ?? []).map((item) => item.key);

    expect(inputKeys).toEqual(
      expect.arrayContaining(['node-start.query', 'node-start.files'])
    );
    expect(inputKeys).not.toContain('node-llm.user_prompt');
    expect(inputKeys).not.toContain('node-answer.answer_template');
    expect(
      inputGroup?.items.find((item) => item.key === 'node-start.query')?.value
    ).toBe('请总结退款政策');

    const outputGroup = variableGroups.find(
      (group) => group.title === 'Node Outputs'
    );

    expect(outputGroup?.items).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          label: 'LLM/text',
          helperText: '模型输出',
          value: '退款政策摘要'
        }),
        expect.objectContaining({
          label: 'Answer/answer',
          helperText: '对话输出',
          value: '退款政策摘要'
        })
      ])
    );
  });

  test('maps run detail node outputs at public output key level for object values', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const runContext = buildRunContextFromDocument(document);
    const runDetail = createRunDetail();
    runDetail.node_runs = [
      {
        id: 'node-run-llm',
        flow_run_id: 'flow-run-1',
        node_id: 'node-llm',
        node_type: 'llm',
        node_alias: 'LLM',
        status: 'succeeded',
        input_payload: {},
        output_payload: {
          record: { id: '1', title: 'A' },
          records: [{ id: '1' }],
          empty_record: {},
          total: 1
        },
        error_payload: null,
        metrics_payload: {},
        started_at: '2026-04-25T10:00:00Z',
        finished_at: '2026-04-25T10:00:01Z'
      }
    ];

    const variableGroups = mapRunDetailToVariableGroups(runDetail, {
      applicationId: 'app-1',
      draftId: 'draft-1',
      runContext
    });

    const outputGroup = variableGroups.find(
      (group) => group.title === 'Node Outputs'
    );
    const outputLabels = (outputGroup?.items ?? []).map((item) => item.label);
    const outputKeys = (outputGroup?.items ?? []).map((item) => item.key);

    expect(outputLabels).toEqual([
      'LLM/record',
      'LLM/records',
      'LLM/empty_record',
      'LLM/total'
    ]);
    expect(outputKeys).toEqual([
      'node-llm.record',
      'node-llm.records',
      'node-llm.empty_record',
      'node-llm.total'
    ]);
    expect(outputLabels).not.toContain('LLM/record.id');
    expect(outputLabels).not.toContain('LLM/record.title');
    expect(outputLabels).not.toContain('LLM/records[0].id');
    expect(
      outputGroup?.items.find((item) => item.label === 'LLM/record')?.value
    ).toEqual({ id: '1', title: 'A' });
    expect(
      outputGroup?.items.find((item) => item.label === 'LLM/records')?.value
    ).toEqual([{ id: '1' }]);
    expect(
      outputGroup?.items.find((item) => item.label === 'LLM/empty_record')?.value
    ).toEqual({});
  });

  test('keeps runtime debug artifact previews as non-expanded values', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const runContext = buildRunContextFromDocument(document);
    const runDetail = createRunDetail();
    runDetail.node_runs = [
      {
        id: 'node-run-llm',
        flow_run_id: 'flow-run-1',
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
        started_at: '2026-04-25T10:00:00Z',
        finished_at: '2026-04-25T10:00:01Z'
      }
    ];

    const variableGroups = mapRunDetailToVariableGroups(runDetail, {
      applicationId: 'app-1',
      draftId: 'draft-1',
      runContext
    });
    const outputGroup = variableGroups.find(
      (group) => group.title === 'Node Outputs'
    );

    expect(outputGroup?.items).toEqual([
      expect.objectContaining({
        key: 'node-llm.text',
        label: 'LLM/text',
        helperText: '模型输出',
        isTruncated: true,
        artifactRef: 'artifact-1'
      })
    ]);
    expect(outputGroup?.items.map((item) => item.key)).not.toContain(
      'node-llm.text.preview'
    );
  });
});
