import { Result } from 'antd';
import { i18nText } from '../../../shared/i18n/text';

export function ToolsPage() {
  return (
    <Result
      status="info"
      title={i18nText("tools", "auto.k_a72ef18d9a")}
      subTitle={i18nText("tools", "auto.k_db821d505e")}
    />
  );
}
