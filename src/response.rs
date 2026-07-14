use crate::error::ApiError;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

pub type ApiResult<T> = Result<ApiResponse<T>, ApiError>;

/// Standard API response structure
///
/// Provides a consistent response format for all API endpoints.
/// Follows common REST API conventions with code, message, and optional data.
///
/// # Type Parameters
/// * `T` - The type of the data payload, must implement Serialize
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub code: i16,
    pub msg: String,

    #[serde(skip_serializing_if = "Option::is_none")] // 忽略序列化，如果Option is none.
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn new(code: i16, msg: String, data: Option<T>) -> Self {
        ApiResponse { code, msg, data }
    }

    /// Creates a successful API response
    ///
    /// Uses HTTP 200 status code for successful operations.
    /// The message can be any type that converts to String.
    pub fn success<M: Into<String>>(message: M, data: Option<T>) -> Self {
        ApiResponse::new(200, message.into(), data)
    }

    /// Creates an error API response
    ///
    /// Uses code 0 for errors (can be customized based on your needs).
    /// Error responses typically don't include data payload.
    pub fn error<M: Into<String>>(message: M) -> Self {
        ApiResponse::new(0, message.into(), None)
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        axum::Json(self).into_response()
    }
}
