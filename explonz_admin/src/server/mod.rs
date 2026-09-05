pub mod auth;
pub mod labels;
pub mod posts;
pub mod spots;

// 后端请求响应的结构体
#[derive(serde::Deserialize)]
pub struct ApiResp<T> {
    pub data: Option<T>,
}

/// 读取后端服务地址，优先使用环境变量 BACKEND_URL，默认 http://127.0.0.1:3000
pub fn backend_url() -> String {
    std::env::var("BACKEND_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string())
}

/// 从请求 Cookie 中提取 access_token，未登录时返回 ServerFnError
/// 注：leptos_axum::extract 仅在 SSR 编译目标可用，#[cfg(feature = "ssr")] 不可省略
#[cfg(feature = "ssr")]
pub async fn extract_token() -> Result<String, leptos::prelude::ServerFnError> {
    use axum_extra::extract::CookieJar;
    use leptos_axum::extract;

    let jar: CookieJar = extract().await?;
    jar.get("access_token")
        .map(|c| c.value().to_string())
        .ok_or_else(|| leptos::prelude::ServerFnError::new("Not authenticated"))
}
