import { Alert, Button, Drawer, Input, Space, Typography } from 'antd';
import type { FC } from 'react';

import { useFrontstageBlockCode } from '../hooks/use-frontstage-block-code';
import type { FrontstageBlockInstance } from '../lib/page-document';

export interface BlockCodeEditorDrawerProps {
  open: boolean;
  onClose: () => void;
  onOpenTrialPanel?: () => void;
  workspaceId: string | null | undefined;
  pageId: string | null | undefined;
  block?: FrontstageBlockInstance | null;
  codeRef?: string | null;
}

function normalizeCodeRef(codeRef: string | null | undefined): string | null {
  if (!codeRef) {
    return null;
  }

  const trimmed = codeRef.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function resolveCodeRef({
  block,
  codeRef
}: Pick<BlockCodeEditorDrawerProps, 'block' | 'codeRef'>): string | null {
  return normalizeCodeRef(codeRef) ?? normalizeCodeRef(block?.codeRef);
}

export const BlockCodeEditorDrawer: FC<BlockCodeEditorDrawerProps> = ({
  open,
  onClose,
  onOpenTrialPanel,
  workspaceId,
  pageId,
  block,
  codeRef
}) => {
  const selectedCodeRef = resolveCodeRef({ block, codeRef });
  const hasSelectedTarget = Boolean(block || selectedCodeRef);
  const canEdit = Boolean(workspaceId && pageId && selectedCodeRef);
  const { draft, dirty, loading, saving, error, setDraft, reset, save } =
    useFrontstageBlockCode({
      workspaceId,
      pageId,
      codeRef: selectedCodeRef
    });
  const saveDisabled = !canEdit || !dirty || loading || saving;
  const resetDisabled = !canEdit || !dirty || saving;
  const editorDisabled = !canEdit || loading || saving;
  const statusText = loading ? '代码加载中' : dirty ? '未保存' : '已同步';
  const emptyDescription = !hasSelectedTarget
    ? '请选择一个带 codeRef 的区块后再编辑代码。'
    : !selectedCodeRef
      ? '当前区块缺少 codeRef，无法加载或保存代码。'
      : !pageId
        ? '当前未选择页面，无法加载或保存代码。'
        : !workspaceId
          ? '当前未选择工作区，无法加载或保存代码。'
          : null;

  const handleSave = () => {
    void save().catch(() => undefined);
  };

  return (
    <Drawer
      open={open}
      onClose={onClose}
      placement="right"
      title="区块代码"
      width={560}
      extra={
        <Space size={8}>
          {onOpenTrialPanel ? (
            <Button disabled={!canEdit} onClick={onOpenTrialPanel}>
              JS Block 试运行
            </Button>
          ) : null}
          <Button disabled={resetDisabled} onClick={reset}>
            重置
          </Button>
          <Button
            type="primary"
            disabled={saveDisabled}
            loading={saving}
            onClick={handleSave}
          >
            保存
          </Button>
        </Space>
      }
    >
      <Space direction="vertical" size={12} style={{ width: '100%' }}>
        <Space direction="vertical" size={2} style={{ width: '100%' }}>
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            Block
          </Typography.Text>
          <Typography.Text strong>
            {block?.id ?? '未选择区块'}
          </Typography.Text>
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            codeRef：{selectedCodeRef ?? '缺失'}
          </Typography.Text>
        </Space>

        {emptyDescription ? (
          <Alert message={emptyDescription} type="info" showIcon />
        ) : null}

        {error ? (
          <Alert
            message="代码加载或保存失败"
            description={error.message}
            type="error"
            showIcon
          />
        ) : null}

        <Space direction="vertical" size={6} style={{ width: '100%' }}>
          <Space size={8}>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              状态
            </Typography.Text>
            <Typography.Text>{statusText}</Typography.Text>
          </Space>
          <Input.TextArea
            aria-label="Block code draft"
            value={draft}
            disabled={editorDisabled}
            onChange={(event) => setDraft(event.target.value)}
            autoSize={{ minRows: 16, maxRows: 24 }}
            spellCheck={false}
          />
        </Space>
      </Space>
    </Drawer>
  );
};
