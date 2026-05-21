import {
  DeleteOutlined,
  EditOutlined,
  FileAddOutlined,
  FileTextOutlined,
  FolderAddOutlined,
  FolderOutlined,
  PlusOutlined,
  ArrowDownOutlined,
  ArrowUpOutlined
} from '@ant-design/icons';
import { Button, Empty, Space, Typography } from 'antd';

import { canMoveNode, type FrontStageTreeNode } from '../lib/page-tree';
import './frontstage-page-tree-sidebar.css';

type FrontStagePageTreeSidebarProps = {
  pageTree: FrontStageTreeNode[];
  selectedPageId: string | null;
  pageNodeTitle: string;
  canEdit: boolean;
  isOperationPending: boolean;
  onAddGroup: () => void;
  onAddPage: () => void;
  onAddPageInGroup: (groupId: string) => void;
  onRenameNode: (nodeId: string, currentTitle: string | null) => void;
  onMoveNode: (nodeId: string, direction: -1 | 1) => void;
  onDeleteNode: (nodeId: string) => void;
  onSelectPage: (nodeId: string) => void;
};

function getNodeTitle(node: FrontStageTreeNode) {
  if (node.title) {
    return node.title;
  }

  return node.kind === 'group' ? '未命名分组' : '未命名页面';
}

function renderTreeNode({
  node,
  level,
  siblings,
  selectedPageId,
  canEdit,
  isOperationPending,
  onAddPageInGroup,
  onRenameNode,
  onMoveNode,
  onDeleteNode,
  onSelectPage
}: {
  node: FrontStageTreeNode;
  level: number;
  siblings: FrontStageTreeNode[];
  selectedPageId: string | null;
  canEdit: boolean;
  isOperationPending: boolean;
  onAddPageInGroup: (groupId: string) => void;
  onRenameNode: (nodeId: string, currentTitle: string | null) => void;
  onMoveNode: (nodeId: string, direction: -1 | 1) => void;
  onDeleteNode: (nodeId: string) => void;
  onSelectPage: (nodeId: string) => void;
}) {
  const isPageNode = node.kind === 'page';
  const isSelected = selectedPageId === node.id;
  const canAddPageToGroup = node.kind === 'group' && level === 0;
  const { canMoveUp, canMoveDown } = canMoveNode(siblings, node.id);
  const childNodes = node.children ?? [];
  const title = getNodeTitle(node);

  return (
    <li
      key={node.id}
      className="frontstage-page-tree-sidebar__node"
      data-testid={`frontstage-tree-node-${node.kind}-${node.title || node.id}`}
      onClick={() => {
        if (isPageNode) {
          onSelectPage(node.id);
        }
      }}
      role={isPageNode ? 'button' : undefined}
      tabIndex={isPageNode ? 0 : -1}
      onKeyDown={(event) => {
        if (!isPageNode) {
          return;
        }

        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          onSelectPage(node.id);
        }
      }}
    >
      <div
        className={[
          'frontstage-page-tree-sidebar__node-row',
          isSelected ? 'frontstage-page-tree-sidebar__node-row--selected' : null,
          isPageNode ? 'frontstage-page-tree-sidebar__node-row--page' : null
        ]
          .filter(Boolean)
          .join(' ')}
        style={{ paddingLeft: 8 + level * 16 }}
      >
        <div className="frontstage-page-tree-sidebar__node-main">
          {node.kind === 'group' ? <FolderOutlined /> : <FileTextOutlined />}
          <span className="frontstage-page-tree-sidebar__node-copy">
            <Typography.Text className="frontstage-page-tree-sidebar__node-title" ellipsis>
              {title}
            </Typography.Text>
            <Typography.Text
              className="frontstage-page-tree-sidebar__node-kind"
              type="secondary"
            >
              {node.kind === 'group' ? '分组节点' : '页面节点'}
            </Typography.Text>
          </span>
        </div>
        {canEdit ? (
          <div className="frontstage-page-tree-sidebar__node-actions">
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
        ) : null}
      </div>
      {childNodes.length > 0 ? (
        <ul className="frontstage-page-tree-sidebar__children">
          {childNodes.map((childNode) =>
            renderTreeNode({
              node: childNode,
              level: level + 1,
              siblings: childNodes,
              selectedPageId,
              canEdit,
              isOperationPending,
              onAddPageInGroup,
              onRenameNode,
              onMoveNode,
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
  pageNodeTitle,
  canEdit,
  isOperationPending,
  onAddGroup,
  onAddPage,
  onAddPageInGroup,
  onRenameNode,
  onMoveNode,
  onDeleteNode,
  onSelectPage
}: FrontStagePageTreeSidebarProps) {
  return (
    <div className="frontstage-page-tree-sidebar">
      <Typography.Text
        className="frontstage-page-tree-sidebar__context"
        type="secondary"
      >
        {pageNodeTitle}
      </Typography.Text>
      {canEdit ? (
        <Space className="frontstage-page-tree-sidebar__actions" size={8} wrap>
          <Button
            aria-label="新建分组"
            disabled={isOperationPending}
            icon={<FolderAddOutlined />}
            onClick={onAddGroup}
            size="small"
          >
            新建分组
          </Button>
          <Button
            aria-label="新建页面"
            disabled={isOperationPending}
            icon={<PlusOutlined />}
            onClick={onAddPage}
            size="small"
          >
            新建页面
          </Button>
        </Space>
      ) : null}
      {pageTree.length > 0 ? (
        <ul className="frontstage-page-tree-sidebar__tree">
          {pageTree.map((node) =>
            renderTreeNode({
              node,
              level: 0,
              siblings: pageTree,
              selectedPageId,
              canEdit,
              isOperationPending,
              onAddPageInGroup,
              onRenameNode,
              onMoveNode,
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
    </div>
  );
}
