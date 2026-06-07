import { PlusOutlined } from '@ant-design/icons';

export function ConnectorAddIcon({ className }: { className?: string }) {
  return (
    <PlusOutlined
      aria-hidden="true"
      className={`agent-flow-connector-add-icon${className ? ` ${className}` : ''}`}
    />
  );
}
