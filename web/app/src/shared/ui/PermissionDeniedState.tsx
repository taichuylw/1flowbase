import { Result } from 'antd';
import { i18nText } from '../i18n/text';

export function PermissionDeniedState() {
  return (
    <Result
      status="403"
      title={i18nText("sharedUi", "auto.no_access")}
      subTitle={i18nText("sharedUi", "auto.permission_required_description")}
    />
  );
}
