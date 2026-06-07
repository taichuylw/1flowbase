import { CaretRightOutlined } from '@ant-design/icons';
import { Button } from 'antd';
import { i18nText } from '../../../../shared/i18n/text';

export function NodeRunButton({
  disabled = false,
  onRunNode,
  loading = false
}: {
  disabled?: boolean;
  onRunNode?: (() => void) | undefined;
  loading?: boolean;
}) {
  return (
    <Button
      aria-label={i18nText("agentFlow", "auto.run_current_node")}
      disabled={!onRunNode || disabled || loading}
      icon={<CaretRightOutlined />}
      loading={loading}
      type="text"
      onClick={() => onRunNode?.()}
    />
  );
}
