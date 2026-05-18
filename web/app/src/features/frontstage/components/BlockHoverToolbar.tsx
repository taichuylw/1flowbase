import {
  ArrowDownOutlined,
  ArrowUpOutlined,
  CodeOutlined,
  DeleteOutlined,
  EditOutlined,
  HolderOutlined,
  MoreOutlined,
  SettingOutlined
} from '@ant-design/icons';
import { Button, Divider, Popconfirm, Popover, Space, Typography, message } from 'antd';
import type { CSSProperties, FC } from 'react';
import { useState } from 'react';

type BlockHoverToolbarProps = {
  blockId: string;
  onMoveUp: () => void;
  onMoveDown: () => void;
  onConfigure: () => void;
  onEditCode: () => void;
  onDelete: () => void;
  canMoveUp: boolean;
  canMoveDown: boolean;
  isVisible: boolean;
  disabled?: boolean;
};

const toolbarStyle: CSSProperties = {
  position: 'absolute',
  top: 10,
  right: 10,
  zIndex: 10,
  background: '#fff',
  border: '1px solid #d9f7e8',
  borderRadius: 8,
  boxShadow: '0 10px 28px rgba(16, 185, 129, 0.12)',
  padding: 3,
  transition: 'opacity 0.15s ease'
};

export const BlockHoverToolbar: FC<BlockHoverToolbarProps> = ({
  blockId,
  onMoveUp,
  onMoveDown,
  onConfigure,
  onEditCode,
  onDelete,
  canMoveUp,
  canMoveDown,
  isVisible,
  disabled = false
}) => {
  const [isMovePopoverOpen, setIsMovePopoverOpen] = useState(false);
  const [isMorePopoverOpen, setIsMorePopoverOpen] = useState(false);
  const [isDeleteConfirmOpen, setIsDeleteConfirmOpen] = useState(false);

  const copyBlockUid = () => {
    if (typeof navigator !== 'undefined' && navigator.clipboard) {
      void navigator.clipboard.writeText(blockId).then(
        () => {
          void message.success('已复制 UID');
        },
        () => {
          void message.warning('复制失败，请手动复制 UID');
        }
      );
      return;
    }

    void message.warning('当前环境不支持自动复制');
  };

  const menuButtonStyle: CSSProperties = {
    width: '100%',
    justifyContent: 'flex-start',
    textAlign: 'left'
  };

  const moveContent = (
    <Space direction="vertical" size={4} style={{ minWidth: 132 }}>
      <Button
        size="small"
        type="text"
        block
        icon={<ArrowUpOutlined />}
        aria-label="上移区块"
        disabled={disabled || !canMoveUp}
        onClick={(e) => {
          e.stopPropagation();
          setIsMovePopoverOpen(false);
          onMoveUp();
        }}
        style={menuButtonStyle}
      >
        上移区块
      </Button>
      <Button
        size="small"
        type="text"
        block
        icon={<ArrowDownOutlined />}
        aria-label="下移区块"
        disabled={disabled || !canMoveDown}
        onClick={(e) => {
          e.stopPropagation();
          setIsMovePopoverOpen(false);
          onMoveDown();
        }}
        style={menuButtonStyle}
      >
        下移区块
      </Button>
    </Space>
  );

  const moreContent = (
    <div
      style={{ minWidth: 168 }}
      onClick={(event) => {
        event.stopPropagation();
      }}
    >
      <Space direction="vertical" size={2} style={{ width: '100%' }}>
        <Button
          size="small"
          type="text"
          block
          icon={<EditOutlined />}
          aria-label="标题和描述"
          disabled={disabled}
          onClick={(event) => {
            event.stopPropagation();
            setIsMorePopoverOpen(false);
            onConfigure();
          }}
          style={menuButtonStyle}
        >
          标题和描述
        </Button>
        <Button
          size="small"
          type="text"
          block
          icon={<SettingOutlined />}
          aria-label="区块联动规则"
          disabled={disabled}
          onClick={(event) => {
            event.stopPropagation();
            setIsMorePopoverOpen(false);
            onConfigure();
          }}
          style={menuButtonStyle}
        >
          区块联动规则
        </Button>
        <Button
          size="small"
          type="text"
          block
          icon={<SettingOutlined />}
          aria-label="区块高度"
          disabled={disabled}
          onClick={(event) => {
            event.stopPropagation();
            setIsMorePopoverOpen(false);
            onConfigure();
          }}
          style={menuButtonStyle}
        >
          区块高度
        </Button>
        <Button
          size="small"
          type="text"
          block
          icon={<CodeOutlined />}
          aria-label="Write JavaScript"
          disabled={disabled}
          onClick={(event) => {
            event.stopPropagation();
            setIsMorePopoverOpen(false);
            onEditCode();
          }}
          style={menuButtonStyle}
        >
          Write JavaScript
        </Button>
        <Divider style={{ margin: '4px 0' }} />
        <Button
          size="small"
          type="text"
          block
          aria-label="保存为模板"
          disabled
          style={menuButtonStyle}
        >
          保存为模板
        </Button>
        <Button
          size="small"
          type="text"
          block
          aria-label="复制 UID"
          disabled={disabled}
          onClick={(event) => {
            event.stopPropagation();
            setIsMorePopoverOpen(false);
            copyBlockUid();
          }}
          style={menuButtonStyle}
        >
          复制 UID
        </Button>
        <Popconfirm
          title="确定删除此区块？"
          trigger="click"
          okText="删除"
          cancelText="取消"
          okButtonProps={{ 'aria-label': '确认删除区块' }}
          open={isDeleteConfirmOpen}
          onOpenChange={(open) => {
            setIsDeleteConfirmOpen(open);
            if (open) {
              setIsMorePopoverOpen(true);
            }
          }}
          destroyOnHidden
          onConfirm={() => {
            setIsDeleteConfirmOpen(false);
            setIsMorePopoverOpen(false);
            onDelete();
          }}
          onCancel={() => setIsDeleteConfirmOpen(false)}
        >
          <Button
            size="small"
            type="text"
            block
            danger
            icon={<DeleteOutlined />}
            aria-label="删除区块"
            disabled={disabled}
            onClick={(event) => {
              event.stopPropagation();
              setIsDeleteConfirmOpen(true);
              setIsMorePopoverOpen(true);
            }}
            style={menuButtonStyle}
          >
            删除
          </Button>
        </Popconfirm>
      </Space>
      <Typography.Text type="secondary" style={{ display: 'block', marginTop: 8, fontSize: 11 }}>
        保存为模板暂未开放
      </Typography.Text>
    </div>
  );

  return (
    <div
      style={{
        ...toolbarStyle,
        opacity: isVisible ? 1 : 0,
        pointerEvents: isVisible ? 'auto' : 'none'
      }}
      onClick={(event) => event.stopPropagation()}
    >
      <Space size={2}>
        <Popover
          content={moveContent}
          trigger="click"
          placement="bottomRight"
          open={isMovePopoverOpen}
          destroyOnHidden
          onOpenChange={setIsMovePopoverOpen}
        >
          <Button
            size="small"
            type="text"
            icon={<HolderOutlined />}
            disabled={disabled}
            aria-label="移动或排序区块"
          />
        </Popover>
        <Button
          size="small"
          type="text"
          icon={<EditOutlined />}
          disabled={disabled}
          onClick={(e) => {
            e.stopPropagation();
            onEditCode();
          }}
          aria-label="编辑区块"
        />
        <Popover
          content={moreContent}
          trigger="click"
          placement="bottomRight"
          open={isMorePopoverOpen}
          destroyOnHidden
          onOpenChange={(open) => {
            if (!open && isDeleteConfirmOpen) {
              return;
            }
            setIsMorePopoverOpen(open);
          }}
        >
          <Button
            size="small"
            type="text"
            icon={<MoreOutlined />}
            disabled={disabled}
            aria-label="更多区块操作"
          />
        </Popover>
      </Space>
    </div>
  );
};
