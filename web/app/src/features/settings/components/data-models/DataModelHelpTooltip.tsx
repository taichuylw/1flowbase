import { useId } from 'react';

import { QuestionCircleOutlined } from '@ant-design/icons';
import { Tooltip } from 'antd';
import { i18nText } from '../../../../shared/i18n/text';

export const dataModelStatusHelp =
  i18nText("settings", "auto.k_841318af0b");

export const defaultApiExposureStatusHelp =
  i18nText("settings", "auto.k_9ae58afd41");

export const dataModelCodeHelp =
  i18nText("settings", "auto.k_5ce5022369");

export const dataModelTitleHelp =
  i18nText("settings", "auto.k_ed415f6486");

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
          aria-label={decorative ? undefined : i18nText("settings", "auto.k_e3c0ac30bb", { value1: label })}
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
