use anyhow::anyhow;
use sea_orm::ActiveValue::{NotSet, Set};
use sea_orm::{
    sea_query::Expr, sqlx::types::chrono::Utc, ColumnTrait, DatabaseConnection, DbBackend,
    EntityTrait, FromQueryResult, QueryFilter, Statement,
};
use sea_orm::{ActiveEnum, ActiveModelTrait};
use sha2::{Digest, Sha256};

use crate::api::auth::dto::UserForResponse;
use crate::auth::{get_jwt, Principal};
use crate::entity::prelude::*;
use crate::entity::user_auth_providers;
use crate::{
    api::auth::dto::{LoginResponse, LoginUser},
    entity::refresh_tokens,
};

use chrono::DateTime;
use sea_orm::sqlx::types::chrono;

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

    let user = user.ok_or_else(|| {
        tracing::warn!(
            "login failed (user not found or wrong password), account: {}",
            email
        );
        anyhow!("user or password is not correct")
    })?;

    // 2. 查该用户所有 auth providers
    let all_providers = UserAuthProviders::find()
        .filter(user_auth_providers::Column::UserId.eq(user.id))
        .all(db)
        .await?
        .iter()
        .map(|p| p.provider.to_value())
        .collect::<Vec<String>>();

    // 3. 带过期时间的 access tokan
    let principal = Principal {
        id: user.id.to_string(),
        name: user.nickname.clone(),
        email: user.email.unwrap_or_default(),
    };
    let (access_token, access_token_expires_at) = get_jwt().encode(principal, true)?;

    // 4. 生成 Refresh Token，哈希后写入 refresh_tokens 表
    let (front_refresh_token, hash_refresh_token, refresh_token_expires_at) =
        get_jwt().generate_refresh_token();

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
    new_refreshs_token.insert(db).await?;
    tracing::info!(
        "refresh token write to table successfull: {}",
        front_refresh_token
    );

    // 5. 构造响应结果
    return Ok(LoginResponse {
        access_token,
        refresh_token: front_refresh_token,
        access_token_expires_at: access_token_expires_at.unwrap_or_default(),
        refresh_token_expires_at,
        user: UserForResponse {
            id: user.id,
            nickname: user.nickname,
            avatar_url: user.avatar_url,
            email: email.into(),
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
