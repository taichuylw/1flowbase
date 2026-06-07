import {
  CodeOutlined,
  GlobalOutlined,
  HistoryOutlined,
  IssuesCloseOutlined,
  PlayCircleOutlined,
  SaveOutlined
} from '@ant-design/icons';
import { Badge, Button, Space, Tag, Tooltip, Typography } from 'antd';
import { i18nText } from '../../../../shared/i18n/text';

interface AgentFlowOverlayProps {
  applicationName: string;
  autosaveLabel: string;
  autosaveStatus: 'idle' | 'saving' | 'saved' | 'error';
  onSaveDraft: () => void;
  saveDisabled: boolean;
  saveLoading: boolean;
  onOpenDebugConsole: () => void;
  onOpenIssues: () => void;
  onOpenHistory: () => void;
  onOpenEnvironmentVariables: () => void;
  onOpenSystemVariables: () => void;
  onOpenPublish: () => void;
  issueErrorCount: number;
  publishDisabled: boolean;
}

export function AgentFlowOverlay({
  applicationName,
  autosaveLabel,
  autosaveStatus,
  onSaveDraft,
  saveDisabled,
  saveLoading,
  onOpenDebugConsole,
  onOpenIssues,
  onOpenHistory,
  onOpenEnvironmentVariables,
  onOpenSystemVariables,
  onOpenPublish,
  issueErrorCount,
  publishDisabled
}: AgentFlowOverlayProps) {
  const statusTag = {
    idle: { color: 'default', label: i18nText("agentFlow", "auto.free") },
    saving: { color: 'blue', label: i18nText("agentFlow", "auto.saving") },
    saved: { color: 'green', label: i18nText("agentFlow", "auto.saved") },
    error: { color: 'red', label: i18nText("agentFlow", "auto.save_failed") }
  }[autosaveStatus];

  return (
    <div
      aria-label={i18nText("agentFlow", "auto.agent_flow_action_bar")}
      className="agent-flow-editor__overlay"
      role="region"
    >
      <Space className="agent-flow-editor__overlay-status" size="small">
        <Typography.Text strong>{applicationName}</Typography.Text>
        <Tag color={statusTag.color} bordered={false}>
          {statusTag.label}
        </Tag>
      </Space>
      <Space size="small">
        <Button
          aria-label={i18nText("agentFlow", "auto.preview")}
          autoInsertSpace={false}
          icon={<PlayCircleOutlined />}
          onClick={onOpenDebugConsole}
          title={i18nText("agentFlow", "auto.preview")}
        >
          {i18nText("agentFlow", "auto.preview")}</Button>
        <Badge count={issueErrorCount} size="small">
          <Button
            aria-label="Issues"
            icon={<IssuesCloseOutlined />}
            onClick={onOpenIssues}
            title="Issues"
          />
        </Badge>
        <Button
          aria-label={i18nText("agentFlow", "auto.system_variables")}
          autoInsertSpace={false}
          icon={<GlobalOutlined />}
          onClick={onOpenSystemVariables}
          title={i18nText("agentFlow", "auto.system_variables")}
        />
        <Button
          aria-label={i18nText("agentFlow", "auto.environment_variables")}
          autoInsertSpace={false}
          icon={<CodeOutlined />}
          onClick={onOpenEnvironmentVariables}
          title={i18nText("agentFlow", "auto.environment_variables")}
        />
        <Tooltip title={autosaveLabel}>
          <Button
            aria-label={i18nText("agentFlow", "auto.save")}
            autoInsertSpace={false}
            disabled={saveDisabled}
            icon={<SaveOutlined />}
            loading={saveLoading}
            onClick={onSaveDraft}
          />
        </Tooltip>
        <Button
          autoInsertSpace={false}
          type="primary"
          disabled={publishDisabled}
          onClick={onOpenPublish}
        >
          {i18nText("agentFlow", "auto.publish")}</Button>
        <Button
          aria-label={i18nText("agentFlow", "auto.historical_version")}
          icon={<HistoryOutlined />}
          onClick={onOpenHistory}
          title={i18nText("agentFlow", "auto.historical_version")}
        />
      </Space>
    </div>
  );
}
