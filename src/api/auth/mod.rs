use axum::{
    routing::{get, patch, post},
    Router,
};

use crate::{
    api::auth::handler::{
        get_user_info, login_with_email, login_with_google, logout, send_code, setup_password,
        verify_code,
    },
    application::AppState,
    middleware::get_auth_layer,
};

pub mod dto;
pub mod handler;

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/get_user_info", get(get_user_info))
        .route_layer(get_auth_layer())
        .route("/login_with_email", post(login_with_email))
        .route("/login_with_google", post(login_with_google))
        .route("/logout/{token_hash}", patch(logout))
        .route("/sign_up/send_code", post(send_code))
        .route("/sign_up/verify_code", post(verify_code))
        .route("/sign_up/setup_password", post(setup_password))
}
