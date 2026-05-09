import { Button, List } from 'antd';

import { SchemaDrawerPanel } from '../../../../shared/schema-ui/overlay-shell/SchemaDrawerPanel';

interface VersionHistoryDrawerProps {
  open: boolean;
  onClose: () => void;
  versions: Array<{
    id: string;
    sequence: number;
    trigger: 'autosave' | 'restore';
    change_kind: 'logical';
    summary: string;
    created_at: string;
  }>;
  restoring: boolean;
  onRestore: (versionId: string) => Promise<unknown>;
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

export function VersionHistoryDrawer({
  open,
  onClose,
  versions,
  restoring,
  onRestore
}: VersionHistoryDrawerProps) {
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

          return (
            <List.Item
              actions={[
                <Button
                  key={version.id}
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
                title={`版本 ${version.sequence}`}
                description={createdAt}
              />
            </List.Item>
          );
        }}
      />
    </SchemaDrawerPanel>
  );
}
