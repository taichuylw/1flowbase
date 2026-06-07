import { BugFilled } from '@ant-design/icons';
import { Button } from 'antd';

import { i18nText } from '../../../../shared/i18n/text';

export function NodeDebugButton({
  onDebugNode,
  loading = false
}: {
  onDebugNode?: (() => void) | undefined;
  loading?: boolean;
}) {
  return (
    <Button
      aria-label={i18nText('agentFlow', 'auto.debug_current_node')}
      disabled={!onDebugNode || loading}
      icon={<BugFilled />}
      loading={loading}
      type="text"
      onClick={() => onDebugNode?.()}
    />
  );
}
