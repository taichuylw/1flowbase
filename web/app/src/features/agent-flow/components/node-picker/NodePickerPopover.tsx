import { Button, Popover } from 'antd';
import type { ReactElement, ReactNode } from 'react';

import {
  BUILTIN_NODE_PICKER_OPTIONS,
  getNodePickerOptionDescription,
  getNodePickerOptionKey,
  type NodePickerOption
} from '../../lib/plugin-node-definitions';

interface NodePickerPopoverProps {
  ariaLabel: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onPickNode: (option: NodePickerOption) => void;
  options?: NodePickerOption[];
  buttonClassName?: string;
  buttonContent?: ReactNode;
  children?: ReactElement;
  placement?: 'top' | 'bottom' | 'left' | 'right' | 'rightTop';
}

function isContainerBuiltinOption(
  option: Extract<NodePickerOption, { kind: 'builtin' }>
) {
  return option.type === 'iteration' || option.type === 'loop';
}

export function NodePickerPopover({
  ariaLabel,
  open,
  onOpenChange,
  onPickNode,
  options = BUILTIN_NODE_PICKER_OPTIONS,
  buttonClassName,
  buttonContent = '+',
  children,
  placement = 'rightTop'
}: NodePickerPopoverProps) {
  const builtinOptions = options.filter(
    (option): option is Extract<NodePickerOption, { kind: 'builtin' }> =>
      option.kind === 'builtin'
  );
  const containerOptions = builtinOptions.filter(isContainerBuiltinOption);
  const regularBuiltinOptions = builtinOptions.filter(
    (option) => !isContainerBuiltinOption(option)
  );
  const pluginOptions = options.filter(
    (option): option is Extract<NodePickerOption, { kind: 'plugin_contribution' }> =>
      option.kind === 'plugin_contribution'
  );

  return (
    <Popover
      destroyOnHidden
      trigger="click"
      open={open}
      placement={placement}
      onOpenChange={onOpenChange}
      content={
        <div className="agent-flow-node-picker" role="menu">
          {regularBuiltinOptions.map((option) => (
            <button
              key={getNodePickerOptionKey(option)}
              className="agent-flow-node-picker__item"
              role="menuitem"
              type="button"
              onClick={() => {
                onOpenChange(false);
                onPickNode(option);
              }}
            >
              <span>{option.label}</span>
            </button>
          ))}
          {containerOptions.length > 0 ? (
            <div
              aria-label="节点分组"
              className="agent-flow-node-picker__group"
              role="group"
            >
              <div className="agent-flow-node-picker__section-label">
                节点分组
              </div>
              {containerOptions.map((option) => (
                <button
                  key={getNodePickerOptionKey(option)}
                  className="agent-flow-node-picker__item agent-flow-node-picker__item--group"
                  role="menuitem"
                  type="button"
                  onClick={() => {
                    onOpenChange(false);
                    onPickNode(option);
                  }}
                >
                  <span className="agent-flow-node-picker__group-title">
                    {option.label}
                  </span>
                  <span className="agent-flow-node-picker__group-preview">
                    <span className="agent-flow-node-picker__group-boundary">
                      开始
                    </span>
                    <span className="agent-flow-node-picker__group-line" />
                    <span className="agent-flow-node-picker__group-boundary agent-flow-node-picker__group-boundary--end">
                      结束
                    </span>
                  </span>
                </button>
              ))}
            </div>
          ) : null}
          {pluginOptions.length > 0 ? (
            <div className="agent-flow-node-picker__section-label">
              插件节点
            </div>
          ) : null}
          {pluginOptions.map((option) => (
            <button
              key={getNodePickerOptionKey(option)}
              className="agent-flow-node-picker__item"
              disabled={option.disabled}
              role="menuitem"
              type="button"
              onClick={() => {
                if (option.disabled) {
                  return;
                }

                onOpenChange(false);
                onPickNode(option);
              }}
            >
              <span>{option.label}</span>
              {getNodePickerOptionDescription(option) ? (
                <span className="agent-flow-node-picker__meta">
                  {getNodePickerOptionDescription(option)}
                </span>
              ) : null}
            </button>
          ))}
        </div>
      }
    >
      {children ?? (
        <Button
          aria-label={ariaLabel}
          className={buttonClassName}
          size="small"
          type="text"
          onClick={(event) => {
            event.stopPropagation();
          }}
        >
          {buttonContent}
        </Button>
      )}
    </Popover>
  );
}
