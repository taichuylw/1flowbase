use anyhow::Result;

use crate::errors::ControlPlaneError;

pub(super) fn normalize_external_resource_key(
    source_kind: domain::DataModelSourceKind,
    value: Option<&str>,
) -> Result<Option<String>, ControlPlaneError> {
    match source_kind {
        domain::DataModelSourceKind::ExternalSource => value
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| Ok(Some(value.to_string())))
            .unwrap_or_else(|| Err(ControlPlaneError::InvalidInput("external_resource_key"))),
        domain::DataModelSourceKind::MainSource => {
            if value.map(str::trim).is_some_and(|value| !value.is_empty()) {
                Err(ControlPlaneError::InvalidInput("external_resource_key"))
            } else {
                Ok(None)
            }
        }
    }
}

fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(super) fn normalize_external_table_id(
    source_kind: domain::DataModelSourceKind,
    value: Option<&str>,
) -> Result<Option<String>, ControlPlaneError> {
    match source_kind {
        domain::DataModelSourceKind::ExternalSource => Ok(normalize_optional_text(value)),
        domain::DataModelSourceKind::MainSource => match normalize_optional_text(value) {
            Some(_) => Err(ControlPlaneError::InvalidInput("external_table_id")),
            None => Ok(None),
        },
    }
}

pub(super) fn normalize_external_field_key(
    source_kind: domain::DataModelSourceKind,
    value: Option<&str>,
) -> Result<Option<String>, ControlPlaneError> {
    match source_kind {
        domain::DataModelSourceKind::ExternalSource => value
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| Ok(Some(value.to_string())))
            .unwrap_or_else(|| Err(ControlPlaneError::InvalidInput("external_field_key"))),
        domain::DataModelSourceKind::MainSource => {
            if value.map(str::trim).is_some_and(|value| !value.is_empty()) {
                Err(ControlPlaneError::InvalidInput("external_field_key"))
            } else {
                Ok(None)
            }
        }
    }
}
