use sea_orm::FromQueryResult;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct LoginParams {
    #[validate(custom(
        function = "crate::request::is_email_valid",
        message = "invalid email format, please check."
    ))]
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct GoogleLoginParams {
    pub id_token: String,
}

#[derive(Debug, FromQueryResult)]
pub struct LoginUser {
    pub id: Uuid,
    pub nickname: String,
    pub avatar_url: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserForResponse {
    pub id: Uuid,
    pub nickname: String,
    pub avatar_url: Option<String>,
    pub email: String,
    pub providers: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub access_token_expires_at: u64,
    pub refresh_token_expires_at: u64,
    pub user: UserForResponse,
}

// Google Token Payload
#[derive(Debug, Deserialize)]
pub struct GoogleTokenResponse {
    pub sub: String,
    pub email: String,
    pub email_verified: String,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub aud: String,
    pub iss: String,
}
