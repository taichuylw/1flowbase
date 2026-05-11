import { afterEach, describe, expect, test, vi } from 'vitest';

vi.mock('@1flowbase/api-client', () => ({
  fetchConsoleAgentFlowDataModelOptions: vi.fn().mockResolvedValue([]),
  getDefaultApiBaseUrl: vi.fn().mockReturnValue('http://127.0.0.1:7800')
}));

import { fetchConsoleAgentFlowDataModelOptions } from '@1flowbase/api-client';

import {
  dataModelOptionsQueryKey,
  fetchDataModelOptions
} from '../../api/data-model-options';

afterEach(() => {
  vi.clearAllMocks();
});

describe('agent flow data model options api', () => {
  test('uses a stable query key', () => {
    expect(dataModelOptionsQueryKey).toEqual([
      'agent-flow',
      'data-model-options'
    ]);
  });

  test('delegates AgentFlow option shaping to the backend scene read model', async () => {
    vi.mocked(fetchConsoleAgentFlowDataModelOptions).mockResolvedValue([
      {
        value: 'order',
        label: 'Order',
        state: 'enabled',
        disabled: false,
        disabledReason: null,
        modelId: 'model-1',
        modelCode: 'order',
        fields: [
          {
            code: 'title',
            title: 'Title',
            valueType: 'text',
            required: true,
            writable: true
          }
        ]
      }
    ]);

    await expect(fetchDataModelOptions()).resolves.toEqual([
      {
        value: 'order',
        label: 'Order',
        state: 'enabled',
        disabled: false,
        disabledReason: null,
        modelId: 'model-1',
        modelCode: 'order',
        fields: [
          {
            code: 'title',
            title: 'Title',
            valueType: 'text',
            required: true,
            writable: true
          }
        ]
      }
    ]);

    expect(fetchConsoleAgentFlowDataModelOptions).toHaveBeenCalledWith(
      'http://127.0.0.1:7800'
    );
  });
});
