import { BugFilled } from '@ant-design/icons';
import { Button } from 'antd';

import { i18nText } from '../../../../shared/i18n/text';

export function NodeDebugButton({
  disabled = false,
  onDebugNode,
  loading = false
}: {
  disabled?: boolean;
  onDebugNode?: (() => void) | undefined;
  loading?: boolean;
}) {
  return (
    <Button
      aria-label={i18nText('agentFlow', 'auto.debug_current_node')}
      disabled={!onDebugNode || disabled || loading}
      icon={<BugFilled />}
      loading={loading}
      type="text"
      onClick={() => onDebugNode?.()}
    />
  );
}
