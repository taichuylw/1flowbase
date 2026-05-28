import { Button, Descriptions, Flex, Space, Tag, Typography } from 'antd';

import type {
  SettingsDataModel,
  UpdateSettingsDataModelApiExposureInput
} from '../../api/data-models';
import { i18nText } from '../../../../shared/i18n/text';

export function DataModelApiTab({
  model,
  canManage,
  saving,
  onUpdateApiExposure
}: {
  model: SettingsDataModel;
  canManage: boolean;
  saving: boolean;
  onUpdateApiExposure: (
    input: UpdateSettingsDataModelApiExposureInput
  ) => void;
}) {
  const ready =
    model.status === 'published' &&
    model.runtime_availability === 'available' &&
    model.api_exposure_status === 'api_exposed_ready';
  const unsafe = model.api_exposure_status === 'unsafe_external_source';
  const canRequest =
    model.status === 'published' &&
    model.api_exposure_status === 'published_not_exposed' &&
    !unsafe;
  const canClose =
    model.api_exposure_status === 'api_exposed_no_permission' ||
    model.api_exposure_status === 'api_exposed_ready';

  return (
    <Flex vertical gap={16}>
      <Descriptions
        size="small"
        column={1}
        items={[
          {
            key: 'status',
            label: i18nText("settings", "auto.api_exposure_status"),
            children: <Tag>{model.api_exposure_status}</Tag>
          },
          {
            key: 'readiness',
            label: 'computed readiness',
            children: (
              <Tag color={ready ? 'green' : 'default'}>api_exposed_ready</Tag>
            )
          },
          {
            key: 'unsafe',
            label: 'unsafe_external_source',
            children: (
              <Tag color={unsafe ? 'red' : 'default'}>
                {unsafe ? 'derived' : 'not_detected'}
              </Tag>
            )
          },
          {
            key: 'runtime',
            label: 'Runtime',
            children: model.runtime_availability
          },
          {
            key: 'namespace',
            label: 'ACL Namespace',
            children: model.acl_namespace
          }
        ]}
      />
      <Space wrap>
        <Button
          type="primary"
          loading={saving}
          disabled={!canManage || !canRequest}
          onClick={() =>
            onUpdateApiExposure({
              api_exposure_status: 'api_exposed_no_permission'
            })
          }
        >
          {i18nText("settings", "auto.request_api_exposure")}</Button>
        <Button
          loading={saving}
          disabled={!canManage || !canClose}
          onClick={() =>
            onUpdateApiExposure({
              api_exposure_status: 'published_not_exposed'
            })
          }
        >
          {i18nText("settings", "auto.turn_off_api_exposure")}</Button>
      </Space>
      {unsafe ? (
        <Typography.Text type="secondary">
          {i18nText("settings", "auto.unsafe_external_source_notice")}</Typography.Text>
      ) : null}
    </Flex>
  );
}
