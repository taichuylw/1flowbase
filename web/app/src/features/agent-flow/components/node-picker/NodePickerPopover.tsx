import { SearchOutlined } from '@ant-design/icons';
import { Button, Input, Popover } from 'antd';
import type { ReactElement, ReactNode } from 'react';
import { useMemo, useState } from 'react';

import {
  BUILTIN_NODE_PICKER_OPTIONS,
  getNodePickerOptionDescription,
  getNodePickerOptionKey,
  type NodePickerOption
} from '../../lib/plugin-node-definitions';
import { getAgentFlowNodeTypeIcon } from '../../lib/node-type-icons';
import { calculateNodePickerMaxHeight } from './node-picker-layout';
import { i18nText } from '../../../../shared/i18n/text';

type BuiltinNodePickerOption = Extract<NodePickerOption, { kind: 'builtin' }>;
type NodePickerTab = 'builtin' | 'plugin';

interface NodePickerGroup {
  key: string;
  title: string;
  description: string;
}

const BUILTIN_NODE_PICKER_GROUPS: NodePickerGroup[] = [
  {
    key: 'io',
    title: i18nText("agentFlow", "auto.k_a8651e4e38"),
    description: i18nText("agentFlow", "auto.k_28784497e1"),
  },
  {
    key: 'generation',
    title: i18nText("agentFlow", "auto.k_5247aeb989"),
    description: i18nText("agentFlow", "auto.k_8fd686e0cc"),
  },
  {
    key: 'control',
    title: i18nText("agentFlow", "auto.k_5bc1919bff"),
    description: i18nText("agentFlow", "auto.k_2c2de4b4c4"),
  },
  {
    key: 'data',
    title: i18nText("agentFlow", "auto.k_bd32031626"),
    description: i18nText("agentFlow", "auto.k_9d33e90522"),
  },
  {
    key: 'external',
    title: i18nText("agentFlow", "auto.k_da7de2c62d"),
    description: i18nText("agentFlow", "auto.k_09c331e725"),
  }
];

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
  const [searchValue, setSearchValue] = useState('');
  const [activeTab, setActiveTab] = useState<NodePickerTab>(() =>
    options.some((option) => option.kind === 'builtin') ? 'builtin' : 'plugin'
  );
  const normalizedSearchValue = searchValue.trim().toLowerCase();
  const builtinOptions = options.filter(
    (option): option is BuiltinNodePickerOption => option.kind === 'builtin'
  );
  const pluginOptions = options.filter(
    (option): option is Extract<NodePickerOption, { kind: 'plugin_contribution' }> =>
      option.kind === 'plugin_contribution'
  );
  const groupedBuiltinOptions = useMemo(
    () =>
      BUILTIN_NODE_PICKER_GROUPS.map((group) => {
        const groupSearchText = `${group.title} ${group.description}`.toLowerCase();
        const groupMatchesSearch =
          normalizedSearchValue.length > 0 &&
          groupSearchText.includes(normalizedSearchValue);
        const groupedOptions = builtinOptions
          .filter((option) => option.category === group.key)
          .filter((option) =>
            groupMatchesSearch ||
            matchesNodePickerSearch(
              option,
              normalizedSearchValue,
              getNodePickerOptionDescription(option)
            )
          );

        return { ...group, options: groupedOptions };
      }).filter((group) => group.options.length > 0),
    [builtinOptions, normalizedSearchValue]
  );
  const groupedBuiltinOptionKeys = new Set(
    BUILTIN_NODE_PICKER_GROUPS.map((group) => group.key)
  );
  const uncategorizedBuiltinOptions = builtinOptions.filter(
    (option) =>
      !groupedBuiltinOptionKeys.has(option.category ?? '') &&
      matchesNodePickerSearch(
        option,
        normalizedSearchValue,
        getNodePickerOptionDescription(option)
      )
  );
  const filteredPluginOptions = pluginOptions.filter((option) =>
    matchesNodePickerSearch(
      option,
      normalizedSearchValue,
      getNodePickerOptionDescription(option)
    )
  );
  const hasVisibleOptions =
    activeTab === 'builtin'
      ? groupedBuiltinOptions.length > 0 || uncategorizedBuiltinOptions.length > 0
      : filteredPluginOptions.length > 0;

  function closePicker() {
    setSearchValue('');
    onOpenChange(false);
  }

  function resolvePopupContainer(triggerNode: HTMLElement) {
    const canvas = triggerNode.closest<HTMLElement>('.agent-flow-canvas');

    if (!canvas) {
      return document.body;
    }

    const canvasRect = canvas.getBoundingClientRect();
    const triggerRect = triggerNode.getBoundingClientRect();
    const anchorY = placement === 'bottom' ? triggerRect.bottom : triggerRect.top;
    const maxHeight = calculateNodePickerMaxHeight(canvasRect.bottom, anchorY);

    canvas.style.setProperty(
      '--agent-flow-node-picker-max-height',
      `${maxHeight}px`
    );

    return canvas;
  }

  return (
    <Popover
      rootClassName="agent-flow-node-picker-popover"
      destroyOnHidden
      getPopupContainer={resolvePopupContainer}
      styles={{
        body: {
          boxSizing: 'border-box',
          maxHeight:
            'var(--agent-flow-node-picker-max-height, calc(100vh - 120px))',
          overflow: 'hidden',
          overscrollBehavior: 'contain'
        }
      }}
      trigger="click"
      open={open}
      placement={placement}
      onOpenChange={onOpenChange}
      content={
        <div className="agent-flow-node-picker">
          <div className="agent-flow-node-picker__header">
            <div
              aria-label={i18nText("agentFlow", "auto.k_39926b9c47")}
              className="agent-flow-node-picker__tabs"
              role="tablist"
            >
              <button
                aria-selected={activeTab === 'builtin'}
                className={`agent-flow-node-picker__tab${activeTab === 'builtin' ? ' agent-flow-node-picker__tab--active' : ''}`}
                role="tab"
                type="button"
                onClick={() => setActiveTab('builtin')}
              >
                {i18nText("agentFlow", "auto.k_09ceea7644")}</button>
              <button
                aria-selected={activeTab === 'plugin'}
                className={`agent-flow-node-picker__tab${activeTab === 'plugin' ? ' agent-flow-node-picker__tab--active' : ''}`}
                role="tab"
                type="button"
                onClick={() => setActiveTab('plugin')}
              >
                {i18nText("agentFlow", "auto.k_17d13aa49d")}</button>
            </div>
            <div className="agent-flow-node-picker__search">
              <Input
                allowClear
                aria-label={i18nText("agentFlow", "auto.k_91ab9a4e6d")}
                placeholder={i18nText("agentFlow", "auto.k_91ab9a4e6d")}
                prefix={<SearchOutlined />}
                size="small"
                value={searchValue}
                onChange={(event) => {
                  setSearchValue(event.target.value);
                }}
              />
            </div>
          </div>
          <div className="agent-flow-node-picker__list" role="menu">
            {activeTab === 'builtin' ? (
              <>
                {groupedBuiltinOptions.map((group) => (
                  <NodePickerSection
                    key={group.key}
                    title={group.title}
                  >
                    {group.options.map((option) => (
                      <NodePickerOptionButton
                        key={getNodePickerOptionKey(option)}
                        option={option}
                        onPick={() => {
                          closePicker();
                          onPickNode(option);
                        }}
                      />
                    ))}
                  </NodePickerSection>
                ))}
                {uncategorizedBuiltinOptions.length > 0 ? (
                  <NodePickerSection
                    title={i18nText("agentFlow", "auto.k_802f06c8f7")}
                  >
                    {uncategorizedBuiltinOptions.map((option) => (
                      <NodePickerOptionButton
                        key={getNodePickerOptionKey(option)}
                        option={option}
                        onPick={() => {
                          closePicker();
                          onPickNode(option);
                        }}
                      />
                    ))}
                  </NodePickerSection>
                ) : null}
              </>
            ) : (
              <>
                {filteredPluginOptions.length > 0 ? (
                  <NodePickerSection
                    title={i18nText("agentFlow", "auto.k_13e64fb2cd")}
                  >
                    {filteredPluginOptions.map((option) => (
                      <NodePickerOptionButton
                        key={getNodePickerOptionKey(option)}
                        option={option}
                        onPick={() => {
                          if (option.disabled) {
                            return;
                          }

                          closePicker();
                          onPickNode(option);
                        }}
                      />
                    ))}
                  </NodePickerSection>
                ) : null}
              </>
            )}
            {!hasVisibleOptions ? (
              <div className="agent-flow-node-picker__empty">
                {normalizedSearchValue.length > 0
                  ? i18nText("agentFlow", "auto.k_1e6d03870e")
                  : activeTab === 'plugin'
                    ? i18nText("agentFlow", "auto.k_31463adc9d")
                    : i18nText("agentFlow", "auto.k_18266961d9")}
              </div>
            ) : null}
          </div>
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

interface NodePickerSectionProps {
  title: string;
  children: ReactNode;
}

function NodePickerSection({
  title,
  children
}: NodePickerSectionProps) {
  return (
    <section className="agent-flow-node-picker__section">
      <div className="agent-flow-node-picker__section-head">
        <div className="agent-flow-node-picker__section-label">
          {title}
        </div>
      </div>
      <div className="agent-flow-node-picker__section-items">
        {children}
      </div>
    </section>
  );
}

interface NodePickerOptionButtonProps {
  option: NodePickerOption;
  onPick: () => void;
}

function NodePickerOptionButton({
  option,
  onPick
}: NodePickerOptionButtonProps) {
  const icon =
    option.kind === 'builtin'
      ? getAgentFlowNodeTypeIcon(option.type)
      : getAgentFlowNodeTypeIcon('plugin_node');

  return (
    <button
      aria-label={option.label}
      className="agent-flow-node-picker__item"
      disabled={option.kind === 'plugin_contribution' && option.disabled}
      role="menuitem"
      type="button"
      onClick={onPick}
    >
      <span className="agent-flow-node-picker__icon" aria-hidden="true">
        {icon}
      </span>
      <span className="agent-flow-node-picker__text">
        <span className="agent-flow-node-picker__name">{option.label}</span>
      </span>
    </button>
  );
}

function matchesNodePickerSearch(
  option: NodePickerOption,
  normalizedSearchValue: string,
  description: string | null
) {
  if (normalizedSearchValue.length === 0) {
    return true;
  }

  const searchText = [
    option.label,
    option.kind === 'builtin'
      ? [option.type, option.category, ...option.inputKeys, ...option.outputKeys].join(' ')
      : option.contribution.category,
    description
  ]
    .filter((value): value is string => Boolean(value))
    .join(' ')
    .toLowerCase();

  return searchText.includes(normalizedSearchValue);
}
