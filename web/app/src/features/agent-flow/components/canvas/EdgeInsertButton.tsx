import { Button } from 'antd';

import type { NodePickerOption } from '../../lib/plugin-node-definitions';
import { NodePickerPopover } from '../node-picker/NodePickerPopover';
import { i18nText } from '../../../../shared/i18n/text';
import { ConnectorAddIcon } from './ConnectorAddIcon';

export function EdgeInsertButton({
  open,
  onOpenChange,
  onPickNode,
  options
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onPickNode: (option: NodePickerOption) => void;
  options: NodePickerOption[];
}) {
  const ariaLabel = i18nText("agentFlow", "auto.add_new_node_connection");

  return (
    <NodePickerPopover
      ariaLabel={ariaLabel}
      open={open}
      options={options}
      onOpenChange={onOpenChange}
      onPickNode={onPickNode}
      placement="rightTop"
    >
      <Button
        aria-label={ariaLabel}
        className="agent-flow-edge-add-button"
        type="primary"
        shape="circle"
        size="small"
        icon={<ConnectorAddIcon />}
        onClick={(e) => {
          e.stopPropagation();
          onOpenChange(!open);
        }}
      />
    </NodePickerPopover>
  );
}
