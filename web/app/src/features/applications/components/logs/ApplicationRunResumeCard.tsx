import { Button, Input, Space, Typography } from 'antd';
import { useState } from 'react';

import type { ApplicationRunDetail } from '../../api/runtime';
import { i18nText } from '../../../../shared/i18n/text';

export function ApplicationRunResumeCard({
  detail,
  onResume,
  onCompleteCallback
}: {
  detail: ApplicationRunDetail;
  onResume: (
    checkpointId: string,
    inputPayload: Record<string, unknown>
  ) => Promise<unknown>;
  onCompleteCallback: (
    callbackTaskId: string,
    responsePayload: Record<string, unknown>
  ) => Promise<unknown>;
}) {
  const [humanInput, setHumanInput] = useState('');
  const [callbackJson, setCallbackJson] = useState('{\n  "result": {}\n}');
  const latestCheckpoint = detail.checkpoints[detail.checkpoints.length - 1] ?? null;
  const pendingCallback =
    detail.callback_tasks.find((task) => task.status === 'pending') ?? null;

  if (detail.flow_run.status === 'waiting_human' && latestCheckpoint) {
    const waitingNodeId =
      (latestCheckpoint.locator_payload?.node_id as string | undefined) ?? 'node-human';

    return (
      <div>
        <Typography.Title level={5}>{i18nText("applications", "auto.k_a18fc7fc48")}</Typography.Title>
        <Space direction="vertical" style={{ width: '100%' }}>
          <Typography.Text>
            {(latestCheckpoint.external_ref_payload?.prompt as string | undefined) ??
              i18nText("applications", "auto.k_376653cd1f")}
          </Typography.Text>
          <Input.TextArea
            aria-label={i18nText("applications", "auto.k_640eee29f1")}
            rows={4}
            value={humanInput}
            onChange={(event) => setHumanInput(event.target.value)}
          />
          <Button
            type="primary"
            onClick={() =>
              void onResume(latestCheckpoint.id, {
                [waitingNodeId]: { input: humanInput }
              })
            }
          >
            {i18nText("applications", "auto.k_4df857eb48")}</Button>
        </Space>
      </div>
    );
  }

  if (detail.flow_run.status === 'waiting_callback' && pendingCallback) {
    return (
      <div>
        <Typography.Title level={5}>{i18nText("applications", "auto.k_d36b025c58")}</Typography.Title>
        <Space direction="vertical" style={{ width: '100%' }}>
          <Typography.Text>{pendingCallback.callback_kind}</Typography.Text>
          <Input.TextArea
            aria-label={i18nText("applications", "auto.k_d986c08e0d")}
            rows={6}
            value={callbackJson}
            onChange={(event) => setCallbackJson(event.target.value)}
          />
          <Button
            type="primary"
            onClick={() =>
              void onCompleteCallback(
                pendingCallback.id,
                JSON.parse(callbackJson) as Record<string, unknown>
              )
            }
          >
            {i18nText("applications", "auto.k_b232bb91ac")}</Button>
        </Space>
      </div>
    );
  }

  return null;
}
