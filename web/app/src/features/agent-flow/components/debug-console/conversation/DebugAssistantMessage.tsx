import { CopyOutlined, DownOutlined, RightOutlined } from '@ant-design/icons';
import { App, Button, Space, Tooltip } from 'antd';
import { useEffect, useState } from 'react';

import type { AgentFlowDebugMessage } from '../../../api/runtime';
import { parseAssistantContent } from '../../../lib/debug-console/assistant-content';
import { copyTextToClipboard } from '../../../../../shared/ui/clipboard/copy-text';
import { DebugMarkdownContent } from './DebugMarkdownContent';
import { DebugWorkflowProcess } from './DebugWorkflowProcess';
import './debug-message.css';

function fallbackContent(message: AgentFlowDebugMessage) {
  if (message.status === 'running') {
    return '运行中...';
  }

  if (message.status === 'waiting_human') {
    return '等待人工介入。';
  }

  if (message.status === 'waiting_callback') {
    return '等待外部回调。';
  }

  if (message.status === 'cancelled') {
    return '已停止运行。';
  }

  if (message.status === 'failed') {
    return '调试运行失败。';
  }

  return '暂无输出。';
}

const TYPEWRITER_INTERVAL_MS = 24;
const TYPEWRITER_CHARS_PER_TICK = 12;

function useProgressiveText(target: string, enabled: boolean) {
  const [visibleText, setVisibleText] = useState(target);

  useEffect(() => {
    if (!enabled) {
      setVisibleText(target);
      return;
    }

    setVisibleText((currentText) => {
      if (!target) {
        return '';
      }

      if (!target.startsWith(currentText)) {
        return target;
      }

      return currentText;
    });
  }, [enabled, target]);

  useEffect(() => {
    if (!enabled) {
      return undefined;
    }

    if (visibleText.length >= target.length) {
      return undefined;
    }

    const timer = window.setTimeout(() => {
      setVisibleText((currentText) =>
        target.slice(
          0,
          Math.min(
            target.length,
            currentText.length + TYPEWRITER_CHARS_PER_TICK
          )
        )
      );
    }, TYPEWRITER_INTERVAL_MS);

    return () => window.clearTimeout(timer);
  }, [enabled, target, visibleText]);

  return visibleText;
}

export function DebugAssistantMessage({
  message
}: {
  message: AgentFlowDebugMessage;
}) {
  const { message: messageApi } = App.useApp();
  const [isReasoningExpanded, setIsReasoningExpanded] = useState(true);
  const visibleContent = useProgressiveText(
    message.content,
    message.status !== 'running'
  );
  const parsedContent = parseAssistantContent(visibleContent);
  const parsedFullContent = parseAssistantContent(message.content);
  const hasReasoning = Boolean(parsedContent.reasoningText.trim());
  const hasAnswer = Boolean(parsedContent.answerText.trim());

  async function handleCopyOutput() {
    if (!parsedFullContent.answerText) {
      return;
    }

    try {
      await copyTextToClipboard(parsedFullContent.answerText);
      messageApi.success('已复制');
    } catch {
      messageApi.error('复制失败');
    }
  }

  return (
    <article className="agent-flow-editor__debug-message agent-flow-editor__debug-message--assistant">
      <div className="agent-flow-editor__debug-message-main">
        <DebugWorkflowProcess items={message.traceSummary} />
        {hasReasoning ? (
          <section
            aria-label="思考"
            className="agent-flow-editor__debug-reasoning"
          >
            <button
              aria-expanded={isReasoningExpanded}
              className="agent-flow-editor__debug-reasoning-toggle"
              type="button"
              onClick={() => setIsReasoningExpanded((current) => !current)}
            >
              {isReasoningExpanded ? <DownOutlined /> : <RightOutlined />}
              <span className="agent-flow-editor__debug-reasoning-title">
                思考
              </span>
            </button>
            {isReasoningExpanded ? (
              <DebugMarkdownContent
                className="agent-flow-editor__debug-reasoning-content"
                content={parsedContent.reasoningText}
              />
            ) : null}
          </section>
        ) : null}
        {hasAnswer || !hasReasoning ? (
          <DebugMarkdownContent
            className="agent-flow-editor__debug-message-content"
            content={
              hasAnswer ? parsedContent.answerText : fallbackContent(message)
            }
          />
        ) : null}
      </div>
      <div
        aria-label="输出动作"
        className="agent-flow-editor__debug-message-action-row"
        role="group"
      >
        <Space
          className="agent-flow-editor__debug-message-actions"
          size={8}
          wrap
        >
          <Tooltip title="复制输出">
            <Button
              aria-label="复制输出"
              disabled={!parsedFullContent.answerText}
              icon={<CopyOutlined />}
              size="small"
              onClick={() => {
                void handleCopyOutput();
              }}
            />
          </Tooltip>
        </Space>
      </div>
    </article>
  );
}
