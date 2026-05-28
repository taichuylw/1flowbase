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
import { i18nText } from '../../../shared/i18n/text';

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
          void message.success(i18nText("frontstage", "auto.k_b5ec70f2f9"));
        },
        () => {
          void message.warning(i18nText("frontstage", "auto.k_7c5c0e741a"));
        }
      );
      return;
    }

    void message.warning(i18nText("frontstage", "auto.k_5a9c502df4"));
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
        aria-label={i18nText("frontstage", "auto.k_a4906091ba")}
        disabled={disabled || !canMoveUp}
        onClick={(e) => {
          e.stopPropagation();
          setIsMovePopoverOpen(false);
          onMoveUp();
        }}
        style={menuButtonStyle}
      >
        {i18nText("frontstage", "auto.k_a4906091ba")}</Button>
      <Button
        size="small"
        type="text"
        block
        icon={<ArrowDownOutlined />}
        aria-label={i18nText("frontstage", "auto.k_abd210090c")}
        disabled={disabled || !canMoveDown}
        onClick={(e) => {
          e.stopPropagation();
          setIsMovePopoverOpen(false);
          onMoveDown();
        }}
        style={menuButtonStyle}
      >
        {i18nText("frontstage", "auto.k_abd210090c")}</Button>
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
          aria-label={i18nText("frontstage", "auto.k_6e4259361d")}
          disabled={disabled}
          onClick={(event) => {
            event.stopPropagation();
            setIsMorePopoverOpen(false);
            onConfigure();
          }}
          style={menuButtonStyle}
        >
          {i18nText("frontstage", "auto.k_6e4259361d")}</Button>
        <Button
          size="small"
          type="text"
          block
          icon={<SettingOutlined />}
          aria-label={i18nText("frontstage", "auto.k_40a60b8acd")}
          disabled={disabled}
          onClick={(event) => {
            event.stopPropagation();
            setIsMorePopoverOpen(false);
            onConfigure();
          }}
          style={menuButtonStyle}
        >
          {i18nText("frontstage", "auto.k_40a60b8acd")}</Button>
        <Button
          size="small"
          type="text"
          block
          icon={<SettingOutlined />}
          aria-label={i18nText("frontstage", "auto.k_0e190a74d0")}
          disabled={disabled}
          onClick={(event) => {
            event.stopPropagation();
            setIsMorePopoverOpen(false);
            onConfigure();
          }}
          style={menuButtonStyle}
        >
          {i18nText("frontstage", "auto.k_0e190a74d0")}</Button>
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
          aria-label={i18nText("frontstage", "auto.k_827613c831")}
          disabled
          style={menuButtonStyle}
        >
          {i18nText("frontstage", "auto.k_827613c831")}</Button>
        <Button
          size="small"
          type="text"
          block
          aria-label={i18nText("frontstage", "auto.k_7213f5e85f")}
          disabled={disabled}
          onClick={(event) => {
            event.stopPropagation();
            setIsMorePopoverOpen(false);
            copyBlockUid();
          }}
          style={menuButtonStyle}
        >
          {i18nText("frontstage", "auto.k_7213f5e85f")}</Button>
        <Popconfirm
          title={i18nText("frontstage", "auto.k_dfdda6d524")}
          trigger="click"
          okText={i18nText("frontstage", "auto.k_3755f56f2f")}
          cancelText={i18nText("frontstage", "auto.k_4d0b4688c7")}
          okButtonProps={{ 'aria-label': i18nText("frontstage", "auto.k_864f33fb1a") }}
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
            aria-label={i18nText("frontstage", "auto.k_b9b918a319")}
            disabled={disabled}
            onClick={(event) => {
              event.stopPropagation();
              setIsDeleteConfirmOpen(true);
              setIsMorePopoverOpen(true);
            }}
            style={menuButtonStyle}
          >
            {i18nText("frontstage", "auto.k_3755f56f2f")}</Button>
        </Popconfirm>
      </Space>
      <Typography.Text type="secondary" style={{ display: 'block', marginTop: 8, fontSize: 11 }}>
        {i18nText("frontstage", "auto.k_050571a2ae")}</Typography.Text>
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
            aria-label={i18nText("frontstage", "auto.k_b3ebc68dc0")}
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
          aria-label={i18nText("frontstage", "auto.k_b57a12863d")}
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
            aria-label={i18nText("frontstage", "auto.k_0adc4aa803")}
          />
        </Popover>
      </Space>
    </div>
  );
};
