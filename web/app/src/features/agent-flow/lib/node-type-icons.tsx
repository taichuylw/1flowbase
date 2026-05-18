import type { ReactNode } from 'react';

import {
  ApiOutlined,
  BlockOutlined,
  DatabaseOutlined,
  EditOutlined,
  FileTextOutlined,
  MessageOutlined,
  PlaySquareOutlined,
  QuestionCircleOutlined,
  ReloadOutlined,
  SearchOutlined,
  SwapOutlined,
  SyncOutlined,
  ThunderboltOutlined,
  ToolOutlined
} from '@ant-design/icons';

function CodeNodeIcon() {
  return (
    <svg
      aria-label="code"
      fill="none"
      focusable="false"
      height="1em"
      role="img"
      viewBox="0 0 24 24"
      width="1em"
    >
      <path
        d="M9.25 7.75 5 12l4.25 4.25"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="2"
      />
      <path
        d="m14.75 7.75 4.25 4.25-4.25 4.25"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="2"
      />
      <path
        d="m13.25 5.75-2.5 12.5"
        stroke="currentColor"
        strokeLinecap="round"
        strokeWidth="2"
      />
    </svg>
  );
}

/** 节点类型图标统一源，画布节点和详情头必须使用同一套映射。 */
const NODE_TYPE_ICONS: Record<string, ReactNode> = {
  start: <PlaySquareOutlined />,
  answer: <MessageOutlined />,
  llm: <ThunderboltOutlined />,
  code: <CodeNodeIcon />,
  template_transform: <FileTextOutlined />,
  knowledge_retrieval: <SearchOutlined />,
  question_classifier: <QuestionCircleOutlined />,
  if_else: <SwapOutlined />,
  http_request: <ApiOutlined />,
  tool: <ToolOutlined />,
  data_model_list: <DatabaseOutlined />,
  data_model_get: <DatabaseOutlined />,
  data_model_create: <DatabaseOutlined />,
  data_model_update: <DatabaseOutlined />,
  data_model_delete: <DatabaseOutlined />,
  variable_assigner: <EditOutlined />,
  iteration: <SyncOutlined />,
  loop: <ReloadOutlined />,
  plugin_node: <BlockOutlined />
};

export function getAgentFlowNodeTypeIcon(nodeType: string) {
  return NODE_TYPE_ICONS[nodeType] ?? null;
}
