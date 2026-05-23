import * as AntIcons from '@ant-design/icons';
import {
  Alert,
  Button,
  Divider,
  Drawer,
  Empty,
  Form,
  Input,
  Modal,
  Popover,
  Typography
} from 'antd';
import type { FC } from 'react';
import type { ElementType } from 'react';
import { useCallback, useEffect, useMemo, useState } from 'react';

import { SectionPageLayout } from '../../../shared/ui/section-page-layout/SectionPageLayout';
import { useAuthStore } from '../../../state/auth-store';
import { useFrontstageDesignModeStore } from '../../../state/frontstage-design-mode-store';
import { saveFrontstageBlockCode } from '../api/block-code';
import type { FrontstagePageContent } from '../api/page-content';
import { AddBlockCatalogPickerDrawer } from '../components/AddBlockCatalogPickerDrawer';
import { BlockCodeEditorDrawer } from '../components/BlockCodeEditorDrawer';
import { BlockConfigurationDrawer } from '../components/BlockConfigurationDrawer';
import { FrontStagePageTreeSidebar } from '../components/FrontStagePageTreeSidebar';
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
  findNodeById,
  getDeleteConfirmMessage,
  getPageDisplayTitle,
  normalizePageTree,
  removeNodeFromTree,
  resolveSelectedPageId
} from '../lib/page-tree';
import type { FrontStageTreeNode } from '../lib/page-tree';
import type { RestrictedBlockLoaderLimits } from '../lib/restricted-block-loader';
import './frontstage-page.css';

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
  onUpdatePageNodeMetadata?: (
    nodeId: string,
    input: UpdatePageTreeNodeMetadataInput
  ) => Promise<PageTreeMutationResult | void>;
  onMovePageNode?: (
    nodeId: string,
    input: MovePageTreeNodeInput
  ) => Promise<PageTreeMutationResult | void>;
  onDeletePageNode?: (nodeId: string) => Promise<void>;
};

type CreatePageTreeNodeInput = {
  title: string | null;
  icon?: string | null;
  tooltip?: string | null;
  parentId: string | null;
  rank: string;
};

type RenamePageTreeNodeInput = {
  title: string | null;
  icon?: string | null;
  tooltip?: string | null;
};

type UpdatePageTreeNodeMetadataInput = {
  icon?: string | null;
  tooltip?: string | null;
  isHidden?: boolean;
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

type PageTreeFormValues = {
  title?: string;
  icon?: string;
  tooltip?: string;
};

type PageTreeFormDialog =
  | {
      kind: 'create';
      nodeKind: 'group' | 'page';
      parentId: string | null;
      rank: string;
      title: string;
      initialTitle: string;
      initialIcon: string;
      initialTooltip: string;
    }
  | {
      kind: 'rename';
      nodeId: string;
      title: string;
      initialTitle: string;
      initialIcon: string;
      initialTooltip: string;
    }
  | {
      kind: 'tooltip';
      nodeId: string;
      title: string;
      initialTooltip: string;
    };

type AntIconComponent = ElementType<{ className?: string }>;

const antIconComponents = AntIcons as Record<string, unknown>;
const pageTreeIconEntries = Object.entries(antIconComponents)
  .filter(
    (entry): entry is [string, AntIconComponent] =>
      /(?:Outlined|Filled|TwoTone)$/.test(entry[0]) &&
      (typeof entry[1] === 'function' ||
        (typeof entry[1] === 'object' && entry[1] !== null))
  )
  .sort(([left], [right]) => left.localeCompare(right));
const pageTreeIconMap = Object.fromEntries(pageTreeIconEntries);
const CloseIcon =
  (pageTreeIconMap.CloseOutlined as AntIconComponent | undefined) ??
  (() => null);
const PlusIcon =
  (pageTreeIconMap.PlusOutlined as AntIconComponent | undefined) ??
  (() => null);

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

function renderPageTreeIconPicker(
  selectedIcon: string | undefined,
  onChange: (icon: string | undefined) => void,
  iconPickerOpen: boolean,
  onIconPickerOpenChange: (open: boolean) => void
) {
  const SelectedIcon = selectedIcon
    ? (pageTreeIconMap[selectedIcon] as AntIconComponent | undefined)
    : undefined;
  const DisplayIcon = SelectedIcon ?? PlusIcon;
  const picker = (
    <div className="frontstage-page-tree-form__icon-popover">
      <div className="frontstage-page-tree-form__icon-grid">
        {pageTreeIconEntries.map(([iconName, Icon]) => (
          <button
            key={iconName}
            aria-label={iconName}
            className={[
              'frontstage-page-tree-form__icon-button',
              selectedIcon === iconName
                ? 'frontstage-page-tree-form__icon-button--selected'
                : null
            ]
              .filter(Boolean)
              .join(' ')}
            type="button"
            onClick={() => {
              onChange(iconName);
              onIconPickerOpenChange(false);
            }}
          >
            <Icon />
          </button>
        ))}
      </div>
    </div>
  );

  return (
    <div className="frontstage-page-tree-form__icon-field">
      <Popover
        arrow={false}
        content={picker}
        open={iconPickerOpen}
        placement="bottomLeft"
        trigger="click"
        onOpenChange={onIconPickerOpenChange}
      >
        <button
          aria-label="选择图标"
          className={[
            'frontstage-page-tree-form__icon-select-button',
            selectedIcon
              ? 'frontstage-page-tree-form__icon-select-button--with-clear'
              : null
          ]
            .filter(Boolean)
            .join(' ')}
          type="button"
        >
          <DisplayIcon />
        </button>
      </Popover>
      {selectedIcon ? (
        <button
          aria-label="清除图标"
          className="frontstage-page-tree-form__icon-clear-button"
          type="button"
          onClick={() => onChange(undefined)}
        >
          <CloseIcon />
        </button>
      ) : null}
    </div>
  );
}

function PageTreeIconPickerField({
  value,
  onChange,
  iconPickerOpen,
  onIconPickerOpenChange
}: {
  value?: string;
  onChange?: (icon: string | undefined) => void;
  iconPickerOpen: boolean;
  onIconPickerOpenChange: (open: boolean) => void;
}) {
  return renderPageTreeIconPicker(
    value,
    (icon) => onChange?.(icon),
    iconPickerOpen,
    onIconPickerOpenChange
  );
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

function isNodeDescendantOf(
  nodes: FrontStageTreeNode[],
  ancestorNodeId: string,
  targetNodeId: string
): boolean {
  const ancestorNode = findNodeById(nodes, ancestorNodeId);
  if (!ancestorNode?.children) {
    return false;
  }

  return Boolean(findNodeById(ancestorNode.children, targetNodeId));
}

function updatePageTreeNode(
  nodes: FrontStageTreeNode[],
  targetNodeId: string,
  patch: Pick<FrontStageTreeNode, 'title' | 'icon' | 'tooltip'>
): FrontStageTreeNode[] {
  return nodes.map((node) => {
    if (node.id === targetNodeId) {
      return {
        ...node,
        ...patch
      };
    }

    return {
      ...node,
      children: node.children
        ? updatePageTreeNode(node.children, targetNodeId, patch)
        : node.children
    };
  });
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
  onUpdatePageNodeMetadata,
  onMovePageNode,
  onDeletePageNode
}) => {
  const [pageTreeForm] = Form.useForm<PageTreeFormValues>();
  const csrfToken = useAuthStore((state) => state.csrfToken);
  const sessionStatus = useAuthStore((state) => state.sessionStatus);
  const actor = useAuthStore((state) => state.actor);
  const me = useAuthStore((state) => state.me);
  const isDesignMode = useFrontstageDesignModeStore(
    (state) => state.isDesignMode
  );
  const setDesignMode = useFrontstageDesignModeStore(
    (state) => state.setDesignMode
  );
  const [operationStatus, setOperationStatus] =
    useState<PageTreeOperationStatus>('idle');
  const [pageTreeFormDialog, setPageTreeFormDialog] =
    useState<PageTreeFormDialog | null>(null);
  const [isPageTreeIconPickerOpen, setIsPageTreeIconPickerOpen] =
    useState(false);
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
  const hasResolvedDesignModePermission = sessionStatus !== 'unknown';
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
    if (
      hasResolvedDesignModePermission &&
      !canEnterDesignMode &&
      isDesignMode
    ) {
      setDesignMode(false);
    }
  }, [
    canEnterDesignMode,
    hasResolvedDesignModePermission,
    isDesignMode,
    setDesignMode
  ]);

  useEffect(() => {
    if (!canEnterDesignMode || !isDesignMode) {
      setSelectedBlockId(null);
      setIsBlockCodeEditorOpen(false);
      setIsBlockConfigurationOpen(false);
      setIsJsBlockTrialPanelOpen(false);
      setIsAddBlockPickerOpen(false);
    }
  }, [canEnterDesignMode, isDesignMode]);

  useEffect(() => {
    setJsBlockTrialContextSnapshot(defaultJsBlockTrialContextSnapshot);
    setJsBlockTrialLimits(DEFAULT_JS_BLOCK_TRIAL_LIMITS);
  }, [defaultJsBlockTrialContextSnapshot]);

  useEffect(() => {
    if (!pageTreeFormDialog) {
      setIsPageTreeIconPickerOpen(false);
      pageTreeForm.resetFields();
      return;
    }

    if (pageTreeFormDialog.kind === 'tooltip') {
      pageTreeForm.setFieldsValue({
        tooltip: pageTreeFormDialog.initialTooltip
      });
      return;
    }

    pageTreeForm.setFieldsValue({
      title: pageTreeFormDialog.initialTitle,
      icon: pageTreeFormDialog.initialIcon,
      tooltip: pageTreeFormDialog.initialTooltip
    });
  }, [pageTreeForm, pageTreeFormDialog]);

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
        const idx = renderItems.findIndex((item) => item.blockId === blockId);
        if (idx <= 0 || !blockCompositionState || !activePageContent) return;
        const next = moveFrontstageBlock(
          blockCompositionState,
          blockId,
          idx - 1
        );
        void saveBlockComposition(activePageContent, next);
      },
      onMoveDown: (blockId: string) => {
        const idx = renderItems.findIndex((item) => item.blockId === blockId);
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
      <SectionPageLayout
        pageTitle="前台"
        navItems={[]}
        activeKey=""
        contentWidth="wide"
        heightMode="viewport"
        sidebarContent={
          <Typography.Text type="secondary" style={{ paddingInline: 16 }}>
            页面树加载中
          </Typography.Text>
        }
      >
        <section className="frontstage-page-workspace">
          <header className="frontstage-page-workspace__header">
            <Typography.Title
              className="frontstage-page-workspace__title"
              level={3}
            >
              页面树加载中…
            </Typography.Title>
          </header>
          <Divider style={{ margin: 0 }} />
          <div className="frontstage-page-workspace__body">
            <Empty
              description={
                <Typography.Text>正在加载页面树，请稍后...</Typography.Text>
              }
            />
          </div>
        </section>
      </SectionPageLayout>
    );
  }

  if (initialPageTree === undefined && hasPageTreeLoadError) {
    return (
      <SectionPageLayout
        pageTitle="前台"
        navItems={[]}
        activeKey=""
        contentWidth="wide"
        heightMode="viewport"
        sidebarContent={
          <Typography.Text type="secondary" style={{ paddingInline: 16 }}>
            页面树不可用
          </Typography.Text>
        }
      >
        <section className="frontstage-page-workspace">
          <header className="frontstage-page-workspace__header">
            <Typography.Title
              className="frontstage-page-workspace__title"
              level={3}
            >
              页面树加载失败
            </Typography.Title>
          </header>
          <Divider style={{ margin: 0 }} />
          <div className="frontstage-page-workspace__body">
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
        </section>
      </SectionPageLayout>
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

  const openCreateNodeDialog = (
    nodeKind: 'group' | 'page',
    parentId: string | null,
    rank: string
  ) => {
    setPageTreeFormDialog({
      kind: 'create',
      nodeKind,
      parentId,
      rank,
      initialTitle: '',
      initialIcon: '',
      initialTooltip: '',
      title: nodeKind === 'page' ? '新增页面' : '新增分组'
    });
  };

  const createPageTreeNode = async (
    nodeKind: 'group' | 'page',
    input: CreatePageTreeNodeInput
  ) => {
    if (nodeKind === 'page') {
      const createdNode = await onCreatePageNode?.(input);
      if (createdNode?.kind === 'page') {
        setSelectedPageId(createdNode.id);
        onNavigatePage?.(createdNode.id);
      }
      return;
    }

    await onCreateGroupNode?.(input);
  };

  const handleAddGroup = () => {
    openCreateNodeDialog('group', null, getNodeAppendRank(pageTree, null));
  };

  const handleAddPage = () => {
    openCreateNodeDialog('page', null, getNodeAppendRank(pageTree, null));
  };

  const handleAddPageInGroup = (groupId: string) => {
    openCreateNodeDialog('page', groupId, getNodeAppendRank(pageTree, groupId));
  };

  const handleAddNodeAtPosition = (
    kind: 'page' | 'group',
    targetNodeId: string,
    position: 'before' | 'after'
  ) => {
    const siblingContext = findSiblingContext(pageTree, targetNodeId);
    if (!siblingContext) {
      return;
    }

    const { parentId, siblings, index } = siblingContext;
    let rank = '';
    if (position === 'before') {
      rank = rankForMoveTarget(index, -1);
    } else {
      if (index === siblings.length - 1) {
        rank = getNodeAppendRank(pageTree, parentId);
      } else {
        rank = rankForMoveTarget(index, 1);
      }
    }

    openCreateNodeDialog(kind, parentId, rank);
  };

  const handleDeleteNode = (nodeId: string) => {
    const node = findNodeById(pageTree, nodeId);
    if (!node) {
      return;
    }

    Modal.confirm({
      title: '删除节点',
      content: getDeleteConfirmMessage(node),
      okText: '删除',
      okButtonProps: { danger: true },
      cancelText: '取消',
      onOk: async () => {
        await runPageTreeOperation(async () => {
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
      }
    });
  };

  const handleSubmitPageTreeForm = async () => {
    if (!pageTreeFormDialog) {
      return;
    }

    const values = await pageTreeForm.validateFields();
    const dialog = pageTreeFormDialog;

    if (dialog.kind === 'create') {
      const title = values.title ?? '';
      const input = {
        title,
        icon: values.icon ?? null,
        tooltip: values.tooltip ?? null,
        parentId: dialog.parentId,
        rank: dialog.rank
      };

      await runPageTreeOperation(async () => {
        await createPageTreeNode(dialog.nodeKind, input);
      });

      setPageTreeFormDialog(null);
      return;
    }

    if (dialog.kind === 'rename') {
      const title = values.title ?? '';
      const icon = values.icon ?? null;
      const tooltip = values.tooltip ?? null;

      await runPageTreeOperation(async () => {
        await onRenamePageNode?.(dialog.nodeId, {
          title,
          icon,
          tooltip
        });
        setPageTree((currentTree) =>
          updatePageTreeNode(currentTree, dialog.nodeId, {
            title,
            icon,
            tooltip
          })
        );
      });
      setPageTreeFormDialog(null);
      return;
    }

    await runPageTreeOperation(async () => {
      await onUpdatePageNodeMetadata?.(dialog.nodeId, {
        tooltip: values.tooltip ?? ''
      });
    });
    setPageTreeFormDialog(null);
  };

  const handleRenameNode = (node: FrontStageTreeNode) => {
    setPageTreeFormDialog({
      kind: 'rename',
      nodeId: node.id,
      initialTitle: node.title ?? '',
      initialIcon: node.icon ?? '',
      initialTooltip: node.tooltip ?? '',
      title: '编辑节点'
    });
  };

  const handleEditNodeTooltip = (
    nodeId: string,
    currentTooltip: string | null
  ) => {
    setPageTreeFormDialog({
      kind: 'tooltip',
      nodeId,
      initialTooltip: currentTooltip ?? '',
      title: '编辑描述'
    });
  };

  const handleUpdateNodeMetadata = (
    nodeId: string,
    input: UpdatePageTreeNodeMetadataInput
  ) => {
    void runPageTreeOperation(async () => {
      await onUpdatePageNodeMetadata?.(nodeId, input);
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

  const handleMoveNodeToPosition = (
    nodeId: string,
    targetNodeId: string,
    position: 'before' | 'after'
  ) => {
    if (
      nodeId === targetNodeId ||
      isNodeDescendantOf(pageTree, nodeId, targetNodeId)
    ) {
      return;
    }

    const targetSiblingContext = findSiblingContext(pageTree, targetNodeId);
    if (!targetSiblingContext) {
      return;
    }

    const { parentId, siblings, index } = targetSiblingContext;
    const rank =
      position === 'before'
        ? rankForMoveTarget(index, -1)
        : index === siblings.length - 1
          ? getNodeAppendRank(pageTree, parentId)
          : rankForMoveTarget(index, 1);

    void runPageTreeOperation(async () => {
      await onMovePageNode?.(nodeId, {
        parentId,
        rank
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

  const canEditPageTree = canEnterDesignMode && isDesignMode;
  const frontstageSidebar = (
    <FrontStagePageTreeSidebar
      pageTree={pageTree}
      selectedPageId={selectedPageId}
      canEdit={canEditPageTree}
      isOperationPending={isOperationPending}
      onAddGroup={handleAddGroup}
      onAddPage={handleAddPage}
      onAddPageInGroup={handleAddPageInGroup}
      onAddNodeAtPosition={handleAddNodeAtPosition}
      onRenameNode={handleRenameNode}
      onUpdateNodeMetadata={handleUpdateNodeMetadata}
      onEditNodeTooltip={handleEditNodeTooltip}
      onMoveNode={handleMoveNode}
      onMoveNodeToPosition={handleMoveNodeToPosition}
      onDeleteNode={handleDeleteNode}
      onSelectPage={handleSelectPage}
    />
  );

  return (
    <SectionPageLayout
      navItems={[]}
      activeKey=""
      contentWidth="wide"
      heightMode="viewport"
      sidebarContent={frontstageSidebar}
    >
      <>
        <section className="frontstage-page-workspace">
          <header className="frontstage-page-workspace__header">
            <Typography.Title
              className="frontstage-page-workspace__title"
              level={3}
            >
              {pageLabel}
            </Typography.Title>
          </header>
          <Divider style={{ margin: 0 }} />
          <div className="frontstage-page-workspace__body">
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
        </section>
        <Modal
          title={pageTreeFormDialog?.title}
          open={Boolean(pageTreeFormDialog)}
          okText="确定"
          cancelText="取消"
          confirmLoading={isOperationPending}
          destroyOnHidden
          forceRender
          onCancel={() => setPageTreeFormDialog(null)}
          onOk={() => pageTreeForm.submit()}
        >
          <Form<PageTreeFormValues>
            form={pageTreeForm}
            layout="vertical"
            preserve={false}
            onFinish={() => {
              void handleSubmitPageTreeForm();
            }}
          >
            {pageTreeFormDialog?.kind === 'tooltip' ? (
              <Form.Item label="描述" name="tooltip">
                <Input.TextArea autoSize={{ minRows: 3, maxRows: 6 }} />
              </Form.Item>
            ) : (
              <>
                <Form.Item
                  label="名称"
                  name="title"
                  rules={[
                    {
                      required: true,
                      whitespace: true,
                      message: '请输入名称'
                    }
                  ]}
                >
                  <Input autoFocus />
                </Form.Item>
                <Form.Item label="图标" name="icon">
                  <PageTreeIconPickerField
                    iconPickerOpen={isPageTreeIconPickerOpen}
                    onIconPickerOpenChange={setIsPageTreeIconPickerOpen}
                  />
                </Form.Item>
                <Form.Item label="描述" name="tooltip">
                  <Input.TextArea autoSize={{ minRows: 3, maxRows: 6 }} />
                </Form.Item>
              </>
            )}
          </Form>
        </Modal>
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
      </>
    </SectionPageLayout>
  );
};
