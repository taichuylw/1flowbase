import { Result } from 'antd';
import { i18nText } from '../../../shared/i18n/text';

export function ToolsPage() {
  return (
    <Result
      status="info"
      title={i18nText("tools", "auto.tools")}
      subTitle={i18nText("tools", "auto.tools_portal_under_construction")}
    />
  );
}
