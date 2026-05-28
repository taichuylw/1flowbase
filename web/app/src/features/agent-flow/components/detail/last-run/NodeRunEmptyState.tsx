import { Card, Empty } from 'antd';
import { i18nText } from '../../../../../shared/i18n/text';

export function NodeRunEmptyState({
  description
}: {
  description: string;
}) {
  return (
    <Card title={i18nText("agentFlow", "auto.k_24fb424dfd")}>
      <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={description} />
    </Card>
  );
}
