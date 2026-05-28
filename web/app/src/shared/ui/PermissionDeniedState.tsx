import { Result } from 'antd';
import { i18nText } from '../i18n/text';

export function PermissionDeniedState() {
  return (
    <Result
      status="403"
      title={i18nText("sharedUi", "auto.k_9dd50f94f3")}
      subTitle={i18nText("sharedUi", "auto.k_d66f2a8fbe")}
    />
  );
}
