import { Input, InputNumber, Space, Switch, Tag, Typography } from 'antd';

import type { AgentFlowRunContext } from '../../api/runtime';
import { formatNodeVariableLabel } from '../../lib/variable-labels';
import { i18nText } from '../../../../shared/i18n/text';

function renderFieldInput(
  field: AgentFlowRunContext['fields'][number],
  onChange: (value: unknown) => void
) {
  if (field.valueType.startsWith('array')) {
    return (
      <Input.TextArea
        autoSize={{ minRows: 2, maxRows: 4 }}
        value={
          typeof field.value === 'string'
            ? field.value
            : JSON.stringify(field.value ?? [], null, 2)
        }
        onChange={(event) => onChange(event.target.value)}
      />
    );
  }

  switch (field.valueType) {
    case 'boolean':
      return <Switch checked={Boolean(field.value)} onChange={onChange} />;
    case 'number':
      return (
        <InputNumber
          style={{ width: '100%' }}
          value={typeof field.value === 'number' ? field.value : 0}
          onChange={(value) => onChange(value ?? 0)}
        />
      );
    case 'json':
    case 'unknown':
      return (
        <Input.TextArea
          autoSize={{ minRows: 2, maxRows: 4 }}
          value={
            typeof field.value === 'string'
              ? field.value
              : JSON.stringify(field.value ?? {}, null, 2)
          }
          onChange={(event) => onChange(event.target.value)}
        />
      );
    case 'string':
    default:
      return (
        <Input
          value={
            typeof field.value === 'string'
              ? field.value
              : String(field.value ?? '')
          }
          onChange={(event) => onChange(event.target.value)}
        />
      );
  }
}

export function RunContextPanel({
  runContext,
  onChangeValue
}: {
  runContext: AgentFlowRunContext;
  onChangeValue: (nodeId: string, key: string, value: unknown) => void;
}) {
  return (
    <section className="agent-flow-editor__debug-console-section">
      <div className="agent-flow-editor__debug-console-section-header">
        <Space direction="vertical" size={2}>
          <Typography.Text strong>{i18nText("agentFlow", "auto.run_context")}</Typography.Text>
          <Typography.Text type="secondary">
            {i18nText("agentFlow", "auto.pre_run_input_debugging_environment")}</Typography.Text>
        </Space>
        <Space size={8} wrap>
          <Tag color="blue">{i18nText("agentFlow", "auto.draft")}</Tag>
          <Tag color={runContext.remembered ? 'green' : 'default'}>
            {runContext.remembered ? i18nText("agentFlow", "auto.reuse_last_input") : i18nText("agentFlow", "auto.current_draft_defaults")}
          </Tag>
        </Space>
      </div>
      <div className="agent-flow-editor__debug-console-field-grid">
        {runContext.fields.map((field) => (
          <label
            key={`${field.nodeId}.${field.key}`}
            className="agent-flow-editor__debug-console-field"
          >
            <Typography.Text type="secondary">
              {formatNodeVariableLabel(field.nodeLabel, field.key)}
            </Typography.Text>
            {renderFieldInput(field, (value) =>
              onChangeValue(field.nodeId, field.key, value)
            )}
          </label>
        ))}
      </div>
    </section>
  );
}
