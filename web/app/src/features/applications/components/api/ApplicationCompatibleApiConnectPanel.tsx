import { ApiOutlined } from '@ant-design/icons';
import { Typography } from 'antd';

import { getApplicationsApiBaseUrl } from '../../api/applications';

function trimTrailingSlash(value: string) {
  return value.replace(/\/+$/, '');
}

function CompatibleEndpoint({
  title,
  baseUrl,
  path,
  auth,
  modelHint
}: {
  title: string;
  baseUrl: string;
  path: string;
  auth: string;
  modelHint: string;
}) {
  return (
    <div className="application-api-connect__endpoint">
      <div className="application-api-connect__endpoint-title">
        <ApiOutlined aria-hidden="true" />
        <Typography.Text strong>{title}</Typography.Text>
      </div>
      <dl className="application-api-connect__facts">
        <div>
          <dt>Base URL</dt>
          <dd>
            <Typography.Text code>{baseUrl}</Typography.Text>
          </dd>
        </div>
        <div>
          <dt>Endpoint</dt>
          <dd>
            <Typography.Text code>{path}</Typography.Text>
          </dd>
        </div>
        <div>
          <dt>Auth</dt>
          <dd>
            <Typography.Text code>{auth}</Typography.Text>
          </dd>
        </div>
        <div>
          <dt>Model</dt>
          <dd>
            <Typography.Text code>{modelHint}</Typography.Text>
          </dd>
        </div>
      </dl>
    </div>
  );
}

export function ApplicationCompatibleApiConnectPanel() {
  const apiBaseUrl = trimTrailingSlash(getApplicationsApiBaseUrl());

  return (
    <section
      aria-label="外部 Agent 接入"
      className="application-api-connect"
    >
      <div className="application-api-panel__header">
        <div>
          <Typography.Title level={4}>外部 Agent 接入</Typography.Title>
          <Typography.Text type="secondary">
            使用应用 API Key，把当前应用配置成 OpenAI 或 Anthropic 兼容模型服务。
          </Typography.Text>
        </div>
      </div>
      <div className="application-api-connect__grid">
        <CompatibleEndpoint
          title="OpenAI 兼容"
          baseUrl={`${apiBaseUrl}/v1`}
          path="/v1/chat/completions"
          auth="Authorization: Bearer <Application API Key>"
          modelHint="任意非空 model 名称"
        />
        <CompatibleEndpoint
          title="Anthropic 兼容"
          baseUrl={apiBaseUrl}
          path="/v1/messages"
          auth="x-api-key: <Application API Key>"
          modelHint="任意非空 model 名称"
        />
      </div>
    </section>
  );
}
