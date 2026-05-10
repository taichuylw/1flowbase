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
    title: '起止输出',
    description: '定义输入入口和最终响应。',
  },
  {
    key: 'generation',
    title: '模型与生成',
    description: '调用模型、检索知识并组织生成内容。',
  },
  {
    key: 'control',
    title: '流程控制',
    description: '分支、分类、循环和批量处理。',
  },
  {
    key: 'data',
    title: '数据处理',
    description: '读写结构化数据并维护流程变量。',
  },
  {
    key: 'external',
    title: '外部能力',
    description: '调用接口、工具和系统外能力。',
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
              aria-label="节点来源"
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
                内置
              </button>
              <button
                aria-selected={activeTab === 'plugin'}
                className={`agent-flow-node-picker__tab${activeTab === 'plugin' ? ' agent-flow-node-picker__tab--active' : ''}`}
                role="tab"
                type="button"
                onClick={() => setActiveTab('plugin')}
              >
                扩展
              </button>
            </div>
            <div className="agent-flow-node-picker__search">
              <Input
                allowClear
                aria-label="搜索节点"
                placeholder="搜索节点"
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
                    title="其他节点"
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
                    title="插件节点"
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
                  ? '没有匹配的节点'
                  : activeTab === 'plugin'
                    ? '暂无扩展节点'
                    : '暂无内置节点'}
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
