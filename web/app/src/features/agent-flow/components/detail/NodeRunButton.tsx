import { CaretRightOutlined } from '@ant-design/icons';
import { Button } from 'antd';
import { i18nText } from '../../../../shared/i18n/text';

export function NodeRunButton({
  onRunNode,
  loading = false
}: {
  onRunNode?: (() => void) | undefined;
  loading?: boolean;
}) {
  return (
    <Button
      aria-label={i18nText("agentFlow", "auto.key_khpanhiadd")}
      disabled={!onRunNode || loading}
      icon={<CaretRightOutlined />}
      loading={loading}
      type="text"
      onClick={() => onRunNode?.()}
    />
  );
}
