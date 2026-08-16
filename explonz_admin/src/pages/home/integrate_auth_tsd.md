# Sidenav02Routes 整合到 AuthGuard 方案

## 1. 现状分析

### 当前路由结构（`app.rs`）

```
<Routes>
    <Sidenav02Routes />                          // /view/sidenav02/...  ← 无鉴权保护
    <Route path="/login"  view=LoginPage/>
    <ParentRoute path="/" view=AuthGuard>
        <Route path="/dashboard" view=Dashboard/>
        <Route path="/posts"     view=PostList/>
        <Route path="/spots"     view=SpotList/>
    </ParentRoute>
</Routes>
```

**问题**：`Sidenav02Routes` 游离于 `AuthGuard` 之外，用户在 token 过期或未登录的情况下仍可直接访问 `/view/sidenav02/...`，不会触发自动跳转登录。

---

### 各关键模块职责

| 模块 | 路径 | 职责 |
|------|------|------|
| `AuthGuard` | `pages/auth_guard.rs` | 调用 `get_current_user()` 验证 token，失效时 navigate 到 `/login` |
| `get_current_user` | `server/auth.rs` | 读取 `access_token` Cookie，用 JWT 解码验证；过期/无效返回 `Ok(None)` |
| `Sidenav02Routes` | `pages/home/index.rs` | `#[component(transparent)]`，展开为 `/view/sidenav02/docs/...` 子树 |
| `SidenavLayout` | `pages/home/index.rs` | 带侧边栏的 layout，作为 Sidenav02Routes 的中间 view |

---

## 2. 鉴权逻辑确认

`AuthGuard` 的判断分支已经覆盖 token 过期场景：

```rust
match user.get() {
    None                    => view! { <Outlet/> },   // 数据加载中，Suspense 已显示 loading
    Some(Ok(None)) | Some(Err(_)) => {                // 未登录 或 token 无效/过期
        navigate("/login", NavigateOptions::default());
        view! {}
    }
    Some(Ok(Some(_user)))   => view! { <Outlet/> },   // 已登录，渲染子路由
}
```

`get_current_user` 在 JWT 解码失败（含过期）时返回 `Ok(None)`，命中上面第二个分支，自动跳转 `/login`。**鉴权逻辑无需修改。**

---

## 3. 整合方案

### 方案：将 `Sidenav02Routes` 移入 `AuthGuard` 的 `ParentRoute` 内部

`Sidenav02Routes` 是 `#[component(transparent)]`，它展开的最外层 segment 是 `StaticSegment("view")`。
`AuthGuard` 的父路由 path 是 `/`，因此子路由完整路径为 `/view/sidenav02/...`，与现有 URL 完全一致，**无需改动任何链接或路由枚举**。

### 改动后路由结构

```
<Routes>
    <Route path="/login" view=LoginPage/>
    <ParentRoute path="/" view=AuthGuard>
        <Sidenav02Routes />                      // /view/sidenav02/...  ← 受 AuthGuard 保护
        <Route path="/dashboard" view=Dashboard/>
        <Route path="/posts"     view=PostList/>
        <Route path="/spots"     view=SpotList/>
    </ParentRoute>
</Routes>
```

---

## 4. 具体代码改动

### 文件：`explonz_admin/src/app.rs`

**改动前：**
```rust
<Routes fallback=|| "404 Not Found">
    <Sidenav02Routes />

    <Route path=path!("/login")  view=LoginPage/>
    <ParentRoute path=path!("/") view=AuthGuard>
        <Route path=path!("/dashboard") view=Dashboard/>
        <Route path=path!("/posts")     view=PostList/>
        <Route path=path!("/spots")     view=SpotList/>
    </ParentRoute>
</Routes>
```

**改动后：**
```rust
<Routes fallback=|| "404 Not Found">
    <Route path=path!("/login")  view=LoginPage/>
    <ParentRoute path=path!("/") view=AuthGuard>
        <Sidenav02Routes />
        <Route path=path!("/dashboard") view=Dashboard/>
        <Route path=path!("/posts")     view=PostList/>
        <Route path=path!("/spots")     view=SpotList/>
    </ParentRoute>
</Routes>
```

**变更说明：**
1. 将 `<Sidenav02Routes />` 从 `<Routes>` 顶层移入 `<ParentRoute path=path!("/") view=AuthGuard>` 内部。
2. 登录页 `<Route path="/login" .../>` 保持在 `AuthGuard` 外部（否则会造成鉴权重定向死循环）。

---

## 5. 用户体验流程（整合后）

```
用户访问 /view/sidenav02/docs/components/button
        ↓
AuthGuard 触发，调用 get_current_user()（Server Function）
        ↓
    ┌── Cookie 存在且 JWT 有效 ──→ 渲染 SidenavLayout + 内容页
    │
    └── Cookie 不存在 / JWT 过期 / JWT 无效
                ↓
        navigate("/login")  ← 自动跳转登录页
```

登录成功后，`admin_login` 写入新的 `access_token` Cookie（Max-Age=3600），用户被重定向回原访问地址即可正常使用。

---

## 6. 注意事项

1. **登录后回跳**：目前 `AuthGuard` 在跳转 `/login` 时使用 `NavigateOptions::default()`，不携带 `?redirect=` 参数。如需登录后回跳原页面，需要在跳转前记录当前 `location.pathname`，并在 `LoginPage` 登录成功后读取该参数进行二次跳转（属于后续优化，不影响本次整合）。

2. **Suspense loading 体验**：`AuthGuard` 包裹所有受保护路由后，每次刷新页面都会有一次 Server Function 网络请求。当前 fallback 是简单的 `"load..."` 文字，可根据设计规范替换为 Skeleton 或 Spinner。

3. **路由匹配顺序**：`Sidenav02Routes` 放在 `ParentRoute` 内部第一个，leptos_router 按声明顺序匹配，不影响其他子路由。

4. **无需改动的内容**：
   - `AuthGuard` 组件本身
   - `get_current_user` Server Function
   - `Sidenav02Routes`、`SidenavLayout` 等组件
   - 所有路由枚举（`SidenavRoutes`、`DocsRoutes` 等）
   - 所有常量链接（`COMPONENT_LINKS`、`HOOKS_LINKS`）

---

## 7. 改动量总结

| 文件 | 改动类型 | 行数变化 |
|------|----------|----------|
| `explonz_admin/src/app.rs` | 移动 1 行代码（`<Sidenav02Routes />`） | 0（净变化） |

**这是一处单行移动，风险极低。**
