use axum::{Router, routing::{get, post}};

use crate::{
    api::spots::handler::{create_spot, get_spots, upload_image}, application::AppState,
};

pub mod dto;
pub mod handler;

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/spots", get(get_spots))      // 新增
        .route("/spots/new", post(create_spot))
        .route("/spots/images", post(upload_image))
}
