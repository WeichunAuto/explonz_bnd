use crate::api::auth::dto::{signUpParams, GoogleLoginParams};
use crate::error::ApiError;
use crate::infrastructure::auth::Principal;
use crate::response::ApiResponse;
use crate::service::auth::{
    check_email_not_exists_service, generate_otp_code_service, login_with_email_service,
    login_with_google_service, logout_service, send_email_service, too_many_sends_service,
};
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

    // let login_response = login_with_email_service(&email, &password, &db).await?;

    return match login_with_email_service(&email, &password, &db).await {
        Ok(login_response) => Ok(ApiResponse::success("login success", Some(login_response))),
        Err(e) => Err(ApiError::UnAuthenticatedError(e.to_string())),
    };

    // tracing::info!("login with email successfully.");
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

// 邮箱注册 发送验证码
#[debug_handler]
#[tracing::instrument(name = "send_code for sign up")]
pub async fn send_code(
    State(AppState { db }): State<AppState>,
    BValidJson(signUpParams { email }): BValidJson<signUpParams>,
) -> ApiResult<()> {
    tracing::info!("Sign up: start verifying the email: {}", email);

    match check_email_not_exists_service(&db, &email).await? {
        // email 未被注册
        true => {
            if too_many_sends_service(&db, &email).await? {
                // 同一 email 在有效期内的累计发送次数；超过 5 次
                return Err(ApiError::TooManySendTimes);
            } else {
                // 生成 CODE
                let otp_code = generate_otp_code_service(&db, &email).await?;

                // 发送邮件
                send_email_service(&email, &otp_code).await?;
                return Ok(ApiResponse::success("", None));
            }
            // Ok(ApiResponse::success("", None))
        }
        false => Err(ApiError::EmailAlreadyRegistered),
    }
}

#[debug_handler]
pub async fn get_user_info(Extension(principal): Extension<Principal>) -> ApiResult<Principal> {
    Ok(ApiResponse::success("", Some(principal)))
}
