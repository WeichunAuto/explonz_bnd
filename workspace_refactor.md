# Workspace 重构建议：集成 Leptos 后台管理系统

## 现状分析

```
explonz_bnd/              ← 单一 Cargo package
├── Cargo.toml
├── src/
│   ├── api/              (auth, user 路由)
│   ├── application.rs    (AppState, Server 启动)
│   ├── entity/           (SeaORM 生成的实体：spots, posts, users…)
│   ├── service/          (auth, email 服务)
│   ├── infrastructure/   (database, logger, auth)
│   ├── config/           (server, db, email)
│   ├── common/           (pagination)
│   ├── error.rs / middleware.rs / request.rs / response.rs
│   └── lib.rs / main.rs
├── config/dev.yaml
└── migrations/
```

## 推荐架构：就地扩展为 Workspace

**核心原则：最小改动现有后端，将 workspace 根设置在现有目录，新增 subcrate。**

```
explonz_bnd/                     ← workspace 根（同时也是 api crate）
├── Cargo.toml                   ← 添加 [workspace] + 保留 [package]
├── src/                         ← 现有后端代码，基本不动
├── config/                      ← 共用配置文件
├── migrations/                  ← 共用数据库迁移
│
├── explonz_shared/              ← 新增：共享类型 crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── entity/              ← 从主 crate 迁移过来
│       └── common/              ← 从主 crate 迁移过来 (pagination 等)
│
└── explonz_admin/               ← 新增：Leptos 后台管理 crate
    ├── Cargo.toml
    ├── Leptos.toml              ← cargo-leptos 配置
    ├── style/
    │   └── main.scss
    └── src/
        ├── main.rs              ← Axum + Leptos SSR 启动
        ├── lib.rs               ← Leptos app 入口（hydration）
        ├── app.rs               ← 根组件、路由
        ├── pages/
        │   ├── mod.rs
        │   ├── dashboard.rs     ← 首页概览
        │   ├── spots/           ← 场地管理
        │   │   ├── mod.rs
        │   │   ├── list.rs
        │   │   ├── detail.rs
        │   │   └── edit.rs
        │   └── posts/           ← 帖子管理
        │       ├── mod.rs
        │       ├── list.rs
        │       ├── detail.rs
        │       └── edit.rs
        └── server/              ← Leptos server functions（后端逻辑）
            ├── mod.rs
            ├── spots.rs
            └── posts.rs
```

---

## 实施步骤

### Step 1：修改根 Cargo.toml，启用 Workspace

在现有 `Cargo.toml` 顶部添加 `[workspace]` 节：

```toml
[workspace]
members = [
    ".",                 # 当前 package（api backend）
    "explonz_shared",
    "explonz_admin",
]
resolver = "2"          # Leptos 需要 resolver = "2"

[workspace.dependencies]
# 统一管理版本，避免各 crate 版本冲突
sea-orm       = { version = "1.1.17", features = ["with-chrono", "sqlx-postgres", "with-rust_decimal", "runtime-tokio"] }
serde         = { version = "1.0.225", features = ["derive"] }
serde_json    = "1.0.145"
tokio         = { version = "1.47.1", features = ["full"] }
anyhow        = "1.0.100"
thiserror     = "2.0.17"
tracing       = { version = "0.1.41", features = ["async-await"] }
leptos        = { version = "0.8", features = ["nightly"] }
leptos_axum   = { version = "0.8" }
axum          = { version = "0.8.4", features = ["macros"] }
```

### Step 2：创建 `explonz_shared` crate

```toml
# explonz_shared/Cargo.toml
[package]
name = "explonz_shared"
version = "0.1.0"
edition = "2021"

[dependencies]
sea-orm    = { workspace = true }
serde      = { workspace = true }
serde_json = { workspace = true }
```

将以下模块从主 crate 迁移到 `explonz_shared/src/`：
- `entity/`（SeaORM 实体，spots/posts/users 等）
- `common/pagination.rs`
- `error.rs`（可选，若 admin 也需要用）

主 crate 的 `Cargo.toml` 和 `lib.rs` 中将原来的 entity 替换为依赖 `explonz_shared`：
```toml
[dependencies]
explonz_shared = { path = "../explonz_shared" }
```

> **注意**：如果迁移成本高，也可以暂时不迁移 entity，让 admin 直接依赖主 crate（`explonz_bnd`），
> 但长期建议分离以避免循环依赖。

### Step 3：创建 `explonz_admin` crate（Leptos SSR）

```toml
# explonz_admin/Cargo.toml
[package]
name = "explonz_admin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]  # SSR + WASM 双目标

[dependencies]
leptos           = { workspace = true }
leptos_axum      = { workspace = true, optional = true }
axum             = { workspace = true, optional = true }
tokio            = { workspace = true, optional = true }
explonz_shared   = { path = "../explonz_shared" }
sea-orm          = { workspace = true, optional = true }
serde            = { workspace = true }
tracing          = { workspace = true, optional = true }
anyhow           = { workspace = true, optional = true }

[features]
default = []
ssr = [
    "dep:leptos_axum",
    "dep:axum",
    "dep:tokio",
    "dep:sea-orm",
    "dep:tracing",
    "dep:anyhow",
    "leptos/ssr",
]
hydrate = ["leptos/hydrate"]

[[bin]]
name = "explonz_admin"
required-features = ["ssr"]
```

```toml
# explonz_admin/Leptos.toml
[package]
name = "explonz_admin"
output-name = "explonz_admin"
site-root = "target/site"
site-pkg-dir = "pkg"
style-file = "style/main.scss"
assets-dir = "public"
site-addr = "0.0.0.0:3006"
reload-port = 3007
end2end-cmd = ""
browserquery = "defaults"
env = "DEV"

[package.bin-features]
ssr = ["ssr"]

[package.lib-features]
hydrate = ["hydrate"]
```

### Step 4：实现 Admin 数据访问（Server Functions）

Admin 通过 Leptos server functions 直接访问数据库（共享 `explonz_shared` 的实体），
不需要绕道主 API，避免额外网络开销：

```rust
// explonz_admin/src/server/spots.rs
#[cfg(feature = "ssr")]
use explonz_shared::entity::spots;

#[server(GetSpots, "/api")]
pub async fn get_spots(page: u64, page_size: u64) -> Result<Vec<spots::Model>, ServerFnError> {
    let db = use_context::<DatabaseConnection>()
        .ok_or(ServerFnError::ServerError("No DB".into()))?;
    // 使用 SeaORM 查询...
}

#[server(UpdateSpot, "/api")]
pub async fn update_spot(id: String, /* fields */) -> Result<(), ServerFnError> {
    // ...
}
```

### Step 5：Admin 运行端口配置

两个服务分别监听不同端口：
- `explonz_bnd`（主 API）：`:3005`（现有）
- `explonz_admin`（Leptos 后台）：`:3006`（新增）

可在 `config/dev.yaml` 中添加 `admin` 节管理：
```yaml
admin:
  host: "0.0.0.0"
  port: 3006
```

---

## 各 Crate 职责总结

| Crate            | 类型           | 职责                                      | 端口  |
|------------------|----------------|-------------------------------------------|-------|
| `explonz_bnd`    | Axum binary    | 移动端/前端 REST API，auth，用户          | 3005  |
| `explonz_shared` | library        | SeaORM 实体、通用类型、分页               | —     |
| `explonz_admin`  | Leptos SSR app | 后台管理 UI：spots/posts CRUD，数据统计   | 3006  |

---

## 开发工具

```bash
# 启动主 API
cargo run -p explonz_bnd

# 启动 Admin（需安装 cargo-leptos）
cargo install cargo-leptos
cd explonz_admin && cargo leptos watch

# 构建 Admin 生产包
cargo leptos build --release -p explonz_admin
```

---

## 注意事项

1. **Leptos 版本**：Leptos 0.8 需要 Rust nightly（或 stable + 特定 feature）。
   建议在项目根添加 `rust-toolchain.toml` 统一 toolchain：
   ```toml
   [toolchain]
   channel = "nightly"
   ```

2. **Admin 鉴权**：Admin 后台应独立于移动端的 JWT 鉴权，建议使用 session cookie 或单独的 admin JWT。

3. **迁移策略**：`explonz_shared` 的拆分可以分批进行。
   第一期可以让 `explonz_admin` 直接依赖 `explonz_bnd` 的 lib 导出（由于已有 `lib.rs`），
   无需立即创建 shared crate：
   ```toml
   explonz_bnd = { path = ".." }
   ```

4. **UI 框架**：Leptos 的 UI 生态可结合 `tailwindcss`（通过 PostCSS 集成）或
   直接使用 `Daisy UI` 组件，在 `style/main.scss` 中引入。
