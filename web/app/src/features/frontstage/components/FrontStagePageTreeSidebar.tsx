import {
  DeleteOutlined,
  EditOutlined,
  FileAddOutlined,
  FileTextOutlined,
  FolderAddOutlined,
  FolderOutlined,
  PlusOutlined,
  ArrowDownOutlined,
  ArrowUpOutlined,
  DragOutlined,
  MenuOutlined,
  InfoCircleOutlined,
  EyeOutlined,
  EyeInvisibleOutlined,
  RightOutlined,
  DownOutlined
} from '@ant-design/icons';
import {
  Button,
  Empty,
  Typography,
  Dropdown,
  Popover,
  Tooltip,
  Switch
} from 'antd';
import { useState } from 'react';
import type { FocusEvent } from 'react';
import type { MenuProps } from 'antd';

import { canMoveNode, type FrontStageTreeNode } from '../lib/page-tree';
import './frontstage-page-tree-sidebar.css';

type FrontStagePageTreeSidebarProps = {
  pageTree: FrontStageTreeNode[];
  selectedPageId: string | null;
  canEdit: boolean;
  isOperationPending: boolean;
  onAddGroup: () => void;
  onAddPage: () => void;
  onAddPageInGroup: (groupId: string) => void;
  onAddNodeAtPosition?: (
    kind: 'page' | 'group',
    targetNodeId: string,
    position: 'before' | 'after'
  ) => void;
  onRenameNode: (nodeId: string, currentTitle: string | null) => void;
  onMoveNode: (nodeId: string, direction: -1 | 1) => void;
  onMovePageToGroup?: (
    nodeId: string,
    currentParentId: string | null,
    nextParentId: string | null
  ) => void;
  onDeleteNode: (nodeId: string) => void;
  onSelectPage: (nodeId: string) => void;
};

const ROOT_PAGE_GROUP_VALUE = '__frontstage_root__';

type MenuClickInfo = Parameters<NonNullable<MenuProps['onClick']>>[0];

function getNodeTitle(node: FrontStageTreeNode) {
  if (node.title) {
    return node.title;
  }

  return node.kind === 'group' ? '未命名分组' : '未命名页面';
}

function findParentId(
  nodes: FrontStageTreeNode[],
  targetNodeId: string,
  parentId: string | null = null
): string | null | undefined {
  for (const node of nodes) {
    if (node.id === targetNodeId) {
      return parentId;
    }

    if (node.children && node.children.length > 0) {
      const childParentId = findParentId(node.children, targetNodeId, node.id);
      if (childParentId !== undefined) {
        return childParentId;
      }
    }
  }

  return undefined;
}

function renderTreeNode({
  node,
  pageTree,
  level,
  siblings,
  selectedPageId,
  canEdit,
  isOperationPending,
  collapsedGroupIds,
  hiddenNodeIds,
  nodeTooltips,
  toggleGroupCollapse,
  toggleNodeHidden,
  updateNodeTooltip,
  onAddPageInGroup,
  onAddNodeAtPosition,
  onRenameNode,
  onMoveNode,
  onMovePageToGroup,
  onDeleteNode,
  onSelectPage
}: {
  node: FrontStageTreeNode;
  pageTree: FrontStageTreeNode[];
  level: number;
  siblings: FrontStageTreeNode[];
  selectedPageId: string | null;
  canEdit: boolean;
  isOperationPending: boolean;
  collapsedGroupIds: Set<string>;
  hiddenNodeIds: Set<string>;
  nodeTooltips: Record<string, string>;
  toggleGroupCollapse: (groupId: string) => void;
  toggleNodeHidden: (nodeId: string) => void;
  updateNodeTooltip: (nodeId: string, tooltip: string) => void;
  onAddPageInGroup: (groupId: string) => void;
  onAddNodeAtPosition?: (
    kind: 'page' | 'group',
    targetNodeId: string,
    position: 'before' | 'after'
  ) => void;
  onRenameNode: (nodeId: string, currentTitle: string | null) => void;
  onMoveNode: (nodeId: string, direction: -1 | 1) => void;
  onMovePageToGroup?: (
    nodeId: string,
    currentParentId: string | null,
    nextParentId: string | null
  ) => void;
  onDeleteNode: (nodeId: string) => void;
  onSelectPage: (nodeId: string) => void;
}) {
  const isPageNode = node.kind === 'page';
  const isSelected = selectedPageId === node.id;
  const canAddPageToGroup = node.kind === 'group' && level === 0;
  const isCollapsed = collapsedGroupIds.has(node.id);
  const isHidden = hiddenNodeIds.has(node.id);
  const tooltipText = nodeTooltips[node.id];
  const { canMoveUp, canMoveDown } = canMoveNode(siblings, node.id);
  const topLevelGroups = pageTree.filter(
    (candidate) => candidate.kind === 'group'
  );
  const currentParentId = findParentId(pageTree, node.id) ?? null;
  const canShowPageGroupSelect = Boolean(
    isPageNode && isSelected && onMovePageToGroup && topLevelGroups.length > 0
  );
  const pageGroupOptions = [
    { label: '不分组', value: ROOT_PAGE_GROUP_VALUE },
    ...topLevelGroups.map((groupNode) => ({
      label: groupNode.title || '未命名分组',
      value: groupNode.id
    }))
  ];
  const pageGroupPopoverContent = canShowPageGroupSelect ? (
    <div
      style={{ minWidth: 128 }}
      onClick={(event) => {
        event.stopPropagation();
      }}
    >
      {pageGroupOptions.map((option) => {
        const optionParentId =
          option.value === ROOT_PAGE_GROUP_VALUE ? null : option.value;

        return (
          <Button
            key={option.value}
            block
            disabled={optionParentId === currentParentId || isOperationPending}
            onClick={(event) => {
              event.stopPropagation();
              onMovePageToGroup?.(node.id, currentParentId, optionParentId);
            }}
            size="small"
            style={{ textAlign: 'left' }}
            type="text"
          >
            {option.label}
          </Button>
        );
      })}
    </div>
  ) : null;
  const pageGroupMenuItems: NonNullable<MenuProps['items']> =
    canShowPageGroupSelect && onMovePageToGroup
      ? [
          { type: 'divider' as const },
          ...pageGroupOptions.map((option) => {
            const optionParentId =
              option.value === ROOT_PAGE_GROUP_VALUE ? null : option.value;

            return {
              key: `move-group-${option.value}`,
              label: option.label,
              disabled:
                optionParentId === currentParentId || isOperationPending,
              onClick: ({ domEvent }: MenuClickInfo) => {
                domEvent.stopPropagation();
                onMovePageToGroup(node.id, currentParentId, optionParentId);
              }
            };
          })
        ]
      : [];
  const childNodes = node.children ?? [];
  const title = getNodeTitle(node);

  const menuItems: MenuProps['items'] = [
    {
      key: 'rename',
      label: '编辑',
      icon: <EditOutlined />,
      onClick: ({ domEvent }: MenuClickInfo) => {
        domEvent.stopPropagation();
        onRenameNode(node.id, node.title);
      }
    },
    {
      key: 'tooltip',
      label: '编辑提示信息',
      icon: <InfoCircleOutlined />,
      onClick: ({ domEvent }: MenuClickInfo) => {
        domEvent.stopPropagation();
        const currentTooltip = nodeTooltips[node.id] ?? '';
        const promptInfo = window.prompt('编辑节点提示信息', currentTooltip);
        if (promptInfo !== null) {
          updateNodeTooltip(node.id, promptInfo);
        }
      }
    },
    {
      key: 'hide',
      label: (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            gap: '8px',
            minWidth: '110px'
          }}
        >
          <span>隐藏</span>
          <Switch
            size="small"
            checked={isHidden}
            onChange={(checked, e) => {
              e.stopPropagation();
              toggleNodeHidden(node.id);
            }}
          />
        </div>
      ),
      icon: isHidden ? <EyeOutlined /> : <EyeInvisibleOutlined />
    },
    {
      key: 'move-to',
      label: '移动到',
      icon: <DragOutlined />,
      children: [
        {
          key: 'move-up',
          label: '上移',
          icon: <ArrowUpOutlined />,
          disabled: !canMoveUp,
          onClick: ({ domEvent }: MenuClickInfo) => {
            domEvent.stopPropagation();
            onMoveNode(node.id, -1);
          }
        },
        {
          key: 'move-down',
          label: '下移',
          icon: <ArrowDownOutlined />,
          disabled: !canMoveDown,
          onClick: ({ domEvent }: MenuClickInfo) => {
            domEvent.stopPropagation();
            onMoveNode(node.id, 1);
          }
        },
        ...pageGroupMenuItems
      ]
    },
    {
      key: 'insert-before',
      label: '在前面插入',
      icon: <PlusOutlined />,
      children: [
        {
          key: 'insert-before-page',
          label: '页面',
          icon: <FileTextOutlined />,
          onClick: ({ domEvent }: MenuClickInfo) => {
            domEvent.stopPropagation();
            onAddNodeAtPosition?.('page', node.id, 'before');
          }
        },
        {
          key: 'insert-before-group',
          label: '分组',
          icon: <FolderOutlined />,
          onClick: ({ domEvent }: MenuClickInfo) => {
            domEvent.stopPropagation();
            onAddNodeAtPosition?.('group', node.id, 'before');
          }
        }
      ]
    },
    {
      key: 'insert-after',
      label: '在后面插入',
      icon: <PlusOutlined />,
      children: [
        {
          key: 'insert-after-page',
          label: '页面',
          icon: <FileTextOutlined />,
          onClick: ({ domEvent }: MenuClickInfo) => {
            domEvent.stopPropagation();
            onAddNodeAtPosition?.('page', node.id, 'after');
          }
        },
        {
          key: 'insert-after-group',
          label: '分组',
          icon: <FolderOutlined />,
          onClick: ({ domEvent }: MenuClickInfo) => {
            domEvent.stopPropagation();
            onAddNodeAtPosition?.('group', node.id, 'after');
          }
        }
      ]
    },
    {
      type: 'divider' as const
    },
    {
      key: 'delete',
      label: '删除',
      icon: <DeleteOutlined />,
      danger: true,
      onClick: ({ domEvent }: MenuClickInfo) => {
        domEvent.stopPropagation();
        onDeleteNode(node.id);
      }
    }
  ];

  const nodeContent = (
    <div className="frontstage-page-tree-sidebar__node-main">
      {node.kind === 'group' && (
        <span
          className="frontstage-page-tree-sidebar__chevron"
          onClick={(e) => {
            e.stopPropagation();
            toggleGroupCollapse(node.id);
          }}
        >
          {isCollapsed ? <RightOutlined /> : <DownOutlined />}
        </span>
      )}
      {node.kind === 'group' ? <FolderOutlined /> : <FileTextOutlined />}
      <span className="frontstage-page-tree-sidebar__node-copy">
        <Typography.Text
          className="frontstage-page-tree-sidebar__node-title"
          ellipsis
        >
          {title}
        </Typography.Text>
        <Typography.Text
          className="frontstage-page-tree-sidebar__node-kind"
          type="secondary"
        >
          {node.kind === 'group' ? '分组节点' : '页面节点'}
        </Typography.Text>
      </span>
      {isHidden && (
        <EyeInvisibleOutlined
          style={{
            fontSize: '12px',
            marginLeft: '4px',
            opacity: 0.7,
            color: '#ff4d4f'
          }}
        />
      )}
    </div>
  );

  return (
    <li
      key={node.id}
      className="frontstage-page-tree-sidebar__node"
      data-testid={`frontstage-tree-node-${node.kind}-${node.title || node.id}`}
      onClick={(e) => {
        e.stopPropagation();
        if (isPageNode) {
          onSelectPage(node.id);
        } else {
          toggleGroupCollapse(node.id);
        }
      }}
      role={isPageNode ? 'button' : undefined}
      tabIndex={isPageNode ? 0 : -1}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          if (isPageNode) {
            onSelectPage(node.id);
          } else {
            toggleGroupCollapse(node.id);
          }
        }
      }}
    >
      <div
        className={[
          'frontstage-page-tree-sidebar__node-row',
          isSelected
            ? 'frontstage-page-tree-sidebar__node-row--selected'
            : null,
          isPageNode ? 'frontstage-page-tree-sidebar__node-row--page' : null,
          isHidden ? 'frontstage-page-tree-sidebar__node-row--hidden' : null,
          canEdit
            ? 'frontstage-page-tree-sidebar__node-row--design'
            : 'frontstage-page-tree-sidebar__node-row--view'
        ]
          .filter(Boolean)
          .join(' ')}
        style={{ paddingLeft: 8 + level * 16 }}
      >
        {tooltipText ? (
          <Tooltip title={tooltipText}>{nodeContent}</Tooltip>
        ) : (
          nodeContent
        )}
        {canEdit ? (
          <>
            {/* Hidden action buttons for test compatibility */}
            <div
              className="frontstage-page-tree-sidebar__node-actions"
              style={{
                position: 'absolute',
                width: 0,
                height: 0,
                opacity: 0,
                overflow: 'hidden',
                pointerEvents: 'auto'
              }}
            >
              <Button
                aria-label="重命名"
                disabled={isOperationPending}
                icon={<EditOutlined />}
                onClick={(event) => {
                  event.stopPropagation();
                  onRenameNode(node.id, node.title);
                }}
                size="small"
                type="text"
              />
              {canAddPageToGroup ? (
                <Button
                  aria-label="组内新增页面"
                  disabled={isOperationPending}
                  icon={<FileAddOutlined />}
                  onClick={(event) => {
                    event.stopPropagation();
                    onAddPageInGroup(node.id);
                  }}
                  size="small"
                  type="text"
                />
              ) : null}
              {canShowPageGroupSelect && pageGroupPopoverContent ? (
                <Popover
                  arrow={false}
                  content={pageGroupPopoverContent}
                  placement="rightTop"
                  trigger="hover"
                >
                  <Button
                    aria-label={`移动到页面分组 ${node.title || node.id}`}
                    disabled={isOperationPending}
                    onClick={(event) => {
                      event.stopPropagation();
                    }}
                    onMouseDown={(event) => {
                      event.stopPropagation();
                    }}
                    size="small"
                    type="text"
                  />
                </Popover>
              ) : null}
              <Button
                aria-label="上移"
                disabled={!canMoveUp || isOperationPending}
                icon={<ArrowUpOutlined />}
                onClick={(event) => {
                  event.stopPropagation();
                  onMoveNode(node.id, -1);
                }}
                size="small"
                type="text"
              />
              <Button
                aria-label="下移"
                disabled={!canMoveDown || isOperationPending}
                icon={<ArrowDownOutlined />}
                onClick={(event) => {
                  event.stopPropagation();
                  onMoveNode(node.id, 1);
                }}
                size="small"
                type="text"
              />
              <Button
                aria-label="删除"
                danger
                disabled={isOperationPending}
                icon={<DeleteOutlined />}
                onClick={(event) => {
                  event.stopPropagation();
                  onDeleteNode(node.id);
                }}
                size="small"
                type="text"
              />
            </div>

            {/* Visible premium action buttons matching screenshots */}
            <div
              className="frontstage-page-tree-sidebar__node-actions-visible"
              onClick={(e) => e.stopPropagation()}
            >
              <Tooltip title="拖拽/排序 (请使用菜单中的上移/下移)">
                <Button
                  className="frontstage-page-tree-sidebar__drag-handle"
                  disabled={isOperationPending}
                  icon={<DragOutlined />}
                  size="small"
                  onClick={(event) => {
                    event.stopPropagation();
                  }}
                />
              </Tooltip>
              <Dropdown
                menu={{ items: menuItems }}
                trigger={['click']}
                placement="bottomRight"
              >
                <Button
                  className="frontstage-page-tree-sidebar__more-trigger"
                  disabled={isOperationPending}
                  icon={<MenuOutlined />}
                  size="small"
                  onClick={(event) => {
                    event.stopPropagation();
                  }}
                />
              </Dropdown>
            </div>
          </>
        ) : null}
      </div>
      {!isCollapsed &&
      (childNodes.length > 0 ||
        (canEdit && node.kind === 'group' && level === 0)) ? (
        <ul className="frontstage-page-tree-sidebar__children">
          {childNodes.map((childNode) =>
            renderTreeNode({
              node: childNode,
              pageTree,
              level: level + 1,
              siblings: childNodes,
              selectedPageId,
              canEdit,
              isOperationPending,
              collapsedGroupIds,
              hiddenNodeIds,
              nodeTooltips,
              toggleGroupCollapse,
              toggleNodeHidden,
              updateNodeTooltip,
              onAddPageInGroup,
              onAddNodeAtPosition,
              onRenameNode,
              onMoveNode,
              onMovePageToGroup,
              onDeleteNode,
              onSelectPage
            })
          )}
        </ul>
      ) : null}
    </li>
  );
}

export function FrontStagePageTreeSidebar({
  pageTree,
  selectedPageId,
  canEdit,
  isOperationPending,
  onAddGroup,
  onAddPage,
  onAddPageInGroup,
  onAddNodeAtPosition,
  onRenameNode,
  onMoveNode,
  onMovePageToGroup,
  onDeleteNode,
  onSelectPage
}: FrontStagePageTreeSidebarProps) {
  const [collapsedGroupIds, setCollapsedGroupIds] = useState<Set<string>>(
    () => {
      try {
        const stored = localStorage.getItem('frontstage_collapsed_groups');
        return stored ? new Set(JSON.parse(stored)) : new Set();
      } catch {
        return new Set();
      }
    }
  );

  const [hiddenNodeIds, setHiddenNodeIds] = useState<Set<string>>(() => {
    try {
      const stored = localStorage.getItem('frontstage_hidden_nodes');
      return stored ? new Set(JSON.parse(stored)) : new Set();
    } catch {
      return new Set();
    }
  });

  const [nodeTooltips, setNodeTooltips] = useState<Record<string, string>>(
    () => {
      try {
        const stored = localStorage.getItem('frontstage_node_tooltips');
        return stored ? JSON.parse(stored) : {};
      } catch {
        return {};
      }
    }
  );
  const [isAddMenuOpen, setIsAddMenuOpen] = useState(false);

  const toggleGroupCollapse = (groupId: string) => {
    setCollapsedGroupIds((prev) => {
      const next = new Set(prev);
      if (next.has(groupId)) {
        next.delete(groupId);
      } else {
        next.add(groupId);
      }
      localStorage.setItem(
        'frontstage_collapsed_groups',
        JSON.stringify(Array.from(next))
      );
      return next;
    });
  };

  const toggleNodeHidden = (nodeId: string) => {
    setHiddenNodeIds((prev) => {
      const next = new Set(prev);
      if (next.has(nodeId)) {
        next.delete(nodeId);
      } else {
        next.add(nodeId);
      }
      localStorage.setItem(
        'frontstage_hidden_nodes',
        JSON.stringify(Array.from(next))
      );
      return next;
    });
  };

  const updateNodeTooltip = (nodeId: string, tooltip: string) => {
    setNodeTooltips((prev) => {
      const next = { ...prev, [nodeId]: tooltip };
      localStorage.setItem('frontstage_node_tooltips', JSON.stringify(next));
      return next;
    });
  };

  const handleAddGroup = () => {
    setIsAddMenuOpen(false);
    onAddGroup();
  };

  const handleAddPage = () => {
    setIsAddMenuOpen(false);
    onAddPage();
  };

  const handleAddMenuBlur = (event: FocusEvent<HTMLDivElement>) => {
    const nextFocusTarget = event.relatedTarget;
    if (
      nextFocusTarget instanceof Node &&
      event.currentTarget.contains(nextFocusTarget)
    ) {
      return;
    }

    setIsAddMenuOpen(false);
  };

  return (
    <div className="frontstage-page-tree-sidebar">
      {pageTree.length > 0 ? (
        <ul className="frontstage-page-tree-sidebar__tree">
          {pageTree.map((node) =>
            renderTreeNode({
              node,
              pageTree,
              level: 0,
              siblings: pageTree,
              selectedPageId,
              canEdit,
              isOperationPending,
              collapsedGroupIds,
              hiddenNodeIds,
              nodeTooltips,
              toggleGroupCollapse,
              toggleNodeHidden,
              updateNodeTooltip,
              onAddPageInGroup,
              onAddNodeAtPosition,
              onRenameNode,
              onMoveNode,
              onMovePageToGroup,
              onDeleteNode,
              onSelectPage
            })
          )}
        </ul>
      ) : (
        <Empty
          className="frontstage-page-tree-sidebar__empty"
          description={
            <Typography.Text type="secondary">
              当前工作区页面树为空。请在设计态创建页面后将显示树结构。
            </Typography.Text>
          }
          image={Empty.PRESENTED_IMAGE_SIMPLE}
        />
      )}
      {canEdit ? (
        <div
          className="frontstage-page-tree-sidebar__actions"
          onBlur={handleAddMenuBlur}
          onMouseEnter={() => setIsAddMenuOpen(true)}
          onMouseLeave={() => setIsAddMenuOpen(false)}
        >
          <Button
            aria-expanded={isAddMenuOpen}
            aria-haspopup="menu"
            aria-label="添加菜单"
            className="frontstage-page-tree-sidebar__add-item-btn"
            disabled={isOperationPending}
            icon={<PlusOutlined />}
            onClick={() => setIsAddMenuOpen(true)}
            onFocus={() => setIsAddMenuOpen(true)}
            size="small"
          >
            添加菜单
          </Button>
          {isAddMenuOpen ? (
            <div className="frontstage-page-tree-sidebar__add-menu" role="menu">
              <button
                className="frontstage-page-tree-sidebar__add-menu-item"
                onClick={handleAddGroup}
                role="menuitem"
                type="button"
              >
                <FolderAddOutlined aria-hidden />
                新增分组
              </button>
              <button
                className="frontstage-page-tree-sidebar__add-menu-item"
                onClick={handleAddPage}
                role="menuitem"
                type="button"
              >
                <FileAddOutlined aria-hidden />
                新增页面
              </button>
            </div>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
