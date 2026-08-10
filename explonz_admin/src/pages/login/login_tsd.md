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

1. **JWT Secret**：`infrastructure/auth.rs` 中 `DEFAULT_KEY` 为硬编码占位值，上线前必须替换为环境变量读取（`AppConfig` 已有配置体系可利用）
2. **Cookie 属性**：生产环境须设 `Secure`（需 HTTPS），本地开发可去掉
3. **管理员身份验证**：`admin_login` 中应验证用户具有 admin 角色，避免普通用户登录后台
4. **Rate Limiting**：`tower-http` 已在依赖中，可在 `main.rs` 的 Axum Router 上为 `/api/AdminLogin` 路径加限速中间件
