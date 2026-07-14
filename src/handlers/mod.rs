use crate::error::ApiError;

pub(crate) mod user;
pub(crate) mod workspace;

/// Index handler
///
/// This is a simple async handler for the root route (`/`).
pub(crate) async fn index() -> &'static str {
    "index"
}

/// Fallback handler
///
/// This async function is used as the default route handler
/// when no other api match the incoming request.
/// It returns a `404 Not Found` status with a simple error message.
pub(crate) async fn fallback() -> ApiError {
    tracing::warn!("No route matched");
    ApiError::NotFoundError
}
