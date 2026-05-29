import { useId } from 'react';

import { QuestionCircleOutlined } from '@ant-design/icons';
import { Tooltip } from 'antd';
import { i18nText } from '../../../../shared/i18n/text';

export const dataModelStatusHelp =
  i18nText("settings", "auto.draft_created_unpublished_state_published_entry_running_availability_api_exposure");

export const defaultApiExposureStatusHelp =
  i18nText("settings", "auto.draft_api_exposure_draft_published_exposed_api_access_surface_generated");

export const dataModelCodeHelp =
  i18nText("settings", "auto.code_stable_identifier_data_model_used_apis_permissions_internal_references");

export const dataModelTitleHelp =
  i18nText("settings", "auto.title_name_displayed_management_console_adjusted_according_business_semantics_affecting");

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
          aria-label={decorative ? undefined : i18nText("settings", "auto.description_option", { value1: label })}
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
