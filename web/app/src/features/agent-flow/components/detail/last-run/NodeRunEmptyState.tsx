import { Card, Empty } from 'antd';

export function NodeRunEmptyState({
  description
}: {
  description: string;
}) {
  return (
    <Card title="运行记录">
      <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={description} />
    </Card>
  );
}
