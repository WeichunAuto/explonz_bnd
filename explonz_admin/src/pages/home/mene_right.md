# 菜单点击显示右侧对应内容 — 技术方案

## 1. 目标效果

| 左侧菜单点击 | 右侧显示内容 |
|------------|------------|
| Adition    | `SpotAdition` 组件（新建） |
| Spots List | `SpotList` 组件（已存在于 `pages/spots/list.rs`） |

---

## 2. 当前问题诊断

```
SidenavInsetRight（view of ParentRoute "explonz"）
    ↓
    ❌ 143-150 行是硬编码占位 div，没有 <Outlet/>
    ↓
子路由内容永远无处渲染
    ↓
sidenav_routes_simplified.rs 里的 view=|| ()
永远不会显示
```

根本原因：**`SidenavInsetRight` 缺少 `<Outlet/>`，且子路由全部是空渲染 `|| ()`**。

---

## 3. 目标路由结构

```
/view/home/explonz/spots/              ← ExplonzRoutes::Spots = "spots"
    ├── (空)                            ← 默认空页
    ├── list                            ← SpotList 组件
    └── adition                         ← SpotAdition 组件（新建）
```

完整 URL：
- `http://localhost:3000/view/home/explonz/spots/list`
- `http://localhost:3000/view/home/explonz/spots/adition`

---

## 4. 需要改动的文件清单

| 文件 | 改动类型 | 说明 |
|------|---------|------|
| `components/blocks/sidenav_inset_right.rs` | 修改 | 替换占位内容为 `<Outlet/>`，补 import |
| `components/blocks/sidenav_routes_simplified.rs` | 修改 | Wildcard 改为 Static 路由 + 挂真实组件 |
| `components/blocks/sidenav02.rs` | 修改 | 更新 `COMPONENT_LINKS` 为正确 URL |
| `pages/spots/adition.rs` | 新建 | `SpotAdition` 组件 |
| `pages/spots/mod.rs` | 修改 | 声明 `pub mod adition;` |

---

## 5. 各文件具体改动

### 5.1 `components/blocks/sidenav_inset_right.rs`

**改动一：顶部新增 `Outlet` import**

```rust
// 在已有 use 块中加入
use leptos_router::components::Outlet;
```

**改动二：第 143–150 行，替换硬编码占位内容**

```rust
// 改前（硬编码占位，内容永不变）
<div class="flex flex-col flex-1 gap-4 p-4 pt-0">
    <div class="grid auto-rows-min gap-4 md:grid-cols-3">
        <div class="rounded-xl bg-muted/50 aspect-video"></div>
        <div class="rounded-xl bg-muted/50 aspect-video"></div>
        <div class="rounded-xl bg-muted/50 aspect-video"></div>
    </div>
    <div class="flex-1 rounded-xl bg-muted/50 min-h-[100vh] md:min-h-min"></div>
</div>

// 改后（Outlet 承接子路由渲染的组件）
<div class="flex flex-col flex-1 gap-4 p-4 pt-0">
    <Outlet />
</div>
```

---

### 5.2 `components/blocks/sidenav_routes_simplified.rs`

**改动一：顶部新增组件 import**

```rust
use crate::pages::spots::list::SpotList;
use crate::pages::spots::adition::SpotAdition;
```

**改动二：第 37–40 行，Spots 路由从通配符改为具体路由 + 真实组件**

```rust
// 改前（通配符，所有子路由渲染空）
<ParentRoute path=StaticSegment(ExplonzRoutes::Spots.as_ref()) view=|| view! { <Outlet /> }>
    <Route path=StaticSegment("") view=|| () />
    <Route path=WildcardSegment("component_path") view=|| () />
</ParentRoute>

// 改后（具体路由 + 真实组件）
<ParentRoute path=StaticSegment(ExplonzRoutes::Spots.as_ref()) view=|| view! { <Outlet /> }>
    <Route path=StaticSegment("")         view=|| () />
    <Route path=StaticSegment("list")     view=SpotList />
    <Route path=StaticSegment("adition")  view=SpotAdition />
</ParentRoute>
```

---

### 5.3 `components/blocks/sidenav02.rs`

**改动：第 192–203 行，更新 `COMPONENT_LINKS`**

```rust
// 改前（旧格式，路径与实际路由不符，点击 404）
const COMPONENT_LINKS: &[(&str, &str)] = &[
    ("/view/sidenav02/docs/components/accordion", "Accordion"),
    ("/view/sidenav02/docs/components/alert", "Alert"),
    ...
];

// 改后（与实际路由对齐）
const COMPONENT_LINKS: &[(&str, &str)] = &[
    ("/view/home/explonz/spots/adition", "Adition"),
    ("/view/home/explonz/spots/list",    "Spots List"),
];
```

---

### 5.4 新建 `pages/spots/adition.rs`

```rust
use leptos::prelude::*;

#[component]
pub fn SpotAdition() -> impl IntoView {
    view! {
        <div>
            <h1>"Adition"</h1>
        </div>
    }
}
```

---

### 5.5 `pages/spots/mod.rs`

```rust
// 改前
pub mod detail;
pub mod edit;
pub mod list;

// 改后
pub mod adition;  // ← 新增
pub mod detail;
pub mod edit;
pub mod list;
```

---

## 6. 改动后完整渲染链路

```
用户点击左侧菜单 "Spots List"
    ↓
URL 跳转到 /view/home/explonz/spots/list
    ↓
路由匹配：
  ParentRoute "/"       → AuthGuard（鉴权通过）
  ParentRoute "view"    → Outlet
  ParentRoute "home"    → SidenavLayout（左侧边栏）
  ParentRoute "explonz" → SidenavInsetRight（header + Outlet）  ← 改动后有 Outlet
  Route "list"          → SpotList                              ← 真实组件
    ↓
右侧渲染 SpotList 组件 ✅

用户点击左侧菜单 "Adition"
    ↓
URL 跳转到 /view/home/explonz/spots/adition
    ↓
右侧渲染 SpotAdition 组件 ✅
```

---

## 7. 注意事项

1. **`sidenav02.rs` 的 `HOOKS_LINKS`** 目前也是旧格式，若 Hooks 菜单有相同问题，同步按照上述方式修改对应的路由。

2. **`WildcardSegment` 已移除**：改为 `StaticSegment` 后，新增菜单项时需要在 `sidenav_routes_simplified.rs` 和 `COMPONENT_LINKS` 两处同步添加，而不能随意填 URL。

3. **`SpotList` 已有组件**（`pages/spots/list.rs`），无需新建，直接引用即可。
