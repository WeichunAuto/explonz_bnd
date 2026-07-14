# GraphQL 集成技术方案

## 1. 项目现状分析

### 当前技术栈
| 组件 | 技术 | 版本 |
|------|------|------|
| Web 框架 | Axum | 0.8.4 |
| 数据库 ORM | SeaORM | 1.1.17 |
| 数据库 | PostgreSQL | - |
| 认证 | JWT (jsonwebtoken) | 10.2.0 |
| 序列化 | Serde | 1.0.225 |
| 异步运行时 | Tokio | 1.47.1 |

### 当前架构
```
src/
├── main.rs                   # 入口点
├── application.rs            # AppState、Server 启动逻辑
├── lib.rs                    # 模块声明
├── auth.rs                   # JWT 编解码
├── middleware.rs             # JWT 认证中间件
├── error.rs                  # 统一错误类型 ApiError
├── response.rs               # 统一响应结构 ApiResponse<T>
├── request.rs                # 自定义请求提取器 BValidQuery
├── common.rs                 # 分页等公共结构
├── database.rs               # 数据库初始化
├── logger.rs                 # 日志初始化
├── config/                   # 配置模块
│   ├── mod.rs
│   ├── server.rs
│   └── database.rs
├── api/                      # 路由定义层
│   ├── mod.rs                # build_routes() 总路由
│   ├── user.rs               # /api/users 路由
│   ├── workspace.rs          # /api/workspaces 路由
│   └── login_auth.rs         # /auth 路由
├── handlers/                 # 业务处理层
│   ├── mod.rs
│   ├── user.rs               # 用户增删查改逻辑
│   └── workspace.rs          # 工作区逻辑
└── entity/                   # SeaORM 实体层
    ├── mod.rs
    ├── prelude.rs
    ├── users.rs              # User 实体 (id, fullname, gender, email, password_hash, create_at, ws_id)
    ├── workspace.rs          # Workspace 实体 (id, name, owner_id, create_at)
    └── sea_orm_active_enums.rs  # Gender 枚举
```

### 现有 REST API
- `GET  /` - 首页
- `POST /auth/login` - 登录获取 JWT
- `GET  /api/users` - 查询用户列表（支持分页/关键词搜索）
- `POST /api/users` - 创建用户
- `PUT  /api/users/:id/:ws_id` - 更新用户工作区
- `DELETE /api/users/:id` - 删除用户
- `GET  /api/workspaces` - 查询工作区

---

## 2. 技术选型

### 推荐方案：async-graphql + async-graphql-axum

**理由**：
- `async-graphql` 是 Rust 生态最成熟的 GraphQL 库，Stars 最多，社区活跃
- 原生支持 Axum 集成（`async-graphql-axum` crate）
- 代码优先（Code-First）方式定义 Schema，与现有 Rust 代码风格一致
- 内置 DataLoader 解决 N+1 查询问题
- 支持 Query、Mutation、Subscription（WebSocket）
- 内置 GraphiQL / GraphQL Playground 调试界面
- 支持通过 Context 传递 AppState 和认证信息，与现有中间件兼容

### 需要新增的依赖
```toml
# Cargo.toml
async-graphql = { version = "7", features = ["chrono", "dataloader"] }
async-graphql-axum = "7"
```

---

## 3. 目标架构设计

### 新增文件结构
```
src/
├── graphql/                        # 新增 GraphQL 模块
│   ├── mod.rs                      # 导出 GraphQL schema 和路由
│   ├── schema.rs                   # Schema 组合（QueryRoot + MutationRoot）
│   ├── context.rs                  # GraphQL 上下文（注入 AppState、Principal）
│   ├── types/                      # GraphQL 输出类型（对应实体）
│   │   ├── mod.rs
│   │   ├── user.rs                 # UserType（async-graphql Object）
│   │   └── workspace.rs            # WorkspaceType
│   ├── query/                      # Query Resolvers
│   │   ├── mod.rs                  # QueryRoot
│   │   ├── user.rs                 # 用户查询
│   │   └── workspace.rs            # 工作区查询
│   ├── mutation/                   # Mutation Resolvers
│   │   ├── mod.rs                  # MutationRoot
│   │   ├── user.rs                 # 用户变更
│   │   └── workspace.rs            # 工作区变更
│   └── loader/                     # DataLoader（解决 N+1 问题）
│       ├── mod.rs
│       └── workspace_loader.rs     # 批量加载 Workspace
└── api/
    └── mod.rs                      # 新增 /graphql 路由注册
```

---

## 4. 具体实现方案

### 4.1 添加依赖

```toml
# Cargo.toml [dependencies]
async-graphql = { version = "7", features = ["chrono", "dataloader"] }
async-graphql-axum = "7"
```

### 4.2 定义 GraphQL 输出类型

将 SeaORM 实体映射为 GraphQL 类型。注意：不直接在实体上加 `#[Object]`，
而是创建单独的 GraphQL 类型结构体，避免污染实体层，同时可隐藏敏感字段（如 `password_hash`）。

```rust
// src/graphql/types/user.rs
use async_graphql::*;
use crate::entity::users::Model as UserModel;

pub struct UserType(pub UserModel);

#[Object]
impl UserType {
    async fn id(&self) -> i64 { self.0.id }
    async fn fullname(&self) -> &str { &self.0.fullname }
    async fn email(&self) -> &str { &self.0.email }
    async fn ws_id(&self) -> i64 { self.0.ws_id }
    // password_hash 不暴露！

    // 关联查询：通过 DataLoader 加载 Workspace
    async fn workspace(&self, ctx: &Context<'_>) -> Result<Option<WorkspaceType>> {
        let loader = ctx.data_unchecked::<DataLoader<WorkspaceLoader>>();
        let workspace = loader.load_one(self.0.ws_id).await?;
        Ok(workspace.map(WorkspaceType))
    }
}
```

```rust
// src/graphql/types/workspace.rs
use async_graphql::*;
use crate::entity::workspace::Model as WorkspaceModel;

pub struct WorkspaceType(pub WorkspaceModel);

#[Object]
impl WorkspaceType {
    async fn id(&self) -> i64 { self.0.id }
    async fn name(&self) -> &str { &self.0.name }
    async fn owner_id(&self) -> i64 { self.0.owner_id }
}
```

### 4.3 定义 Query Resolvers

```rust
// src/graphql/query/user.rs
use async_graphql::*;
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};
use crate::application::AppState;
use crate::entity::{users, prelude::Users};
use crate::graphql::types::user::UserType;
use crate::graphql::context::GqlContext;

#[derive(Default)]
pub struct UserQuery;

#[Object]
impl UserQuery {
    /// 根据 ID 查询单个用户
    async fn user(&self, ctx: &Context<'_>, id: i64) -> Result<Option<UserType>> {
        let state = ctx.data_unchecked::<AppState>();
        let user = Users::find_by_id(id).one(&state.db).await?;
        Ok(user.map(UserType))
    }

    /// 分页查询用户列表，支持关键词搜索
    async fn users(
        &self,
        ctx: &Context<'_>,
        keyword: Option<String>,
        page: Option<u64>,
        size: Option<u64>,
    ) -> Result<Vec<UserType>> {
        let state = ctx.data_unchecked::<AppState>();
        let mut query = Users::find();
        if let Some(kw) = keyword {
            query = query.filter(
                sea_orm::Condition::any()
                    .add(users::Column::Fullname.contains(&kw))
                    .add(users::Column::Email.contains(&kw)),
            );
        }
        let page = page.unwrap_or(1);
        let size = size.unwrap_or(20);
        let users = query
            .paginate(&state.db, size)
            .fetch_page(page - 1)
            .await?;
        Ok(users.into_iter().map(UserType).collect())
    }
}
```

### 4.4 定义 Mutation Resolvers

```rust
// src/graphql/mutation/user.rs
use async_graphql::*;
use sea_orm::{Set, EntityTrait, ActiveModelTrait};
use crate::application::AppState;
use crate::entity::users;
use crate::graphql::types::user::UserType;

#[derive(InputObject)]
pub struct CreateUserInput {
    pub fullname: String,
    pub email: String,
    pub password_hash: String,
    pub ws_id: i64,
}

#[derive(Default)]
pub struct UserMutation;

#[Object]
impl UserMutation {
    /// 创建用户
    async fn create_user(&self, ctx: &Context<'_>, input: CreateUserInput) -> Result<UserType> {
        let state = ctx.data_unchecked::<AppState>();
        let new_user = users::ActiveModel {
            fullname: Set(input.fullname),
            email: Set(input.email),
            password_hash: Set(input.password_hash),
            ws_id: Set(input.ws_id),
            ..Default::default()
        };
        let user = new_user.insert(&state.db).await?;
        Ok(UserType(user))
    }

    /// 删除用户
    async fn delete_user(&self, ctx: &Context<'_>, id: i64) -> Result<bool> {
        let state = ctx.data_unchecked::<AppState>();
        let result = users::Entity::delete_by_id(id).exec(&state.db).await?;
        Ok(result.rows_affected > 0)
    }
}
```

### 4.5 DataLoader（解决 N+1 问题）

当查询用户列表并关联工作区时，如果每个用户都单独查一次 workspace，会产生 N+1 查询。
DataLoader 会将 N 次请求合并为 1 次批量查询。

```rust
// src/graphql/loader/workspace_loader.rs
use async_graphql::dataloader::Loader;
use sea_orm::{EntityTrait, ColumnTrait, QueryFilter};
use std::collections::HashMap;
use crate::entity::{workspace, prelude::Workspace};
use crate::entity::workspace::Model as WorkspaceModel;

pub struct WorkspaceLoader(pub sea_orm::DatabaseConnection);

impl Loader<i64> for WorkspaceLoader {
    type Value = WorkspaceModel;
    type Error = async_graphql::Error;

    async fn load(&self, keys: &[i64]) -> Result<HashMap<i64, Self::Value>, Self::Error> {
        let workspaces = Workspace::find()
            .filter(workspace::Column::Id.is_in(keys.to_vec()))
            .all(&self.0)
            .await?;
        Ok(workspaces.into_iter().map(|w| (w.id, w)).collect())
    }
}
```

### 4.6 组合 Schema

```rust
// src/graphql/schema.rs
use async_graphql::{EmptySubscription, MergedObject, Schema};
use crate::graphql::query::user::UserQuery;
use crate::graphql::query::workspace::WorkspaceQuery;
use crate::graphql::mutation::user::UserMutation;
use crate::graphql::mutation::workspace::WorkspaceMutation;

#[derive(MergedObject, Default)]
pub struct QueryRoot(UserQuery, WorkspaceQuery);

#[derive(MergedObject, Default)]
pub struct MutationRoot(UserMutation, WorkspaceMutation);

pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;
```

### 4.7 GraphQL 上下文：注入认证信息

现有的 JWT 认证中间件将 `Principal` 注入到 `request.extensions()`，
在 GraphQL handler 中可以读取并注入到 GraphQL Context，供 resolver 使用。

```rust
// src/graphql/mod.rs
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::Extension;
use axum::extract::State;
use crate::application::AppState;
use crate::auth::Principal;
use crate::graphql::schema::AppSchema;

pub async fn graphql_handler(
    State(state): State<AppState>,
    Extension(principal): Extension<Principal>,  // 从 JWT 中间件获取
    schema: axum::Extension<AppSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let req = req.into_inner()
        .data(state)           // 注入 AppState（数据库连接）
        .data(principal);      // 注入当前认证用户

    schema.execute(req).await.into()
}

/// GraphiQL 调试界面 handler（仅开发环境启用）
pub async fn graphiql_handler() -> impl axum::response::IntoResponse {
    axum::response::Html(async_graphql::http::graphiql_source("/graphql", None))
}
```

### 4.8 Schema 构建与路由注册

```rust
// src/api/mod.rs（修改）
use async_graphql::{Schema, EmptySubscription, dataloader::DataLoader};
use async_graphql_axum::GraphQL;
use crate::graphql::schema::{QueryRoot, MutationRoot, AppSchema};
use crate::graphql::loader::workspace_loader::WorkspaceLoader;

pub async fn build_routes(db: sea_orm::DatabaseConnection) -> Router<AppState> {
    // 构建 GraphQL Schema，注入 DataLoader
    let schema = Schema::build(
        QueryRoot::default(),
        MutationRoot::default(),
        EmptySubscription
    )
    .data(DataLoader::new(
        WorkspaceLoader(db.clone()),
        tokio::spawn,
    ))
    .finish();

    Router::new()
        // 原有 REST 路由保持不变
        .route("/", get(handlers::index))
        .nest("/api", user::routes())
        .nest("/api", workspace::routes())
        .route_layer(get_auth_layer())
        .nest("/auth", login_auth::routes())
        // 新增 GraphQL 路由（受 JWT 认证保护）
        .route("/graphql", post(graphql_handler).get(graphiql_handler))
        .layer(Extension(schema))
        .route_layer(get_auth_layer())   // GraphQL 也走 JWT 认证
        .fallback(handlers::fallback)
        .method_not_allowed_fallback(/* ... */)
}
```

---

## 5. 认证集成策略

### 策略：复用现有 JWT 中间件

现有 `middleware.rs` 中的 `JWTAuth` 将 `Principal` 注入 `request.extensions()`。
GraphQL handler 可以通过 `Extension<Principal>` 提取，并注入 GraphQL Context。

```rust
// 在 resolver 中检查权限
async fn delete_user(&self, ctx: &Context<'_>, id: i64) -> Result<bool> {
    // 从 Context 获取当前用户，做权限判断
    let principal = ctx.data::<Principal>()?;
    // 例如：只允许操作自己的数据
    if principal.id != id.to_string() {
        return Err(Error::new("Permission denied"));
    }
    // ... 删除逻辑
}
```

---

## 6. GraphQL Schema 示例

集成完成后，对外暴露的 GraphQL Schema 如下：

```graphql
type User {
  id: Int!
  fullname: String!
  email: String!
  wsId: Int!
  workspace: Workspace
}

type Workspace {
  id: Int!
  name: String!
  ownerId: Int!
}

type Query {
  user(id: Int!): User
  users(keyword: String, page: Int, size: Int): [User!]!
  workspace(id: Int!): Workspace
  workspaces: [Workspace!]!
}

input CreateUserInput {
  fullname: String!
  email: String!
  passwordHash: String!
  wsId: Int!
}

type Mutation {
  createUser(input: CreateUserInput!): User!
  deleteUser(id: Int!): Boolean!
  updateUserWorkspace(id: Int!, wsId: Int!): User!
  createWorkspace(name: String!, ownerId: Int!): Workspace!
}
```

---

## 7. 实施步骤

### Phase 1：基础搭建
1. 在 `Cargo.toml` 添加 `async-graphql` 和 `async-graphql-axum` 依赖
2. 创建 `src/graphql/` 目录结构
3. 定义 `UserType` 和 `WorkspaceType` GraphQL 输出类型
4. 实现 `QueryRoot`：`user(id)` 和 `users(keyword, page, size)`
5. 在 `api/mod.rs` 注册 `/graphql` 路由，并接入 JWT 中间件
6. 验证 GraphiQL 界面可以访问并执行简单查询

### Phase 2：完善功能
7. 实现 `MutationRoot`：`createUser`、`deleteUser`、`updateUserWorkspace`
8. 实现 `WorkspaceQuery` 和 `WorkspaceMutation`
9. 实现 `WorkspaceLoader`（DataLoader），解决用户列表关联工作区的 N+1 问题
10. 在 resolver 中接入 `Principal`，实现权限控制

### Phase 3：优化与收尾
11. 添加 GraphQL 错误统一映射（将 `ApiError` 映射为 `async_graphql::Error`）
12. 为常用查询添加字段级别的参数验证（借助 `#[graphql(validator(...))]`）
13. 编写集成测试

---

## 8. 兼容性说明

- **REST API 完全保留**：GraphQL 以独立的 `/graphql` 端点新增，不影响现有 `/api/*` 路由
- **认证机制复用**：GraphQL 路由复用现有 `AsyncRequireAuthorizationLayer`，Bearer Token 认证方式不变
- **AppState 共享**：GraphQL resolver 和 REST handler 共用同一个 `AppState`（数据库连接池）
- **实体层不修改**：SeaORM 实体保持原样，GraphQL 类型单独定义在 `graphql/types/` 中

---

## 9. 关键依赖版本参考

```toml
async-graphql = { version = "7", features = ["chrono", "dataloader"] }
async-graphql-axum = "7"
```

> `async-graphql` v7 支持 Axum 0.8.x，与当前项目完全兼容。
> `chrono` feature 使 GraphQL 能正确处理 SeaORM 中的 `DateTimeWithTimeZone` 类型。
