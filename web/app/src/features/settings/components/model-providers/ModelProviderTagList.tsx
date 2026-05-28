import { Tag, Typography } from 'antd';
import { i18nText } from '../../../../shared/i18n/text';

export function ModelProviderTagList({
  modelIds,
  emptyText = i18nText("settings", "auto.k_55a04b58cd")
}: {
  modelIds: string[];
  emptyText?: string;
}) {
  if (modelIds.length === 0) {
    return <Typography.Text type="secondary">{emptyText}</Typography.Text>;
  }

  return (
    <div className="model-provider-panel__model-tag-list">
      {modelIds.map((modelId) => (
        <Tag
          key={modelId}
          bordered={false}
          className="model-provider-panel__model-tag"
        >
          {modelId}
        </Tag>
      ))}
    </div>
  );
}
