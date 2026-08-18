use jsonwebtoken::{
    decode, encode, get_current_timestamp, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::LazyLock;
use std::time::Duration;

use sha2::{Digest, Sha256};
use uuid::Uuid;

static DEFAULT_KEY: &str = "MIIEpAIBAAKCAQEAu6L5Jk7J2Yc6X5r2Z2b4L8a9V1C7H3pN6tK8jW0xYv3fGqS";
static JWT_INSTANCE: LazyLock<Jwt> = LazyLock::new(Jwt::default);

//  access_token 有效期 1小时；
static ACCESS_TOKEN_TTL_SECS: u64 = 3600;

// refresh_token 有效期 30天；30天内用户免登录。
static REFRESH_TOKEN_TTL_SECS: u64 = 30 * 24 * 3600;

#[derive(Debug, Clone, Serialize)]
pub struct Principal {
    pub id: String,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// 主题 (Subject) - 通常是用户ID
    pub sub: String,
    /// 签发者 (Issuer)
    pub iss: String,
    /// 受众 (Audience)
    pub aud: String,
    /// 过期时间 (Expiration Time) - 时间戳
    pub exp: u64,
    /// 生效时间 (Not Before) - 时间戳
    pub nbf: u64,
    /// 签发时间 (Issued At) - 时间戳
    pub iat: u64,
    /// JWT ID - 唯一标识
    pub jti: String,
    /// 自定义声明：用户角色
    pub roles: Vec<String>,
    /// 自定义声明：额外数据
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug)]
pub struct JwtConfig {
    pub secret: String,
    pub issuer: String,
    pub audience: String,
    pub expiration: Duration,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: DEFAULT_KEY.to_string(),
            issuer: "https://www.axum-template.com".to_string(),
            audience: "https://www.axum-template.com".to_string(),
            expiration: Duration::from_secs(ACCESS_TOKEN_TTL_SECS),
        }
    }
}

pub struct Jwt {
    encode_secret: EncodingKey,
    decode_secret: DecodingKey,
    header: Header,
    validation: Validation,
    expires_in: Duration,
    audience: String,
    issuer: String,
}

impl Jwt {
    pub fn new(config: JwtConfig) -> Self {
        let algorithm = Algorithm::HS256;
        let mut validation = Validation::new(algorithm);
        validation.set_issuer(&[&config.issuer]);
        validation.set_audience(&[&config.audience]);
        validation.set_required_spec_claims(&["jti", "iat", "exp", "nbf", "iss", "aud", "sub"]);

        Self {
            encode_secret: EncodingKey::from_secret(config.secret.as_bytes()),
            decode_secret: DecodingKey::from_secret(config.secret.as_bytes()),
            header: Header::new(algorithm),
            validation,
            expires_in: config.expiration,
            audience: config.audience,
            issuer: config.issuer,
        }
    }

    // 返回的 token 是否带有 expire 时间戳
    pub fn encode(
        &self,
        principal: Principal,
        is_with_exp: bool,
    ) -> anyhow::Result<(String, Option<u64>)> {
        let current_timestamp = get_current_timestamp();
        let exp = current_timestamp.saturating_add(self.expires_in.as_secs());
        let claims = Claims {
            sub: format!("{}:{}:{}", principal.id, principal.name, principal.email), // will be extracted in decode for '/get_user_info'
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            exp,
            nbf: current_timestamp,
            iat: current_timestamp,
            jti: xid::new().to_string(),
            roles: vec![],
            extra: Default::default(),
        };

        let access_token = encode(&self.header, &claims, &self.encode_secret)?;
        if is_with_exp {
            Ok((access_token, Some(exp)))
        } else {
            Ok((access_token, None))
        }
    }

    pub fn decode(&self, token: &str) -> anyhow::Result<Principal> {
        let claims = decode::<Claims>(token, &self.decode_secret, &self.validation)?.claims;

        let mut parts = claims.sub.splitn(3, ':');

        let principal = Principal {
            id: parts.next().unwrap().to_string(),
            name: parts.next().unwrap().to_string(),
            email: parts.next().unwrap_or("default role").to_string(),
        };

        Ok(principal)
    }

    /// 生成裸 refresh token（返回给客户端）和对应的 SHA-256 哈希（存入 DB）
    pub fn generate_refresh_token(&self) -> (String, String, u64) {
        let raw = Uuid::new_v4().to_string();
        let refresh_token = hex::encode(Sha256::digest(raw.as_bytes()));

        let current_timestamp = get_current_timestamp();
        let exp = current_timestamp.saturating_add(REFRESH_TOKEN_TTL_SECS);
        (raw, refresh_token, exp)
    }
}

impl Default for Jwt {
    fn default() -> Self {
        Self::new(JwtConfig::default())
    }
}

pub fn get_jwt() -> &'static Jwt {
    &JWT_INSTANCE
}

impl Display for Principal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.id, self.name, self.email)
    }
}
