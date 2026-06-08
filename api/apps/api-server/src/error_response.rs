use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use control_plane::errors::ControlPlaneError;
use plugin_framework::error::PluginFrameworkError;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug)]
pub struct ApiError(pub anyhow::Error);

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(value: E) -> Self {
        Self(value.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, expose_message) = match self.0.downcast_ref::<ControlPlaneError>() {
            Some(ControlPlaneError::NotAuthenticated) => {
                (StatusCode::UNAUTHORIZED, "not_authenticated", true)
            }
            Some(ControlPlaneError::PermissionDenied(reason)) => {
                (StatusCode::FORBIDDEN, *reason, true)
            }
            Some(ControlPlaneError::NotFound(name)) => (StatusCode::NOT_FOUND, *name, true),
            Some(ControlPlaneError::Conflict(name)) => (StatusCode::CONFLICT, *name, true),
            Some(ControlPlaneError::InvalidInput(name)) => (StatusCode::BAD_REQUEST, *name, true),
            Some(ControlPlaneError::InvalidStateTransition { .. }) => {
                (StatusCode::CONFLICT, "invalid_state_transition", true)
            }
            Some(ControlPlaneError::UpstreamUnavailable(name)) => {
                (StatusCode::BAD_GATEWAY, *name, true)
            }
            None => match self.0.downcast_ref::<PluginFrameworkError>() {
                Some(PluginFrameworkError::RuntimeContract { .. }) => {
                    (StatusCode::BAD_GATEWAY, "provider_runtime", true)
                }
                Some(_) => (StatusCode::BAD_REQUEST, "provider_package", true),
                None => {
                    tracing::error!(error = %self.0, "internal api error");
                    (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", false)
                }
            },
        };
        let message = if expose_message {
            // Provider runtime errors are upstream passthrough. Surface the message so
            // callers can debug provider and protocol failures without host rewriting.
            self.0.to_string()
        } else {
            "internal server error".to_string()
        };

        (
            status,
            Json(ErrorBody {
                code: code.to_string(),
                message,
            }),
        )
            .into_response()
    }
}
