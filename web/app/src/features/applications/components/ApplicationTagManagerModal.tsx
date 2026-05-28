import { Button, Checkbox, Empty, Flex, Form, Input, Modal, Space, Tag, Typography } from 'antd';
import { useEffect, useState } from 'react';

import type { ApplicationTagCatalogEntry } from '../api/applications';
import { i18nText } from '../../../shared/i18n/text';

interface ApplicationTagManagerModalProps {
  open: boolean;
  application:
    | {
        name: string;
        tags: Array<{ id: string; name: string }>;
      }
    | null;
  catalogTags: ApplicationTagCatalogEntry[];
  saving?: boolean;
  creating?: boolean;
  onCancel: () => void;
  onSubmit: (tagIds: string[]) => void;
  onCreateTag: (name: string) => Promise<{ id: string; name: string }>;
}

export function ApplicationTagManagerModal({
  open,
  application,
  catalogTags,
  saving = false,
  creating = false,
  onCancel,
  onSubmit,
  onCreateTag
}: ApplicationTagManagerModalProps) {
  const [selectedTagIds, setSelectedTagIds] = useState<string[]>([]);
  const [newTagName, setNewTagName] = useState('');
  const [errorText, setErrorText] = useState<string | null>(null);

  useEffect(() => {
    if (!open) {
      setSelectedTagIds([]);
      setNewTagName('');
      setErrorText(null);
      return;
    }

    setSelectedTagIds(application?.tags.map((tag) => tag.id) ?? []);
    setNewTagName('');
    setErrorText(null);
  }, [application, open]);

  const handleCreateTag = async () => {
    const normalizedName = newTagName.trim();
    if (!normalizedName) {
      setErrorText(i18nText("applications", "auto.k_71878d6b32"));
      return;
    }

    setErrorText(null);
    await onCreateTag(normalizedName);
    setNewTagName('');
  };

  return (
    <Modal
      open={open}
      title={i18nText("applications", "auto.k_d4ef5f914b")}
      okText={i18nText("applications", "auto.k_4a69baf51e")}
      cancelText={i18nText("applications", "auto.k_4d0b4688c7")}
      confirmLoading={saving}
      onCancel={onCancel}
      onOk={() => onSubmit(selectedTagIds)}
      destroyOnHidden
      forceRender
    >
      <Flex vertical gap={16}>
        <Form layout="vertical">
          <Form.Item label={i18nText("applications", "auto.k_b4240a63a6")} validateStatus={errorText ? 'error' : ''} help={errorText}>
            <Space.Compact style={{ width: '100%' }}>
              <Input
                aria-label={i18nText("applications", "auto.k_b4240a63a6")}
                value={newTagName}
                onChange={(event) => setNewTagName(event.target.value)}
              />
              <Button loading={creating} onClick={() => void handleCreateTag()}>
                {i18nText("applications", "auto.k_ee0c198f75")}</Button>
            </Space.Compact>
          </Form.Item>
        </Form>

        {catalogTags.length === 0 ? (
          <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={i18nText("applications", "auto.k_9d85acf9d5")} />
        ) : (
          <Flex vertical gap={12}>
            <Typography.Text type="secondary">{i18nText("applications", "auto.k_629321f112")}</Typography.Text>
            <Flex wrap gap={12}>
              {catalogTags.map((tag) => (
                <Checkbox
                  key={tag.id}
                  aria-label={tag.name}
                  checked={selectedTagIds.includes(tag.id)}
                  onChange={(event) => {
                    setSelectedTagIds((current) =>
                      event.target.checked
                        ? [...current, tag.id]
                        : current.filter((item) => item !== tag.id)
                    );
                  }}
                >
                  <Space size={6}>
                    <span>{tag.name}</span>
                    <Tag bordered={false}>{tag.application_count}</Tag>
                  </Space>
                </Checkbox>
              ))}
            </Flex>
          </Flex>
        )}
      </Flex>
    </Modal>
  );
}
