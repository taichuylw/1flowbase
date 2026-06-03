import {
  DeleteOutlined,
  MoreOutlined,
  PlayCircleOutlined,
  SwapOutlined
} from '@ant-design/icons';
import { Button, Dropdown, Tooltip, type MenuProps } from 'antd';
import { Position, useUpdateNodeInternals, type NodeProps } from '@xyflow/react';
import {
  useEffect,
  useRef,
  useState,
  type MouseEvent as ReactMouseEvent
} from 'react';

import { SchemaRenderer } from '../../../../shared/schema-ui/runtime/SchemaRenderer';
import { CanvasHandle } from '../canvas/CanvasHandle';
import { NodePickerPopover } from '../node-picker/NodePickerPopover';
import type { AgentFlowCanvasNode } from '../canvas/node-types';
import { agentFlowRendererRegistry } from '../../schema/agent-flow-renderer-registry';
import { getNodeDefinitionMeta } from '../../lib/node-definitions';
import {
  getNodePickerOptionDescription,
  getNodePickerOptionKey
} from '../../lib/plugin-node-definitions';
import { i18nText } from '../../../../shared/i18n/text';

const QUICK_ACTION_HIDE_DELAY_MS = 1000;

export function AgentFlowNodeCard({
  data,
  selected
}: NodeProps<AgentFlowCanvasNode>) {
  const [quickActionsVisible, setQuickActionsVisible] = useState(false);
  const updateNodeInternals = useUpdateNodeInternals();
  const hideQuickActionsTimerRef = useRef<ReturnType<typeof setTimeout> | null>(
    null
  );
  const stopActionEvent = (event: ReactMouseEvent<HTMLElement>) => {
    event.stopPropagation();
  };
  const clearHideQuickActionsTimer = () => {
    if (hideQuickActionsTimerRef.current === null) {
      return;
    }

    clearTimeout(hideQuickActionsTimerRef.current);
    hideQuickActionsTimerRef.current = null;
  };
  const showQuickActions = () => {
    clearHideQuickActionsTimer();
    setQuickActionsVisible(true);
  };
  const scheduleHideQuickActions = () => {
    clearHideQuickActionsTimer();
    hideQuickActionsTimerRef.current = setTimeout(() => {
      setQuickActionsVisible(false);
      hideQuickActionsTimerRef.current = null;
    }, QUICK_ACTION_HIDE_DELAY_MS);
  };

  useEffect(() => {
    return () => {
      if (hideQuickActionsTimerRef.current !== null) {
        clearTimeout(hideQuickActionsTimerRef.current);
      }
    };
  }, []);
  const nodePickerOptions = data.nodePickerOptions ?? [];
  const replaceItems: MenuProps['items'] = nodePickerOptions.map((option) => ({
    key: getNodePickerOptionKey(option),
    label: getNodePickerOptionDescription(option)
      ? `${option.label} · ${getNodePickerOptionDescription(option)}`
      : option.label,
    disabled: option.kind === 'plugin_contribution' && option.disabled,
    onClick: ({ domEvent }) => {
      domEvent.stopPropagation();
      data.onReplaceNode(data.nodeId, option);
    }
  }));
  const menuItems: MenuProps['items'] = [
    {
      key: 'run',
      icon: <PlayCircleOutlined />,
      label: i18nText("agentFlow", "auto.execute_this_node"),
      onClick: ({ domEvent }) => {
        domEvent.stopPropagation();
        data.onSelectNode(data.nodeId);
        data.onRunNode(data.nodeId);
      }
    },
    {
      key: 'replace',
      icon: <SwapOutlined />,
      label: i18nText("agentFlow", "auto.replace_node"),
      children: replaceItems
    },
    {
      key: 'delete',
      icon: <DeleteOutlined />,
      label: i18nText("agentFlow", "auto.delete_node"),
      danger: true,
      onClick: ({ domEvent }) => {
        domEvent.stopPropagation();
        data.onDeleteNode(data.nodeId);
      }
    }
  ];
  const cardAdapter = {
    getValue(path: string) {
      if (path === 'alias') {
        return data.alias;
      }

      if (path === 'description') {
        return data.description;
      }

      if (path.startsWith('config.')) {
        return data.config[path.slice('config.'.length)];
      }

      return null;
    },
    setValue: () => undefined,
    getDerived(key: string) {
      if (key === 'node') {
        return {
          id: data.nodeId,
          type: data.nodeType,
          alias: data.alias,
          description: data.description,
          config: data.config,
          outputs: []
        };
      }

      if (key === 'typeLabel') {
        return data.typeLabel;
      }

      if (key === 'definitionMeta') {
        return getNodeDefinitionMeta(data.nodeType);
      }

      return null;
    },
    dispatch: () => undefined
  } as const;
  const branchSourceHandles = data.branchSourceHandles ?? [];
  const branchHandleSignature = branchSourceHandles
    .map((handle) => handle.id)
    .join('|');
  const sourceHandles =
    branchSourceHandles.length > 0
      ? branchSourceHandles
      : [{ id: null, title: null }];

  useEffect(() => {
    updateNodeInternals(data.nodeId);
  }, [branchHandleSignature, data.nodeId, updateNodeInternals]);

  function renderSourceHandle(
    handle: { id: string | null; title: string | null },
    index: number
  ) {
    const pickerSourceHandleId = data.pickerSourceHandleId ?? null;
    const pickerOpen =
      data.pickerOpen && pickerSourceHandleId === handle.id;
    const ariaLabel = handle.title
      ? i18nText("agentFlow", "auto.add_node_after_branch", {
          value1: data.alias,
          value2: handle.title
        })
      : i18nText("agentFlow", "auto.add_node_after", { value1: data.alias });
    const top =
      sourceHandles.length > 1
        ? `${((index + 1) / (sourceHandles.length + 1)) * 100}%`
        : undefined;
    const style = top ? { top } : undefined;
    const tooltipTitle = handle.title ? (
      <div style={{ textAlign: 'center', fontSize: 12, padding: '2px 0' }}>
        <div>{handle.title}</div>
        <div>{i18nText("agentFlow", "auto.click_add_node")}</div>
      </div>
    ) : (
      <div style={{ textAlign: 'center', fontSize: 12, padding: '2px 0' }}>
        <div>{i18nText("agentFlow", "auto.click_add_node")}</div>
        <div>{i18nText("agentFlow", "auto.drag_drop_connect_nodes")}</div>
      </div>
    );

    return (
      <div key={handle.id ?? 'default'}>
        <NodePickerPopover
          ariaLabel={ariaLabel}
          open={pickerOpen}
          options={data.nodePickerOptions}
          onOpenChange={(open) => {
            if (open) {
              if (handle.id) {
                data.onOpenPicker(data.nodeId, handle.id);
              } else {
                data.onOpenPicker(data.nodeId);
              }
              return;
            }

            data.onClosePicker();
          }}
          onPickNode={(option) =>
            handle.id
              ? data.onInsertNode(data.nodeId, option, handle.id)
              : data.onInsertNode(data.nodeId, option)
          }
        >
          <Tooltip
            title={tooltipTitle}
            placement="top"
            color="#ffffff"
            styles={{
              body: {
                color: '#333',
                borderRadius: 8,
                boxShadow: '0 4px 12px rgba(0,0,0,0.1)'
              }
            }}
            open={!pickerOpen ? undefined : false}
          >
            <CanvasHandle
              id={handle.id ?? undefined}
              type="source"
              position={Position.Right}
              aria-expanded={pickerOpen}
              aria-haspopup="menu"
              aria-label={ariaLabel}
              className={`agent-flow-node-handle agent-flow-node-handle--source${handle.id ? ' agent-flow-node-handle--branch' : ''}`}
              role="button"
              style={style}
              tabIndex={0}
              onClick={(event) => {
                event.stopPropagation();
              }}
              onKeyDown={(event) => {
                if (event.key !== 'Enter' && event.key !== ' ') {
                  return;
                }

                event.preventDefault();
                event.stopPropagation();

                if (pickerOpen) {
                  data.onClosePicker();
                  return;
                }

                if (handle.id) {
                  data.onOpenPicker(data.nodeId, handle.id);
                } else {
                  data.onOpenPicker(data.nodeId);
                }
              }}
            >
              <span aria-hidden="true" className="agent-flow-node-handle__icon">
                +
              </span>
            </CanvasHandle>
          </Tooltip>
        </NodePickerPopover>
      </div>
    );
  }

  return (
    <>
      {data.showTargetHandle ? (
        <CanvasHandle
          type="target"
          position={Position.Left}
          className="agent-flow-node-handle agent-flow-node-handle--target"
        />
      ) : null}
      <div
        className={`agent-flow-node-card agent-flow-node-card--theme-unified agent-flow-node-card--type-${data.nodeType}${selected ? ' agent-flow-node-card--selected' : ''}`}
        role="button"
        tabIndex={0}
        onClick={() => data.onSelectNode(data.nodeId)}
        onMouseEnter={showQuickActions}
        onMouseLeave={scheduleHideQuickActions}
        onFocus={showQuickActions}
        onBlur={scheduleHideQuickActions}
        onDoubleClick={() => {
          if (data.canEnterContainer) {
            data.onOpenContainer(data.nodeId);
          }
        }}
        onKeyDown={(event) => {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault();
            data.onSelectNode(data.nodeId);
          }
        }}
      >
        <SchemaRenderer
          adapter={cardAdapter}
          blocks={data.nodeSchema.card.blocks}
          registry={agentFlowRendererRegistry}
        />
        <div
          className={`agent-flow-node-card__quick-actions${quickActionsVisible ? ' agent-flow-node-card__quick-actions--visible' : ''}`}
          data-testid={`agent-flow-node-quick-actions-${data.nodeId}`}
          onClick={stopActionEvent}
          onDoubleClick={stopActionEvent}
          onMouseEnter={showQuickActions}
          onMouseLeave={scheduleHideQuickActions}
          onMouseDown={stopActionEvent}
        >
          <Tooltip title={i18nText("agentFlow", "auto.execute_this_node")}>
            <Button
              aria-label={i18nText("agentFlow", "auto.execute", { value1: data.alias })}
              className="agent-flow-node-card__quick-action"
              icon={<PlayCircleOutlined />}
              shape="circle"
              size="small"
              type="text"
              onClick={(event) => {
                stopActionEvent(event);
                data.onSelectNode(data.nodeId);
                data.onRunNode(data.nodeId);
              }}
            />
          </Tooltip>
          <Dropdown menu={{ items: menuItems }} trigger={['click']}>
            <Button
              aria-label={i18nText("agentFlow", "auto.more_actions", { value1: data.alias })}
              className="agent-flow-node-card__quick-action"
              icon={<MoreOutlined />}
              shape="circle"
              size="small"
              type="text"
              onClick={stopActionEvent}
            />
          </Dropdown>
        </div>
      </div>
      {data.showSourceHandle
        ? sourceHandles.map((handle, index) => renderSourceHandle(handle, index))
        : null}
    </>
  );
}
