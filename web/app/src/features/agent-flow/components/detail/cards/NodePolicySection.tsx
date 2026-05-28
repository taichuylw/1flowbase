import type { FlowNodeDocument } from '@1flowbase/flow-schema';
import { Select, Switch, Typography } from 'antd';
import type { SchemaAdapter } from '../../../../../shared/schema-ui/registry/create-renderer-registry';

import { useInspectorInteractions } from '../../../hooks/interactions/use-inspector-interactions';
import { useAgentFlowEditorStore } from '../../../store/editor/provider';
import {
  selectSelectedNodeId,
  selectWorkingDocument
} from '../../../store/editor/selectors';
import { i18nText } from '../../../../../shared/i18n/text';

export function NodePolicySection({
  adapter
}: {
  adapter?: SchemaAdapter;
} = {}) {
  const document = useAgentFlowEditorStore(selectWorkingDocument);
  const selectedNodeId = useAgentFlowEditorStore(selectSelectedNodeId);
  const { updateField } = useInspectorInteractions();
  const selectedNode: FlowNodeDocument | null =
    (adapter?.getDerived('node') as FlowNodeDocument | null | undefined) ??
    (selectedNodeId
      ? document.graph.nodes.find((node) => node.id === selectedNodeId) ?? null
      : null);

  if (!selectedNode || !selectedNodeId) {
    return null;
  }

  const errorPolicyOptions = [
    {
      value: 'none',
      label: i18nText("agentFlow", "auto.k_72077749f7"),
      description: i18nText("agentFlow", "auto.k_544a35b974")
    },
    {
      value: 'default_value',
      label: i18nText("agentFlow", "auto.k_d93919c37b"),
      description: i18nText("agentFlow", "auto.k_9be0a808e3")
    },
    {
      value: 'error_branch',
      label: i18nText("agentFlow", "auto.k_92879700a0"),
      description: i18nText("agentFlow", "auto.k_938451d34e")
    }
  ] satisfies Array<{
    value: string;
    label: string;
    description: string;
  }>;

  return (
    <div className="agent-flow-node-detail__policies">
      <div className="agent-flow-node-detail__policy-row" data-testid="node-policy-row">
        <Typography.Text className="agent-flow-node-detail__policy-label">
          {i18nText("agentFlow", "auto.k_794ff5f046")}</Typography.Text>
        <Switch
          aria-label={i18nText("agentFlow", "auto.k_794ff5f046")}
          checked={Boolean(selectedNode.config.retry_enabled)}
          className="agent-flow-node-detail__policy-control"
          onChange={(checked) => {
            if (adapter) {
              adapter.setValue('config.retry_enabled', checked);
              return;
            }

            updateField('config.retry_enabled', checked);
          }}
        />
      </div>
      <div
        className="agent-flow-node-detail__policy-row agent-flow-node-detail__policy-row--select"
        data-testid="node-policy-row"
      >
        <Typography.Text className="agent-flow-node-detail__policy-label">
          {i18nText("agentFlow", "auto.k_066247572d")}</Typography.Text>
        <div
          className="agent-flow-node-detail__policy-select-shell agent-flow-node-detail__policy-select-shell--compact"
          data-testid="node-policy-error"
        >
          <Select
            aria-label={i18nText("agentFlow", "auto.k_066247572d")}
            className="agent-flow-node-detail__policy-control agent-flow-node-detail__policy-select"
            options={errorPolicyOptions}
            optionRender={(option) => {
              const policy = option.data as (typeof errorPolicyOptions)[number];

              return (
                <div className="agent-flow-node-detail__policy-option">
                  <div className="agent-flow-node-detail__policy-option-title">
                    {policy.label}
                  </div>
                  <div className="agent-flow-node-detail__policy-option-description">
                    {policy.description}
                  </div>
                </div>
              );
            }}
            classNames={{
              popup: {
                root: 'agent-flow-node-detail__policy-dropdown'
              }
            }}
            popupMatchSelectWidth={false}
            value={(selectedNode.config.error_policy as string | undefined) ?? 'none'}
            onChange={(value) => {
              if (adapter) {
                adapter.setValue('config.error_policy', value);
                return;
              }

              updateField('config.error_policy', value);
            }}
          />
        </div>
      </div>
    </div>
  );
}
