import {
  EditOutlined,
  PushpinFilled,
  PushpinOutlined
} from '@ant-design/icons';
import { Button, Input, List, Modal, Space, Typography } from 'antd';
import { useState } from 'react';

import type {
  ConsoleFlowVersionSummary,
  UpdateConsoleApplicationVersionInput
} from '@1flowbase/api-client';

import { AgentFlowDockPanel } from '../editor/AgentFlowDockPanel';
import { i18nText } from '../../../../shared/i18n/text';

interface VersionHistoryPanelProps {
  onClose: () => void;
  versions: ConsoleFlowVersionSummary[];
  restoring: boolean;
  updatingVersionId?: string | null;
  onRestore: (versionId: string) => Promise<unknown>;
  onUpdate: (
    versionId: string,
    input: UpdateConsoleApplicationVersionInput
  ) => Promise<unknown>;
}

function formatVersionCreatedAt(value: string) {
  const isoMatch = value.match(/^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2}:\d{2})/);

  if (isoMatch) {
    return `${isoMatch[1]} ${isoMatch[2]}`;
  }

  const date = new Date(value);

  if (Number.isNaN(date.getTime())) {
    return value;
  }

  const pad = (part: number) => String(part).padStart(2, '0');

  return [
    `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`,
    `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`
  ].join(' ');
}

function getVersionTitle(version: ConsoleFlowVersionSummary) {
  return version.summary_is_custom && version.summary.trim().length > 0
    ? version.summary
    : i18nText("agentFlow", "auto.key_hoehimgeif", { value1: version.sequence });
}

export function VersionHistoryPanel({
  onClose,
  versions,
  restoring,
  updatingVersionId,
  onRestore,
  onUpdate
}: VersionHistoryPanelProps) {
  const [editingVersion, setEditingVersion] =
    useState<ConsoleFlowVersionSummary | null>(null);
  const [editingTitle, setEditingTitle] = useState('');

  async function saveTitle() {
    if (!editingVersion) {
      return;
    }

    const summary = editingTitle.trim();

    if (!summary) {
      return;
    }

    await onUpdate(editingVersion.id, {
      summary,
      summary_is_custom: true
    });
    setEditingVersion(null);
    setEditingTitle('');
  }

  return (
    <AgentFlowDockPanel
      bodyClassName="agent-flow-editor__history-panel-body"
      className="agent-flow-editor__history-panel"
      closeLabel={i18nText("agentFlow", "auto.key_dnagiikfek")}
      title={i18nText("agentFlow", "auto.key_fomefcfipi")}
      onClose={onClose}
    >
      <List
        className="agent-flow-editor__history-list"
        dataSource={versions}
        locale={{ emptyText: i18nText("agentFlow", "auto.key_nemmaagdnf") }}
        renderItem={(version) => {
          const createdAt = formatVersionCreatedAt(version.created_at);
          const title = getVersionTitle(version);
          const updating = updatingVersionId === version.id;

          return (
            <List.Item
              actions={[
                <Button
                  aria-label={`${version.is_protected ? i18nText("agentFlow", "auto.key_ghjkbdhlej") : i18nText("agentFlow", "auto.key_lachjcifhi")} ${title}`}
                  icon={
                    version.is_protected ? (
                      <PushpinFilled />
                    ) : (
                      <PushpinOutlined />
                    )
                  }
                  key="protect"
                  loading={updating}
                  type={version.is_protected ? 'primary' : 'text'}
                  onClick={() => {
                    void onUpdate(version.id, {
                      is_protected: !version.is_protected
                    });
                  }}
                />,
                <Button
                  aria-label={i18nText("agentFlow", "auto.key_jkjaimollk", { value1: title })}
                  icon={<EditOutlined />}
                  key="edit"
                  type="text"
                  onClick={() => {
                    setEditingVersion(version);
                    setEditingTitle(title);
                  }}
                />,
                <Button
                  key="restore"
                  loading={restoring}
                  onClick={() => {
                    void onRestore(version.id);
                  }}
                >
                  {i18nText("agentFlow", "auto.key_amdlcedghd")}{version.sequence}
                </Button>
              ]}
            >
              <List.Item.Meta
                title={
                  <Space size={6}>
                    <span>{title}</span>
                    {version.is_protected ? (
                      <Typography.Text type="secondary">{i18nText("agentFlow", "auto.key_ohafhplfgf")}</Typography.Text>
                    ) : null}
                  </Space>
                }
                description={createdAt}
              />
            </List.Item>
          );
        }}
      />
      <Modal
        confirmLoading={
          editingVersion ? updatingVersionId === editingVersion.id : false
        }
        destroyOnHidden
        okButtonProps={{
          'aria-label': i18nText("agentFlow", "auto.key_nennfjollh"),
          disabled: editingTitle.trim().length === 0
        }}
        okText={i18nText("agentFlow", "auto.key_pknpcenlmf")}
        open={Boolean(editingVersion)}
        title={i18nText("agentFlow", "auto.key_cbbfmolegm")}
        onCancel={() => {
          setEditingVersion(null);
          setEditingTitle('');
        }}
        onOk={() => {
          void saveTitle();
        }}
      >
        <Input
          aria-label={i18nText("agentFlow", "auto.key_fbfhfphcpa")}
          maxLength={80}
          placeholder={i18nText("agentFlow", "auto.key_dcifigbkjd")}
          value={editingTitle}
          onChange={(event) => setEditingTitle(event.target.value)}
        />
      </Modal>
    </AgentFlowDockPanel>
  );
}
