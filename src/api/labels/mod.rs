use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::{
    api::labels::handler::{create_label, delete_label, get_labels},
    application::AppState,
};

pub mod dto;
pub mod handler;

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/labels", get(get_labels))
        .route("/labels/new", post(create_label))
        .route("/labels/{label_id}", delete(delete_label))
}
