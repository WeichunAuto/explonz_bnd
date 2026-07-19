use crate::application::AppState;
use crate::error::ApiError;
use crate::middleware::get_auth_layer;
use axum::routing::get;
use axum::Router;

pub mod auth;
pub(crate) mod user;
mod workspace;

/// Creates and configures the application API routes.
pub async fn build_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .nest("/api", user::routes())
        // .nest("/api", workspace::routes())
        .route_layer(get_auth_layer())
        .nest("/auth", auth::routes())
        .fallback(fallback)
        .method_not_allowed_fallback(async || -> ApiError {
            tracing::warn!("Method not allowed!");
            ApiError::MethodNotAllowedError
        })
}

/// Fallback handler
///
/// This async function is used as the default route handler
/// when no other api match the incoming request.
/// It returns a `404 Not Found` status with a simple error message.
pub(crate) async fn fallback() -> ApiError {
    tracing::warn!("Goes into fallback, no route matched");
    ApiError::NotFoundError
}

/// This is a simple async handler for the root route (`/`).
pub(crate) async fn index() -> &'static str {
    "index"
}
