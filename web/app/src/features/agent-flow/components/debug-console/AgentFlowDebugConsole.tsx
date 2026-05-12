import { ReloadOutlined } from '@ant-design/icons';
import { Button } from 'antd';
import { useMemo, useState } from 'react';

import type {
  AgentFlowDebugMessage,
  AgentFlowRunContext
} from '../../api/runtime';
import type { AgentFlowDebugSessionStatus } from '../../hooks/runtime/useAgentFlowDebugSession';
import { AgentFlowDockPanel } from '../editor/AgentFlowDockPanel';
import { ConversationLogPanel } from './ConversationLogPanel';
import { DebugConversationPane } from './conversation/DebugConversationPane';

export function AgentFlowDebugConsole({
  messages,
  runContext,
  status,
  stopping,
  onChangeRunContextValue,
  onClearSession,
  onClose,
  onLoadArtifact,
  onStopRun,
  onSubmitPrompt
}: {
  messages: AgentFlowDebugMessage[];
  runContext: AgentFlowRunContext;
  status: AgentFlowDebugSessionStatus;
  stopping: boolean;
  onChangeRunContextValue: (
    nodeId: string,
    key: string,
    value: unknown
  ) => void;
  onClearSession: () => void;
  onClose: () => void;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  onStopRun: () => void;
  onSubmitPrompt: (prompt: string) => void;
}) {
  const [openLogMessageId, setOpenLogMessageId] = useState<string | null>(null);
  const openLogMessage = useMemo(
    () =>
      messages.find(
        (message) =>
          message.id === openLogMessageId && message.role === 'assistant'
      ) ?? null,
    [messages, openLogMessageId]
  );

  return (
    <>
      {openLogMessage ? (
        <ConversationLogPanel
          message={openLogMessage}
          onClose={() => setOpenLogMessageId(null)}
          onLoadArtifact={onLoadArtifact}
        />
      ) : null}
      <AgentFlowDockPanel
        actions={
          <Button
            aria-label="清空预览"
            disabled={messages.length === 0}
            icon={<ReloadOutlined />}
            size="small"
            type="text"
            onClick={() => {
              setOpenLogMessageId(null);
              onClearSession();
            }}
          />
        }
        bodyClassName="agent-flow-editor__debug-console-body"
        className="agent-flow-editor__debug-console"
        closeLabel="关闭预览"
        title="预览"
        onClose={onClose}
      >
        <DebugConversationPane
          messages={messages}
          runContext={runContext}
          status={status}
          stopping={stopping}
          onLoadArtifact={onLoadArtifact}
          onOpenMessageLog={(message) => setOpenLogMessageId(message.id)}
          onChangeQuery={(value) => {
            const queryField =
              runContext.fields.find((field) => field.key === 'query') ?? null;

            if (!queryField) {
              return;
            }

            onChangeRunContextValue(queryField.nodeId, queryField.key, value);
          }}
          onStopRun={onStopRun}
          onSubmitPrompt={onSubmitPrompt}
        />
      </AgentFlowDockPanel>
    </>
  );
}
