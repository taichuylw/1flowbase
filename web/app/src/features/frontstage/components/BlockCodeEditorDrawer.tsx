import { Alert, Button, Drawer, Input, Space, Typography } from 'antd';
import type { FC } from 'react';

import { useFrontstageBlockCode } from '../hooks/use-frontstage-block-code';
import type { FrontstageBlockInstance } from '../lib/page-document';
import { i18nText } from '../../../shared/i18n/text';

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
  const statusText = loading ? i18nText("frontstage", "auto.k_1f37320940") : dirty ? i18nText("frontstage", "auto.k_4123f1faaa") : i18nText("frontstage", "auto.k_76ce8e3d5e");
  const emptyDescription = !hasSelectedTarget
    ? i18nText("frontstage", "auto.k_637390fc4f")
    : !selectedCodeRef
      ? i18nText("frontstage", "auto.k_f09775c006")
      : !pageId
        ? i18nText("frontstage", "auto.k_ed0dec3817")
        : !workspaceId
          ? i18nText("frontstage", "auto.k_d0e05ae721")
          : null;

  const handleSave = () => {
    void save().catch(() => undefined);
  };

  return (
    <Drawer
      open={open}
      onClose={onClose}
      placement="right"
      title={i18nText("frontstage", "auto.k_9d86669742")}
      width={560}
      extra={
        <Space size={8}>
          {onOpenTrialPanel ? (
            <Button disabled={!canEdit} onClick={onOpenTrialPanel}>
              {i18nText("frontstage", "auto.k_cc0f333429")}</Button>
          ) : null}
          <Button disabled={resetDisabled} onClick={reset}>
            {i18nText("frontstage", "auto.k_3d81345303")}</Button>
          <Button
            type="primary"
            disabled={saveDisabled}
            loading={saving}
            onClick={handleSave}
          >
            {i18nText("frontstage", "auto.k_fadf24dbc5")}</Button>
        </Space>
      }
    >
      <Space direction="vertical" size={12} style={{ width: '100%' }}>
        <Space direction="vertical" size={2} style={{ width: '100%' }}>
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            Block
          </Typography.Text>
          <Typography.Text strong>
            {block?.id ?? i18nText("frontstage", "auto.k_8080e5501a")}
          </Typography.Text>
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            codeRef：{selectedCodeRef ?? i18nText("frontstage", "auto.k_2fe9b75856")}
          </Typography.Text>
        </Space>

        {emptyDescription ? (
          <Alert message={emptyDescription} type="info" showIcon />
        ) : null}

        {error ? (
          <Alert
            message={i18nText("frontstage", "auto.k_7627f34231")}
            description={error.message}
            type="error"
            showIcon
          />
        ) : null}

        <Space direction="vertical" size={6} style={{ width: '100%' }}>
          <Space size={8}>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              {i18nText("frontstage", "auto.k_62e951a692")}</Typography.Text>
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
