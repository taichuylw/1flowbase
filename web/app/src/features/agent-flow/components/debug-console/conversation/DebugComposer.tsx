import {
  ArrowRightOutlined,
  ArrowUpOutlined,
  CloseCircleOutlined,
  MessageOutlined,
} from '@ant-design/icons';
import { Button, Input, Typography } from 'antd';
import { useState } from 'react';
import { i18nText } from '../../../../../shared/i18n/text';

export function DebugComposer({
  value,
  disabled,
  showFeatureBar = true,
  submitting,
  stopping,
  onChange,
  onStop,
  onSubmit
}: {
  value: string;
  disabled: boolean;
  showFeatureBar?: boolean;
  submitting: boolean;
  stopping: boolean;
  onChange: (value: string) => void;
  onStop: () => void;
  onSubmit: (value: string) => void;
}) {
  const [isComposing, setIsComposing] = useState(false);
  const showStop = submitting || stopping;

  function handleSubmit() {
    if (disabled || submitting || stopping) {
      return;
    }

    onSubmit(value);
    onChange('');
  }

  return (
    <div className="agent-flow-editor__debug-composer">
      <div className="agent-flow-editor__debug-composer-box">
        <Input.TextArea
          autoSize={{ minRows: 1, maxRows: 4 }}
          variant="borderless"
          placeholder={i18nText("agentFlow", "auto.key_bhdoedfmcg")}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          onCompositionStart={() => setIsComposing(true)}
          onCompositionEnd={() => setIsComposing(false)}
          onKeyDown={(event) => {
            // 中文输入法组合态期间不能把 Enter 误判成发送。
            if (
              event.key !== 'Enter' ||
              event.shiftKey ||
              isComposing ||
              event.nativeEvent.isComposing
            ) {
              return;
            }

            event.preventDefault();

            handleSubmit();
          }}
        />
        <div className="agent-flow-editor__debug-composer-actions">
          {showStop ? (
            <Button
              aria-label={stopping ? i18nText("agentFlow", "auto.key_okefjephko") : i18nText("agentFlow", "auto.key_oigafhjklm")}
              className="agent-flow-editor__debug-composer-submit agent-flow-editor__debug-composer-stop"
              disabled={stopping}
              icon={<CloseCircleOutlined />}
              loading={stopping}
              shape="circle"
              onClick={onStop}
            />
          ) : (
            <Button
              aria-label={i18nText("agentFlow", "auto.key_kbicbbmnan")}
              className="agent-flow-editor__debug-composer-submit"
              disabled={disabled}
              icon={<ArrowUpOutlined />}
              shape="circle"
              type="primary"
              onClick={handleSubmit}
            />
          )}
        </div>
      </div>
      {showFeatureBar ? (
        <div className="agent-flow-editor__debug-feature-bar">
          <span className="agent-flow-editor__debug-feature-icon">
            <MessageOutlined />
          </span>
          <Typography.Text>{i18nText("agentFlow", "auto.key_fcnmjlfefg")}</Typography.Text>
          <Button
            aria-label={i18nText("agentFlow", "auto.key_hlkheolkdl")}
            className="agent-flow-editor__debug-feature-manage"
            icon={<ArrowRightOutlined />}
            iconPosition="end"
            size="small"
            type="link"
          >
            {i18nText("agentFlow", "auto.key_ejijlfmpje")}</Button>
        </div>
      ) : null}
    </div>
  );
}
