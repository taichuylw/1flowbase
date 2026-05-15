import { Button, Divider, Empty, Flex, Layout, List, Space, Typography } from 'antd';
import type { FC } from 'react';
import { useEffect, useMemo, useRef, useState } from 'react';

import { useAuthStore } from '../../../state/auth-store';

const DESIGN_MODE_PERMISSION = 'frontstage.page.design';

type FrontStagePageProps = {
  workspaceId: string;
  pageId?: string;
};

type FrontStageTreeNode = {
  id: string;
  title: string;
  kind: 'group' | 'page';
  children?: FrontStageTreeNode[];
};

function isPageInTree(nodes: FrontStageTreeNode[], targetPageId: string): boolean {
  return nodes.some((node) => {
    if (node.kind === 'page' && node.id === targetPageId) {
      return true;
    }

    return Boolean(node.children && isPageInTree(node.children, targetPageId));
  });
}

function removeNodeFromTree(nodes: FrontStageTreeNode[], targetNodeId: string): FrontStageTreeNode[] {
  const nextNodes = [];

  for (const node of nodes) {
    if (node.id === targetNodeId) {
      continue;
    }

    nextNodes.push({
      ...node,
      children: node.children ? removeNodeFromTree(node.children, targetNodeId) : node.children
    });
  }

  return nextNodes;
}

export const FrontStagePage: FC<FrontStagePageProps> = ({ workspaceId, pageId }) => {
  const actor = useAuthStore((state) => state.actor);
  const me = useAuthStore((state) => state.me);
  const [isDesignMode, setIsDesignMode] = useState(false);
  const [pageTree, setPageTree] = useState<FrontStageTreeNode[]>(() => {
    return pageId ? [{ id: pageId, title: `页面 ${pageId}`, kind: 'page' }] : [];
  });
  const { Sider, Content } = Layout;
  const nextGroupNumber = useRef(1);
  const nextPageNumber = useRef(1);

  const canEnterDesignMode = useMemo(() => {
    return actor?.effective_display_role === 'root' || Boolean(me?.permissions.includes(DESIGN_MODE_PERMISSION));
  }, [actor, me]);

  useEffect(() => {
    if (!pageId) {
      return;
    }

    setPageTree((prev) => {
      if (isPageInTree(prev, pageId)) {
        return prev;
      }

      return [...prev, { id: pageId, title: `页面 ${pageId}`, kind: 'page' }];
    });
  }, [pageId]);

  const selectedPageLabel = pageId && isPageInTree(pageTree, pageId) ? pageId : null;
  const pageLabel = selectedPageLabel
    ? `页面 ${selectedPageLabel}`
    : '未选择 pageId（将使用默认首页）';
  const pageNodeTitle = selectedPageLabel ? `当前页面：${selectedPageLabel}` : '当前未选中页面';

  const handleAddGroup = () => {
    const next = nextGroupNumber.current;

    setPageTree((prev) => [
      ...prev,
      {
        id: `group-${next}`,
        title: `分组 ${next}`,
        kind: 'group',
        children: []
      }
    ]);

    nextGroupNumber.current = next + 1;
  };

  const handleAddPage = () => {
    const next = nextPageNumber.current;

    setPageTree((prev) => [
      ...prev,
      {
        id: `page-${next}`,
        title: `页面 新建 ${next}`,
        kind: 'page'
      }
    ]);

    nextPageNumber.current = next + 1;
  };

  const handleDeleteNode = (nodeId: string) => {
    setPageTree((prev) => removeNodeFromTree(prev, nodeId));
  };

  const renderTreeNode = (node: FrontStageTreeNode) => {
    return (
      <List.Item key={node.id} style={{ padding: '8px 0', borderBlockStart: 'none' }}>
        <List.Item.Meta
          title={<Typography.Text>{node.title}</Typography.Text>}
          description={
            <Typography.Text type="secondary">{node.kind === 'group' ? '分组节点' : '页面节点'}</Typography.Text>
          }
        />
        {canEnterDesignMode && isDesignMode ? (
          <Button size="small" danger onClick={() => handleDeleteNode(node.id)}>
            删除
          </Button>
        ) : null}
      </List.Item>
    );
  };

  return (
    <div style={{ width: '100%', padding: '24px 0', maxWidth: 1240, margin: '0 auto' }}>
      <Flex justify="space-between" align="center" wrap gap={12} style={{ marginBottom: 12 }}>
        <Space direction="vertical" size={0}>
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            前台
          </Typography.Text>
          <Typography.Title level={4} style={{ margin: 0 }}>
            空态占位 · {pageLabel}
          </Typography.Title>
          <Typography.Text type="secondary" style={{ marginTop: 4 }}>
            Workspace：{workspaceId}
          </Typography.Text>
        </Space>

        {canEnterDesignMode ? (
          <Space align="center" size={8} direction="vertical">
            <Button
              type={isDesignMode ? 'default' : 'primary'}
              onClick={() => setIsDesignMode((current) => !current)}
            >
              {isDesignMode ? '退出设计模式' : '进入设计模式'}
            </Button>
          </Space>
        ) : null}
      </Flex>

      <Divider style={{ margin: '0 0 16px' }} />

      {canEnterDesignMode && isDesignMode ? (
        <Space wrap size={8} style={{ marginBottom: 12 }}>
          <Button size="small">新增区块</Button>
          <Button size="small">页面管理</Button>
          <Button size="small">当前页面设置</Button>
          <Button size="small">JS Block 试运行</Button>
          <Button size="small">保存设计</Button>
        </Space>
      ) : null}
      <Layout style={{ background: 'transparent' }}>
        <Sider width={280} theme="light" style={{ background: 'white', borderRight: '1px solid #f0f0f0', padding: 12 }}>
          <Typography.Title level={5} style={{ margin: 0 }}>
            页面管理
          </Typography.Title>
          <Divider style={{ margin: '12px 0' }} />
          <Typography.Text type="secondary" style={{ marginBottom: 8, display: 'block' }}>
            {pageNodeTitle}
          </Typography.Text>
          {canEnterDesignMode && isDesignMode ? (
            <Space size={8} wrap style={{ marginBottom: 12 }}>
              <Button size="small" onClick={handleAddGroup}>
                新建分组
              </Button>
              <Button size="small" onClick={handleAddPage}>
                新建页面
              </Button>
            </Space>
          ) : null}
          {pageTree.length > 0 ? (
            <List size="small" dataSource={pageTree} renderItem={renderTreeNode} />
          ) : (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              imageStyle={{ height: 48 }}
              description={
                <Typography.Text type="secondary">
                  当前工作区页面树为空。请在设计态创建页面后将显示树结构。
                </Typography.Text>
              }
            />
          )}
        </Sider>
        <Content style={{ padding: 16, background: 'white' }}>
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={
              <div style={{ marginTop: 8 }}>
                {selectedPageLabel ? (
                  <Typography.Text>
                    当前页面尚未接入区块内容，浏览态仅展示空状态。请在设计态添加页面区块与内容。
                  </Typography.Text>
                ) : (
                  <Typography.Text>
                    当前前台未指定 pageId，后续将默认加载该工作区页面树里的首页。
                  </Typography.Text>
                )}
                {canEnterDesignMode && isDesignMode ? (
                  <Typography.Paragraph type="secondary" style={{ marginTop: 8, marginBottom: 0 }}>
                    设计模式已开启，后续在此承载区块编排与页面树管理能力。
                  </Typography.Paragraph>
                ) : null}
              </div>
            }
          />
        </Content>
      </Layout>
    </div>
  );
};
