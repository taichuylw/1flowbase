import type { CSSProperties, ReactNode } from 'react';

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

const CODE_NODE_ICON_STYLE: CSSProperties = {
  display: 'inline-block',
  width: '1em',
  height: '1em',
  backgroundColor: 'currentColor',
  maskImage: 'url("/icons/code-node.svg")',
  maskPosition: 'center',
  maskRepeat: 'no-repeat',
  maskSize: 'contain',
  WebkitMaskImage: 'url("/icons/code-node.svg")',
  WebkitMaskPosition: 'center',
  WebkitMaskRepeat: 'no-repeat',
  WebkitMaskSize: 'contain'
};

function CodeNodeIcon() {
  return (
    <span
      aria-label="code"
      role="img"
      style={CODE_NODE_ICON_STYLE}
    />
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
