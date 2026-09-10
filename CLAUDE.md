# Explonz Backend — CLAUDE.md

## 项目概述

Explonz 是一个景点探索平台，本仓库包含三个 Rust crate，组织为 Cargo workspace：

| Crate | 路径 | 职责 |
|-------|------|------|
| `explonz_bnd` | `.` (workspace root) | Axum REST API 后端，鉴权、业务逻辑、DB 写入 |
| `explonz_shared` | `./explonz_shared` | 共享实体（SeaORM entity）、DTO、工具函数 |
| `explonz_admin` | `./explonz_admin` | Leptos SSR + WASM 管理后台 |

---

## 技术栈

- **语言**：Rust（edition 2021）
- **后端框架**：Axum 0.8
- **ORM**：SeaORM 1.1（PostgreSQL）
- **前端框架**：Leptos 0.8（SSR + WASM hydration）
- **CSS**：Tailwind CSS（通过 `cargo leptos` 构建）
- **图标**：`icons` crate（Lucide icons for Leptos）
- **路由**：`leptos_router` 0.8
- **认证**：JWT（`jsonwebtoken`）+ bcrypt 密码哈希 + Cookie（`access_token`）

---

## AI 开发规则

### 基本原则

- 优先复用现有代码、组件、DTO、Service 和工具函数。
- 修改代码前，应先搜索项目中是否已有类似实现。
- 尽量进行最小范围修改，不要对无关代码进行重构。
- 不要为了一个简单需求引入新的 crate，除非现有依赖无法满足需求。
- 遵循现有项目的架构和代码风格，不要自行引入新的架构模式。
- 如果需求与现有架构约定冲突，应优先保持现有架构，除非用户明确要求修改架构。

---

## 开发命令

### 后端（explonz_bnd）

```bash
# 启动后端 API 服务（默认端口见 config/）
cargo run

# 检查编译（不运行）
cargo check
```

### 管理后台（explonz_admin，Leptos SSR）

```bash
# 开发模式（热重载，端口 3001）
cargo leptos watch --package explonz_admin

# 生产构建
cargo leptos build --package explonz_admin --release
```

> `cargo leptos` 需要先安装：`cargo install cargo-leptos`

---

## 代码验证

AI 修改代码后，应根据修改范围执行适当的检查。

### Backend

```bash
cargo check
cargo check --workspace
cargo fmt --all -- --check
```

### Admin（Leptos）

```bash
cargo leptos build --package explonz_admin
```

## 环境变量

### `explonz_bnd/.env`（后端）

```dotenv
DATABASE_URL=postgres://user:pass@localhost:5432/explonz_app
UPLOAD_DIR=./uploads          # 图片本地存储目录
PUBLIC_URL=http://127.0.0.1:3000  # 后端对外地址，用于拼接图片 URL
IMAGE_CLEANUP_INTERVAL_SECS=3600  # 孤立图片清理间隔（秒）
IMAGE_CLEANUP_GRACE_SECS=1800     # 孤立图片宽限期（秒）
```

### `explonz_admin/.env`（管理后台）

```dotenv
ADMIN_ACCOUNT=admin
ADMIN_PASSWORD_HASH=$2b$12$...    # bcrypt hash
BACKEND_URL=http://127.0.0.1:3000 # admin SSR 端调用后端 API 的地址
```

---

## 项目结构

### `explonz_bnd/src/`（API 后端）

```
src/
├── main.rs                  # 入口：application::run(api::build_routes())
├── lib.rs
├── application.rs           # AppState, Server 启动, ServeDir 静态文件
├── api/
│   ├── mod.rs               # build_routes()，所有路由注册，JWT 鉴权中间件
│   ├── auth/                # 注册/登录/OTP 相关接口
│   ├── spots/               # Spot CRUD + 图片上传
│   │   ├── handler.rs
│   │   ├── dto.rs           # CreateSpotRequest（含 label_ids, opening_hours）
│   │   └── mod.rs
│   ├── labels/              # Label CRUD
│   │   ├── handler.rs
│   │   ├── dto.rs
│   │   └── mod.rs
│   └── user.rs
├── service/
│   ├── spots.rs             # create_spot_service（事务：spot + label关联 + 营业时间）
│   ├── label.rs             # label CRUD service
│   └── auth.rs
├── entity/                  # 本地 entity（与 explonz_shared 同步，用于 explonz_bnd 内部）
├── config/                  # AppConfig（YAML），数据库、服务器、邮件配置
├── infrastructure/          # logger, database 初始化, JWT 工具
├── error.rs                 # ApiError 枚举，IntoResponse 实现
├── response.rs              # ApiResponse<T>，ApiResult<T> 类型别名
└── middleware.rs            # JWT Bearer token 验证中间件
```

### `explonz_shared/src/`（共享库）

```
src/
├── entity/                  # SeaORM 生成的实体（sea-orm-codegen）
│   ├── spots.rs
│   ├── spot_opening_hours.rs
│   ├── spot_label_assignments.rs
│   ├── spot_labels.rs
│   ├── prelude.rs           # pub use Entity as XxxEntity
│   └── ...
├── common/
│   ├── dto.rs               # SpotDto, LabelDto, PostDto 等跨 crate 共享的传输对象
│   └── utils.rs
└── icons/
    └── mod.rs               # LabelIcon 枚举（18 个变体，实现 EnumString/Display）
```

### `explonz_admin/src/`（Leptos 管理后台）

```
src/
├── main.rs                  # Leptos SSR 入口（leptos_axum）
├── app.rs                   # 顶层路由（Router + AuthGuard）
├── lib.rs
├── server/                  # Leptos server fn（SSR 端执行，reqwest 转发到后端）
│   ├── mod.rs               # backend_url(), extract_token(), ApiResp<T>
│   ├── auth.rs
│   ├── spots.rs             # CreateSpot, UploadPhoto, DeletePhoto, GeocodeLocation
│   └── labels.rs            # GetLabels, CreateLabel, UpdateLabel, DeleteLabel
├── pages/
│   ├── spots/
│   │   ├── addition.rs      # 创建 Spot 页面（Dropzone上传 + Labels多选 + 营业时间）
│   │   ├── list.rs          # Spot 列表页
│   │   ├── edit.rs          # TODO: Spot 编辑页（路由 /spots/edit/:spot_id）
│   │   └── detail.rs        # TODO: Spot 详情页
│   ├── labels/
│   │   └── list.rs          # Label 列表 + 右侧编辑面板（含图标选择器）
│   ├── login/
│   ├── home/
│   └── auth_guard.rs        # 未登录重定向
└── components/
    ├── ui/                  # 通用 UI 组件
    │   ├── label_icon.rs    # LabelIconView 组件（LabelIcon -> icons::XxxIcon 静态分发）
    │   ├── button.rs
    │   ├── card.rs
    │   ├── input.rs
    │   └── ...
    └── blocks/              # 布局块
        ├── sidenav_routes.rs       # 路由枚举定义（ExplonzRoutes, SpotsRoutes 等）
        ├── sidenav_routes_simplified.rs  # Sidenav 嵌套路由注册
        └── ...
```

---

## Backend 分层规则

Backend API 遵循以下调用关系：

HTTP Request
    ↓
Handler
    ↓
Service
    ↓
SeaORM Entity / Database

### Handler

Handler 负责：

- 接收和解析 HTTP 请求
- 参数校验
- 调用 Service
- 将结果转换为 `ApiResponse<T>`
- 处理 HTTP 层面的认证/授权需求

Handler 不应：

- 编写复杂业务逻辑
- 直接执行数据库查询
- 直接操作 SeaORM Entity
- 包含本应属于 Service 的业务逻辑

### Service

Service 负责：

- 业务逻辑
- 数据库操作
- Transaction
- 业务规则和数据一致性

Service 不应：

- 依赖 Axum 的 Request / Response
- 直接构造 HTTP Response
- 处理前端 UI 逻辑

---

## 关键架构约定

### API 响应格式

所有接口统一返回 `ApiResponse<T>`：

```json
{ "code": 200, "msg": "ok", "data": { ... } }
{ "code": 0,   "msg": "error message" }
```

handler 返回类型统一为 `ApiResult<T>`（即 `Result<ApiResponse<T>, ApiError>`）。

### Admin 路由结构

Admin 只有一套路由树，入口在 `app.rs`：

```
/login                             → LoginPage（无需鉴权）
/                                  → AuthGuard（需登录）
  └─ Sidenav02Routes               → home/index.rs
       └─ /admin/home/             → SidenavLayout（含 Sidenav + Outlet）
            └─ /explonz/
                 └─ /spots/
                      ├─ /addition   → SpotAddition
                      ├─ /spot_list  → SpotList
                      └─ /label_list → LabelList
```

- 路由注册集中在 `sidenav_routes_simplified.rs`，通过 `#[component(transparent)]` + `MatchNestedRoutes` 组合进 `app.rs`
- Sidenav 侧边栏链接硬编码在 `home/index.rs` 的 `SPOTS_LINKS` 常量中
- 页面内跳转使用 `use_location()` 动态计算同级路径，**不硬编码完整路径前缀**：

```rust
let location = leptos_router::hooks::use_location();
let current = location.pathname.get_untracked();
let target = current
    .rsplit_once('/')
    .map(|(base, _)| format!("{}/spot_list", base))
    .unwrap_or_else(|| "/".to_string());
navigate(&target, NavigateOptions::default());
```

### 后端路由规范

#### 整体结构

路由在 `src/api/mod.rs` 的 `build_routes()` 中统一注册：

```rust
Router::new()
    .route("/", get(index))
    .nest("/api", user::routes())
    .nest("/api", spots::routes())
    .nest("/api", labels::routes())
    .route_layer(get_auth_layer())   // JWT 鉴权应用于以上所有 /api 路由
    .nest("/auth", auth::routes())   // /auth/** 无需鉴权
```

- **需要鉴权的路由**：通过 `.nest("/api", xxx::routes())` 注册，放在 `.route_layer(get_auth_layer())` **之前**
- **不需要鉴权的路由**：通过 `.nest("/auth", xxx::routes())` 或在 `.route_layer` **之后** 注册

#### 模块结构

每个业务模块在 `src/api/<module>/` 下包含三个文件：

```
src/api/<module>/
├── mod.rs      # routes() 函数，注册本模块所有路由
├── handler.rs  # 具体 handler 函数
└── dto.rs      # 请求/响应 DTO 定义（Serialize/Deserialize）
```

新增模块时在 `api/mod.rs` 中 `pub mod <module>;` 并 `.nest("/api", <module>::routes())`。

#### URL 命名约定

| 操作 | 方法 | 路径示例 |
|------|------|----------|
| 列表查询 | `GET` | `/api/labels` |
| 创建 | `POST` | `/api/labels/new` |
| 更新 | `PUT` | `/api/labels/{label_id}` |
| 删除 | `DELETE` | `/api/labels/{label_id}` |
| 文件上传 | `POST` | `/api/spots/images` |

路径参数使用 `{param_name}` 格式（Axum 0.8），在 handler 签名中用 `Path(param): Path<Type>` 提取。

#### Handler 签名规范

```rust
pub async fn handler_name(
    State(AppState { db, .. }): State<AppState>,
    // Path(id): Path<Uuid>,          // 路径参数（按需）
    // Json(body): Json<RequestDto>,  // JSON body（按需）
) -> ApiResult<ResponseDto> {
    let data = some_service(&db, ...).await.map_err(ApiError::InternalError)?;
    Ok(ApiResponse::success("ok", Some(data)))
}
```

### 数据库 UUID 主键

所有表的 `id` 均使用 `DEFAULT uuidv7()`，由数据库自动生成。
SeaORM ActiveModel 中 `id` 字段使用 `..Default::default()` 保持 `NotSet`，不在 Rust 端生成。

### Entity 同步

`explonz_shared/src/entity/` 和 `explonz_bnd/src/entity/` 是两份独立的 SeaORM entity（由 `sea-orm-codegen` 生成），**不能跨 crate 混用**。新增实体或字段时需在两处同步更新，并在 `prelude.rs` 中导出。

### WASM 条件编译

`web_sys`、`FormData`、`FileList` 等 WASM-only API 均用 `#[cfg(target_arch = "wasm32")]` 隔离，确保 SSR（native）编译通过。rust-analyzer 会将这些块标注为 inactive（灰色），属于正常现象，`cargo leptos build` 时以 wasm32 目标编译可正常生效。

### 多值表单字段

HTML 表单提交 `Vec<String>` 参数时，input name 必须使用括号格式（`name="photo_urls[]"`、`name="label_ids[]"`），以符合 `serde_qs` 的数组反序列化语义。

### 图标组件

新增 `LabelIcon` 变体时，需同步：
1. `explonz_shared/src/icons/mod.rs` — 枚举定义
2. `explonz_admin/src/components/ui/label_icon.rs` — `LabelIconView` match arm

---

## Admin Server Function 规则

Admin 前端禁止从 WASM 直接调用 Backend REST API。

正确的数据流：

Browser / WASM
    ↓
Leptos Server Function
    ↓
reqwest
    ↓
Backend REST API
    ↓
Database

Server Function 负责：

1. 使用 `extract_token()` 获取 `access_token`。
2. 使用 `backend_url()` 获取 Backend URL。
3. 构造 Backend API Request。
4. 添加 `Authorization: Bearer <token>`。
5. 调用 Backend API。
6. 解析 `ApiResp<T>`。
7. 将 Backend 错误转换为 `ServerFnError`。

不要在 Leptos Component 中直接调用 Backend API。
不要在 WASM 中直接访问 Backend REST API。

## DTO 规则

- 修改或创建 DTO 前，先检查 `explonz_shared/src/common/dto.rs` 是否已经存在可复用的 DTO。
- 优先复用已有 DTO，避免创建重复结构体。
- 跨 crate 使用的 DTO 应放在 `explonz_shared`。
- 仅属于某个 API endpoint 的 Request DTO，可以放在对应的 `api/<module>/dto.rs`。
- 不要为每个 Server Function 创建重复的 Response Wrapper。
- Admin Server Function 应优先使用通用的 `ApiResp<T>`。

## 数据库规则

### 关键约束

迁移文件位于 `./migrations/*.sql`，手动执行（项目暂未集成 migrate CLI）。

- `spot_opening_hours`：`UNIQUE(spot_id, day_of_week)`，每个 spot 每天只能有一条记录
- `spot_label_assignments`：`PRIMARY KEY(spot_id, label_id)`，复合主键
- `spots.photo_urls`：`TEXT[] NOT NULL DEFAULT '{}'`，图片 URL 数组

### AI 操作限制

数据库 Schema、Migration 和 SeaORM Entity 的维护由开发者手动完成。

AI 默认不得：

- 修改数据库 Schema
- 执行 SQL Migration
- 执行 `sqlx migrate run`
- 直接修改数据库
- 自动生成 SeaORM Entity
- 自动重新生成 SeaORM Entity
- 假设某个 Migration 已经执行成功

除非用户明确要求，否则 AI 不应执行上述操作。

### 数据库变更流程

当需要进行数据库结构变更时：

1. 开发者手动修改 SQL Schema / Migration。
2. 开发者手动执行 Migration。
3. 开发者手动生成或更新 SeaORM Entity。
4. 开发者确保以下两个目录中的 Entity 已同步：
   - `explonz_shared/src/entity/`
   - `explonz_bnd/src/entity/`
5. AI 基于更新后的 Entity 修改 DTO、Service、Handler、Server Function 和 Admin UI。

AI 可以检查现有 Entity，并根据 Entity 的实际定义修改业务代码，但不负责数据库 Schema 的生命周期管理。

---

## 图片上传流程

```
用户选文件 → upload_photo server fn(SSR) → POST /api/spots/images → 写 $UPLOAD_DIR/spots/images/
                                                                    ← { id: filename, url }
用户点删除 → delete_photo server fn(SSR) → DELETE /api/images/:id → 删除文件
```

孤立文件（上传后未绑定 spot）由后台定时任务清理（待实现，详见 `explonz_admin/src/pages/spots/photo_upload_tsd.md` Section 九）。
