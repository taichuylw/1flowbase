use axum::Json;
use uuid::Uuid;

use crate::{error_response::ApiError, response::ApiSuccess};

pub(crate) type ApiJson<T> = Json<ApiSuccess<T>>;

pub(crate) fn ok<T>(data: T) -> ApiJson<T> {
    Json(ApiSuccess::new(data))
}

pub(crate) fn parse_uuid(raw: &str, field: &'static str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw)
        .map_err(|_| control_plane::errors::ControlPlaneError::InvalidInput(field).into())
}
