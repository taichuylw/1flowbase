import { List, Space, Typography } from 'antd';
import { i18nText } from '../../../shared/i18n/text';

const embeddedAppCapabilities = [
  i18nText("embeddedApps", "auto.k_f223c2c4fe"),
  i18nText("embeddedApps", "auto.k_6e356eadf5"),
  i18nText("embeddedApps", "auto.k_87e2ff5369")
];

export function EmbeddedAppsPage() {
  return (
    <Space direction="vertical" size="large" style={{ width: '100%' }}>
      <div>
        <Typography.Title level={2}>{i18nText("embeddedApps", "auto.k_1c41ed3edc")}</Typography.Title>
        <Typography.Paragraph>
          {i18nText("embeddedApps", "auto.k_8be9a885e6")}</Typography.Paragraph>
      </div>
      <Typography.Paragraph>
        {i18nText("embeddedApps", "auto.k_7f32bd041b")}</Typography.Paragraph>
      <List
        dataSource={embeddedAppCapabilities}
        renderItem={(item) => <List.Item>{item}</List.Item>}
      />
    </Space>
  );
}
