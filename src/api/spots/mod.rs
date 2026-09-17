use axum::{
    routing::{get, post, put},
    Router,
};

use crate::{
    api::spots::handler::{create_spot, get_spot, get_spots, update_spot, upload_image, get_seasonal_picking_types},
    application::AppState,
};

pub mod dto;
pub mod handler;

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/spots", get(get_spots))
        .route("/spots/{spot_id}", get(get_spot))
        .route("/spots/{spot_id}", put(update_spot))
        .route("/spots/new", post(create_spot))
        .route("/spots/images", post(upload_image))
        .route("/spots/seasonal_picking_types", get(get_seasonal_picking_types))
}
