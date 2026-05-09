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

import { SchemaDrawerPanel } from '../../../../shared/schema-ui/overlay-shell/SchemaDrawerPanel';

interface VersionHistoryDrawerProps {
  open: boolean;
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

const historyDrawerSchema = {
  schemaVersion: '1.0.0',
  shellType: 'drawer_panel',
  title: '历史版本',
  width: 420,
  getContainer: false
} as const;

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
    : `版本 ${version.sequence}`;
}

export function VersionHistoryDrawer({
  open,
  onClose,
  versions,
  restoring,
  updatingVersionId,
  onRestore,
  onUpdate
}: VersionHistoryDrawerProps) {
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
    <SchemaDrawerPanel
      open={open}
      schema={historyDrawerSchema}
      onClose={onClose}
    >
      <List
        dataSource={versions}
        locale={{ emptyText: '当前还没有可恢复的历史版本' }}
        renderItem={(version) => {
          const createdAt = formatVersionCreatedAt(version.created_at);
          const title = getVersionTitle(version);
          const updating = updatingVersionId === version.id;

          return (
            <List.Item
              actions={[
                <Button
                  aria-label={`${version.is_protected ? '取消置顶保护' : '置顶保护'} ${title}`}
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
                  aria-label={`编辑标题 ${title}`}
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
                  恢复版本 {version.sequence}
                </Button>
              ]}
            >
              <List.Item.Meta
                title={
                  <Space size={6}>
                    <span>{title}</span>
                    {version.is_protected ? (
                      <Typography.Text type="secondary">已保护</Typography.Text>
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
          'aria-label': '保存版本标题',
          disabled: editingTitle.trim().length === 0
        }}
        okText="保存"
        open={Boolean(editingVersion)}
        title="编辑版本标题"
        onCancel={() => {
          setEditingVersion(null);
          setEditingTitle('');
        }}
        onOk={() => {
          void saveTitle();
        }}
      >
        <Input
          aria-label="版本标题"
          maxLength={80}
          placeholder="输入版本标题"
          value={editingTitle}
          onChange={(event) => setEditingTitle(event.target.value)}
        />
      </Modal>
    </SchemaDrawerPanel>
  );
}
