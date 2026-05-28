import {
  Descriptions,
  Drawer,
  Empty,
  Space,
  Tabs,
  Tag,
  Typography
} from 'antd';
import type { DescriptionsProps } from 'antd';
import type { FC, ReactNode } from 'react';

import type {
  FrontstageBlockBasicConfiguration,
  FrontstageBlockCodeConfiguration,
  FrontstageBlockConfigurationModel,
  FrontstageBlockContextConfiguration,
  FrontstageBlockDataConfiguration,
  FrontstageBlockLimitsConfiguration
} from '../lib/block-configuration';
import { i18nText } from '../../../shared/i18n/text';

export interface BlockConfigurationDrawerProps {
  open: boolean;
  onClose: () => void;
  model: FrontstageBlockConfigurationModel | null;
}

function asText(value: unknown, fallback = 'Not configured'): string {
  if (value === null || value === undefined) {
    return fallback;
  }

  if (typeof value === 'string') {
    const trimmed = value.trim();
    return trimmed.length > 0 ? trimmed : fallback;
  }

  if (
    typeof value === 'number' ||
    typeof value === 'boolean' ||
    typeof value === 'bigint'
  ) {
    return String(value);
  }

  return 'Configured';
}

function formatCount(count: number, singular: string): string {
  return `${count} ${singular}${count === 1 ? '' : 's'}`;
}

function formatPagination(value: unknown): string {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return 'Not configured';
  }

  const entries = Object.entries(value).filter(
    ([, entry]) => entry !== null && entry !== undefined
  );
  if (entries.length === 0) {
    return 'Not configured';
  }

  return entries.map(([key, entry]) => `${key} ${asText(entry)}`).join(', ');
}

function formatRuntime(runtime: FrontstageBlockCodeConfiguration['runtime']) {
  return (
    <Space size={6} wrap>
      <Tag>{runtime.kind}</Tag>
      <Typography.Text>{asText(runtime.entry)}</Typography.Text>
      <Typography.Text type="secondary">hint {runtime.hint}</Typography.Text>
    </Space>
  );
}

function renderTags(values: readonly string[], emptyText: string): ReactNode {
  if (values.length === 0) {
    return <Typography.Text type="secondary">{emptyText}</Typography.Text>;
  }

  return (
    <Space size={4} wrap>
      {values.map((value) => (
        <Tag key={value}>{value}</Tag>
      ))}
    </Space>
  );
}

function renderOperationState(enabled: boolean): ReactNode {
  return (
    <Tag color={enabled ? 'success' : 'default'}>
      {enabled ? 'Enabled' : 'Disabled'}
    </Tag>
  );
}

function renderDescriptions(items: DescriptionsProps['items']) {
  return <Descriptions column={1} size="small" items={items} />;
}

function getSectionModel<T>(
  model: FrontstageBlockConfigurationModel,
  sectionId: string
): T {
  const section = model.sections.find((item) => item.id === sectionId);
  return section?.model as T;
}

function BasicSection({ model }: { model: FrontstageBlockBasicConfiguration }) {
  return (
    <div data-testid="frontstage-block-configuration-section-basic">
      {renderDescriptions([
        { key: 'block-id', label: 'Block ID', children: model.blockId },
        {
          key: 'source-id',
          label: 'Source ID',
          children: asText(model.sourceId)
        },
        {
          key: 'title',
          label: 'Title',
          children: model.title.value ?? model.title.placeholder
        },
        {
          key: 'description',
          label: 'Description',
          children: model.description.value ?? model.description.placeholder
        },
        { key: 'code-ref', label: 'Code Ref', children: model.codeRef },
        {
          key: 'source-code-ref',
          label: 'Source Code Ref',
          children: asText(model.sourceCodeRef)
        },
        { key: 'width', label: 'Width', children: asText(model.width) },
        { key: 'height', label: 'Height', children: asText(model.height) },
        { key: 'order', label: 'Order', children: asText(model.order) }
      ])}
    </div>
  );
}

function DataSection({ model }: { model: FrontstageBlockDataConfiguration }) {
  return (
    <div data-testid="frontstage-block-configuration-section-data">
      {renderDescriptions([
        { key: 'model', label: 'Model', children: asText(model.model) },
        {
          key: 'fields',
          label: 'Fields',
          children: formatCount(model.fields.length, 'field')
        },
        {
          key: 'query',
          label: 'Query',
          children: renderOperationState(model.operations.query.enabled)
        },
        {
          key: 'create',
          label: 'Create',
          children: renderOperationState(model.operations.create.enabled)
        },
        {
          key: 'update',
          label: 'Update',
          children: renderOperationState(model.operations.update.enabled)
        },
        {
          key: 'delete',
          label: 'Delete',
          children: renderOperationState(model.operations.delete.enabled)
        },
        { key: 'filter', label: 'Filter', children: asText(model.filter) },
        { key: 'sort', label: 'Sort', children: asText(model.sort) },
        {
          key: 'pagination',
          label: 'Pagination',
          children: formatPagination(model.pagination)
        }
      ])}
    </div>
  );
}

function CodeSection({ model }: { model: FrontstageBlockCodeConfiguration }) {
  return (
    <div data-testid="frontstage-block-configuration-section-code">
      {renderDescriptions([
        { key: 'code-ref', label: 'Code Ref', children: model.codeRef },
        {
          key: 'source-code-ref',
          label: 'Source Code Ref',
          children: asText(model.sourceCodeRef)
        },
        {
          key: 'runtime',
          label: 'Runtime',
          children: formatRuntime(model.runtime)
        },
        {
          key: 'catalog-id',
          label: 'Catalog ID',
          children: asText(model.contribution.catalogId)
        },
        {
          key: 'catalog-title',
          label: 'Catalog Title',
          children: asText(model.contribution.catalogTitle)
        },
        {
          key: 'contribution',
          label: 'Contribution',
          children: model.contribution.code
        },
        {
          key: 'plugin',
          label: 'Plugin',
          children: asText(model.contribution.pluginId)
        },
        {
          key: 'provider',
          label: 'Provider',
          children: asText(model.contribution.providerCode)
        },
        {
          key: 'template',
          label: 'Template',
          children: asText(model.template.id)
        }
      ])}
    </div>
  );
}

function ContextSection({
  model
}: {
  model: FrontstageBlockContextConfiguration;
}) {
  return (
    <div data-testid="frontstage-block-configuration-section-context">
      {renderDescriptions([
        {
          key: 'catalog',
          label: 'Catalog',
          children: model.catalog.available ? 'Matched' : 'Not matched'
        },
        {
          key: 'primitives',
          label: 'Primitives',
          children: renderTags(model.catalog.primitives, 'No primitives')
        },
        {
          key: 'input-schema',
          label: 'Input schema',
          children: asText(model.catalog.inputSchema.type)
        },
        {
          key: 'ctx-current-user',
          label: 'Current user',
          children: model.ctx.currentUser.path
        },
        { key: 'ctx-page', label: 'Page', children: model.ctx.page.path },
        { key: 'ctx-params', label: 'Params', children: model.ctx.params.path },
        { key: 'ctx-props', label: 'Props', children: model.ctx.props.path },
        { key: 'ctx-state', label: 'State', children: model.ctx.state.path },
        {
          key: 'ctx-data',
          label: 'Data',
          children: (
            <Space size={4} wrap>
              <Typography.Text>{model.ctx.data.path}</Typography.Text>
              {renderTags(model.ctx.data.models, 'No allowed data models')}
              {renderTags(
                model.ctx.data.operations,
                'No allowed data operations'
              )}
            </Space>
          )
        },
        {
          key: 'ctx-actions',
          label: 'Actions',
          children: (
            <Space size={4} wrap>
              <Typography.Text>{model.ctx.actions.path}</Typography.Text>
              {renderTags(model.ctx.actions.allowed, 'No allowed actions')}
            </Space>
          )
        },
        {
          key: 'ctx-events',
          label: 'Events',
          children: (
            <Space size={4} wrap>
              <Typography.Text>{model.ctx.events.path}</Typography.Text>
              {renderTags(model.ctx.events.allowed, 'No allowed events')}
            </Space>
          )
        }
      ])}
    </div>
  );
}

function LimitsSection({
  model
}: {
  model: FrontstageBlockLimitsConfiguration;
}) {
  return (
    <div data-testid="frontstage-block-configuration-section-limits">
      {renderDescriptions([
        {
          key: 'timeout',
          label: 'Timeout',
          children:
            model.timeoutMs === null
              ? 'Not configured'
              : `${model.timeoutMs} ms`
        },
        {
          key: 'render-depth',
          label: 'Max render depth',
          children: asText(model.maxRenderDepth)
        },
        {
          key: 'render-nodes',
          label: 'Max render nodes',
          children: asText(model.maxRenderNodes)
        },
        {
          key: 'event-chain-depth',
          label: 'Max event chain depth',
          children: asText(model.maxEventChainDepth)
        },
        {
          key: 'actions',
          label: 'Allowed actions',
          children: renderTags(model.allowedActions, 'No allowed actions')
        },
        {
          key: 'events',
          label: 'Allowed events',
          children: renderTags(model.allowedEvents, 'No allowed events')
        },
        {
          key: 'data-models',
          label: 'Allowed data models',
          children: renderTags(
            model.allowedDataModels,
            'No allowed data models'
          )
        },
        {
          key: 'data-operations',
          label: 'Allowed data operations',
          children: renderTags(
            model.allowedDataOperations,
            'No allowed data operations'
          )
        }
      ])}
    </div>
  );
}

export const BlockConfigurationDrawer: FC<BlockConfigurationDrawerProps> = ({
  open,
  onClose,
  model
}) => {
  const items = model
    ? [
        {
          key: 'basic',
          label: 'Basic',
          children: (
            <BasicSection
              model={getSectionModel<FrontstageBlockBasicConfiguration>(
                model,
                'basic'
              )}
            />
          )
        },
        {
          key: 'data',
          label: 'Data',
          children: (
            <DataSection
              model={getSectionModel<FrontstageBlockDataConfiguration>(
                model,
                'data'
              )}
            />
          )
        },
        {
          key: 'code',
          label: 'Code',
          children: (
            <CodeSection
              model={getSectionModel<FrontstageBlockCodeConfiguration>(
                model,
                'code'
              )}
            />
          )
        },
        {
          key: 'context',
          label: 'Context',
          children: (
            <ContextSection
              model={getSectionModel<FrontstageBlockContextConfiguration>(
                model,
                'context'
              )}
            />
          )
        },
        {
          key: 'limits',
          label: 'Limits',
          children: (
            <LimitsSection
              model={getSectionModel<FrontstageBlockLimitsConfiguration>(
                model,
                'limits'
              )}
            />
          )
        }
      ]
    : [];

  return (
    <Drawer
      open={open}
      onClose={onClose}
      placement="right"
      title={i18nText("frontstage", "auto.block_configuration")}
      width={640}
    >
      {model ? (
        <Space direction="vertical" size={12} style={{ width: '100%' }}>
          <Space direction="vertical" size={2}>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              Block
            </Typography.Text>
            <Typography.Text strong>{model.blockId}</Typography.Text>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              codeRef：{model.codeRef}
            </Typography.Text>
          </Space>
          <Tabs items={items} />
        </Space>
      ) : (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={i18nText("frontstage", "auto.select_block_for_configuration")}
        />
      )}
    </Drawer>
  );
};
