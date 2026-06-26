import { DeleteOutlined, PlusOutlined } from '@ant-design/icons';
import {
  Button,
  Checkbox,
  Empty,
  Flex,
  Input,
  Select,
  Space,
  Tabs,
  Typography
} from 'antd';
import { useEffect, useMemo, useState } from 'react';
import type {
  ConsoleMcpInterfaceCapability,
  ConsoleMcpParameterDescriptor,
  ConsoleMcpParameterType
} from '@1flowbase/api-client';

import { i18nText } from '../../../../shared/i18n/text';

type McpInputInterfaceParameter = {
  name: string;
  field_type: string;
  parameter_type: ConsoleMcpParameterType;
  description: string;
  required: boolean;
};

type McpInputParameterMapping = {
  interface_param: string;
  mcp_param: string;
  description: string;
  required: boolean;
};

export type McpInputMappingValue = {
  interface_parameters: McpInputInterfaceParameter[];
  mappings: McpInputParameterMapping[];
};

export const emptyInputMapping: McpInputMappingValue = {
  interface_parameters: [],
  mappings: []
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function stringValue(value: unknown) {
  return typeof value === 'string' ? value : '';
}

function booleanValue(value: unknown) {
  return typeof value === 'boolean' ? value : false;
}

function parameterTypeValue(value: unknown): ConsoleMcpParameterType {
  return value === 'url' || value === 'form' || value === 'json_body'
    ? value
    : 'json_body';
}

function normalizeInterfaceParameter(value: unknown): McpInputInterfaceParameter | null {
  if (!isRecord(value)) {
    return null;
  }
  const name = stringValue(value.name);
  if (!name) {
    return null;
  }

  return {
    name,
    field_type: stringValue(value.field_type),
    parameter_type: parameterTypeValue(value.parameter_type),
    description: stringValue(value.description),
    required: booleanValue(value.required)
  };
}

function normalizeMapping(value: unknown): McpInputParameterMapping | null {
  if (!isRecord(value)) {
    return null;
  }
  const interfaceParam = stringValue(value.interface_param);
  if (!interfaceParam) {
    return null;
  }

  return {
    interface_param: interfaceParam,
    mcp_param: stringValue(value.mcp_param) || interfaceParam,
    description: stringValue(value.description),
    required: booleanValue(value.required)
  };
}

export function normalizeInputMapping(value: unknown): McpInputMappingValue {
  if (!isRecord(value)) {
    return emptyInputMapping;
  }

  return {
    interface_parameters: Array.isArray(value.interface_parameters)
      ? value.interface_parameters
          .map(normalizeInterfaceParameter)
          .filter((parameter): parameter is McpInputInterfaceParameter =>
            Boolean(parameter)
          )
      : [],
    mappings: Array.isArray(value.mappings)
      ? value.mappings
          .map(normalizeMapping)
          .filter((mapping): mapping is McpInputParameterMapping =>
            Boolean(mapping)
          )
      : []
  };
}

function stringifyMapping(value: McpInputMappingValue) {
  return JSON.stringify(value, null, 2);
}

function mappingFromInterfaceParameter(
  parameter: McpInputInterfaceParameter
): McpInputParameterMapping {
  return {
    interface_param: parameter.name,
    mcp_param: parameter.name,
    description: parameter.description,
    required: parameter.required
  };
}

function nextInterfaceParameterName(parameters: McpInputInterfaceParameter[]) {
  const names = new Set(parameters.map((parameter) => parameter.name));
  let index = parameters.length + 1;
  let name = `param_${index}`;

  while (names.has(name)) {
    index += 1;
    name = `param_${index}`;
  }

  return name;
}

function emptyInterfaceParameter(
  parameters: McpInputInterfaceParameter[]
): McpInputInterfaceParameter {
  return {
    name: nextInterfaceParameterName(parameters),
    field_type: 'string',
    parameter_type: 'json_body',
    description: '',
    required: false
  };
}

function parameterTypeOptions() {
  return [
    { label: 'URL', value: 'url' },
    { label: 'form', value: 'form' },
    {
      label: i18nText('settings', 'auto.json_request_body'),
      value: 'json_body'
    }
  ];
}

export function buildInputMappingFromParameterDescriptors(
  descriptors: ConsoleMcpParameterDescriptor[]
): McpInputMappingValue {
  const interfaceParameters = descriptors.map((descriptor) => ({
    name: descriptor.name,
    field_type: descriptor.field_type,
    parameter_type: descriptor.parameter_type,
    description: descriptor.description ?? '',
    required: descriptor.required
  }));

  return {
    interface_parameters: interfaceParameters,
    mappings: []
  };
}

export function buildInputMappingFromInterface(
  entry: ConsoleMcpInterfaceCapability,
  currentValue?: unknown
): McpInputMappingValue {
  const nextMapping = buildInputMappingFromParameterDescriptors(
    entry.parameter_descriptors
  );
  const currentMappings = new Map(
    normalizeInputMapping(currentValue).mappings.map((mapping) => [
      mapping.interface_param,
      mapping
    ])
  );

  return {
    ...nextMapping,
    mappings: nextMapping.interface_parameters.flatMap((parameter) => {
      const mapping = currentMappings.get(parameter.name);
      return mapping ? [mapping] : [];
    })
  };
}

export function inputMappingHasContent(value: unknown): boolean {
  const mapping = normalizeInputMapping(value);
  return (
    mapping.interface_parameters.length > 0 ||
    mapping.mappings.some(
      (entry) =>
        entry.interface_param ||
        entry.mcp_param ||
        entry.description ||
        entry.required
    )
  );
}

export function McpInputMappingEditor({
  value,
  resetKey,
  onChange,
  onValidityChange
}: {
  value: unknown;
  resetKey?: string | number | null;
  onChange: (value: McpInputMappingValue) => void;
  onValidityChange?: (valid: boolean) => void;
}) {
  const mapping = useMemo(() => normalizeInputMapping(value), [value]);
  const [jsonText, setJsonText] = useState(() => stringifyMapping(mapping));
  const [jsonError, setJsonError] = useState('');
  const [pendingInterfaceParam, setPendingInterfaceParam] = useState<
    string | undefined
  >();

  useEffect(() => {
    setJsonText(stringifyMapping(mapping));
    setJsonError('');
    onValidityChange?.(true);
  }, [mapping, onValidityChange, resetKey]);

  function emit(nextMapping: McpInputMappingValue) {
    setJsonText(stringifyMapping(nextMapping));
    setJsonError('');
    onValidityChange?.(true);
    onChange(nextMapping);
  }

  function updateMapping(
    index: number,
    patch: Partial<McpInputParameterMapping>
  ) {
    emit({
      ...mapping,
      mappings: mapping.mappings.map((entry, entryIndex) =>
        entryIndex === index ? { ...entry, ...patch } : entry
      )
    });
  }

  function addInterfaceParameter() {
    emit({
      ...mapping,
      interface_parameters: [
        ...mapping.interface_parameters,
        emptyInterfaceParameter(mapping.interface_parameters)
      ]
    });
  }

  function updateInterfaceParameter(
    index: number,
    patch: Partial<McpInputInterfaceParameter>
  ) {
    const currentParameter = mapping.interface_parameters[index];
    if (!currentParameter) {
      return;
    }

    const nextParameter = { ...currentParameter, ...patch };
    const nextMappings =
      patch.name === undefined
        ? mapping.mappings
        : mapping.mappings.map((entry) => {
            if (entry.interface_param !== currentParameter.name) {
              return entry;
            }

            return {
              ...entry,
              interface_param: nextParameter.name,
              mcp_param:
                entry.mcp_param === currentParameter.name
                  ? nextParameter.name
                  : entry.mcp_param
            };
          });

    emit({
      interface_parameters: mapping.interface_parameters.map(
        (entry, entryIndex) => (entryIndex === index ? nextParameter : entry)
      ),
      mappings: nextMappings
    });
  }

  function removeInterfaceParameter(index: number) {
    const parameter = mapping.interface_parameters[index];
    if (!parameter) {
      return;
    }

    emit({
      interface_parameters: mapping.interface_parameters.filter(
        (_, entryIndex) => entryIndex !== index
      ),
      mappings: mapping.mappings.filter(
        (entry) => entry.interface_param !== parameter.name
      )
    });
  }

  function addMapping(interfaceParam: string | undefined) {
    const parameter = mapping.interface_parameters.find(
      (entry) => entry.name === interfaceParam
    );
    if (!parameter) {
      return;
    }

    emit({
      ...mapping,
      mappings: [...mapping.mappings, mappingFromInterfaceParameter(parameter)]
    });
    setPendingInterfaceParam(undefined);
  }

  function removeMapping(index: number) {
    emit({
      ...mapping,
      mappings: mapping.mappings.filter((_, entryIndex) => entryIndex !== index)
    });
  }

  function updateJsonText(nextText: string) {
    setJsonText(nextText);
    try {
      const parsed = JSON.parse(nextText) as unknown;
      const nextMapping = normalizeInputMapping(parsed);
      setJsonError('');
      onValidityChange?.(true);
      onChange(nextMapping);
    } catch {
      setJsonError(i18nText('settings', 'auto.enter_valid_json'));
      onValidityChange?.(false);
    }
  }

  const mappedParameters = new Set(
    mapping.mappings.map((entry) => entry.interface_param)
  );
  const addableOptions = mapping.interface_parameters
    .filter((entry) => entry.name && !mappedParameters.has(entry.name))
    .map((entry) => ({ label: entry.name, value: entry.name }));

  return (
    <div className="mcp-input-mapping-editor">
      <Tabs
        items={[
          {
            key: 'interface',
            label: i18nText('settings', 'auto.mcp_input_interface_layer'),
            children: (
              <Space
                className="mcp-input-mapping-editor__stack"
                direction="vertical"
                size="middle"
              >
                <Flex justify="flex-end">
                  <Button icon={<PlusOutlined />} onClick={addInterfaceParameter}>
                    {i18nText('settings', 'auto.add_new_field')}
                  </Button>
                </Flex>
                {mapping.interface_parameters.length > 0 ? (
                  <div className="mcp-input-mapping-editor__table">
                    <div className="mcp-input-mapping-editor__head">
                      <span>{i18nText('settings', 'auto.field_name')}</span>
                      <span>{i18nText('settings', 'auto.field_type')}</span>
                      <span>{i18nText('settings', 'auto.parameter_type')}</span>
                      <span>{i18nText('settings', 'auto.required')}</span>
                      <span />
                    </div>
                    {mapping.interface_parameters.map((parameter, index) => (
                      <div
                        className="mcp-input-mapping-editor__row"
                        key={`${parameter.name}:${index}`}
                      >
                        <Input
                          aria-label={`field_name ${index + 1}`}
                          value={parameter.name}
                          onChange={(event) =>
                            updateInterfaceParameter(index, {
                              name: event.target.value
                            })
                          }
                        />
                        <Input
                          aria-label={`field_type ${
                            parameter.name || index + 1
                          }`}
                          value={parameter.field_type}
                          onChange={(event) =>
                            updateInterfaceParameter(index, {
                              field_type: event.target.value
                            })
                          }
                        />
                        <Select
                          aria-label={`parameter_type ${
                            parameter.name || index + 1
                          }`}
                          options={parameterTypeOptions()}
                          value={parameter.parameter_type}
                          onChange={(nextParameterType) =>
                            updateInterfaceParameter(index, {
                              parameter_type: nextParameterType
                            })
                          }
                        />
                        <Checkbox
                          aria-label={`required ${parameter.name || index + 1}`}
                          checked={parameter.required}
                          onChange={(event) =>
                            updateInterfaceParameter(index, {
                              required: event.target.checked
                            })
                          }
                        />
                        <Button
                          aria-label={`delete_field ${
                            parameter.name || index + 1
                          }`}
                          icon={<DeleteOutlined />}
                          onClick={() => removeInterfaceParameter(index)}
                        />
                      </div>
                    ))}
                  </div>
                ) : (
                  <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} />
                )}
              </Space>
            )
          },
          {
            key: 'mapping',
            label: i18nText('settings', 'auto.mcp_input_mapping_layer'),
            children: (
              <Space
                className="mcp-input-mapping-editor__stack"
                direction="vertical"
                size="middle"
              >
                <Flex gap={8} wrap="wrap">
                  <Select
                    aria-label="interface_param"
                    placeholder="interface_param"
                    options={addableOptions}
                    value={pendingInterfaceParam}
                    style={{ minWidth: 220 }}
                    onChange={setPendingInterfaceParam}
                  />
                  <Button
                    icon={<PlusOutlined />}
                    disabled={!pendingInterfaceParam}
                    onClick={() => addMapping(pendingInterfaceParam)}
                  >
                    {i18nText('settings', 'auto.add_mapping')}
                  </Button>
                </Flex>
                {mapping.mappings.length > 0 ? (
                  <div className="mcp-input-mapping-editor__table">
                    <div className="mcp-input-mapping-editor__mapping-head">
                      <span>{i18nText('settings', 'auto.interface_param')}</span>
                      <span>{i18nText('settings', 'auto.mcp_param')}</span>
                      <span>{i18nText('settings', 'auto.description')}</span>
                      <span>{i18nText('settings', 'auto.required')}</span>
                      <span />
                    </div>
                    {mapping.mappings.map((entry, index) => (
                      <div
                        className="mcp-input-mapping-editor__mapping-row"
                        key={`${entry.interface_param}:${index}`}
                      >
                        <Input readOnly value={entry.interface_param} />
                        <Input
                          aria-label={`mcp_param ${entry.interface_param}`}
                          value={entry.mcp_param}
                          onChange={(event) =>
                            updateMapping(index, {
                              mcp_param: event.target.value
                            })
                          }
                        />
                        <Input
                          aria-label={`description ${entry.interface_param}`}
                          value={entry.description}
                          onChange={(event) =>
                            updateMapping(index, {
                              description: event.target.value
                            })
                          }
                        />
                        <Checkbox
                          aria-label={`required ${entry.interface_param}`}
                          checked={entry.required}
                          onChange={(event) =>
                            updateMapping(index, {
                              required: event.target.checked
                            })
                          }
                        />
                        <Button
                          aria-label={`delete_mapping ${entry.interface_param}`}
                          icon={<DeleteOutlined />}
                          onClick={() => removeMapping(index)}
                        />
                      </div>
                    ))}
                  </div>
                ) : (
                  <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} />
                )}
              </Space>
            )
          },
          {
            key: 'json',
            label: i18nText('settings', 'auto.json_parse'),
            children: (
              <Space direction="vertical" style={{ width: '100%' }}>
                <Input.TextArea
                  aria-label="input_mapping JSON"
                  rows={12}
                  value={jsonText}
                  onChange={(event) => updateJsonText(event.target.value)}
                />
                <Typography.Text type={jsonError ? 'danger' : 'secondary'}>
                  {jsonError ||
                    i18nText('settings', 'auto.support_json_parse')}
                </Typography.Text>
              </Space>
            )
          }
        ]}
      />
    </div>
  );
}
