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
    idle: { color: 'default', label: i18nText("agentFlow", "auto.key_idhohkbajk") },
    saving: { color: 'blue', label: i18nText("agentFlow", "auto.key_bfbchmcmep") },
    saved: { color: 'green', label: i18nText("agentFlow", "auto.key_mnpkljgphf") },
    error: { color: 'red', label: i18nText("agentFlow", "auto.key_eafcfkhdci") }
  }[autosaveStatus];

  return (
    <div
      aria-label={i18nText("agentFlow", "auto.key_nkllnpbmmi")}
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
          aria-label={i18nText("agentFlow", "auto.key_nogbkkiobm")}
          autoInsertSpace={false}
          icon={<PlayCircleOutlined />}
          onClick={onOpenDebugConsole}
          title={i18nText("agentFlow", "auto.key_nogbkkiobm")}
        >
          {i18nText("agentFlow", "auto.key_nogbkkiobm")}</Button>
        <Badge count={issueErrorCount} size="small">
          <Button
            aria-label="Issues"
            icon={<IssuesCloseOutlined />}
            onClick={onOpenIssues}
            title="Issues"
          />
        </Badge>
        <Button
          aria-label={i18nText("agentFlow", "auto.key_ihcnbhnljd")}
          autoInsertSpace={false}
          icon={<GlobalOutlined />}
          onClick={onOpenSystemVariables}
          title={i18nText("agentFlow", "auto.key_ihcnbhnljd")}
        >
          {i18nText("agentFlow", "auto.key_ihcnbhnljd")}</Button>
        <Button
          aria-label={i18nText("agentFlow", "auto.key_inkahhafkl")}
          autoInsertSpace={false}
          icon={<CodeOutlined />}
          onClick={onOpenEnvironmentVariables}
          title={i18nText("agentFlow", "auto.key_inkahhafkl")}
        >
          {i18nText("agentFlow", "auto.key_inkahhafkl")}</Button>
        <Tooltip title={autosaveLabel}>
          <Button
            aria-label={i18nText("agentFlow", "auto.key_pknpcenlmf")}
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
          {i18nText("agentFlow", "auto.key_jepbhcnacp")}</Button>
        <Button
          aria-label={i18nText("agentFlow", "auto.key_fomefcfipi")}
          icon={<HistoryOutlined />}
          onClick={onOpenHistory}
          title={i18nText("agentFlow", "auto.key_fomefcfipi")}
        />
      </Space>
    </div>
  );
}
