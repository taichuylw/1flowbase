use anyhow::Result;
use domain::DataModelScopeKind;
use uuid::Uuid;

use crate::errors::ControlPlaneError;

pub(super) fn normalize_api_exposure_for_status(
    status: domain::DataModelStatus,
    exposure: domain::ApiExposureStatus,
) -> Result<domain::ApiExposureStatus> {
    let effective_exposure = if status == domain::DataModelStatus::Draft {
        domain::ApiExposureStatus::Draft
    } else {
        exposure
    };
    if domain::ApiExposureStatus::validate_for_status(
        status,
        effective_exposure,
        domain::ApiExposureReadiness::default(),
    )
    .is_rejected()
    {
        Err(ControlPlaneError::InvalidInput("api_exposure_status").into())
    } else {
        Ok(effective_exposure)
    }
}

pub(super) fn build_physical_table_name(scope_kind: DataModelScopeKind, code: &str) -> String {
    let prefix = match scope_kind {
        DataModelScopeKind::Workspace => "workspace",
        DataModelScopeKind::System => "system",
    };
    let suffix = Uuid::now_v7().simple().to_string();
    let sanitized_code = code.replace('-', "_");

    format!(
        "rtm_{prefix}_{}_{}",
        &suffix[suffix.len() - 8..],
        sanitized_code
    )
}

pub(super) fn build_physical_column_name(code: &str) -> String {
    code.replace('-', "_")
}
