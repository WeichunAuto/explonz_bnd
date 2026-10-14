use axum::{
    routing::{get, post, put},
    Router,
};

use crate::{
    api::spots::handler::{
        create_seasonal_picking, create_spot, delete_seasonal_picking,
        get_all_seasonal_pickings_with_spots, get_seasonal_picking_types, get_seasonal_pickings,
        get_spot, get_spots, update_spot, upload_image, upsert_actual_seasonal_pick,
    },
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
        .route("/spots/{spot_id}/seasonal_pickings/new", post(create_seasonal_picking))
        .route("/spots/{spot_id}/seasonal_pickings", get(get_seasonal_pickings))
        .route("/spots/seasonal_pickings/{picking_id}", axum::routing::delete(delete_seasonal_picking))
        .route("/spots/seasonal_pickings/all", get(get_all_seasonal_pickings_with_spots))
        .route("/spots/seasonal_pickings/{picking_id}/actuals/new", post(upsert_actual_seasonal_pick))
}
