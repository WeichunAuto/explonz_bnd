use crate::application::AppState;
use crate::error::ApiError;
use crate::handlers;
use crate::middleware::get_auth_layer;
use axum::{routing::get, Router};

mod login_auth;
pub(crate) mod user;
mod workspace;

/// Creates and configures the application API routes.
pub async fn build_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::index))
        // .nest("/api", user::routes())
        // .nest("/api", workspace::routes())
        .route_layer(get_auth_layer())
        .nest("/auth", login_auth::routes())
        .fallback(handlers::fallback)
        .method_not_allowed_fallback(async || -> ApiError {
            tracing::warn!("Method not allowed!");
            ApiError::MethodNotAllowedError
        })
}
