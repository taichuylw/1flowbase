import { CloseOutlined, ReloadOutlined } from '@ant-design/icons';
import { Button, Space, Typography } from 'antd';

export function DebugConsoleHeader({
  clearDisabled,
  onClear,
  onClose
}: {
  clearDisabled: boolean;
  onClear: () => void;
  onClose: () => void;
}) {
  return (
    <div className="agent-flow-editor__debug-console-header">
      <div className="agent-flow-editor__debug-console-title">
        <Space size={8}>
          <Typography.Text strong>预览</Typography.Text>
        </Space>
      </div>
      <Space size={4} wrap>
        <Button
          aria-label="清空预览"
          disabled={clearDisabled}
          icon={<ReloadOutlined />}
          size="small"
          type="text"
          onClick={onClear}
        />
        <Button
          aria-label="关闭预览"
          icon={<CloseOutlined />}
          size="small"
          type="text"
          onClick={onClose}
        />
      </Space>
    </div>
  );
}
