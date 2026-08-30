use axum::{routing::post, Router};

use crate::{
    api::spots::handler::{create_spot, upload_image},
    application::AppState,
};

pub mod dto;
pub mod handler;

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/spots/new", post(create_spot))
        .route("/spots/images", post(upload_image))
}
