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
import { i18nText } from '../../../shared/i18n/text';

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
      title={i18nText("frontstage", "auto.k_d9b1f67999")}
      width={520}
    >
      <Space direction="vertical" size={12} style={{ width: '100%' }}>
        <Space direction="vertical" size={6} style={{ width: '100%' }}>
          <Typography.Text strong>{i18nText("frontstage", "auto.k_1cddfcf703")}</Typography.Text>
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
            message={i18nText("frontstage", "auto.k_21a4702b5d")}
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
                {i18nText("frontstage", "auto.k_fa781eade0")}</Typography.Text>
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
                    aria-label={i18nText("frontstage", "auto.k_70b208202c")}
                    type="primary"
                    size="small"
                    disabled={isBusy}
                    loading={saving}
                    onClick={() => onSelect(entry, selectedTemplateId)}
                  >
                    {i18nText("frontstage", "auto.k_70b208202c")}</Button>
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
