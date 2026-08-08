use std::format;

use anyhow::{anyhow, bail};
use askama::Template;
use rand::RngExt;
use sea_orm::ActiveValue::{NotSet, Set};
use sea_orm::{
    sea_query::Expr, sqlx::types::chrono::Utc, ColumnTrait, DatabaseConnection, DbBackend,
    EntityTrait, FromQueryResult, QueryFilter, Statement,
};
use sea_orm::{
    ActiveEnum, ActiveModelTrait, Condition, ConnectionTrait, DatabaseBackend, DatabaseTransaction,
    TransactionTrait,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::api::auth::dto::{GoogleTokenResponse, OtpTokenResponse, UserForResponse};
use crate::config::AppConfig;
use crate::entity::sea_orm_active_enums::AuthProviderType;
use crate::entity::users::Model;
use crate::entity::{prelude::*, users};
use crate::entity::{sign_up_otps, user_auth_providers};
use crate::error::ApiError;
use crate::infrastructure::auth::{get_jwt, Principal};
use crate::response::{ApiResponse, ApiResult};
use crate::service::email::VerifyEmailTemplate;
use crate::{
    api::auth::dto::{LoginResponse, LoginUser},
    entity::refresh_tokens,
};

use chrono::DateTime;
use sea_orm::sqlx::types::chrono;

use resend_rs::types::CreateEmailBaseOptions;
use resend_rs::{Resend, Result};

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
            // email 未被注册，创建新用户
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

// 检查Google 账号的邮箱是否之前被注册过
async fn is_google_email_signed_up(
    db: &DatabaseTransaction,
    email: &str,
) -> anyhow::Result<Option<users::Model>> {
    let user_opt = users::Entity::find()
        .filter(users::Column::Email.eq(email))
        .one(db)
        .await?;

    match user_opt {
        Some(user) => Ok(Some(user)),
        None => Ok(None),
    }
}

// 使用 Google的token response 创建账号
async fn create_google_user(
    token_response: &GoogleTokenResponse,
    db: &DatabaseConnection,
) -> anyhow::Result<users::Model> {
    // 1. 开启事务
    let transaction = db.begin().await?;

    let user: users::Model;

    // 检查Google的email是否被注册过？
    if let Some(exist_user) = is_google_email_signed_up(&transaction, &token_response.email).await?
    {
        user = exist_user;
    } else {
        user = users::ActiveModel {
            nickname: Set(token_response.name.clone().unwrap_or("-".to_string())),
            avatar_url: Set(token_response.picture.clone()),
            email: Set(Some(token_response.email.clone())),
            ..Default::default()
        }
        .insert(&transaction)
        .await?
        .into();
    }

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

pub async fn check_email_not_exists_service(
    db: &DatabaseConnection,
    email: &str,
) -> anyhow::Result<bool> {
    let sql = r#"
        SELECT u.id, u.nickname, u.avatar_url, u.email
        FROM users u
        JOIN user_auth_providers uap ON uap.user_id = u.id
        WHERE u.email = $1
        LIMIT 1
    "#;

    let user = LoginUser::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        [email.into()],
    ))
    .one(db)
    .await?;

    match user {
        Some(_) => {
            tracing::warn!("email already exists: {}", email);
            Ok(false)
        }
        None => Ok(true),
    }
}

// 同一 email 在有效期内的累计发送次数；是否超过 5 次
pub async fn too_many_sends_service(db: &DatabaseConnection, email: &str) -> anyhow::Result<bool> {
    let sql = r#"
        SELECT EXISTS(
                SELECT 1
                FROM sign_up_otps
                WHERE email = $1
                  AND expires_at > NOW()
                  AND sent_count >= 5
            ) AS too_many
    "#;
    let result = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            vec![email.into()],
        ))
        .await?;
    let too_many = result.unwrap().try_get::<bool>("", "too_many")?;

    if too_many {
        tracing::warn!(
            "Too many attempts of sending otp code, the email is: {}",
            email
        );
        Ok(true)
    } else {
        Ok(false)
    }
}

// 生成六位 OTP CODE, 并入库
pub async fn generate_otp_code_service(
    db: &DatabaseConnection,
    email: &str,
) -> anyhow::Result<String> {
    let opt_code = {
        let mut rng = rand::rng(); // rng 离开作用域后就被Drop
        format!("{:06}", rng.random_range(0..1_000_000))
    };

    let validity_minutes: i32 = AppConfig::get()
        .email()
        .get_otp_code_validity_time()
        .parse()
        .map_err(|_| {
            ApiError::InternalError(anyhow!("Invalid OTP validity time configuration."))
        })?;

    let sql = r#"
        INSERT INTO sign_up_otps
        (
            email, code, otp_token, attempts, sent_count, expires_at
        )
        VALUES ($1, $2, NULL, 0, 1, NOW() + ($3 * INTERVAL '1 minute'))

        ON CONFLICT(email)

        DO UPDATE SET
            code = EXCLUDED.code,
            otp_token = NULL,
            attempts = 0,
            sent_count =
                sign_up_otps.sent_count + 1,
            expires_at =
                EXCLUDED.expires_at,
            updated_at =
                NOW()
        "#;

    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        sql,
        vec![
            email.into(),
            opt_code.clone().into(),
            validity_minutes.into(),
        ],
    ))
    .await?;

    Ok(opt_code)
}

// 发送电子邮件
pub async fn send_email_service(email: &str, code: &str) -> anyhow::Result<bool> {
    let email_config = AppConfig::get().email();

    let resend = Resend::new(email_config.get_api_key());

    let from = email_config.get_sender();
    let to = [email];
    let subject = "Please Verify Your Email";

    let html = VerifyEmailTemplate {
        code,
        expire_minutes: 10,
    }
    .render()?;

    let email = CreateEmailBaseOptions::new(from, to, subject).with_html(&html);

    let _email = resend.emails.send(email).await?;

    Ok(true)
}

// 邮箱注册 校验验证码
pub async fn verify_otp_code_service(
    db: &DatabaseConnection,
    email: &str,
    code: &str,
) -> ApiResult<OtpTokenResponse> {
    tracing::info!("verify otp code, email: {}, code: {}", email, code);

    // 1. 查询 sign_up_otps WHERE email = $1 AND expires_at > NOW()
    let conditions = Condition::all()
        .add(sign_up_otps::Column::Email.eq(email))
        .add(sign_up_otps::Column::ExpiresAt.gt(Utc::now().naive_utc()));

    let otp = sign_up_otps::Entity::find()
        .filter(conditions)
        .one(db)
        .await?;

    let attempts = AppConfig::get()
        .email()
        .get_attempts()
        .parse::<i16>()
        .map_err(|_| ApiError::BizError("Invalid OTP code attempts config".to_string()))?;

    return match otp {
        Some(otp) => {
            if otp.attempts > attempts {
                tracing::warn!("Too many attempts, please try later.");
                Err(ApiError::ValidationError(
                    "Too many attempts, please try later.".to_string(),
                ))
            } else if otp.code.as_deref() != Some(code) {
                // attempts + 1
                let mut active_model: sign_up_otps::ActiveModel = otp.into();
                active_model.attempts = Set(active_model.attempts.unwrap() + 1);
                active_model.update(db).await?;

                tracing::warn!("Incorrect otp code: {}.", code);
                Err(ApiError::ValidationError("Incorrect otp code.".to_string()))
            } else {
                // 通过，生成 opt_token
                let otp_token = Uuid::new_v4();

                // opt_token 入库
                let mut active_model: sign_up_otps::ActiveModel = otp.into();
                active_model.otp_token = Set(Some(otp_token));
                active_model.update(db).await?;

                Ok(ApiResponse::success(
                    "success",
                    Some(OtpTokenResponse {
                        otp_token: otp_token.to_string(),
                    }),
                ))
            }
        }
        None => Err(ApiError::ValidationError(
            "email is not exists or expired.".to_string(),
        )),
    };
}

// 邮箱注册，设置密码
pub async fn setup_password_service(
    db: &DatabaseConnection,
    email: &str,
    password: &str,
    otp_token: Uuid,
) -> ApiResult<LoginResponse> {
    // 1. 查询 sign_up_otps WHERE email = $1 AND expires_at > NOW()
    let conditions = Condition::all()
        .add(sign_up_otps::Column::OtpToken.eq(otp_token))
        .add(sign_up_otps::Column::ExpiresAt.gt(Utc::now().naive_utc()));

    let otp = sign_up_otps::Entity::find()
        .filter(conditions)
        .one(db)
        .await?;

    return match otp {
        Some(otp) => {
            // 邮件可能已被篡改
            if otp.email.ne(email) {
                Err(ApiError::ValidationError("Email is not valid.".to_string()))
            }
            // 正常逻辑分支
            else {
                // 1. 再次检查 users 表 email 是否已存在（防并发重复）→ 返回 409
                let email_exists = check_email_exists_in_users(db, email).await?;
                if email_exists {
                    Err(ApiError::EmailAlreadyRegistered)
                } else {
                    // 2. 写入用户信息
                    let login_user = sign_up_users(db, email, password, otp.id).await?;

                    // 3. 构建并返回响应
                    let login_response = construct_login_response(&login_user, db).await?;
                    Ok(ApiResponse::success("", Some(login_response)))
                }
            }
        }
        // otp token 不存在或已过期
        None => {
            tracing::warn!("Otp token is not exists or expired.");
            Err(ApiError::ValidationError(
                "Otp token is not exists or expired.".to_string(),
            ))
        }
    };
}

// 写入注册用户的信息
async fn sign_up_users(
    db: &DatabaseConnection,
    email: &str,
    password: &str,
    sign_up_id: Uuid,
) -> anyhow::Result<LoginUser> {
    // 1. 开启事务
    let transaction = db.begin().await?;

    // 2. 创建 Users
    let user = users::ActiveModel {
        nickname: Set(String::from("Unknow")),
        email: Set(Some(String::from(email))),
        ..Default::default()
    }
    .insert(&transaction)
    .await?;
    tracing::info!("User created successfully.");

    // 3. 创建 provider
    let sql = r#"
        INSERT INTO user_auth_providers
        (
            user_id,
            provider,
            password_hash
        )
        VALUES
        (
            $1,
            $2::auth_provider_type,
            crypt($3, gen_salt('bf'))
        )
        "#;
    transaction
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            sql,
            vec![
                user.id.into(),
                AuthProviderType::Email.into(),
                password.into(),
            ],
        ))
        .await?;
    tracing::info!("Auth provider created successfully.");

    // 4. 注册成功后删除整行记录
    sign_up_otps::Entity::delete_by_id(sign_up_id)
        .exec(&transaction)
        .await?;
    tracing::info!("Sign up token record deleted successfully.");

    // 提交事务
    transaction.commit().await?;

    tracing::info!("Email: {}, Sign up successfully.", email);

    Ok(LoginUser {
        id: user.id,
        nickname: user.nickname,
        avatar_url: user.avatar_url,
        email: user.email,
    })
}

// 检查 email 在 users表中是否存在
async fn check_email_exists_in_users(db: &DatabaseConnection, email: &str) -> anyhow::Result<bool> {
    let conditions = Condition::all().add(users::Column::Email.eq(email));
    let user = users::Entity::find().filter(conditions).one(db).await?;

    match user {
        Some(_) => Ok(true),
        None => Ok(false),
    }
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
