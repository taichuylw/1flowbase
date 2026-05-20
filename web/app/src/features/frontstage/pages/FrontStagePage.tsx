import {
  Alert,
  Button,
  Divider,
  Drawer,
  Empty,
  Flex,
  Grid,
  Layout,
  Space,
  Typography
} from 'antd';
import type { FC } from 'react';
import { useCallback, useEffect, useMemo, useState } from 'react';

import { useAuthStore } from '../../../state/auth-store';
import { saveFrontstageBlockCode } from '../api/block-code';
import type { FrontstagePageContent } from '../api/page-content';
import { AddBlockCatalogPickerDrawer } from '../components/AddBlockCatalogPickerDrawer';
import { BlockCodeEditorDrawer } from '../components/BlockCodeEditorDrawer';
import { BlockConfigurationDrawer } from '../components/BlockConfigurationDrawer';
import { JsBlockTrialPanel } from '../components/JsBlockTrialPanel';
import { PageCanvas } from '../components/PageCanvas';
import { useFrontstageBlockCatalog } from '../hooks/use-frontstage-block-catalog';
import { useFrontstageBlockCode } from '../hooks/use-frontstage-block-code';
import { useFrontstagePageCanvasRuntimeSessions } from '../hooks/use-frontstage-page-canvas-runtime-sessions';
import { useFrontstagePageCanvasRuntimeSources } from '../hooks/use-frontstage-page-canvas-runtime-sources';
import { useFrontstagePageContentSave } from '../hooks/use-frontstage-page-content-save';
import type { NormalizedFrontstageBlockCatalogEntry } from '../lib/block-catalog';
import {
  appendFrontstageBlock,
  createFrontstageBlockCompositionState,
  moveFrontstageBlock,
  removeFrontstageBlock,
  type FrontstageBlockCompositionInput,
  type FrontstageBlockCompositionState
} from '../lib/block-composition';
import {
  createFrontstageBuiltInJsBlockTemplateCode,
  type FrontstageBuiltInJsBlockTemplateId
} from '../lib/block-templates';
import { createFrontstageBlockConfigurationModel } from '../lib/block-configuration';
import { createFrontstageJsBlockDataEffectHandler } from '../lib/js-block-data-effect-handler';
import {
  createFrontstagePageDocument,
  createFrontstagePageDocumentSaveInput,
  type FrontstageBlockInstance
} from '../lib/page-document';
import { createFrontstagePageRenderPlan } from '../lib/page-canvas/render-plan';
import { createFrontstagePageCanvasRuntimeRunPlanState } from '../lib/page-canvas/runtime-run-plan';
import {
  canMoveNode,
  findNodeById,
  getDeleteConfirmMessage,
  getNextGroupTitleIndex,
  getNextPageTitleIndex,
  getPageDisplayTitle,
  normalizePageTree,
  removeNodeFromTree,
  resolveSelectedPageId
} from '../lib/page-tree';
import type { FrontStageTreeNode } from '../lib/page-tree';
import type { RestrictedBlockLoaderLimits } from '../lib/restricted-block-loader';

const DESIGN_MODE_PERMISSION = 'frontstage.page.design';
const DEFAULT_JS_BLOCK_TRIAL_LIMITS: RestrictedBlockLoaderLimits = {
  timeoutMs: 1000,
  maxRenderDepth: 8,
  maxRenderNodes: 250,
  maxEventChainDepth: 4,
  allowedActions: [],
  allowedEvents: [],
  allowedDataModels: [],
  allowedDataOperations: []
};

type FrontStagePageProps = {
  workspaceId: string;
  pageId?: string;
  onNavigatePage?: (pageId?: string) => void;
  initialPageTree?: FrontStageTreeNode[];
  isPageTreeLoading?: boolean;
  hasPageTreeLoadError?: boolean;
  onRetryLoadPageTree?: () => void;
  pageContent?: FrontstagePageContent;
  isPageContentLoading?: boolean;
  hasPageContentLoadError?: boolean;
  onRetryLoadPageContent?: () => void;
  isPageTreeMutating?: boolean;
  pageTreeMutationError?: Error | null;
  onCreateGroupNode?: (
    input: CreatePageTreeNodeInput
  ) => Promise<PageTreeMutationResult | void>;
  onCreatePageNode?: (
    input: CreatePageTreeNodeInput
  ) => Promise<PageTreeMutationResult | void>;
  onRenamePageNode?: (
    nodeId: string,
    input: RenamePageTreeNodeInput
  ) => Promise<PageTreeMutationResult | void>;
  onMovePageNode?: (
    nodeId: string,
    input: MovePageTreeNodeInput
  ) => Promise<PageTreeMutationResult | void>;
  onDeletePageNode?: (nodeId: string) => Promise<void>;
};

type CreatePageTreeNodeInput = {
  title: string | null;
  parentId: string | null;
  rank: string;
};

type RenamePageTreeNodeInput = {
  title: string | null;
};

type MovePageTreeNodeInput = {
  parentId: string | null;
  rank: string;
};

type PageTreeMutationResult = {
  id: string;
  kind: 'group' | 'page';
};

type PageTreeOperationStatus = 'idle' | 'pending' | 'error';

function createCatalogBlockInput(
  entry: NormalizedFrontstageBlockCatalogEntry,
  blockIndex: number
): FrontstageBlockCompositionInput {
  const blockNumber = blockIndex + 1;
  const blockId = `frontstage-js-block-${blockNumber}`;

  return {
    id: blockId,
    codeRef: `${blockId}-code`,
    catalog: {
      providerCode: entry.providerCode,
      installationId: entry.installationId
    },
    contribution: {
      pluginId: entry.pluginId,
      pluginVersion: entry.pluginVersion,
      code: entry.contributionCode
    },
    props: {},
    layout: {
      order: blockIndex,
      region: 'main'
    },
    runtime: {
      kind: entry.runtimeKind,
      entry: entry.entry,
      hint: entry.runtimeKind
    }
  };
}

function toDisplayErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message.trim().length > 0) {
    return error.message;
  }

  return '页面内容保存失败，请稍后重试。';
}

function requireCsrfToken(csrfToken: string | null): string {
  if (!csrfToken) {
    throw new Error('missing csrf token');
  }

  return csrfToken;
}

function rankForAppendIndex(index: number): string {
  return String((index + 1) * 1000).padStart(6, '0');
}

function rankForMoveTarget(index: number, direction: -1 | 1): string {
  if (direction < 0) {
    return index === 0 ? '000000' : String(index * 1000 + 500).padStart(6, '0');
  }

  return String((index + 1) * 1000 + 500).padStart(6, '0');
}

function findSiblingContext(
  nodes: FrontStageTreeNode[],
  targetNodeId: string,
  parentId: string | null = null
): {
  parentId: string | null;
  siblings: FrontStageTreeNode[];
  index: number;
} | null {
  const index = nodes.findIndex((node) => node.id === targetNodeId);
  if (index >= 0) {
    return {
      parentId,
      siblings: nodes,
      index
    };
  }

  for (const node of nodes) {
    if (!node.children) {
      continue;
    }

    const childContext = findSiblingContext(
      node.children,
      targetNodeId,
      node.id
    );
    if (childContext) {
      return childContext;
    }
  }

  return null;
}

function getNodeAppendRank(
  nodes: FrontStageTreeNode[],
  parentId: string | null
): string {
  if (!parentId) {
    return rankForAppendIndex(nodes.length);
  }

  const parentNode = findNodeById(nodes, parentId);
  return rankForAppendIndex(parentNode?.children?.length ?? 0);
}

function findMatchingFrontstageBlockCatalogEntry(
  block: FrontstageBlockInstance | null | undefined,
  catalogItems: NormalizedFrontstageBlockCatalogEntry[]
): NormalizedFrontstageBlockCatalogEntry | null {
  if (!block) {
    return null;
  }

  return (
    catalogItems.find(
      (item) =>
        block.catalog.providerCode === item.providerCode &&
        block.catalog.installationId === item.installationId &&
        block.contribution.pluginId === item.pluginId &&
        block.contribution.pluginVersion === item.pluginVersion &&
        block.contribution.code === item.contributionCode
    ) ?? null
  );
}

export const FrontStagePage: FC<FrontStagePageProps> = ({
  workspaceId,
  pageId,
  onNavigatePage,
  initialPageTree,
  isPageTreeLoading,
  hasPageTreeLoadError,
  onRetryLoadPageTree,
  pageContent,
  isPageContentLoading,
  hasPageContentLoadError,
  onRetryLoadPageContent,
  isPageTreeMutating,
  pageTreeMutationError,
  onCreateGroupNode,
  onCreatePageNode,
  onRenamePageNode,
  onMovePageNode,
  onDeletePageNode
}) => {
  const csrfToken = useAuthStore((state) => state.csrfToken);
  const actor = useAuthStore((state) => state.actor);
  const me = useAuthStore((state) => state.me);
  const [isDesignMode, setIsDesignMode] = useState(false);
  const [operationStatus, setOperationStatus] =
    useState<PageTreeOperationStatus>('idle');
  const [selectedBlockId, setSelectedBlockId] = useState<string | null>(null);
  const [isBlockCodeEditorOpen, setIsBlockCodeEditorOpen] = useState(false);
  const [isBlockConfigurationOpen, setIsBlockConfigurationOpen] =
    useState(false);
  const [isJsBlockTrialPanelOpen, setIsJsBlockTrialPanelOpen] = useState(false);
  const [jsBlockTrialContextSnapshot, setJsBlockTrialContextSnapshot] =
    useState<Record<string, unknown>>({});
  const [jsBlockTrialLimits, setJsBlockTrialLimits] =
    useState<RestrictedBlockLoaderLimits>(DEFAULT_JS_BLOCK_TRIAL_LIMITS);
  const [savedPageContent, setSavedPageContent] =
    useState<FrontstagePageContent | null>(null);
  const [isBlockSavePending, setIsBlockSavePending] = useState(false);
  const [blockSaveError, setBlockSaveError] = useState<string | null>(null);
  const [isAddBlockPickerOpen, setIsAddBlockPickerOpen] = useState(false);
  const [pageTree, setPageTree] = useState<FrontStageTreeNode[]>(() =>
    normalizePageTree(initialPageTree ?? [])
  );
  const [selectedPageId, setSelectedPageId] = useState<string | null>(
    () =>
      resolveSelectedPageId({
        pageId,
        pageTree: normalizePageTree(initialPageTree ?? [])
      }).selectedPageId
  );
  const screens = Grid.useBreakpoint();
  const { Sider, Content } = Layout;
  const isCompactLayout = screens.md === false;
  const blockCatalog = useFrontstageBlockCatalog();
  const pageContentSave = useFrontstagePageContentSave({
    workspaceId,
    pageId: selectedPageId
  });
  const jsBlockDataEffectHandler = useMemo(
    () => createFrontstageJsBlockDataEffectHandler({ csrfToken }),
    [csrfToken]
  );
  const displayedPageContent = savedPageContent ?? pageContent;
  const hasLoadedSelectedPageContent = Boolean(
    selectedPageId && displayedPageContent?.page.id === selectedPageId
  );
  const activePageContent = hasLoadedSelectedPageContent
    ? displayedPageContent
    : undefined;
  const displayedPageDocument = useMemo(
    () =>
      activePageContent
        ? createFrontstagePageDocument(activePageContent)
        : null,
    [activePageContent]
  );
  const activePageRenderPlan = useMemo(
    () =>
      displayedPageDocument
        ? createFrontstagePageRenderPlan(displayedPageDocument)
        : null,
    [displayedPageDocument]
  );
  const pageCanvasRuntimeSources = useFrontstagePageCanvasRuntimeSources({
    workspaceId,
    renderPlan: activePageRenderPlan
  });
  const pageCanvasRuntimeRunPlanState = useMemo(() => {
    const sourceState = pageCanvasRuntimeSources.sourceState;
    if (!sourceState) {
      return null;
    }

    return createFrontstagePageCanvasRuntimeRunPlanState({
      sourceState,
      catalogEntries: blockCatalog.items,
      contextSnapshot: (source) => ({
        workspaceId,
        pageId: activePageContent?.page.id ?? sourceState.pageId,
        pageTitle: activePageContent?.page.title ?? null,
        blockId: source.blockId,
        codeRef: source.codeRef,
        props: source.block.props
      }),
      limits: jsBlockTrialLimits
    });
  }, [
    activePageContent?.page.id,
    activePageContent?.page.title,
    blockCatalog.items,
    jsBlockTrialLimits,
    pageCanvasRuntimeSources.sourceState,
    workspaceId
  ]);
  const pageCanvasRuntimeSessions = useFrontstagePageCanvasRuntimeSessions({
    runtimeRunPlanState: pageCanvasRuntimeRunPlanState,
    dataEffectHandler: jsBlockDataEffectHandler
  });
  const blockCompositionState = useMemo(
    () =>
      displayedPageDocument
        ? createFrontstageBlockCompositionState(
            displayedPageDocument,
            selectedBlockId
          )
        : null,
    [displayedPageDocument, selectedBlockId]
  );
  const isOperationPending =
    operationStatus === 'pending' || Boolean(isPageTreeMutating);
  const hasOperationError =
    operationStatus === 'error' || Boolean(pageTreeMutationError);
  const isPageContentSavePending =
    isBlockSavePending || pageContentSave.saving || pageContentSave.isPending;
  const pageContentSaveError =
    blockSaveError ??
    (pageContentSave.error
      ? toDisplayErrorMessage(pageContentSave.error)
      : null);
  const canAddBlock =
    Boolean(activePageContent) &&
    !isPageContentLoading &&
    !hasPageContentLoadError &&
    !isPageContentSavePending;
  const operationStatusText = isOperationPending
    ? '保存中'
    : hasOperationError
      ? '操作失败'
      : '页面树已同步';

  const canEnterDesignMode = useMemo(() => {
    return (
      actor?.effective_display_role === 'root' ||
      Boolean(me?.permissions.includes(DESIGN_MODE_PERMISSION))
    );
  }, [actor, me]);
  const selectedBlockIndex =
    blockCompositionState?.selectedBlockId === selectedBlockId
      ? blockCompositionState.document.blocks.findIndex(
          (block) => block.id === selectedBlockId
        )
      : -1;
  const selectedBlock =
    selectedBlockIndex >= 0
      ? blockCompositionState?.document.blocks[selectedBlockIndex]
      : null;
  const canShowSelectedBlockActions = Boolean(
    canEnterDesignMode &&
    isDesignMode &&
    activePageContent &&
    blockCompositionState &&
    selectedBlock
  );
  const canRunSelectedBlockAction =
    canShowSelectedBlockActions &&
    !isPageContentLoading &&
    !hasPageContentLoadError &&
    !isPageContentSavePending;
  const selectedBlockCode = useFrontstageBlockCode({
    workspaceId: canShowSelectedBlockActions ? workspaceId : null,
    pageId: canShowSelectedBlockActions ? selectedPageId : null,
    codeRef: canShowSelectedBlockActions ? selectedBlock?.codeRef : null
  });
  const matchingJsBlockCatalogEntry = useMemo(
    () =>
      findMatchingFrontstageBlockCatalogEntry(
        selectedBlock,
        blockCatalog.items
      ),
    [blockCatalog.items, selectedBlock]
  );
  const defaultJsBlockTrialContextSnapshot = useMemo(
    () => ({
      workspaceId,
      pageId: activePageContent?.page.id ?? selectedPageId,
      pageTitle: activePageContent?.page.title ?? null,
      blockId: selectedBlock?.id ?? null,
      blockCodeRef: selectedBlock?.codeRef ?? null,
      props: selectedBlock?.props ?? {}
    }),
    [activePageContent, selectedBlock, selectedPageId, workspaceId]
  );
  const selectedBlockConfigurationModel = useMemo(
    () =>
      selectedBlock
        ? createFrontstageBlockConfigurationModel({
            block: selectedBlock,
            catalogEntry: matchingJsBlockCatalogEntry,
            limits: jsBlockTrialLimits
          })
        : null,
    [jsBlockTrialLimits, matchingJsBlockCatalogEntry, selectedBlock]
  );
  useEffect(() => {

    const resolution = resolveSelectedPageId({
      currentSelectedPageId: selectedPageId,
      pageId,
      pageTree
    });

    if (selectedPageId !== resolution.selectedPageId) {
      setSelectedPageId(resolution.selectedPageId);
    }

    if (resolution.shouldNavigate) {
      onNavigatePage?.(resolution.navigationTarget);
    }
  }, [onNavigatePage, pageId, pageTree, selectedPageId]);

  useEffect(() => {
    if (!initialPageTree) {
      return;
    }

    setPageTree(normalizePageTree(initialPageTree));
    setOperationStatus('idle');
  }, [initialPageTree]);

  useEffect(() => {
    setSavedPageContent(null);
    setSelectedBlockId(null);
    setIsBlockCodeEditorOpen(false);
    setIsBlockConfigurationOpen(false);
    setIsJsBlockTrialPanelOpen(false);
    setIsAddBlockPickerOpen(false);
    setBlockSaveError(null);
  }, [selectedPageId]);

  useEffect(() => {
    setSavedPageContent(null);
    setSelectedBlockId((currentBlockId) => {
      if (!currentBlockId || !pageContent) {
        setIsBlockCodeEditorOpen(false);
        setIsBlockConfigurationOpen(false);
        setIsJsBlockTrialPanelOpen(false);
        setIsAddBlockPickerOpen(false);
        return null;
      }

      const document = createFrontstagePageDocument(pageContent);
      const hasCurrentBlock = document.blocks.some(
        (block) => block.id === currentBlockId
      );
      if (!hasCurrentBlock) {
        setIsBlockCodeEditorOpen(false);
        setIsBlockConfigurationOpen(false);
        setIsJsBlockTrialPanelOpen(false);
      }

      return hasCurrentBlock ? currentBlockId : null;
    });
  }, [pageContent]);

  useEffect(() => {
    if (!canShowSelectedBlockActions) {
      setIsBlockCodeEditorOpen(false);
      setIsBlockConfigurationOpen(false);
      setIsJsBlockTrialPanelOpen(false);
    }
  }, [canShowSelectedBlockActions]);

  useEffect(() => {
    if (!canEnterDesignMode || !isDesignMode) {
      setIsAddBlockPickerOpen(false);
    }
  }, [canEnterDesignMode, isDesignMode]);

  useEffect(() => {
    setJsBlockTrialContextSnapshot(defaultJsBlockTrialContextSnapshot);
    setJsBlockTrialLimits(DEFAULT_JS_BLOCK_TRIAL_LIMITS);
  }, [defaultJsBlockTrialContextSnapshot]);

  const selectedPageDisplayTitle = getPageDisplayTitle(
    pageTree,
    selectedPageId
  );
  const selectedPageLabel = selectedPageDisplayTitle
    ? selectedPageDisplayTitle
    : selectedPageId
      ? `页面 ${selectedPageId}`
      : null;
  const pageLabel = selectedPageLabel
    ? selectedPageLabel
    : '未选择 pageId（将使用默认首页）';
  const pageNodeTitle = selectedPageLabel
    ? `当前页面：${selectedPageLabel}`
    : '当前未选中页面';

  const saveBlockComposition = useCallback(
    async (
      sourceContent: FrontstagePageContent,
      compositionState: FrontstageBlockCompositionState
    ) => {
      setIsBlockSavePending(true);
      setBlockSaveError(null);
      pageContentSave.clearError();

      try {
        const input = createFrontstagePageDocumentSaveInput(
          sourceContent,
          compositionState.document
        );
        const nextContent = await pageContentSave.save(input);

        setSavedPageContent(nextContent);
        setSelectedBlockId(compositionState.selectedBlockId);
      } catch (error) {
        setBlockSaveError(toDisplayErrorMessage(error));
      } finally {
        setIsBlockSavePending(false);
      }
    },
    [pageContentSave]
  );

  const designActions = useMemo(() => {
    if (!canEnterDesignMode || !isDesignMode) {
      return undefined;
    }

    const renderItems = activePageRenderPlan?.items ?? [];

    return {
      onMoveUp: (blockId: string) => {
        const idx = renderItems.findIndex(
          (item) => item.blockId === blockId
        );
        if (idx <= 0 || !blockCompositionState || !activePageContent) return;
        const next = moveFrontstageBlock(
          blockCompositionState,
          blockId,
          idx - 1
        );
        void saveBlockComposition(activePageContent, next);
      },
      onMoveDown: (blockId: string) => {
        const idx = renderItems.findIndex(
          (item) => item.blockId === blockId
        );
        if (
          idx < 0 ||
          idx >= renderItems.length - 1 ||
          !blockCompositionState ||
          !activePageContent
        )
          return;
        const next = moveFrontstageBlock(
          blockCompositionState,
          blockId,
          idx + 1
        );
        void saveBlockComposition(activePageContent, next);
      },
      onConfigure: (blockId: string) => {
        setSelectedBlockId(blockId);
        setIsBlockConfigurationOpen(true);
      },
      onEditCode: (blockId: string) => {
        setSelectedBlockId(blockId);
        setIsBlockCodeEditorOpen(true);
      },
      onDelete: (blockId: string) => {
        if (!blockCompositionState || !activePageContent) return;
        const next = removeFrontstageBlock(blockCompositionState, blockId);
        void saveBlockComposition(activePageContent, next);
      }
    };
  }, [
    canEnterDesignMode,
    isDesignMode,
    activePageRenderPlan?.items,
    blockCompositionState,
    activePageContent,
    saveBlockComposition,
    setSelectedBlockId,
    setIsBlockConfigurationOpen,
    setIsBlockCodeEditorOpen
  ]);

  if (initialPageTree === undefined && isPageTreeLoading) {
    return (
      <div
        style={{
          width: '100%',
          padding: '24px 0',
          maxWidth: 1240,
          margin: '0 auto'
        }}
      >
        <Flex
          justify="space-between"
          align="center"
          style={{ marginBottom: 12 }}
        >
          <Space direction="vertical" size={0}>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              前台
            </Typography.Text>
            <Typography.Title level={4} style={{ margin: 0 }}>
              页面树加载中…
            </Typography.Title>
          </Space>
        </Flex>
        <Divider style={{ margin: '0 0 16px' }} />
        <Empty
          description={
            <Typography.Text>正在加载页面树，请稍后...</Typography.Text>
          }
        />
      </div>
    );
  }

  if (initialPageTree === undefined && hasPageTreeLoadError) {
    return (
      <div
        style={{
          width: '100%',
          padding: '24px 0',
          maxWidth: 1240,
          margin: '0 auto'
        }}
      >
        <Flex
          justify="space-between"
          align="center"
          style={{ marginBottom: 12 }}
        >
          <Space direction="vertical" size={0}>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              前台
            </Typography.Text>
            <Typography.Title level={4} style={{ margin: 0 }}>
              页面树加载失败
            </Typography.Title>
          </Space>
        </Flex>
        <Divider style={{ margin: '0 0 16px' }} />
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={
            <Typography.Text>
              页面树加载失败，请检查网络后重试。点击“重试”按钮重新发起加载。
            </Typography.Text>
          }
        >
          <Button type="primary" onClick={onRetryLoadPageTree}>
            重试
          </Button>
        </Empty>
      </div>
    );
  }

  const renderPageTreeErrorBanner = hasPageTreeLoadError ? (
    <Alert
      style={{ marginBottom: 12 }}
      message="页面树加载失败"
      description="页面树加载失败，当前页面树仍可查看；请点击“重试”恢复最新数据。"
      type="error"
      showIcon
      action={
        onRetryLoadPageTree ? (
          <Button size="small" onClick={() => onRetryLoadPageTree()}>
            重试
          </Button>
        ) : null
      }
    />
  ) : null;

  const runPageTreeOperation = async <T,>(
    operation: () => Promise<T | void>
  ): Promise<T | null> => {
    setOperationStatus('pending');

    try {
      const result = await operation();
      setOperationStatus('idle');
      return result ?? null;
    } catch {
      setOperationStatus('error');
      return null;
    }
  };

  const handleAddGroup = () => {
    const input = {
      title: `分组 ${getNextGroupTitleIndex(pageTree)}`,
      parentId: null,
      rank: getNodeAppendRank(pageTree, null)
    };

    void runPageTreeOperation(async () => {
      await onCreateGroupNode?.(input);
    });
  };

  const handleAddPage = () => {
    const input = {
      title: `页面 新建 ${getNextPageTitleIndex(pageTree)}`,
      parentId: null,
      rank: getNodeAppendRank(pageTree, null)
    };

    void runPageTreeOperation(async () => {
      const createdNode = await onCreatePageNode?.(input);
      if (createdNode?.kind === 'page') {
        setSelectedPageId(createdNode.id);
        onNavigatePage?.(createdNode.id);
      }
    });
  };

  const handleAddPageInGroup = (groupId: string) => {
    const input = {
      title: `页面 新建 ${getNextPageTitleIndex(pageTree)}`,
      parentId: groupId,
      rank: getNodeAppendRank(pageTree, groupId)
    };

    void runPageTreeOperation(async () => {
      const createdNode = await onCreatePageNode?.(input);
      if (createdNode?.kind === 'page') {
        setSelectedPageId(createdNode.id);
        onNavigatePage?.(createdNode.id);
      }
    });
  };

  const handleDeleteNode = (nodeId: string) => {
    const node = findNodeById(pageTree, nodeId);
    if (!node) {
      return;
    }

    const confirmed = window.confirm(getDeleteConfirmMessage(node));
    if (!confirmed) {
      return;
    }

    void runPageTreeOperation(async () => {
      await onDeletePageNode?.(nodeId);
      const next = removeNodeFromTree(pageTree, nodeId);
      const nextResolution = resolveSelectedPageId({
        pageId: selectedPageId ?? undefined,
        pageTree: next
      });
      const nextSelectedPageId = nextResolution.selectedPageId;

      setSelectedPageId(nextSelectedPageId);
      if (nextResolution.shouldNavigate) {
        onNavigatePage?.(nextResolution.navigationTarget);
      }
    });
  };

  const handleRenameNode = (nodeId: string, currentTitle: string | null) => {
    const nextTitle = window.prompt('重命名节点', currentTitle ?? '');
    if (nextTitle === null) {
      return;
    }

    void runPageTreeOperation(async () => {
      await onRenamePageNode?.(nodeId, { title: nextTitle });
    });
  };

  const handleMoveNode = (nodeId: string, direction: -1 | 1) => {
    const siblingContext = findSiblingContext(pageTree, nodeId);
    if (!siblingContext) {
      return;
    }

    const targetIndex = siblingContext.index + direction;
    if (targetIndex < 0 || targetIndex >= siblingContext.siblings.length) {
      return;
    }

    void runPageTreeOperation(async () => {
      await onMovePageNode?.(nodeId, {
        parentId: siblingContext.parentId,
        rank: rankForMoveTarget(targetIndex, direction)
      });
    });
  };

  const handleSelectPage = (nodeId: string) => {
    if (selectedPageId === nodeId) {
      return;
    }

    setSelectedPageId(nodeId);
    onNavigatePage?.(nodeId);
  };

  const handleAddBlock = () => {
    if (!canAddBlock) {
      return;
    }

    setBlockSaveError(null);
    pageContentSave.clearError();
    setIsAddBlockPickerOpen(true);
  };

  const handleSelectBlockCatalogEntry = async (
    entry: NormalizedFrontstageBlockCatalogEntry,
    templateId: FrontstageBuiltInJsBlockTemplateId
  ) => {
    const sourceContent = activePageContent;
    if (!canAddBlock || !sourceContent || !blockCompositionState) {
      return;
    }

    const nextBlockInput = createCatalogBlockInput(
      entry,
      blockCompositionState.document.blocks.length
    );
    const nextCompositionState = appendFrontstageBlock(
      blockCompositionState,
      nextBlockInput
    );

    setIsBlockSavePending(true);
    setBlockSaveError(null);
    pageContentSave.clearError();

    try {
      const input = createFrontstagePageDocumentSaveInput(
        sourceContent,
        nextCompositionState.document
      );
      const nextContent = await pageContentSave.save(input);
      const createdBlock =
        nextCompositionState.document.blocks.find(
          (block) => block.id === nextCompositionState.selectedBlockId
        ) ??
        nextCompositionState.document.blocks[
          nextCompositionState.document.blocks.length - 1
        ];
      if (!createdBlock) {
        throw new Error('created block is missing');
      }

      const codeRef = createdBlock.codeRef;
      const blockId = createdBlock.id;

      await saveFrontstageBlockCode(
        workspaceId,
        selectedPageId ?? sourceContent.page.id,
        {
          codeRef,
          code: createFrontstageBuiltInJsBlockTemplateCode({
            templateId,
            blockId,
            codeRef,
            contributionCode: entry.contributionCode
          })
        },
        requireCsrfToken(csrfToken)
      );

      setSavedPageContent(nextContent);
      setSelectedBlockId(nextCompositionState.selectedBlockId);
      setIsAddBlockPickerOpen(false);
    } catch (error) {
      setBlockSaveError(toDisplayErrorMessage(error));
    } finally {
      setIsBlockSavePending(false);
    }
  };

  const handleOpenJsBlockTrialPanel = () => {
    if (!canRunSelectedBlockAction) {
      return;
    }

    setJsBlockTrialContextSnapshot(defaultJsBlockTrialContextSnapshot);
    setJsBlockTrialLimits(DEFAULT_JS_BLOCK_TRIAL_LIMITS);
    setIsJsBlockTrialPanelOpen(true);
  };

  const renderTreeNode = (
    node: FrontStageTreeNode,
    level: number = 0,
    parentNodes: FrontStageTreeNode[] = pageTree
  ) => {
    const isPageNode = node.kind === 'page';
    const isSelected = selectedPageId === node.id;
    const canAddPageToGroup = node.kind === 'group' && level === 0;
    const { canMoveUp, canMoveDown } = canMoveNode(parentNodes, node.id);
    const rowStyle = {
      padding: '8px',
      borderRadius: 6,
      marginTop: 4,
      marginBottom: 4,
      marginLeft: `${level * 16}px`,
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      flexWrap: 'wrap',
      justifyContent: 'space-between',
      border: isSelected ? '1px solid #91caff' : '1px solid transparent',
      background: isSelected ? '#e6f7ff' : 'transparent',
      cursor: isPageNode ? 'pointer' : 'default'
    } as const;

    const childNodes = node.children ?? [];

    return (
      <li
        key={node.id}
        data-testid={`frontstage-tree-node-${node.kind}-${node.title || node.id}`}
        style={{ listStyle: 'none' }}
        onClick={() => {
          if (isPageNode) {
            handleSelectPage(node.id);
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
            handleSelectPage(node.id);
          }
        }}
      >
        <div style={rowStyle}>
          <div
            style={{
              minWidth: 0,
              flex: '1 1 96px',
              overflow: 'hidden',
              display: 'flex',
              flexDirection: 'column'
            }}
          >
            <Typography.Text style={{ fontSize: 12 }} ellipsis>
              {node.title
                ? node.title
                : node.kind === 'group'
                  ? '未命名分组'
                  : '未命名页面'}
            </Typography.Text>
            <Typography.Text
              type="secondary"
              style={{ fontSize: 12, display: 'block' }}
            >
              {node.kind === 'group' ? '分组节点' : '页面节点'}
            </Typography.Text>
          </div>
          {canEnterDesignMode && isDesignMode ? (
            <Space size={6} wrap>
              <Button
                size="small"
                disabled={isOperationPending}
                onClick={(event) => {
                  event.stopPropagation();
                  handleRenameNode(node.id, node.title);
                }}
              >
                重命名
              </Button>
              {canAddPageToGroup ? (
                <Button
                  size="small"
                  onClick={(event) => {
                    event.stopPropagation();
                    handleAddPageInGroup(node.id);
                  }}
                  disabled={isOperationPending}
                >
                  组内新增页面
                </Button>
              ) : null}
              <Button
                size="small"
                disabled={!canMoveUp || isOperationPending}
                onClick={(event) => {
                  event.stopPropagation();
                  handleMoveNode(node.id, -1);
                }}
              >
                上移
              </Button>
              <Button
                size="small"
                disabled={!canMoveDown || isOperationPending}
                onClick={(event) => {
                  event.stopPropagation();
                  handleMoveNode(node.id, 1);
                }}
              >
                下移
              </Button>
              <Button
                size="small"
                danger
                disabled={isOperationPending}
                onClick={(event) => {
                  event.stopPropagation();
                  handleDeleteNode(node.id);
                }}
              >
                删除
              </Button>
            </Space>
          ) : null}
        </div>
        {childNodes.length > 0 ? (
          <ul style={{ listStyle: 'none', margin: 0, paddingLeft: 0 }}>
            {childNodes.map((childNode) =>
              renderTreeNode(childNode, level + 1, childNodes)
            )}
          </ul>
        ) : null}
      </li>
    );
  };

  return (
    <div
      style={{
        width: '100%',
        minHeight: 'calc(100vh - 96px)',
        padding: isCompactLayout ? 12 : 18,
        maxWidth: 1480,
        margin: '0 auto',
        background:
          'linear-gradient(180deg, rgba(240, 253, 248, 0.95) 0%, rgba(246, 252, 249, 0.95) 100%)'
      }}
    >
      <Layout
        style={{
          background: 'transparent',
          gap: isCompactLayout ? 12 : 20,
          flexDirection: isCompactLayout ? 'column' : 'row'
        }}
      >
        <Sider
          width={isCompactLayout ? '100%' : 280}
          theme="light"
          style={{
            background: 'white',
            border: '1px solid #edf7f1',
            borderRadius: 8,
            boxShadow: '0 18px 45px rgba(15, 118, 110, 0.08)',
            padding: isCompactLayout ? 16 : 18,
            overflow: 'hidden',
            flex: isCompactLayout ? '0 0 auto' : undefined,
            maxWidth: '100%',
            minWidth: 0
          }}
        >
          <Flex justify="space-between" align="center">
            <Typography.Title level={5} style={{ margin: 0 }}>
              分组
            </Typography.Title>
            <Typography.Text type="secondary">⌃</Typography.Text>
          </Flex>
          <Divider style={{ margin: '16px 0' }} />
          <Typography.Text
            type="secondary"
            style={{ marginBottom: 10, display: 'block', fontSize: 12 }}
          >
            {pageNodeTitle}
          </Typography.Text>
          {canEnterDesignMode && isDesignMode ? (
            <Space size={8} wrap style={{ marginBottom: 14 }}>
              <Button
                size="small"
                onClick={handleAddGroup}
                disabled={isOperationPending}
              >
                新建分组
              </Button>
              <Button
                size="small"
                aria-label="新建页面"
                onClick={handleAddPage}
                disabled={isOperationPending}
                style={{
                  borderStyle: 'dashed',
                  borderColor: '#20d48a',
                  color: '#00a86b'
                }}
              >
                添加菜单项
              </Button>
            </Space>
          ) : null}
          {pageTree.length > 0 ? (
            <ul style={{ listStyle: 'none', margin: 0, padding: 0 }}>
              {pageTree.map((node) => renderTreeNode(node, 0, pageTree))}
            </ul>
          ) : (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              styles={{ image: { height: 48 } }}
              description={
                <Typography.Text type="secondary">
                  当前工作区页面树为空。请在设计态创建页面后将显示树结构。
                </Typography.Text>
              }
            />
          )}
        </Sider>
        <Content
          style={{
            background: 'white',
            border: '1px solid #edf7f1',
            borderRadius: 8,
            boxShadow: '0 18px 45px rgba(15, 118, 110, 0.08)',
            minHeight: 720,
            overflow: 'hidden',
            minWidth: 0,
            width: '100%'
          }}
        >
          <Flex
            justify="space-between"
            align="center"
            gap={16}
            wrap
            style={{
              padding: isCompactLayout ? '18px 16px 16px' : '24px 34px 20px'
            }}
          >
            <Typography.Title
              level={3}
              style={{ margin: 0, overflowWrap: 'anywhere' }}
            >
              {pageLabel}
            </Typography.Title>
            {canEnterDesignMode ? (
              <Button
                type={isDesignMode ? 'default' : 'primary'}
                style={
                  isDesignMode
                    ? undefined
                    : {
                        background: '#00c875',
                        borderColor: '#00c875',
                        boxShadow: '0 8px 20px rgba(0, 200, 117, 0.22)'
                      }
                }
                onClick={() => {
                  if (isDesignMode) {
                    setSelectedBlockId(null);
                    setIsBlockCodeEditorOpen(false);
                    setIsBlockConfigurationOpen(false);
                    setIsJsBlockTrialPanelOpen(false);
                  }

                  setIsDesignMode((current) => !current);
                }}
              >
                {isDesignMode ? '退出设计模式' : '进入设计模式'}
              </Button>
            ) : null}
          </Flex>
          <Divider style={{ margin: 0 }} />
          <div
            style={{
              padding: isCompactLayout ? '24px 14px 24px' : '76px 36px 28px'
            }}
          >
            {renderPageTreeErrorBanner}

            {canEnterDesignMode && isDesignMode && isPageContentSavePending ? (
              <Typography.Text
                type="secondary"
                style={{ marginBottom: 12, display: 'block' }}
              >
                区块保存中
              </Typography.Text>
            ) : null}
            {canEnterDesignMode && isDesignMode && pageContentSaveError ? (
              <Alert
                style={{ marginBottom: 12 }}
                message="区块保存失败"
                description={pageContentSaveError}
                type="error"
                showIcon
              />
            ) : null}
            {canEnterDesignMode && isDesignMode ? (
              <Typography.Text
                type={hasOperationError ? 'danger' : 'secondary'}
                style={{ marginBottom: 12, display: 'block' }}
              >
                {operationStatusText}
              </Typography.Text>
            ) : null}
            <PageCanvas
              content={
                selectedPageLabel && hasLoadedSelectedPageContent
                  ? displayedPageContent
                  : undefined
              }
              isLoading={Boolean(selectedPageLabel && isPageContentLoading)}
              hasError={Boolean(selectedPageLabel && hasPageContentLoadError)}
              selectedBlockId={
                canEnterDesignMode && isDesignMode ? selectedBlockId : null
              }
              onSelectBlock={
                canEnterDesignMode && isDesignMode
                  ? (blockId) => {
                      setSelectedBlockId((currentBlockId) =>
                        currentBlockId === blockId ? null : blockId
                      );
                    }
                  : undefined
              }
              onRetry={onRetryLoadPageContent}
              runtimeSourceState={pageCanvasRuntimeSources.sourceState}
              runtimeRunPlanState={pageCanvasRuntimeRunPlanState}
              runtimeSessionEntries={pageCanvasRuntimeSessions.entries}
              isDesignMode={canEnterDesignMode && isDesignMode}
              designActions={designActions}
              toolbarDisabled={isPageContentSavePending}
              showTitle={false}
            />
            {canEnterDesignMode && isDesignMode ? (
              <Button
                size="middle"
                aria-label="创建区块"
                onClick={handleAddBlock}
                disabled={!canAddBlock}
                style={{
                  marginTop: 20,
                  borderStyle: 'dashed',
                  borderColor: '#20d48a',
                  color: '#00a86b'
                }}
              >
                + 创建区块
              </Button>
            ) : null}
          </div>
          {/* Block actions moved to BlockHoverToolbar inside PageCanvas */}
          <Drawer
            title="JS 区块试运行"
            open={isJsBlockTrialPanelOpen}
            onClose={() => setIsJsBlockTrialPanelOpen(false)}
            width={600}
            destroyOnClose
          >
            {selectedBlock && (
              <JsBlockTrialPanel
                block={selectedBlock}
                catalogEntry={matchingJsBlockCatalogEntry}
                code={selectedBlockCode.draft}
                contextSnapshot={jsBlockTrialContextSnapshot}
                dataEffectHandler={jsBlockDataEffectHandler}
                limits={jsBlockTrialLimits}
                onCodeChange={selectedBlockCode.setDraft}
                onContextSnapshotChange={setJsBlockTrialContextSnapshot}
                onLimitsChange={setJsBlockTrialLimits}
              />
            )}
          </Drawer>
        </Content>
      </Layout>
      {canEnterDesignMode && isDesignMode ? (
        <AddBlockCatalogPickerDrawer
          open={isAddBlockPickerOpen}
          items={blockCatalog.items}
          loading={blockCatalog.loading}
          error={blockCatalog.error}
          saving={isPageContentSavePending}
          onSelect={(entry, templateId) => {
            void handleSelectBlockCatalogEntry(entry, templateId);
          }}
          onClose={() => setIsAddBlockPickerOpen(false)}
        />
      ) : null}
      <BlockCodeEditorDrawer
        open={isBlockCodeEditorOpen && canShowSelectedBlockActions}
        onClose={() => setIsBlockCodeEditorOpen(false)}
        onOpenTrialPanel={handleOpenJsBlockTrialPanel}
        workspaceId={workspaceId}
        pageId={selectedPageId}
        block={canShowSelectedBlockActions ? selectedBlock : null}
      />
      <BlockConfigurationDrawer
        open={isBlockConfigurationOpen && canShowSelectedBlockActions}
        onClose={() => setIsBlockConfigurationOpen(false)}
        model={
          canShowSelectedBlockActions ? selectedBlockConfigurationModel : null
        }
      />
    </div>
  );
};
