import type {
  FlowAuthoringDocument,
  FlowNodeOutputDocument,
  FlowNodeType
} from '@1flowbase/flow-schema';

import {
  buildFlowDebugRunInput,
  type AgentFlowRunContext,
  type AgentFlowVariableGroup,
  type AgentFlowVariableItem,
  type FlowDebugRunDetail,
  type NodeDebugPreviewVariableCache
} from '../../api/runtime';
import { getNodeVariableOutputs } from '../start-node-variables';
import { formatNodeVariablePathLabel } from '../variable-labels';
import { getBuiltinNodeRuntimeContract } from '../node-definitions/contracts';

export interface NodeVariableDisplayMeta {
  label: string;
  nodeType?: string;
  outputs?: FlowNodeOutputDocument[];
}

function normalizeNodeVariableDisplayMeta(
  value: string | NodeVariableDisplayMeta | undefined,
  fallbackNodeId: string
): NodeVariableDisplayMeta {
  if (typeof value === 'string') {
    return { label: value };
  }

  return value ?? { label: fallbackNodeId };
}

function getOutputHelperText(
  nodeType: string | undefined,
  key: string,
  outputs?: FlowNodeOutputDocument[]
) {
  const documentOutput = outputs?.find((candidate) => candidate.key === key);

  if (documentOutput?.title && documentOutput.title !== key) {
    return documentOutput.title;
  }

  if (!nodeType) {
    return undefined;
  }

  const contract = getBuiltinNodeRuntimeContract(nodeType as FlowNodeType);
  const output =
    contract?.runtime.outputs.find((candidate) => candidate.key === key) ??
    contract?.defaults.outputs.find((candidate) => candidate.key === key);

  if (!output?.title || output.title === key) {
    return undefined;
  }

  return output.title;
}

function mapNodeOutputVariables(
  nodeId: string,
  nodeLabel: string,
  nodeType: string | undefined,
  value: unknown,
  outputs?: FlowNodeOutputDocument[]
): AgentFlowVariableItem[] {
  if (isRuntimeDebugArtifactPreview(value)) {
    return [
      toRuntimeArtifactVariableItem(
        nodeId,
        nodeLabel,
        value,
        getOutputHelperText(nodeType, nodeId, outputs)
      )
    ];
  }

  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return [{ key: nodeId, label: nodeLabel, value }];
  }

  const valueRecord = value as Record<string, unknown>;
  const entries = outputs?.length
    ? outputs.flatMap((output) => {
        const selectorValue = readOutputSelector(valueRecord, output);
        return selectorValue.found ? [[output.key, selectorValue.value] as const] : [];
      })
    : Object.entries(valueRecord);

  return entries.map(([key, entryValue]) => ({
    key: `${nodeId}.${key}`,
    label: formatNodeVariablePathLabel(nodeLabel, key),
    helperText: getOutputHelperText(nodeType, key, outputs),
    value: entryValue,
    ...(isRuntimeDebugArtifactPreview(entryValue)
      ? {
          isTruncated: true,
          artifactRef: entryValue.artifact_ref
        }
      : {})
  }));
}

function readOutputSelector(
  value: Record<string, unknown>,
  output: FlowNodeOutputDocument
): { found: true; value: unknown } | { found: false } {
  const selector = output.selector?.length ? output.selector : [output.key];
  let current: unknown = value;

  for (const segment of selector) {
    if (!current || typeof current !== 'object' || Array.isArray(current)) {
      return { found: false };
    }

    const record = current as Record<string, unknown>;

    if (!Object.prototype.hasOwnProperty.call(record, segment)) {
      return { found: false };
    }

    current = record[segment];
  }

  return { found: true, value: current };
}

function isRuntimeDebugArtifactPreview(
  value: unknown
): value is {
  __runtime_debug_artifact: true;
  is_truncated: boolean;
  artifact_ref: string;
  preview: string;
} {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return false;
  }

  const record = value as Record<string, unknown>;
  return (
    record.__runtime_debug_artifact === true &&
    typeof record.artifact_ref === 'string'
  );
}

function toRuntimeArtifactVariableItem(
  key: string,
  label: string,
  value: {
    artifact_ref: string;
  },
  helperText?: string
): AgentFlowVariableItem {
  return {
    key,
    label,
    value,
    helperText,
    isTruncated: true,
    artifactRef: value.artifact_ref
  };
}

function readRunDetailInputValue(
  detail: FlowDebugRunDetail,
  nodeId: string,
  key: string
) {
  const inputPayload = detail.flow_run.input_payload;

  if (
    !inputPayload ||
    typeof inputPayload !== 'object' ||
    Array.isArray(inputPayload)
  ) {
    return undefined;
  }

  const nodePayload = (inputPayload as Record<string, unknown>)[nodeId];

  if (
    !nodePayload ||
    typeof nodePayload !== 'object' ||
    Array.isArray(nodePayload)
  ) {
    return undefined;
  }

  const nodeInputPayload = nodePayload as Record<string, unknown>;

  return Object.prototype.hasOwnProperty.call(nodeInputPayload, key)
    ? nodeInputPayload[key]
    : undefined;
}

export function getRunContextValues(
  runContext: AgentFlowRunContext
): Record<string, unknown> {
  return runContext.fields.reduce<Record<string, unknown>>((result, field) => {
    result[field.key] = field.value;
    return result;
  }, {});
}

export function buildRunContextFromDocument(
  document: FlowAuthoringDocument,
  rememberedInputs?: Record<string, unknown> | null
): AgentFlowRunContext {
  const startNode = document.graph.nodes.find((node) => node.type === 'start');
  const startPayload =
    buildFlowDebugRunInput(document, rememberedInputs ?? undefined).input_payload[
      startNode?.id ?? 'node-start'
    ] ?? {};

  return {
    environmentLabel: 'draft',
    remembered: Boolean(rememberedInputs && Object.keys(rememberedInputs).length > 0),
    fields: (startNode ? getNodeVariableOutputs(startNode) : []).map((output) => ({
      nodeId: startNode?.id ?? 'node-start',
      nodeLabel: startNode?.alias ?? startNode?.id ?? 'node-start',
      key: output.key,
      title: output.title,
      valueType: output.valueType,
      value: startPayload[output.key]
    }))
  };
}

export function mapRunContextToVariableGroups(
  runContext: AgentFlowRunContext,
  options: {
    applicationId: string;
    draftId: string;
  }
): AgentFlowVariableGroup[] {
  return [
    {
      title: 'Input Variables',
      items: runContext.fields.map((field) => ({
        key: `${field.nodeId}.${field.key}`,
        label: formatNodeVariablePathLabel(field.nodeLabel, field.key),
        value: field.value
      }))
    },
    {
      title: 'Conversation / Session',
      items: [
        {
          key: 'session.remembered',
          label: 'session.remembered',
          value: runContext.remembered,
          isReadOnly: true
        }
      ]
    },
    {
      title: 'Environment',
      items: [
        {
          key: 'environment.label',
          label: 'environment.label',
          value: runContext.environmentLabel,
          isReadOnly: true
        },
        {
          key: 'environment.application_id',
          label: 'environment.application_id',
          value: options.applicationId,
          isReadOnly: true
        },
        {
          key: 'environment.draft_id',
          label: 'environment.draft_id',
          value: options.draftId,
          isReadOnly: true
        }
      ]
    }
  ];
}

export function mapVariableCacheToVariableGroup(
  variableCache: NodeDebugPreviewVariableCache,
  nodeMetadata: Record<string, string | NodeVariableDisplayMeta> = {}
): AgentFlowVariableGroup | null {
  const items = Object.entries(variableCache).flatMap(([nodeId, value]) => {
    const metadata = normalizeNodeVariableDisplayMeta(
      nodeMetadata[nodeId],
      nodeId
    );

    return mapNodeOutputVariables(
      nodeId,
      metadata.label,
      metadata.nodeType,
      value,
      metadata.outputs
    );
  });

  if (items.length === 0) {
    return null;
  }

  return {
    title: 'Variable Cache',
    items
  };
}

export function mapRunDetailToVariableGroups(
  detail: FlowDebugRunDetail,
  options: {
    applicationId: string;
    draftId: string;
    runContext: AgentFlowRunContext;
    nodeMetadata?: Record<string, string | NodeVariableDisplayMeta>;
  }
): AgentFlowVariableGroup[] {
  const inputItems = options.runContext.fields.map((field) => {
    const detailValue = readRunDetailInputValue(detail, field.nodeId, field.key);

    return {
      key: `${field.nodeId}.${field.key}`,
      label: formatNodeVariablePathLabel(field.nodeLabel, field.key),
      value: detailValue === undefined ? field.value : detailValue
    };
  });
  const nodeOutputItems = detail.node_runs.flatMap((nodeRun) =>
    {
      const metadata = normalizeNodeVariableDisplayMeta(
        options.nodeMetadata?.[nodeRun.node_id],
        nodeRun.node_id
      );

      return mapNodeOutputVariables(
        nodeRun.node_id,
        metadata.label,
        metadata.nodeType ?? nodeRun.node_type,
        nodeRun.output_payload,
        metadata.outputs
      );
    }
  );
  const sessionItems: AgentFlowVariableItem[] = [
    {
      key: 'flow_run.id',
      label: 'flow_run.id',
      value: detail.flow_run.id
    },
    {
      key: 'flow_run.status',
      label: 'flow_run.status',
      value: detail.flow_run.status
    },
    {
      key: 'flow_run.started_at',
      label: 'flow_run.started_at',
      value: detail.flow_run.started_at
    },
    {
      key: 'flow_run.finished_at',
      label: 'flow_run.finished_at',
      value: detail.flow_run.finished_at
    }
  ];

  return [
    {
      title: 'Input Variables',
      items: inputItems
    },
    {
      title: 'Node Outputs',
      items: nodeOutputItems
    },
    {
      title: 'Conversation / Session',
      items: [
        ...sessionItems.map((item) => ({
          ...item,
          isReadOnly: true
        })),
        {
          key: 'session.remembered',
          label: 'session.remembered',
          value: options.runContext.remembered,
          isReadOnly: true
        }
      ]
    },
    {
      title: 'Environment',
      items: [
        {
          key: 'environment.label',
          label: 'environment.label',
          value: options.runContext.environmentLabel,
          isReadOnly: true
        },
        {
          key: 'environment.application_id',
          label: 'environment.application_id',
          value: options.applicationId,
          isReadOnly: true
        },
        {
          key: 'environment.draft_id',
          label: 'environment.draft_id',
          value: options.draftId,
          isReadOnly: true
        },
        {
          key: 'environment.run_mode',
          label: 'environment.run_mode',
          value: detail.flow_run.run_mode,
          isReadOnly: true
        }
      ]
    }
  ];
}
