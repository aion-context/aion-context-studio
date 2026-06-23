//! Map `studio-core` errors to HTTP responses with appropriate status codes and a JSON body.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use studio_core::StudioError;

/// Newtype so we can implement `IntoResponse` for the foreign error.
pub struct ApiError(pub StudioError);

impl From<StudioError> for ApiError {
    fn from(e: StudioError) -> Self {
        Self(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self.0 {
            StudioError::NotFound(_) => StatusCode::NOT_FOUND,
            StudioError::AlreadyExists(_) => StatusCode::CONFLICT,
            StudioError::InvalidId(_) | StudioError::Invalid(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (
            status,
            Json(serde_json::json!({ "error": self.0.to_string() })),
        )
            .into_response()
    }
}

pub type ApiResult<T> = std::result::Result<T, ApiError>;
