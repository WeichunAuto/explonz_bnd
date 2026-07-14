// use crate::application::AppState;
// use crate::handlers::user;
// use axum::routing::{delete, patch};
// use axum::{
//     routing::{get, post},
//     Router,
// };

// /// Define user-related api for the application.
// pub(crate) fn routes() -> Router<AppState> {
//     Router::new()
//         .route("/create_user", post(user::create))
//         .route("/get_user", get(user::query_all_by_id_or_name))
//         .route("/query_by_keyword", get(user::query_by_keyword))
//         .route(
//             "/update_user_ws_by_id/{id}/{ws_id}",
//             patch(user::update_ws_by_id),
//         )
//         .route("/delete_user_by_id/{id}", delete(user::delete_by_id))
// }
