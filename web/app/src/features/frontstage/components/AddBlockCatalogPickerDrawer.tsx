import { Alert, Button, Drawer, Empty, List, Space, Tag, Typography } from 'antd';
import type { FC } from 'react';

import type { NormalizedFrontstageBlockCatalogEntry } from '../lib/block-catalog';

export interface AddBlockCatalogPickerDrawerProps {
  open: boolean;
  items: NormalizedFrontstageBlockCatalogEntry[];
  loading?: boolean;
  error?: Error | null;
  saving?: boolean;
  onSelect: (entry: NormalizedFrontstageBlockCatalogEntry) => void;
  onClose: () => void;
}

export const AddBlockCatalogPickerDrawer: FC<
  AddBlockCatalogPickerDrawerProps
> = ({ open, items, loading, error, saving, onSelect, onClose }) => {
  const isBusy = Boolean(loading || saving);

  return (
    <Drawer
      open={open}
      onClose={onClose}
      placement="right"
      title="新增区块"
      width={520}
    >
      <Space direction="vertical" size={12} style={{ width: '100%' }}>
        {error ? (
          <Alert
            message="区块目录加载失败"
            description={error.message}
            type="error"
            showIcon
          />
        ) : null}

        {items.length === 0 && !loading ? (
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={
              <Typography.Text type="secondary">
                当前没有可用区块目录项，暂时无法新增区块。
              </Typography.Text>
            }
          />
        ) : (
          <List
            loading={loading}
            dataSource={items}
            rowKey={(entry) => entry.id}
            renderItem={(entry) => (
              <List.Item
                actions={[
                  <Button
                    key="select"
                    type="primary"
                    size="small"
                    disabled={isBusy}
                    loading={saving}
                    onClick={() => onSelect(entry)}
                  >
                    选择
                  </Button>
                ]}
              >
                <List.Item.Meta
                  title={entry.title}
                  description={
                    <Space size={6} wrap>
                      <Tag>{entry.runtimeKind}</Tag>
                      <Typography.Text type="secondary">
                        {entry.providerCode}
                      </Typography.Text>
                      <Typography.Text type="secondary">
                        {entry.contributionCode}
                      </Typography.Text>
                    </Space>
                  }
                />
              </List.Item>
            )}
          />
        )}
      </Space>
    </Drawer>
  );
};
