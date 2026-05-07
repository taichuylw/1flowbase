import { useId } from 'react';

import { QuestionCircleOutlined } from '@ant-design/icons';
import { Tooltip } from 'antd';

export const dataModelStatusHelp =
  'draft: 草稿，默认新建为未发布状态；published: 已发布，允许进入运行可用性和 API 暴露判断；disabled: 已停用，不进入运行面；broken: 当前定义、运行依赖或外部资源异常，需要修复后再发布。';

export const defaultApiExposureStatusHelp =
  'draft: API 暴露草稿；published_not_exposed: 默认不生成 API 访问面；api_exposed_no_permission: 已请求生成 API 访问面，但默认不授予访问权限。';

export const dataModelCodeHelp =
  'Code: Data Model 的稳定标识，用于 API、权限和内部引用；创建后不可编辑。';

export const dataModelTitleHelp =
  '标题: 管理台展示名称，可按业务语义调整，不影响稳定 Code。';

export function DataModelFieldLabel({
  label,
  title,
  decorativeHelp = false
}: {
  label: string;
  title: string;
  decorativeHelp?: boolean;
}) {
  return (
    <span className="data-model-panel__field-label">
      <span>{label}</span>
      <DataModelHelpTooltip
        decorative={decorativeHelp}
        label={label}
        title={title}
      />
    </span>
  );
}

export function DataModelHelpTooltip({
  decorative = false,
  label,
  title
}: {
  decorative?: boolean;
  label: string;
  title: string;
}) {
  const descriptionId = useId();

  return (
    <>
      <Tooltip title={title}>
        <QuestionCircleOutlined
          aria-describedby={decorative ? undefined : descriptionId}
          aria-hidden={decorative ? true : undefined}
          aria-label={decorative ? undefined : `${label}说明`}
          className="data-model-panel__help-icon"
          tabIndex={decorative ? -1 : 0}
        />
      </Tooltip>
      {decorative ? null : (
        <span id={descriptionId} className="data-model-panel__sr-only">
          {title}
        </span>
      )}
    </>
  );
}
