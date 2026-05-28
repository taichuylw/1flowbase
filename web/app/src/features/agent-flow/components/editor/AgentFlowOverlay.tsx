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
    idle: { color: 'default', label: i18nText("agentFlow", "auto.k_837e7a109a") },
    saving: { color: 'blue', label: i18nText("agentFlow", "auto.k_15127c2c4f") },
    saved: { color: 'green', label: i18nText("agentFlow", "auto.k_cdfab96f75") },
    error: { color: 'red', label: i18nText("agentFlow", "auto.k_40525a7328") }
  }[autosaveStatus];

  return (
    <div
      aria-label={i18nText("agentFlow", "auto.k_dabbdf1cc8")}
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
          aria-label={i18nText("agentFlow", "auto.k_de61aa8e1c")}
          autoInsertSpace={false}
          icon={<PlayCircleOutlined />}
          onClick={onOpenDebugConsole}
          title={i18nText("agentFlow", "auto.k_de61aa8e1c")}
        >
          {i18nText("agentFlow", "auto.k_de61aa8e1c")}</Button>
        <Badge count={issueErrorCount} size="small">
          <Button
            aria-label="Issues"
            icon={<IssuesCloseOutlined />}
            onClick={onOpenIssues}
            title="Issues"
          />
        </Badge>
        <Button
          aria-label={i18nText("agentFlow", "auto.k_872d17db93")}
          autoInsertSpace={false}
          icon={<GlobalOutlined />}
          onClick={onOpenSystemVariables}
          title={i18nText("agentFlow", "auto.k_872d17db93")}
        >
          {i18nText("agentFlow", "auto.k_872d17db93")}</Button>
        <Button
          aria-label={i18nText("agentFlow", "auto.k_8da07705ab")}
          autoInsertSpace={false}
          icon={<CodeOutlined />}
          onClick={onOpenEnvironmentVariables}
          title={i18nText("agentFlow", "auto.k_8da07705ab")}
        >
          {i18nText("agentFlow", "auto.k_8da07705ab")}</Button>
        <Tooltip title={autosaveLabel}>
          <Button
            aria-label={i18nText("agentFlow", "auto.k_fadf24dbc5")}
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
          {i18nText("agentFlow", "auto.k_94f172d02f")}</Button>
        <Button
          aria-label={i18nText("agentFlow", "auto.k_5ec45258f8")}
          icon={<HistoryOutlined />}
          onClick={onOpenHistory}
          title={i18nText("agentFlow", "auto.k_5ec45258f8")}
        />
      </Space>
    </div>
  );
}
