# Sidenav02Routes 路由结构分析

## 1. 路由树完整展开

从 `app.rs` 开始，`Sidenav02Routes` 在 `AuthGuard` 内的完整展开：

```
/ (AuthGuard)
└── view/                          ← SidenavRoutes::view_segment() = "view"
    └── sidenav-02/                ← SidenavRoutes::Sidenav02.as_ref() = "sidenav-02"
        │                             view = SidenavLayout（侧边栏框架 + Outlet）
        └── explonz/               ← ExplonzRoutes::base_segment() = "explonz"
            │                         view = SidenavInsetRight（内容区 header + Outlet）
            ├── (空)               ← Route path=""   view = || ()
            ├── components/        ← ExplonzRoutes::Components.as_ref() = "components"
            │   ├── (空)           ← Route path=""   view = || ()
            │   └── *              ← WildcardSegment("component_path")   view = || ()
            └── hooks/             ← ExplonzRoutes::Hooks.as_ref() = "hooks"
                ├── (空)           ← Route path=""   view = || ()
                └── *              ← WildcardSegment("hook_path")   view = || ()
```

### 当前生效的 URL

| URL | 渲染组件链 |
|-----|-----------|
| `/view/sidenav-02/explonz` | `SidenavLayout` → `SidenavInsetRight`（空） |
| `/view/sidenav-02/explonz/components` | `SidenavLayout` → `SidenavInsetRight`（空） |
| `/view/sidenav-02/explonz/components/{任意}` | `SidenavLayout` → `SidenavInsetRight`（空） |
| `/view/sidenav-02/explonz/hooks` | `SidenavLayout` → `SidenavInsetRight`（空） |
| `/view/sidenav-02/explonz/hooks/{任意}` | `SidenavLayout` → `SidenavInsetRight`（空） |

叶子路由当前全部渲染空（`view = || ()`），页面内容区为空白。

---

## 2. 代码文件与路由段的对应关系

构成这棵路由树的代码分布在 4 个文件：

```
app.rs
└── Sidenav02Routes（透明组件展开）
    └── pages/home/index.rs
        └── SidenavRoutesSimplified（透明组件展开）
            └── components/blocks/sidenav_routes_simplified.rs
                └── 依赖枚举常量
                    └── components/blocks/sidenav_routes.rs
```

---

## 3. 每段 URL 的来源

| URL 段 | 当前值 | 来源 | 文件与行 |
|--------|--------|------|----------|
| `view` | `"view"` | `SidenavRoutes::view_segment()` 返回值 | `sidenav_routes.rs:25-27` |
| `sidenav-02` | `"sidenav-02"` | `SidenavRoutes::Sidenav02` strum kebab-case 序列化 | `sidenav_routes.rs:12` |
| `explonz` | `"explonz"` | `ExplonzRoutes::base_segment()` 返回值 | `sidenav_routes.rs:62-64` |
| `components` | `"components"` | `ExplonzRoutes::Components` strum kebab-case 序列化 | `sidenav_routes.rs:57` |
| `hooks` | `"hooks"` | `ExplonzRoutes::Hooks` strum kebab-case 序列化 | `sidenav_routes.rs:58` |
| `{任意}` | 通配 | `WildcardSegment("component_path" / "hook_path")` | `sidenav_routes_simplified.rs:39,45` |

---

## 4. 如何修改 URL

### 4.1 修改第一段（`view` → 其他）

只改 `sidenav_routes.rs` 的 `view_segment()` 方法：

```rust
// 当前
pub fn view_segment() -> &'static str {
    "view"
}

// 修改示例：改为 "admin"
pub fn view_segment() -> &'static str {
    "admin"
}
```

**影响范围**：所有 `SidenavRoutes` 下的路由都跟着改，`to_route()`、`base_url_with_sidenav()` 等方法自动更新。

---

### 4.2 修改第二段（`sidenav-02` → 其他）

有两种方式，根据需求选择：

**方式 A：改枚举变体名**（strum 自动生成 URL 段，影响全局）

```rust
// 当前
pub enum SidenavRoutes {
    Sidenav02,  // → "sidenav-02"
    ...
}

// 改为
pub enum SidenavRoutes {
    Home,  // → "home"
    ...
}
```

同时需要更新所有引用 `SidenavRoutes::Sidenav02` 的地方（`app.rs`、`index.rs`、`sidenav_inset_right.rs` 等）。

**方式 B：在 `index.rs` 直接写死 `StaticSegment`**（只影响这一条路由，枚举不动）

```rust
// index.rs:28，当前
path=StaticSegment(SidenavRoutes::Sidenav02.as_ref())

// 改为直接写死
path=StaticSegment("home")
```

---

### 4.3 修改第三段（`explonz` → 其他）

改 `sidenav_routes.rs` 的 `ExplonzRoutes::base_segment()` 方法：

```rust
// 当前
pub fn base_segment() -> &'static str {
    "explonz"
}

// 改回 "docs" 或其他值
pub fn base_segment() -> &'static str {
    "docs"
}
```

**影响范围**：`SidenavRoutesSimplified` 的根路径段，以及 `ComponentsRoutes::base_url_with_sidenav()`、`HooksRoutes::base_url_with_sidenav()` 生成的 URL。

---

### 4.4 修改第四段（`components` / `hooks` → 其他）

改 `ExplonzRoutes` 的枚举变体名（strum kebab-case 自动生成 URL 段）：

```rust
// 当前
pub enum ExplonzRoutes {
    Components,  // → "components"
    Hooks,       // → "hooks"
}

// 改为
pub enum ExplonzRoutes {
    Ui,     // → "ui"
    Utils,  // → "utils"
}
```

---

## 5. 如何修改对应的路由组件

路由树有三层渲染组件，各自独立：

### 5.1 层一：整体 Layout（含侧边栏框架）

**控制文件**：`pages/home/index.rs:27-34`（`Sidenav02Routes` 内）

```rust
// 当前：使用 SidenavLayout
<ParentRoute
    path=StaticSegment(SidenavRoutes::Sidenav02.as_ref())
    view=move || view! { <SidenavLayout sidenav_route=SidenavRoutes::Sidenav02 /> }
>

// 修改：替换为自定义 Layout
<ParentRoute
    path=StaticSegment(SidenavRoutes::Sidenav02.as_ref())
    view=move || view! { <MyAdminLayout /> }
>
```

---

### 5.2 层二：内容区外壳（Header + Outlet）

**控制文件**：`components/blocks/sidenav_routes_simplified.rs:17-33`（`SidenavRoutesSimplified` 内）

```rust
// 当前：使用 SidenavInsetRight
<ParentRoute
    path=StaticSegment(ExplonzRoutes::base_segment())
    view=move || view! { <SidenavInsetRight /> }
>

// 修改：替换为自定义内容区组件
<ParentRoute
    path=StaticSegment(ExplonzRoutes::base_segment())
    view=move || view! { <MyContentShell /> }
>
```

---

### 5.3 层三：叶子页面（当前全为空）

**控制文件**：`components/blocks/sidenav_routes_simplified.rs:34-46`

当前所有叶子路由 `view = || ()` 渲染空内容，挂载真实页面组件需要逐一替换：

```rust
// 当前（空渲染）
<ParentRoute path=StaticSegment(ExplonzRoutes::Components.as_ref()) view=|| view! { <Outlet /> }>
    <Route path=StaticSegment("") view=|| () />
    <Route path=WildcardSegment("component_path") view=|| () />
</ParentRoute>

// 修改：挂载真实页面组件
<ParentRoute path=StaticSegment(ExplonzRoutes::Components.as_ref()) view=|| view! { <Outlet /> }>
    <Route path=StaticSegment("") view=ComponentsIndexPage />
    <Route path=WildcardSegment("component_path") view=ComponentDetailPage />
</ParentRoute>
```

如果不同路由需要挂不同组件，可以把通配符拆成具体的 `StaticSegment`：

```rust
<ParentRoute path=StaticSegment(ExplonzRoutes::Components.as_ref()) view=|| view! { <Outlet /> }>
    <Route path=StaticSegment("") view=ComponentsIndexPage />
    <Route path=StaticSegment("accordion") view=AccordionPage />
    <Route path=StaticSegment("button")    view=ButtonPage />
    <Route path=StaticSegment("dialog")    view=DialogPage />
</ParentRoute>
```

---

## 6. 已发现的 Bug：侧边栏链接与实际路由不匹配

`pages/home/index.rs` 末尾的 `COMPONENT_LINKS` / `HOOKS_LINKS` 常量还使用旧格式，与当前路由不符：

```rust
// 当前常量（旧格式，404）
"/view/sidenav02/docs/components/accordion"
//         ↑ 无连字符   ↑ 旧 segment

// 当前路由实际生效的路径
"/view/sidenav-02/explonz/components/accordion"
//          ↑ 有连字符    ↑ 已改为 explonz
```

**需要同步更新的常量（`index.rs:189-212`）：**

```rust
// 修改后
const COMPONENT_LINKS: &[(&str, &str)] = &[
    ("/view/sidenav-02/explonz/components/accordion",    "Accordion"),
    ("/view/sidenav-02/explonz/components/alert",        "Alert"),
    ("/view/sidenav-02/explonz/components/alert-dialog", "Alert Dialog"),
    ("/view/sidenav-02/explonz/components/button",       "Button"),
    ("/view/sidenav-02/explonz/components/card",         "Card"),
    ("/view/sidenav-02/explonz/components/checkbox",     "Checkbox"),
    ("/view/sidenav-02/explonz/components/dialog",       "Dialog"),
];

const HOOKS_LINKS: &[(&str, &str)] = &[
    ("/view/sidenav-02/explonz/hooks/use-copy-clipboard",    "Use Copy Clipboard"),
    ("/view/sidenav-02/explonz/hooks/use-lock-body-scroll",  "Use Lock Body Scroll"),
    ("/view/sidenav-02/explonz/hooks/use-random",            "Use Random"),
];
```

---

## 7. 改动影响矩阵

| 想改的内容 | 改哪个文件 | 改哪一行 | 影响范围 |
|-----------|-----------|---------|---------|
| 第一段 `view` | `sidenav_routes.rs` | `view_segment()` 函数体 | 全部 SidenavRoutes URL |
| 第二段 `sidenav-02` | `index.rs` | `Sidenav02Routes` 的 `StaticSegment` | 仅此路由 |
| 第三段 `explonz` | `sidenav_routes.rs` | `ExplonzRoutes::base_segment()` 函数体 | 所有内容区路由 |
| 第四段 `components/hooks` | `sidenav_routes.rs` | `ExplonzRoutes` 枚举变体名 | 对应子路由 |
| 整体 Layout 组件 | `index.rs` | `Sidenav02Routes` 的 `view=` | 侧边栏框架渲染 |
| 内容区外壳组件 | `sidenav_routes_simplified.rs` | `SidenavRoutesSimplified` 根 `ParentRoute` 的 `view=` | 内容区 header 渲染 |
| 叶子页面组件 | `sidenav_routes_simplified.rs` | 各 `Route` 的 `view=` | 对应 URL 的页面内容 |
| 侧边栏链接文字/地址 | `index.rs` | `COMPONENT_LINKS` / `HOOKS_LINKS` 常量 | 侧边栏导航项 |
