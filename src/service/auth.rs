use anyhow::{anyhow, bail};
use sea_orm::ActiveValue::{NotSet, Set};
use sea_orm::{
    sea_query::Expr, sqlx::types::chrono::Utc, ColumnTrait, DatabaseConnection, DbBackend,
    EntityTrait, FromQueryResult, QueryFilter, Statement,
};
use sea_orm::{ActiveEnum, ActiveModelTrait, TransactionTrait};
use sha2::{Digest, Sha256};

use crate::api::auth::dto::{GoogleTokenResponse, UserForResponse};
use crate::entity::sea_orm_active_enums::AuthProviderType;
use crate::entity::user_auth_providers;
use crate::entity::{prelude::*, users};
use crate::infrastructure::auth::{get_jwt, Principal};
use crate::{
    api::auth::dto::{LoginResponse, LoginUser},
    entity::refresh_tokens,
};

use chrono::DateTime;
use sea_orm::sqlx::types::chrono;

// Google OAuth2 的验证请求URL
const GOOGLE_OAUTH2_URL: &str = "https://oauth2.googleapis.com/tokeninfo";

// Google OAuth 2.0 客户端 ID
const GOOGLE_CLIENT_ID: &str =
    "868040329476-436691fgdbv3h37ap444m1pn1egfh34d.apps.googleusercontent.com";

// Google 登录验证
pub async fn login_with_google_service(
    id_token: &str,
    db: &DatabaseConnection,
) -> anyhow::Result<LoginResponse> {
    let url = format!("{}?id_token={}", GOOGLE_OAUTH2_URL, id_token);

    let token_response = reqwest::Client::new()
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json::<GoogleTokenResponse>()
        .await?;

    tracing::info!("google token response: {:?}", token_response);

    // 1. 先验证 client id
    if token_response.aud != GOOGLE_CLIENT_ID {
        bail!("Invalid aud.");
    }
    // 2. 验证是否是Google签发
    if !token_response.iss.ends_with("accounts.google.com") {
        bail!("Invalid iss.");
    }
    // 3. 验证邮箱是否被Google Verified
    if token_response.email_verified.ne("true") {
        bail!("email is not verified by google.");
    }
    // 4. 验证用户是否存在
    let sql = r#"
        SELECT u.id, u.nickname, u.avatar_url, u.email
        FROM users u
        JOIN user_auth_providers uap ON uap.user_id = u.id
        WHERE uap.provider_user_id = $1
          AND uap.provider = 'google'
        LIMIT 1
    "#;

    let user_opt = LoginUser::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        [token_response.sub.clone().into()],
    ))
    .one(db)
    .await?;

    let login_user = match user_opt {
        Some(user) => {
            tracing::info!("用户存在：{:?}", user);
            user
        }
        None => {
            tracing::info!("用户不存在，开始创建用户");
            let user = create_google_user(&token_response, db).await?;
            tracing::info!("用户不存在，已创建用户：{:?}", user);
            LoginUser {
                id: user.id,
                nickname: user.nickname,
                avatar_url: user.avatar_url,
                email: user.email,
            }
        }
    };

    construct_login_response(&login_user, db).await
}

// 使用 Google的token response 创建账号
async fn create_google_user(
    token_response: &GoogleTokenResponse,
    db: &DatabaseConnection,
) -> anyhow::Result<users::Model> {
    // 1. 开启事务
    let transaction = db.begin().await?;

    // 2. 创建 Users
    let user = users::ActiveModel {
        nickname: Set(token_response.name.clone().unwrap_or("-".to_string())),
        avatar_url: Set(token_response.picture.clone()),
        email: Set(Some(token_response.email.clone())),
        ..Default::default()
    }
    .insert(&transaction)
    .await?;

    // 3. 创建 provider
    user_auth_providers::ActiveModel {
        user_id: Set(user.id),
        provider: Set(AuthProviderType::Google),
        provider_user_id: Set(Some(token_response.sub.clone())),
        provider_email: Set(Some(token_response.email.clone())),
        password_hash: Set(None),
        ..Default::default()
    }
    .insert(&transaction)
    .await?;

    // 4. 提交事物
    transaction.commit().await?;

    Ok(user)
}

// email 登录
pub async fn login_with_email_service(
    email: &str,
    password: &str,
    db: &DatabaseConnection,
) -> anyhow::Result<LoginResponse> {
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
        [email.into(), password.into()],
    ))
    .one(db)
    .await?;

    let login_user = user.ok_or_else(|| {
        tracing::warn!(
            "login failed (user not found or wrong password), account: {}",
            email
        );
        anyhow!("user or password is not correct")
    })?;

    construct_login_response(&login_user, db).await
}

// 构建登录响应结构数据
async fn construct_login_response(
    login_user: &LoginUser,
    db: &DatabaseConnection,
) -> anyhow::Result<LoginResponse> {
    // 1. 查询所有 providers
    let all_providers = UserAuthProviders::find()
        .filter(user_auth_providers::Column::UserId.eq(login_user.id))
        .all(db)
        .await?
        .iter()
        .map(|p| p.provider.to_value())
        .collect::<Vec<String>>();

    // 2. 构建 auth
    let principal = Principal {
        id: login_user.id.to_string(),
        name: login_user.nickname.clone(),
        email: login_user.email.clone().unwrap_or_default(),
    };

    // 3. 构建 access token
    let (access_token, access_token_expires_at) = get_jwt().encode(principal, true)?;

    // 4. 构建 refresh token
    let (front_refresh_token, hash_refresh_token, refresh_token_expires_at) =
        get_jwt().generate_refresh_token();
    let expires_at = DateTime::<Utc>::from_timestamp(refresh_token_expires_at as i64, 0)
        .expect("invalid timestamp");

    // 5. 将 hash refresh token 写入表演员
    let new_refreshs_token = refresh_tokens::ActiveModel {
        user_id: Set(login_user.id),
        token_hash: Set(hash_refresh_token),
        expires_at: Set(expires_at.into()),
        revoked_at: NotSet,
        ..Default::default()
    };
    new_refreshs_token.insert(db).await?;
    tracing::info!(
        "refresh token write to table successfull: {}",
        front_refresh_token
    );
    return Ok(LoginResponse {
        access_token,
        refresh_token: front_refresh_token,
        access_token_expires_at: access_token_expires_at.unwrap_or_default(),
        refresh_token_expires_at,
        user: UserForResponse {
            id: login_user.id,
            nickname: login_user.nickname.clone(),
            avatar_url: login_user.avatar_url.clone(),
            email: login_user.email.clone().unwrap_or_default(),
            providers: all_providers,
        },
    });
}

// 登出
pub async fn logout_service(
    front_refresh_token: &str,
    db: &DatabaseConnection,
) -> anyhow::Result<()> {
    let refresh_token = hex::encode(Sha256::digest(front_refresh_token.as_bytes()));

    println!("the token hash is : {refresh_token}");

    let rt = refresh_tokens::Entity::update_many()
        .col_expr(refresh_tokens::Column::RevokedAt, Expr::value(Utc::now()))
        .filter(refresh_tokens::Column::TokenHash.eq(refresh_token))
        .filter(refresh_tokens::Column::RevokedAt.is_null()) // 避免重复登出
        .exec(db)
        .await?;
    if rt.rows_affected == 0 {
        tracing::info!("none token hash matched");
        return Err(anyhow!("Refresh token is invalid or already revoked"));
    }
    Ok(())
}
