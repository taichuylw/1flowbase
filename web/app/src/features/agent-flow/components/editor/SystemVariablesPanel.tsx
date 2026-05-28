import { CloseOutlined } from '@ant-design/icons';
import { Button, Tag, Typography } from 'antd';

import { agentFlowSystemVariables } from '../../lib/system-variables';
import { i18nText } from '../../../../shared/i18n/text';

interface SystemVariablesPanelProps {
  onClose: () => void;
}

export function SystemVariablesPanel({ onClose }: SystemVariablesPanelProps) {
  return (
    <section
      aria-label={i18nText("agentFlow", "auto.k_872d17db93")}
      className="agent-flow-editor__system-variables-panel"
    >
      <header className="agent-flow-editor__system-variables-header">
        <div className="agent-flow-editor__system-variables-heading">
          <Typography.Title level={3}>{i18nText("agentFlow", "auto.k_872d17db93")}</Typography.Title>
          <Typography.Text type="secondary">
            {i18nText("agentFlow", "auto.k_0cab8a6c06")}</Typography.Text>
        </div>
        <Button
          aria-label={i18nText("agentFlow", "auto.k_71f2b1a617")}
          icon={<CloseOutlined />}
          type="text"
          onClick={onClose}
        />
      </header>
      <div className="agent-flow-editor__system-variables-list">
        {agentFlowSystemVariables.map((variable) => (
          <article
            className="agent-flow-editor__system-variable-row"
            key={variable.key}
          >
            <div className="agent-flow-editor__system-variable-main">
              <Typography.Text code>{variable.title}</Typography.Text>
              <Tag bordered={false}>{variable.valueType}</Tag>
            </div>
            <Typography.Text type="secondary">
              {variable.description}
            </Typography.Text>
          </article>
        ))}
      </div>
    </section>
  );
}
