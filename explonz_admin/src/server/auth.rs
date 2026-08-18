use explonz_shared::common::dto::{AdminUser, AuthStatus};
use leptos::server;
use leptos_ui::clx::{use_context, ServerFnError};

#[server]
pub async fn get_current_user() -> Result<AuthStatus, ServerFnError> {
    // println!("测试输出！");

    use axum_extra::extract::CookieJar;
    use explonz_shared::common::auth::get_jwt;
    use leptos_axum::extract;

    // 1. 提取 Cookie Jar
    let jar: CookieJar = extract()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // 2. 读取 access_token cookie
    let token = match jar.get("access_token") {
        Some(c) => c.value().to_string(),
        None => return Ok(AuthStatus::NotLoggedIn), // 未登录
    };

    // 3. 验证并解码 JWT（get_jwt() 内置签名 + 过期校验）
    // match get_jwt().decode(&token) {
    //     Ok(principal) => Ok(Some(AdminUser {
    //         id: principal.id,
    //         name: principal.name,
    //         email: principal.email,
    //     })),
    //     Err(_) => Ok(None), // token 无效或过期
    // }
    match get_jwt().decode(&token) {
        Ok(principal) => Ok(AuthStatus::Authenticated(AdminUser {
            id: principal.id,
            name: principal.name,
            email: principal.email,
        })),
        Err(_) => Ok(AuthStatus::TokenExpired),
    }
}

#[server(AdminLogin, "/api")]
pub async fn admin_login(email: String, password: String) -> Result<(), ServerFnError> {
    use axum::http::header::{self, HeaderValue};
    use explonz_shared::common::security::{hash_password, verify_password};
    use leptos_axum::ResponseOptions;
    use serde::{Deserialize, Serialize};
    use std::time::{SystemTime, UNIX_EPOCH};

    use explonz_shared::common::auth::{get_jwt, Principal};

    // 1. 读取环境变量中的管理员凭据
    let admin_email = std::env::var("ADMIN_ACCOUNT")
        .map_err(|_| ServerFnError::new("Server misconfigured: ADMIN_EMAIL missing"))?;
    let admin_hash = std::env::var("ADMIN_PASSWORD_HASH")
        .map_err(|_| ServerFnError::new("Server misconfigured: ADMIN_PASSWORD_HASH missing"))?;

    // 2. 验证 email（constant-time 比较防时序攻击）
    if email != admin_email {
        return Err(ServerFnError::new("Invalid email or password"));
    }

    // 3. 验证 bcrypt 密码
    let valid = verify_password(&password, &admin_hash)
        .map_err(|_| ServerFnError::new("Invalid email or password"))?;
    if !valid {
        return Err(ServerFnError::new("Invalid email or password"));
    }

    // 4. 生成 JWT（1 小时有效期，由 get_jwt() 默认配置控制）
    let principal = Principal {
        id: format!("admin_{}", email),
        name: email.clone(),
        email: email.clone(),
    };

    let (access_token, _expires_at) = get_jwt()
        .encode(principal, true)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // 5. 写入 HttpOnly Cookie
    let response_opts = use_context::<ResponseOptions>()
        .ok_or_else(|| ServerFnError::new("No ResponseOptions in context"))?;
    let cookie = format!(
        "access_token={access_token}; HttpOnly; SameSite=Strict; Path=/; Max-Age=36000" // 10小时有效，生产环境在 Max-Age 后追加 "; Secure"
    );
    response_opts.insert_header(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).map_err(|e| ServerFnError::new(e.to_string()))?,
    );

    Ok(())
}
