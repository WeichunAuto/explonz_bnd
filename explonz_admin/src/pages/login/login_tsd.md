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

---

## 16. 保存原 URL + 登录后回跳方案

### 16.1 总体流程

```
用户访问 /dashboard（未登录）
    ↓
AuthGuard 判断未登录
    ↓
重定向到 /login?redirect=%2Fdashboard&reason=login_required
    ↓
LoginPage 读取 reason 参数 → 展示对应提示语
LoginPage 读取 redirect 参数 → 登录成功后跳回
    ↓
用户提交表单，登录成功
    ↓
navigate("/dashboard")  ← 从 redirect 参数读取
```

---

### 16.2 `reason` 参数值约定

| reason 值 | 触发场景 | LoginPage 展示文案 |
|-----------|----------|-------------------|
| `login_required` | 无 Cookie，从未登录 | "请先登录以继续" |
| `session_expired` | Cookie 存在但 JWT 已过期/无效 | "登录已过期，请重新登录" |

---

### 16.3 （可选）改动 `get_current_user` 以区分 reason

当前 `get_current_user` 将"无 Cookie"和"JWT 过期"都归并为 `Ok(None)`，无法区分。若需展示不同文案，需调整返回类型。

**方案 A：新增 `AuthStatus` 枚举（推荐）**

在 `explonz_shared/src/common/dto.rs` 新增：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthStatus {
    Authenticated(AdminUser),
    NotLoggedIn,    // 无 access_token Cookie
    TokenExpired,   // Cookie 存在但 JWT 校验失败
}
```

`get_current_user` 返回类型改为 `Result<AuthStatus, ServerFnError>`，`AuthGuard` 据此选择 reason 值。

**方案 B：统一使用 `login_required`（更简单）**

若不需要区分，跳过此步骤，始终传 `reason=login_required`。

---

### 16.4 改动一：`pages/auth_guard.rs` — 构建带参数的跳转 URL

在 `AuthGuard` 组件中：

1. 引入 `use_location` 读取当前路径（SSR 和 Client 均可用）
2. 拼接 `/login?redirect={pathname}&reason={reason}` 字符串
3. SSR 分支传给 `leptos_axum::redirect`，Client 分支传给 `navigate`

**关键改动思路（伪代码）：**

```rust
// 1. 引入
use leptos_router::hooks::use_location;

// 2. 在组件内获取当前路径
let location = use_location();

// 3. 在未登录分支构建 URL
let pathname = location.pathname.get_untracked();
// 简单路径无需 percent-encode（admin 路由仅含字母/数字/连字符）
let login_url = format!("/login?redirect={}&reason=login_required", pathname);

// SSR 分支
#[cfg(feature = "ssr")]
leptos_axum::redirect(&login_url);

// Client 分支
#[cfg(not(feature = "ssr"))]
navigate(&login_url, NavigateOptions::default());
```

若已按 16.3 区分了 `AuthStatus`，则根据状态选择 reason：

```rust
let reason = match result {
    AuthStatus::NotLoggedIn  => "login_required",
    AuthStatus::TokenExpired => "session_expired",
    _                        => "login_required",
};
let login_url = format!("/login?redirect={}&reason={}", pathname, reason);
```

---

### 16.5 改动二：`pages/login/login.rs` — 读取参数、展示提示、登录后回跳

#### 读取 query params

```rust
use leptos_router::hooks::use_query_map;

let query         = use_query_map();
let redirect_path = move || query.read().get("redirect").cloned().unwrap_or_default();
let reason        = move || query.read().get("reason").cloned().unwrap_or_default();
```

#### 展示 reason 提示语（放在表单最上方）

```rust
{move || {
    let msg = match reason().as_str() {
        "session_expired" => Some("登录已过期，请重新登录"),
        "login_required"  => Some("请先登录以继续"),
        _                 => None,
    };
    msg.map(|text| view! {
        <p class="text-sm text-amber-600 text-center">{text}</p>
    })
}}
```

#### 登录成功后回跳（替换当前写死的目标路径）

将现有的：

```rust
Ok(_) => navigate("/view/sidenav02/docs", NavigateOptions::default()),
```

改为：

```rust
Ok(_) => {
    let dest = safe_redirect_dest(redirect_path());
    navigate(&dest, NavigateOptions::default());
}
```

其中 `safe_redirect_dest` 是一个校验函数（见 16.6）。

---

### 16.6 安全校验：防止开放重定向（Open Redirect）

`redirect` 参数是用户可控的 URL 参数，必须校验，否则攻击者可构造：
```
/login?redirect=//evil.com
```
让登录成功后跳转到外部恶意站点。

**校验函数：**

```rust
fn safe_redirect_dest(raw: String) -> String {
    // 规则：
    // 1. 必须以 / 开头（站内路径）
    // 2. 不能以 // 开头（协议相对 URL，会跳出站外）
    // 3. 不能跳回 /login 自身（防止循环）
    let valid = raw.starts_with('/')
        && !raw.starts_with("//")
        && !raw.starts_with("/login");

    if valid { raw } else { "/dashboard".to_string() }
}
```

---

### 16.7 改动文件清单

| 文件 | 改动内容 | 是否必须 |
|------|----------|----------|
| `pages/auth_guard.rs` | 引入 `use_location`，拼接带 `redirect` 和 `reason` 的 login URL | 必须 |
| `pages/login/login.rs` | 引入 `use_query_map`，读取 reason 展示提示，登录后 navigate 到 redirect 参数 | 必须 |
| `explonz_shared/src/common/dto.rs` | 新增 `AuthStatus` 枚举 | 可选（区分 reason 时需要） |
| `server/auth.rs` | `get_current_user` 返回 `AuthStatus` | 可选（区分 reason 时需要） |

---

## 17. `get_current_user` 返回 `AuthStatus` 详细改造

### 17.1 前置状态确认

`AuthStatus` **已存在**于 `explonz_shared/src/common/dto.rs`（99–104 行），无需新增：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthStatus {
    Authenticated(AdminUser),
    NotLoggedIn,    // 无 access_token Cookie
    TokenExpired,   // Cookie 存在但 JWT 校验失败
}
```

---

### 17.2 改动一：`explonz_admin/src/server/auth.rs`

#### 17.2.1 改动点一览

| 行 | 改动前 | 改动后 |
|----|--------|--------|
| 第 1 行 import | `use explonz_shared::common::dto::AdminUser;` | `use explonz_shared::common::dto::{AdminUser, AuthStatus};` |
| 第 6 行 返回类型 | `Result<Option<AdminUser>, ServerFnError>` | `Result<AuthStatus, ServerFnError>` |
| 第 21 行（无 Cookie） | `return Ok(None);` | `return Ok(AuthStatus::NotLoggedIn);` |
| 第 26–30 行（解码成功） | `Ok(principal) => Ok(Some(AdminUser { ... }))` | `Ok(principal) => Ok(AuthStatus::Authenticated(AdminUser { ... }))` |
| 第 31 行（解码失败） | `Err(_) => Ok(None),` | `Err(_) => Ok(AuthStatus::TokenExpired),` |

#### 17.2.2 改动后完整函数

```rust
use explonz_shared::common::dto::{AdminUser, AuthStatus};  // ← AuthStatus 补入
use leptos::server;
use leptos_ui::clx::{use_context, ServerFnError};

#[server]
pub async fn get_current_user() -> Result<AuthStatus, ServerFnError> {  // ← 返回类型
    use axum_extra::extract::CookieJar;
    use explonz_shared::common::auth::get_jwt;
    use leptos_axum::extract;

    // 1. 提取 Cookie Jar
    let jar: CookieJar = extract()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // 2. 读取 access_token cookie
    //    无 Cookie → 从未登录
    let token = match jar.get("access_token") {
        Some(c) => c.value().to_string(),
        None => return Ok(AuthStatus::NotLoggedIn),  // ← 原 Ok(None)
    };

    // 3. 验证并解码 JWT
    match get_jwt().decode(&token) {
        Ok(principal) => Ok(AuthStatus::Authenticated(AdminUser {  // ← 原 Ok(Some(...))
            id: principal.id,
            name: principal.name,
            email: principal.email,
        })),
        Err(_) => Ok(AuthStatus::TokenExpired),  // ← 原 Ok(None)
    }
}
```

> **为什么解码错误统一归为 `TokenExpired`？**
> `get_jwt().decode()` 内部使用 `anyhow::Result`，错误经过包装后无法用 `downcast`
> 区分 `ExpiredSignature` 与 `InvalidSignature`。实践上当 Cookie 存在时解码失败的
> 最常见原因是过期，统一显示"登录已过期"对用户最友好，且不泄露签名细节。
> 如后续需要精细区分，可让 `Jwt::decode` 直接返回 `jsonwebtoken::errors::Error`。

---

### 17.3 改动二：`explonz_admin/src/pages/auth_guard.rs`

#### 17.3.1 import 变化

```rust
// 新增
use explonz_shared::common::dto::AuthStatus;
use leptos_router::hooks::use_location;

// 保持（仅 hydrate 编译）
#[cfg(not(feature = "ssr"))]
use leptos_router::{hooks::use_navigate, NavigateOptions};

// 移除（不再需要单独引入 get_current_user 的旧返回类型相关 import）
```

#### 17.3.2 组件函数体变化

在 `Resource::new` 之后，`view!` 之前，新增一行：

```rust
let location = use_location();
```

#### 17.3.3 match 结构变化

**改动前（3 个分支）：**
```rust
match user.get() {
    None                           => view! { <Outlet/> }.into_any(),
    Some(Ok(None)) | Some(Err(_)) => { navigate("/login", ...); view! {}.into_any() }
    Some(Ok(Some(_user)))          => view! { <Outlet/> }.into_any(),
}
```

**改动后（4 个分支）：**
```rust
match user.get() {
    // 资源未就绪，由 Suspense fallback 处理
    None => view! { <Outlet/> }.into_any(),

    // 已登录 → 正常渲染子路由
    Some(Ok(AuthStatus::Authenticated(_))) => view! { <Outlet/> }.into_any(),

    // 无 Cookie（从未登录）→ reason=login_required
    Some(Ok(AuthStatus::NotLoggedIn)) => {
        let pathname = location.pathname.get_untracked();
        let url = format!("/login?redirect={}&reason=login_required", pathname);

        #[cfg(feature = "ssr")]
        leptos_axum::redirect(&url);

        #[cfg(not(feature = "ssr"))]
        navigate(&url, NavigateOptions::default());

        view! {}.into_any()
    }

    // Cookie 存在但 JWT 无效/过期 → reason=session_expired
    Some(Ok(AuthStatus::TokenExpired)) => {
        let pathname = location.pathname.get_untracked();
        let url = format!("/login?redirect={}&reason=session_expired", pathname);

        #[cfg(feature = "ssr")]
        leptos_axum::redirect(&url);

        #[cfg(not(feature = "ssr"))]
        navigate(&url, NavigateOptions::default());

        view! {}.into_any()
    }

    // Server Function 本身调用出错 → 安全起见按未登录处理
    Some(Err(_)) => {
        #[cfg(feature = "ssr")]
        leptos_axum::redirect("/login?reason=login_required");

        #[cfg(not(feature = "ssr"))]
        navigate("/login?reason=login_required", NavigateOptions::default());

        view! {}.into_any()
    }
}
```

#### 17.3.4 完整改动后文件预览

```rust
use leptos::prelude::*;
use leptos::{component, server::Resource, view, IntoView};
use leptos_router::components::Outlet;
use leptos_router::hooks::use_location;
use leptos_ui::clx::{IntoAny, Suspense};

#[cfg(not(feature = "ssr"))]
use leptos_router::{hooks::use_navigate, NavigateOptions};

use explonz_shared::common::dto::AuthStatus;
use crate::server::auth::get_current_user;

#[component]
pub fn AuthGuard() -> impl IntoView {
    let user     = Resource::new(|| (), |_| get_current_user());
    let location = use_location();

    #[cfg(not(feature = "ssr"))]
    let navigate = use_navigate();

    view! {
        <Suspense fallback=move || view! {
            <div class="flex h-screen items-center justify-center">"load..."</div>
        }>
            {move || {
                match user.get() {
                    None =>
                        view! { <Outlet/> }.into_any(),

                    Some(Ok(AuthStatus::Authenticated(_))) =>
                        view! { <Outlet/> }.into_any(),

                    Some(Ok(AuthStatus::NotLoggedIn)) => {
                        let url = format!(
                            "/login?redirect={}&reason=login_required",
                            location.pathname.get_untracked()
                        );
                        #[cfg(feature = "ssr")]
                        leptos_axum::redirect(&url);
                        #[cfg(not(feature = "ssr"))]
                        navigate(&url, NavigateOptions::default());
                        view! {}.into_any()
                    }

                    Some(Ok(AuthStatus::TokenExpired)) => {
                        let url = format!(
                            "/login?redirect={}&reason=session_expired",
                            location.pathname.get_untracked()
                        );
                        #[cfg(feature = "ssr")]
                        leptos_axum::redirect(&url);
                        #[cfg(not(feature = "ssr"))]
                        navigate(&url, NavigateOptions::default());
                        view! {}.into_any()
                    }

                    Some(Err(_)) => {
                        #[cfg(feature = "ssr")]
                        leptos_axum::redirect("/login?reason=login_required");
                        #[cfg(not(feature = "ssr"))]
                        navigate("/login?reason=login_required", NavigateOptions::default());
                        view! {}.into_any()
                    }
                }
            }}
        </Suspense>
    }
}
```

---

### 17.4 改动三：`explonz_admin/src/pages/login/login.rs`

#### 17.4.1 新增 import

```rust
use leptos_router::hooks::use_query_map;
```

#### 17.4.2 组件函数体：新增 query 读取（放在 `let show_password` 之后）

```rust
// 读取 URL 查询参数
let query         = use_query_map();
let redirect_path = move || query.read().get("redirect").cloned().unwrap_or_default();
let reason        = move || query.read().get("reason").cloned().unwrap_or_default();
```

#### 17.4.3 修改 Effect：登录成功后跳转到 redirect 参数

```rust
// 改动前
Effect::new(move |_| {
    let rs_opt = login_action.value().get();
    if let Some(rs) = rs_opt {
        match rs {
            Ok(_) => navigate("/view/sidenav02/docs", NavigateOptions::default()),
            Err(e) => leptos::logging::log!("服务器返回错误: {}", e),
        };
    }
});

// 改动后
Effect::new(move |_| {
    if let Some(rs) = login_action.value().get() {
        match rs {
            Ok(_) => {
                let raw = redirect_path();
                // 安全校验：站内路径 + 不回到 /login（防循环）
                let dest = if raw.starts_with('/')
                    && !raw.starts_with("//")
                    && !raw.starts_with("/login")
                {
                    raw
                } else {
                    "/dashboard".to_string()
                };
                navigate(&dest, NavigateOptions::default());
            }
            Err(e) => leptos::logging::log!("登录失败: {}", e),
        }
    }
});
```

#### 17.4.4 视图：在 `CardHeader` 下方插入 reason 提示语

在 `<CardHeader>...</CardHeader>` 关闭标签之后、`<CardContent>` 之前插入：

```rust
{move || {
    let msg = match reason().as_str() {
        "session_expired" => Some("登录已过期，请重新登录"),
        "login_required"  => Some("请先登录以继续"),
        _                 => None,
    };
    msg.map(|text| view! {
        <p class="px-6 pb-2 text-sm text-amber-600 text-center">{text}</p>
    })
}}
```

---

### 17.5 完整改动文件清单

| 文件 | 改动内容 | 改动量 |
|------|----------|--------|
| `explonz_shared/src/common/dto.rs` | 无需改动（`AuthStatus` 已存在） | 0 行 |
| `explonz_admin/src/server/auth.rs` | import + 返回类型 + 3 处返回值 | ~5 行 |
| `explonz_admin/src/pages/auth_guard.rs` | import + `use_location` + match 由 3 臂改为 4 臂 | ~25 行 |
| `explonz_admin/src/pages/login/login.rs` | import + query 读取 + Effect + reason 提示语 | ~15 行 |
