# 登录页集成技术设计文档

## 1. 目标

- 访问后台任意受保护路由时，若未登录则自动重定向到 `/login`
- 登录成功后跳转到 `/dashboard`
- 登录状态通过 **HttpOnly Cookie** 持久化（access_token）

---

## 2. 现状分析

| 层面 | 当前状态 |
|------|----------|
| 路由 | `app.rs` 无路由配置，仅渲染占位 `<h1>` |
| 登录 UI | `pages/login/login.rs` 有完整 `Login02` 组件（email + password 表单） |
| 后端 Auth | `src/infrastructure/auth.rs` 已有 JWT 工具（HS256，access_token 1h，refresh_token 30天） |
| Server Functions | `server/posts.rs`, `server/spots.rs` 已有，暂无 auth 相关 |

---

## 3. Token 存储策略

采用 **HttpOnly Cookie** 存储 `access_token`：

- JavaScript 不可读，防止 XSS 窃取 token
- SSR 渲染时服务器可直接读取 Cookie 做鉴权，不依赖客户端水合
- 通过 `leptos_axum::ResponseOptions` 在登录 server function 中写入 `Set-Cookie` 响应头
- 通过 `leptos_axum::extract::<axum_extra::extract::CookieJar>()` 在后续请求中读取

Cookie 属性：
```
Set-Cookie: access_token=<jwt>; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=3600
```

---

## 4. 路由架构

```
/                     →  重定向到 /dashboard
/login                →  <Login02 />（公开，不受 AuthGuard 保护）
/dashboard            →  <Dashboard />      ┐
/posts                →  <PostList />        ├── 受 <AuthGuard> 包裹
/spots                →  <SpotList />       ┘
```

`app.rs` 最终结构：

```rust
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Stylesheet id="leptos" href="/pkg/explonz_admin.css"/>
        <Title text="Explonz Admin"/>
        <Router>
            <Routes fallback=|| "404 Not Found">
                <Route path="/"       view=|| view! { <Redirect path="/dashboard"/> }/>
                <Route path="/login"  view=Login02/>
                <ParentRoute path="/" view=AuthGuard>
                    <Route path="/dashboard" view=Dashboard/>
                    <Route path="/posts"     view=PostList/>
                    <Route path="/spots"     view=SpotList/>
                </ParentRoute>
            </Routes>
        </Router>
    }
}
```

---

## 5. 新增 Server Functions（`server/auth.rs`）

### 5.1 `admin_login`

```rust
#[server(AdminLogin, "/api")]
pub async fn admin_login(email: String, password: String) -> Result<(), ServerFnError> {
    // 1. 从 Axum 状态取 DatabaseConnection
    // 2. 验证 email + bcrypt password
    // 3. 调用 get_jwt().encode(principal, true) 生成 access_token
    // 4. 通过 ResponseOptions 写入 Set-Cookie 响应头
    // 5. 成功返回 Ok(())，前端收到后 navigate("/dashboard")
}
```

依赖：
- `use leptos_axum::{extract, ResponseOptions};`
- `use axum_extra::extract::CookieJar;`
- `use explonz_shared::entity::users;`（SSR-only，已有 ssr feature 保护）

### 5.2 `get_current_user`

```rust
#[server(GetCurrentUser, "/api")]
pub async fn get_current_user() -> Result<Option<AdminUser>, ServerFnError> {
    // 1. extract CookieJar，读取 access_token
    // 2. 调用 get_jwt().decode(token) 验证并解析 Principal
    // 3. 返回 Ok(Some(AdminUser { id, name, email }))
    //    token 不存在或过期 → 返回 Ok(None)
}
```

`AdminUser` 为共享 DTO，编译到 SSR 和 WASM 两侧（见第 7 节）。

---

## 6. `AuthGuard` 组件（`pages/auth_guard.rs`）

```rust
#[component]
pub fn AuthGuard() -> impl IntoView {
    let user = Resource::new(|| (), |_| get_current_user());
    let navigate = use_navigate();

    view! {
        <Suspense fallback=|| view! { <div class="flex h-screen items-center justify-center">"Loading..."</div> }>
            {move || {
                match user.get() {
                    // 数据未就绪，Suspense fallback 已处理
                    None => view! { <Outlet/> }.into_any(),
                    // 未登录或 token 失效 → 跳转登录页
                    Some(Ok(None)) | Some(Err(_)) => {
                        navigate("/login", NavigateOptions::default());
                        view! {}.into_any()
                    }
                    // 已登录 → 渲染子路由
                    Some(Ok(Some(_user))) => view! { <Outlet/> }.into_any(),
                }
            }}
        </Suspense>
    }
}
```

---

## 7. 共享 DTO 变更（`explonz_shared/common/dto.rs`）

新增 `AdminUser`，不依赖 sea-orm，编译进 WASM：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminUser {
    pub id: String,
    pub name: String,
    pub email: String,
}
```

---

## 8. 登录表单绑定（`pages/login/login.rs`）

在 `Login02` 组件中：

```rust
let login_action = ServerAction::<AdminLogin>::new();

// 监听 action 结果，成功则跳转
let navigate = use_navigate();
Effect::new(move |_| {
    if let Some(Ok(())) = login_action.value().get() {
        navigate("/dashboard", NavigateOptions::default());
    }
});

// 表单绑定
view! {
    <ActionForm action=login_action>
        <input type="email"    name="email"    .../>
        <input type="password" name="password" .../>
        <button type="submit">Login</button>
    </ActionForm>
}
```

`ServerAction` 替代手动 `on:submit`，自动处理序列化、加载状态和错误。

---

## 9. Cargo 依赖补充

在 `explonz_admin/Cargo.toml` 的 `ssr` feature 中需要：

```toml
[dependencies]
axum-extra = { workspace = true, features = ["cookie"] }   # CookieJar

# ssr feature 追加：
ssr = [
    ...
    "dep:axum-extra",
]
```

在根 `Cargo.toml` workspace.dependencies 中添加：

```toml
axum-extra = { version = "0.10", features = ["cookie"] }
```

---

## 10. 文件变更清单

| 文件 | 操作 | 主要内容 |
|------|------|----------|
| `server/auth.rs` | **新增** | `admin_login`、`get_current_user` server functions |
| `server/mod.rs` | **修改** | `pub mod auth;` |
| `pages/auth_guard.rs` | **新增** | `AuthGuard` 组件 |
| `pages/mod.rs` | **修改** | `pub mod auth_guard;` |
| `pages/login/login.rs` | **修改** | 表单改用 `ActionForm + ServerAction<AdminLogin>` |
| `app.rs` | **修改** | 添加 `Router` + `Routes`，引入各页面组件 |
| `explonz_shared/common/dto.rs` | **修改** | 新增 `AdminUser` 结构体 |
| `Cargo.toml`（workspace） | **修改** | 添加 `axum-extra` workspace dep |
| `explonz_admin/Cargo.toml` | **修改** | `axum-extra` optional dep，加入 `ssr` feature |

---

## 11. 鉴权流程时序

```
浏览器访问 /dashboard
    │
    ▼
Axum 服务器收到请求
    │
    ├─ SSR 渲染 App → AuthGuard → 触发 get_current_user server fn
    │       │
    │       ├─ Cookie 中有效 access_token → 渲染 Dashboard HTML
    │       └─ 无 token / 过期 → 渲染重定向指令（navigate("/login")）
    │
    ▼
HTML 发送到浏览器 → Hydrate
    │
    ▼
客户端 AuthGuard Resource 重新执行 get_current_user
    │
    ├─ 有效 → 渲染内容
    └─ 无效 → useNavigate → 跳转 /login
```

---

## 12. 安全注意事项

1. **JWT Secret**：通过 `JWT_SECRET` 环境变量注入，本地开发可用任意字符串，生产必须使用 32+ 字符随机串
2. **Cookie 属性**：生产环境须设 `Secure`（需 HTTPS），本地开发可去掉
3. **管理员身份验证**：凭据存储在环境变量中，不依赖普通用户表，无越权风险
4. **Rate Limiting**：`tower-http` 已在依赖中，可在 `main.rs` 的 Axum Router 上为 `/api/AdminLogin` 路径加限速中间件

---

## 13. 管理员账号密码存储方案

### 结论：使用环境变量存储

管理员账号不存入数据库，而是通过 `.env` / 系统环境变量提供：

```
ADMIN_EMAIL=admin@explonz.com
ADMIN_PASSWORD_HASH=$2b$12$xxxxxx...   # bcrypt hash，cost=12
JWT_SECRET=your-32-char-random-secret
```

### 原因

| 方案 | 优点 | 缺点 |
|------|------|------|
| 环境变量（推荐） | 简单、无 DB 查询、不暴露在代码中 | 不支持多管理员 |
| `users` + `user_auth_providers` 表 | 复用现有 schema | 需额外 `is_admin` 字段；与普通用户混用 |
| 独立 `admin_users` 表 | 隔离清晰 | 需新建表、migration |

管理员通常只有 1-3 人，环境变量方案完全满足需求，后期如需多管理员再迁移到独立表。

### 生成初始密码 Hash

```bash
# 用 Rust 一次性脚本生成 bcrypt hash
cargo run --example gen_hash -- "your_password"

# 或用 htpasswd 工具（macOS/Linux 自带）
htpasswd -bnBC 12 "" "your_password" | tr -d ':\n'
```

---

## 14. 完整实现代码

### 14.1 `explonz_admin/Cargo.toml` — 新增依赖

在 `[dependencies]` 中追加：

```toml
jsonwebtoken = { workspace = true, optional = true }
bcrypt       = { workspace = true, optional = true }
```

在 `ssr` feature 中追加：

```toml
ssr = [
    # ... 已有项 ...
    "dep:jsonwebtoken",
    "dep:bcrypt",
]
```

---

### 14.2 `server/auth.rs` — 完整实现

> **采用方案**：使用 `explonz_shared::common::auth::get_jwt()` 统一 JWT 编解码，`explonz_shared::common::security::verify_password` 验证 bcrypt 密码。
> 无需在 `explonz_admin` 内额外依赖 `jsonwebtoken` / `bcrypt`，由 `explonz_shared/ssr` feature 提供。

```rust
use explonz_shared::common::dto::AdminUser;
use leptos::prelude::*;

// ── get_current_user ─────────────────────────────────────────────────────────

#[server(GetCurrentUser, "/api")]
pub async fn get_current_user() -> Result<Option<AdminUser>, ServerFnError> {
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
        None => return Ok(None), // 未登录
    };

    // 3. 验证并解码 JWT（get_jwt() 内置签名 + 过期校验）
    match get_jwt().decode(&token) {
        Ok(principal) => Ok(Some(AdminUser {
            id: principal.id,
            name: principal.name,
            email: principal.email,
        })),
        Err(_) => Ok(None), // token 无效或过期
    }
}

// ── admin_login ──────────────────────────────────────────────────────────────

#[server(AdminLogin, "/api")]
pub async fn admin_login(email: String, password: String) -> Result<(), ServerFnError> {
    use axum::http::header::{self, HeaderValue};
    use explonz_shared::common::auth::{get_jwt, Principal};
    use explonz_shared::common::security::verify_password;
    use leptos_axum::ResponseOptions;

    // 1. 读取环境变量中的管理员凭据
    let admin_email = std::env::var("ADMIN_ACCOUNT")
        .map_err(|_| ServerFnError::new("Server misconfigured: ADMIN_ACCOUNT missing"))?;
    let admin_hash = std::env::var("ADMIN_PASSWORD_HASH")
        .map_err(|_| ServerFnError::new("Server misconfigured: ADMIN_PASSWORD_HASH missing"))?;

    // 2. 验证 email
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
    let (access_token, _) = get_jwt()
        .encode(principal, true)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // 5. 写入 HttpOnly Cookie
    let response_opts = use_context::<ResponseOptions>()
        .ok_or_else(|| ServerFnError::new("No ResponseOptions in context"))?;
    let cookie = format!(
        "access_token={access_token}; HttpOnly; SameSite=Strict; Path=/; Max-Age=3600"
        // 生产环境在 Max-Age 后追加 "; Secure"
    );
    response_opts.insert_header(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).map_err(|e| ServerFnError::new(e.to_string()))?,
    );

    Ok(())
}
```

**关键依赖**（已在 `explonz_admin/Cargo.toml` ssr feature 中配置）：

| 符号 | 来源 |
|------|------|
| `get_jwt()`, `Principal` | `explonz_shared::common::auth`（ssr feature） |
| `verify_password` | `explonz_shared::common::security`（ssr feature） |
| `AdminUser` | `explonz_shared::common::dto`（无 feature gate） |
| `CookieJar` | `axum_extra::extract`（`axum-extra` optional dep） |
| `ResponseOptions` | `leptos_axum` |

---

### 14.3 `pages/login/login.rs` — 修改部分

在 use 列表顶部追加：

```rust
use crate::server::auth::AdminLogin;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;
```

替换 `login_action` 及其后的逻辑（在 `LoginPage` 函数体内）：

```rust
let login_action = ServerAction::<AdminLogin>::new();

// 登录成功后跳转 dashboard
let navigate = use_navigate();
Effect::new(move |_| {
    if let Some(Ok(())) = login_action.value().get() {
        navigate("/dashboard", NavigateOptions::default());
    }
});

// 错误信息展示
let error_msg = move || {
    login_action.value().get().and_then(|r| r.err()).map(|e| e.to_string())
};
```

在 `<ActionForm>` 内的两个 `<Input>` 标签上补充 `name` 属性（ActionForm 依赖 name 做序列化）：

```rust
// email input
<Input
    attr:r#type="email"
    attr:name="email"      // ← 必须
    attr:id="email"
    ...
/>

// password input
<Input
    node_ref=password_input_ref
    attr:r#type="password"
    attr:name="password"   // ← 必须
    attr:id="password"
    ...
/>
```

在提交按钮上方展示错误：

```rust
{move || error_msg().map(|msg| view! {
    <p class="text-sm text-destructive">{msg}</p>
})}
<Button
    class="w-full"
    attr:disabled=move || login_action.pending().get()
>
    {move || if login_action.pending().get() { "Logging in..." } else { "Login" }}
</Button>
```

password = bobbybobby
password_hash = "$2b$12$sTj.1a7akgwVOGpDikTKsO8raBcG9MJ8t/OCSPlBT.Dg1sl2/Prx6"

---

## 15. 关于调用 workspace 中 `get_jwt().encode(principal, true)` 的分析

### 15.1 `get_jwt()` 在哪里

`get_jwt()` 定义在 `explonz_bnd`（根 package）的 `src/infrastructure/auth.rs:159`：

```rust
pub fn get_jwt() -> &'static Jwt {
    &JWT_INSTANCE
}
```

返回全局单例 `&'static Jwt`。`encode` 签名为：

```rust
pub fn encode(&self, principal: Principal, is_with_exp: bool) -> anyhow::Result<(String, Option<u64>)>
```

- 返回 `(access_token: String, expires_at: Option<u64>)`
- `is_with_exp = true` 时同时返回过期时间戳

`explonz_bnd/src/service/auth.rs:555` 已有完整使用范例：

```rust
use crate::infrastructure::auth::{get_jwt, Principal};

let principal = Principal {
    id: login_user.id.to_string(),
    name: login_user.nickname.clone(),
    email: login_user.email.clone().unwrap_or_default(),
};

let (access_token, access_token_expires_at) = get_jwt().encode(principal, true)?;
```

### 15.2 为什么 `explonz_admin` 不能直接 `use explonz_bnd::...`

`explonz_admin/Cargo.toml` 中有一行被注释掉的依赖：

```toml
# explonz_bnd = { path = "../../explonz_bnd" }
```

原因有两点：

1. **编译目标冲突**：`explonz_admin` 同时编译为 `wasm32`（hydrate feature）和 `x86_64`（ssr feature）。`explonz_bnd` 的众多依赖（`sea-orm`、`reqwest`、`resend-rs` 等）不支持 wasm32，会导致 hydrate 编译失败。
2. **架构耦合**：`explonz_admin` 是独立的 Admin 前端服务，`explonz_bnd` 是 REST API 后端服务，两者应保持解耦。

### 15.3 根本原因

JWT 工具代码（`Jwt`、`Principal`、`get_jwt()`）目前只存在于 `explonz_bnd/src/infrastructure/auth.rs`，没有放入可跨 crate 共享的 `explonz_shared`，导致 `explonz_admin` 无法访问。

### 15.4 推荐方案：将 JWT 代码迁移至 `explonz_shared`

`explonz_shared` 已被两个 crate 共同依赖，且已有 `ssr` feature gate 机制，是放置 JWT 工具的最合适位置。

**步骤一**：为 `explonz_shared` 添加所需依赖（`explonz_shared/Cargo.toml`）

```toml
[dependencies]
jsonwebtoken = { workspace = true, optional = true }
sha2         = { workspace = true, optional = true }
hex          = { workspace = true, optional = true }
xid          = { workspace = true, optional = true }

[features]
ssr = [
    "dep:sea-orm",
    "dep:bcrypt",
    "dep:anyhow",
    "dep:jsonwebtoken",  # 新增
    "dep:sha2",          # 新增
    "dep:hex",           # 新增
    "dep:xid",           # 新增
]
```

**步骤二**：新建 `explonz_shared/src/common/auth.rs`，将 `explonz_bnd/src/infrastructure/auth.rs` 的全部内容复制过来，并在 `explonz_shared/src/common/mod.rs` 中导出：

```rust
#[cfg(feature = "ssr")]
pub mod auth;
```

**步骤三**：更新 `explonz_bnd` 的导入（`src/service/auth.rs`、`src/middleware.rs`）：

```rust
// 修改前
use crate::infrastructure::auth::{get_jwt, Principal};

// 修改后
use explonz_shared::common::auth::{get_jwt, Principal};
```

`src/infrastructure/auth.rs` 可改为 re-export：

```rust
pub use explonz_shared::common::auth::*;
```

**步骤四**：在 `admin_login` 中调用（`explonz_admin/Cargo.toml` 的 `ssr` feature 已包含 `"explonz_shared/ssr"`，无需额外配置）：

```rust
use explonz_shared::common::auth::{get_jwt, Principal};

let principal = Principal {
    id: "admin".to_string(),
    name: "Admin".to_string(),
    email: email.clone(),
};

let (access_token, _expires_at) = get_jwt()
    .encode(principal, true)
    .map_err(|e| ServerFnError::new(e.to_string()))?;
```

### 15.5 方案对比

| 方案 | 可行性 | 优点 | 缺点 |
|------|--------|------|------|
| **迁移 JWT 至 `explonz_shared`（推荐）** | 高 | 架构清晰，符合现有模式 | 需要少量重构 |
| 添加 `explonz_bnd` 为 `explonz_admin` 依赖 | 低 | 改动小 | wasm32 编译失败，架构耦合 |
| 在 `explonz_admin` 内自行实现 JWT（第 14.2 节方案） | 高 | 无需重构，立即可用 | JWT 逻辑与主服务不共享，密钥管理需单独配置 |

> 第 14.2 节的实现采用了"在 admin 内独立实现 JWT"的方式，短期可用。
> 长期建议按本节推荐方案将 JWT 迁移到 `explonz_shared`，统一密钥和算法配置。
