use crate::application::AppState;
use crate::auth::{get_jwt, Principal};
use crate::entity::refresh_tokens::ActiveModel;
use crate::entity::user_auth_providers;
use crate::entity::{prelude::*, refresh_tokens};
use crate::error::ApiError;
use crate::middleware::get_auth_layer;
use crate::request::{BJson, BValidJson};
use crate::response::{ApiResponse, ApiResult};
use axum::extract::{ConnectInfo, Path, State};
use axum::routing::{get, patch, post};
use axum::Json;
use axum::{debug_handler, Extension, Router};
use chrono::{DateTime, Utc};
use sea_orm::sqlx::types::chrono;
use sea_orm::ActiveValue::{NotSet, Set};
use sea_orm::{prelude::*, DbBackend, FromQueryResult, Statement};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct LoginParams {
    #[validate(custom(
        function = "crate::request::is_email_valid",
        message = "invalid email format, please check."
    ))]
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
pub struct GoogleLoginParams {
    id_token: String,
}

#[derive(Debug, FromQueryResult)]
pub struct LoginUser {
    id: Uuid,
    nickname: String,
    avatar_url: Option<String>,
    email: Option<String>,
}

#[derive(Debug, Serialize)]
struct UserForResponse {
    id: Uuid,
    nickname: String,
    avatar_url: Option<String>,
    email: String,
    providers: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    access_token: String,
    refresh_token: String,
    access_token_expires_at: u64,
    refresh_token_expires_at: u64,
    user: UserForResponse,
}

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/get_user_info", get(get_user_info))
        .route_layer(get_auth_layer())
        .route("/login_with_email", post(login_with_email))
        .route("/login_with_google", post(login_with_google))
        .route("/logout/{token_hash}", patch(logout))
}

// 登出，根据 token_hash 将对应的 revoked_at 设置为当前时间
#[debug_handler]
#[tracing::instrument(name = "logout")]
pub async fn logout(
    State(AppState { db }): State<AppState>,
    Path(front_refresh_token): Path<String>,
) -> ApiResult<()> {
    tracing::info!(
        "the raw token is : {}, and the raw token length is : {}",
        front_refresh_token,
        front_refresh_token.len()
    );
    let refresh_token = hex::encode(Sha256::digest(front_refresh_token.as_bytes()));

    println!("the token hash is : {refresh_token}");

    let rt = refresh_tokens::Entity::update_many()
        .col_expr(refresh_tokens::Column::RevokedAt, Expr::value(Utc::now()))
        .filter(refresh_tokens::Column::TokenHash.eq(refresh_token))
        .filter(refresh_tokens::Column::RevokedAt.is_null()) // 避免重复登出
        .exec(&db)
        .await?;
    if rt.rows_affected == 0 {
        println!("none token hash matched");
        return Err(ApiError::UnAuthenticatedError(
            "Refresh token is invalid or already revoked".to_string(),
        ));
    }

    tracing::info!("logout successfully.");

    return Ok(ApiResponse::success("logout success.", None));
}

#[debug_handler]
#[tracing::instrument(name = "login_with_google", skip_all, fields(account = %id_token, IP = %addr))]
pub async fn login_with_google(
    State(AppState { db }): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(GoogleLoginParams { id_token }): Json<GoogleLoginParams>,
) -> ApiResult<LoginResponse> {
    tracing::info!("start login with google, id_token: {}", id_token);
    Ok(ApiResponse::success("login success", None))
}

#[debug_handler]
#[tracing::instrument(name = "login_with_email", skip_all, fields(account = %email, IP = %addr))]
pub async fn login_with_email(
    State(AppState { db }): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    BValidJson(LoginParams { email, password }): BValidJson<LoginParams>,
) -> ApiResult<LoginResponse> {
    tracing::info!("start login with email, email: {}", email);

    // 1. 单条 SQL：JOIN + pgcrypto crypt 验证，一步完成
    let sql = r#"
        SELECT u.id, u.nickname, u.avatar_url, u.email
        FROM users u
        JOIN user_auth_providers uap ON uap.user_id = u.id
        WHERE u.email = $1
          AND uap.provider = 'email'
          AND uap.password_hash = crypt($2, uap.password_hash)
        LIMIT 1
    "#;
    let user = LoginUser::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        [email.clone().into(), password.into()],
    ))
    .one(&db)
    .await?;

    let user = user.ok_or_else(|| {
        tracing::warn!(
            "login failed (user not found or wrong password), account: {}",
            email
        );
        ApiError::ValidationError("user or password is not correct".to_string())
    })?;

    // 2. 查该用户所有 auth providers
    let all_providers = UserAuthProviders::find()
        .filter(user_auth_providers::Column::UserId.eq(user.id))
        .all(&db)
        .await?
        .iter()
        .map(|p| p.provider.to_value())
        .collect::<Vec<String>>();
    tracing::info!("return providers: {}", all_providers.len());

    // 3. 带过期时间的 access tokan
    let principal = Principal {
        id: user.id.to_string(),
        name: user.nickname.clone(),
        email: user.email.unwrap_or_default(),
    };
    let (access_token, access_token_expires_at) = get_jwt().encode(principal, true)?;
    tracing::info!(
        "login success, IP: {}, access_token: {}",
        addr,
        access_token
    );

    // 4. 生成 Refresh Token，哈希后写入 refresh_tokens 表
    let (front_refresh_token, hash_refresh_token, refresh_token_expires_at) =
        get_jwt().generate_refresh_token();

    tracing::info!(
        "The raw token is: {}, the raw token length is: {}, and the token_hash is: {}",
        front_refresh_token,
        front_refresh_token.len(),
        hash_refresh_token
    );

    // 写入 refresh_tokens 表
    let expires_at = DateTime::<Utc>::from_timestamp(refresh_token_expires_at as i64, 0)
        .expect("invalid timestamp");

    let new_refreshs_token = refresh_tokens::ActiveModel {
        user_id: Set(user.id),
        token_hash: Set(hash_refresh_token),
        expires_at: Set(expires_at.into()),
        revoked_at: NotSet,
        ..Default::default()
    };
    new_refreshs_token.insert(&db).await?;
    tracing::info!(
        "refresh token write to table successfull: {}",
        front_refresh_token
    );

    // 5. 构造响应结果
    Ok(ApiResponse::success(
        "login success",
        Some(LoginResponse {
            access_token,
            refresh_token: front_refresh_token,
            access_token_expires_at: access_token_expires_at.unwrap_or_default(),
            refresh_token_expires_at,
            user: UserForResponse {
                id: user.id,
                nickname: user.nickname,
                avatar_url: user.avatar_url,
                email,
                providers: all_providers,
            },
        }),
    ))
}

#[debug_handler]
pub async fn get_user_info(Extension(principal): Extension<Principal>) -> ApiResult<Principal> {
    Ok(ApiResponse::success("", Some(principal)))
}
