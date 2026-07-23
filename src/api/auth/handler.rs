use crate::api::auth::dto::GoogleLoginParams;
use crate::infrastructure::auth::Principal;
use crate::response::ApiResponse;
use crate::service::auth::{login_with_email_service, login_with_google_service, logout_service};
use crate::{
    api::auth::dto::{LoginParams, LoginResponse},
    application::AppState,
    request::BValidJson,
    response::ApiResult,
};

use axum::Json;

use axum::extract::{ConnectInfo, Path, State};
use axum::{debug_handler, Extension};
use std::net::SocketAddr;

// Google 登录
#[debug_handler]
#[tracing::instrument(name = "login_with_google", skip_all, fields(account = %id_token, IP = %addr))]
pub async fn login_with_google(
    State(AppState { db }): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(GoogleLoginParams { id_token }): Json<GoogleLoginParams>,
) -> ApiResult<LoginResponse> {
    tracing::info!(
        "start login with google, address: {}, id_token: {}",
        addr,
        id_token
    );
    let google_response = login_with_google_service(&id_token, &db).await?;
    tracing::info!("google response is : {:?}", google_response);

    Ok(ApiResponse::success("login success", Some(google_response)))
}

// 邮箱 登录
#[debug_handler]
#[tracing::instrument(name = "login_with_email", skip_all, fields(account = %email, IP = %addr))]
pub async fn login_with_email(
    State(AppState { db }): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    BValidJson(LoginParams { email, password }): BValidJson<LoginParams>,
) -> ApiResult<LoginResponse> {
    tracing::info!(
        "start login with email, address: {}, email: {}",
        addr,
        email
    );

    let login_response = login_with_email_service(&email, &password, &db).await?;

    tracing::info!("login with email successfully.");

    Ok(ApiResponse::success("login success", Some(login_response)))
}

// 登出，根据 token_hash 将对应的 revoked_at 设置为当前时间
#[debug_handler]
#[tracing::instrument(name = "logout")]
pub async fn logout(
    State(AppState { db }): State<AppState>,
    Path(front_refresh_token): Path<String>,
) -> ApiResult<()> {
    tracing::info!(
        "start logout now, and the raw token is : {}",
        front_refresh_token,
    );

    logout_service(&front_refresh_token, &db).await?;

    tracing::info!("logout successfully.");

    return Ok(ApiResponse::success("logout success.", None));
}

#[debug_handler]
pub async fn get_user_info(Extension(principal): Extension<Principal>) -> ApiResult<Principal> {
    Ok(ApiResponse::success("", Some(principal)))
}
