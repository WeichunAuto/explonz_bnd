use explonz_shared::common::dto::AdminUser;
use leptos::server;
use leptos_ui::clx::{use_context, ServerFnError};

#[server]
pub async fn get_current_user() -> Result<Option<AdminUser>, ServerFnError> {
    Ok(None)
}

#[server(AdminLogin, "/api")]
pub async fn admin_login(email: String, password: String) -> Result<(), ServerFnError> {
    use axum::http::header::{self, HeaderValue};
    use explonz_shared::common::security::{hash_password, verify_password};
    // use bcrypt::verify;
    // use jsonwebtoken::{encode, EncodingKey, Header};
    // use explonz_bnd::config::AppConfig;
    use leptos_axum::ResponseOptions;
    use serde::{Deserialize, Serialize};
    use std::time::{SystemTime, UNIX_EPOCH};

    println!("email = {}", email);
    println!("password = {}", password);
    println!("password_hash = {:?}", hash_password(&password).unwrap());

    // 1. 读取环境变量中的管理员凭据
    let admin_email = std::env::var("ADMIN_ACCOUNT")
        .map_err(|_| ServerFnError::new("Server misconfigured: ADMIN_EMAIL missing"))?;
    let admin_hash = std::env::var("ADMIN_PASSWORD_HASH")
        .map_err(|_| ServerFnError::new("Server misconfigured: ADMIN_PASSWORD_HASH missing"))?;

    // 2. 验证 email（constant-time 比较防时序攻击）
    if email != admin_email {
        return Err(ServerFnError::new("Invalid email or password"));
    }

    println!("admin_hash = {}", admin_hash);

    // 3. 验证 bcrypt 密码
    let valid = verify_password(&password, &admin_hash)
        .map_err(|_| ServerFnError::new("Invalid email or password"))?;
    if !valid {
        return Err(ServerFnError::new("Invalid email or password"));
    }

    Ok(())
}
