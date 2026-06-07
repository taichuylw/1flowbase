import type {
  ConsolePluginFormFieldSchema,
  ConsolePluginFormSchema
} from '@1flowbase/api-client';

export interface LlmNodeModelProvider {
  provider_code: string;
  model_id: string;
  protocol?: string;
  provider_label?: string;
  model_label?: string;
  schema_fetched_at?: string;
}

export interface LlmParameterItem {
  enabled: boolean;
  value: unknown;
}

export interface LlmNodeParameters {
  schema_version: string;
  items: Record<string, LlmParameterItem>;
}

export interface LlmNodeResponseFormat {
  mode: 'text' | 'json_object' | 'json_schema';
  schema?: Record<string, unknown>;
}

export interface LlmNodeContextPolicy {
  integration_context: 'enabled' | 'disabled';
  context_selector?: string[];
}

export interface LlmNodeExternalReasoningPolicy {
  follow_external_reasoning: boolean;
}

export type LlmNodeExecutionRole = 'standard' | 'visible_internal_llm_tool';

export interface LlmVisibleInternalTool {
  type: 'visible_internal_llm_tool';
  tool_name: string;
  target_node_id: string;
  description?: string;
  input_schema?: Record<string, unknown>;
}

export const DEFAULT_LLM_CONTEXT_POLICY: LlmNodeContextPolicy = {
  integration_context: 'enabled',
  context_selector: ['node-start', 'history']
};

export const DEFAULT_LLM_EXTERNAL_REASONING_POLICY: LlmNodeExternalReasoningPolicy =
  {
    follow_external_reasoning: false
  };

export const DEFAULT_LLM_PARAMETERS: LlmNodeParameters = {
  schema_version: '1.0.0',
  items: {}
};

export const DEFAULT_LLM_RESPONSE_FORMAT: LlmNodeResponseFormat = {
  mode: 'text'
};

export const DEFAULT_LLM_EXECUTION_ROLE: LlmNodeExecutionRole = 'standard';
export const DEFAULT_LLM_VISIBLE_INTERNAL_TOOLS: LlmVisibleInternalTool[] = [];

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function asString(value: unknown) {
  return typeof value === 'string' ? value : '';
}

export function getLlmParameterDefaultValue(
  field: ConsolePluginFormFieldSchema
): unknown {
  if (field.default_value !== undefined) {
    return field.default_value;
  }

  switch (field.type) {
    case 'boolean':
      return false;
    case 'integer':
    case 'number':
      return 0;
    case 'string_list':
      return [];
    case 'json':
      return {};
    default:
      return '';
  }
}

export function buildLlmParameterState(
  schema?: ConsolePluginFormSchema | null
): LlmNodeParameters {
  if (!schema) {
    return DEFAULT_LLM_PARAMETERS;
  }

  return {
    schema_version: schema.schema_version,
    items: Object.fromEntries(
      schema.fields.map((field) => {
        const enabled =
          field.send_mode === 'always'
            ? true
            : Boolean(field.enabled_by_default);

        return [
          field.key,
          {
            enabled,
            value: getLlmParameterDefaultValue(field)
          } satisfies LlmParameterItem
        ];
      })
    )
  };
}

export function resolveLlmParameterStateOnModelChange({
  currentProviderCode,
  nextProviderCode,
  currentParameters,
  nextSchema
}: {
  currentProviderCode: string;
  nextProviderCode: string;
  currentParameters: LlmNodeParameters;
  nextSchema?: ConsolePluginFormSchema | null;
}) {
  if (
    currentProviderCode.trim().length > 0 &&
    currentProviderCode.trim() === nextProviderCode.trim()
  ) {
    return currentParameters;
  }

  return buildLlmParameterState(nextSchema);
}

export function getLlmModelProvider(
  config: Record<string, unknown>
): LlmNodeModelProvider {
  const provider = config.model_provider;

  if (!isRecord(provider)) {
    return {
      provider_code: '',
      model_id: '',
      protocol: undefined,
      provider_label: undefined,
      model_label: undefined,
      schema_fetched_at: undefined
    };
  }

  return {
    provider_code: asString(provider.provider_code),
    model_id: asString(provider.model_id),
    protocol: asString(provider.protocol) || undefined,
    provider_label: asString(provider.provider_label) || undefined,
    model_label: asString(provider.model_label) || undefined,
    schema_fetched_at: asString(provider.schema_fetched_at) || undefined
  };
}

export function getLlmParameters(
  config: Record<string, unknown>
): LlmNodeParameters {
  const llmParameters = config.llm_parameters;

  if (isRecord(llmParameters)) {
    const items = isRecord(llmParameters.items) ? llmParameters.items : {};

    return {
      schema_version: asString(llmParameters.schema_version) || '1.0.0',
      items: Object.fromEntries(
        Object.entries(items).map(([key, item]) => {
          if (!isRecord(item)) {
            return [
              key,
              { enabled: false, value: item } satisfies LlmParameterItem
            ];
          }

          return [
            key,
            {
              enabled: Boolean(item.enabled),
              value: item.value
            } satisfies LlmParameterItem
          ];
        })
      )
    };
  }

  return DEFAULT_LLM_PARAMETERS;
}

export function getLlmContextPolicy(
  config: Record<string, unknown>
): LlmNodeContextPolicy {
  const contextPolicy = config.context_policy;

  if (!isRecord(contextPolicy)) {
    return DEFAULT_LLM_CONTEXT_POLICY;
  }

  return {
    integration_context:
      contextPolicy.integration_context === 'disabled' ? 'disabled' : 'enabled',
    context_selector: Array.isArray(contextPolicy.context_selector)
      ? contextPolicy.context_selector.filter(
          (segment): segment is string => typeof segment === 'string'
        )
      : DEFAULT_LLM_CONTEXT_POLICY.context_selector
  };
}

export function getLlmExternalReasoningPolicy(
  config: Record<string, unknown>
): LlmNodeExternalReasoningPolicy {
  const externalReasoningPolicy = config.external_reasoning_policy;

  if (!isRecord(externalReasoningPolicy)) {
    return DEFAULT_LLM_EXTERNAL_REASONING_POLICY;
  }

  return {
    follow_external_reasoning:
      externalReasoningPolicy.follow_external_reasoning === true
  };
}

export function getLlmExecutionRole(
  config: Record<string, unknown>
): LlmNodeExecutionRole {
  return config.execution_role === 'visible_internal_llm_tool'
    ? 'visible_internal_llm_tool'
    : DEFAULT_LLM_EXECUTION_ROLE;
}

export function getLlmVisibleInternalTools(
  config: Record<string, unknown>
): LlmVisibleInternalTool[] {
  const tools = config.visible_internal_llm_tools;

  if (!Array.isArray(tools)) {
    return DEFAULT_LLM_VISIBLE_INTERNAL_TOOLS;
  }

  return tools.flatMap((tool): LlmVisibleInternalTool[] => {
    if (!isRecord(tool) || tool.type !== 'visible_internal_llm_tool') {
      return [];
    }

    const toolName = asString(tool.tool_name).trim();
    const targetNodeId = asString(tool.target_node_id).trim();

    if (!toolName || !targetNodeId) {
      return [];
    }

    return [
      {
        type: 'visible_internal_llm_tool',
        tool_name: toolName,
        target_node_id: targetNodeId,
        description: asString(tool.description).trim() || undefined,
        input_schema: isRecord(tool.input_schema)
          ? (tool.input_schema as Record<string, unknown>)
          : undefined
      }
    ];
  });
}

export function getLlmResponseFormat(
  config: Record<string, unknown>
): LlmNodeResponseFormat {
  const responseFormat = config.response_format;

  if (!isRecord(responseFormat)) {
    return DEFAULT_LLM_RESPONSE_FORMAT;
  }

  const mode = asString(responseFormat.mode);
  if (mode !== 'json_object' && mode !== 'json_schema') {
    return DEFAULT_LLM_RESPONSE_FORMAT;
  }

  const schema = isRecord(responseFormat.schema)
    ? (responseFormat.schema as Record<string, unknown>)
    : undefined;

  return {
    mode,
    schema
  };
}
