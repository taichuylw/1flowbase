import {
  Alert,
  Button,
  Drawer,
  Empty,
  List,
  Radio,
  Space,
  Tag,
  Typography
} from 'antd';
import type { FC } from 'react';
import { useEffect, useMemo, useState } from 'react';

import type { NormalizedFrontstageBlockCatalogEntry } from '../lib/block-catalog';
import {
  listFrontstageBuiltInJsBlockTemplates,
  type FrontstageBuiltInJsBlockTemplateId
} from '../lib/block-templates';

export interface AddBlockCatalogPickerDrawerProps {
  open: boolean;
  items: NormalizedFrontstageBlockCatalogEntry[];
  loading?: boolean;
  error?: Error | null;
  saving?: boolean;
  onSelect: (
    entry: NormalizedFrontstageBlockCatalogEntry,
    templateId: FrontstageBuiltInJsBlockTemplateId
  ) => void;
  onClose: () => void;
}

export const AddBlockCatalogPickerDrawer: FC<
  AddBlockCatalogPickerDrawerProps
> = ({ open, items, loading, error, saving, onSelect, onClose }) => {
  const isBusy = Boolean(loading || saving);
  const templates = useMemo(() => listFrontstageBuiltInJsBlockTemplates(), []);
  const [selectedTemplateId, setSelectedTemplateId] =
    useState<FrontstageBuiltInJsBlockTemplateId>('blank');
  const selectedTemplate = templates.find(
    (template) => template.id === selectedTemplateId
  );

  useEffect(() => {
    if (open) {
      setSelectedTemplateId('blank');
    }
  }, [open]);

  return (
    <Drawer
      open={open}
      onClose={onClose}
      placement="right"
      title="新增区块"
      width={520}
    >
      <Space direction="vertical" size={12} style={{ width: '100%' }}>
        <Space direction="vertical" size={6} style={{ width: '100%' }}>
          <Typography.Text strong>内置模板</Typography.Text>
          <Radio.Group
            value={selectedTemplateId}
            disabled={isBusy}
            onChange={(event) =>
              setSelectedTemplateId(
                event.target.value as FrontstageBuiltInJsBlockTemplateId
              )
            }
          >
            <Space direction="vertical" size={4}>
              {templates.map((template) => (
                <Radio key={template.id} value={template.id}>
                  {template.title}
                </Radio>
              ))}
            </Space>
          </Radio.Group>
          {selectedTemplate ? (
            <Typography.Text type="secondary">
              {selectedTemplate.description}
            </Typography.Text>
          ) : null}
        </Space>

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
                    aria-label="选择"
                    type="primary"
                    size="small"
                    disabled={isBusy}
                    loading={saving}
                    onClick={() => onSelect(entry, selectedTemplateId)}
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
