# IP 访问黑名单控制技术方案

## 1. 需求分析

### 功能目标
| 功能 | 描述 |
|------|------|
| 频率检测 | 统计每个 IP 在滑动时间窗口内的请求次数 |
| 自动封禁 | 超过阈值时自动将该 IP 加入黑名单 |
| 请求拦截 | 黑名单 IP 的请求直接返回 `429 Too Many Requests`，不进入业务逻辑 |
| 自动解封 | 黑名单条目带 TTL，到期后自动移除 |
| 手动管理 | 提供管理接口，支持手动添加/移除黑名单 IP |

### 关键参数（可配置）
| 参数 | 默认值 | 说明 |
|------|--------|------|
| `window_secs` | 60 | 滑动时间窗口大小（秒） |
| `max_requests` | 200 | 窗口内最大允许请求数 |
| `ban_duration_secs` | 3600 | 封禁时长（秒），0 表示永久 |
| `whitelist` | `["127.0.0.1"]` | IP 白名单，永不封禁 |

---

## 2. 现状分析

### 已有基础
- `application.rs:101` 中已启用 `into_make_service_with_connect_info::<SocketAddr>()`，整个应用已具备提取客户端 IP 的能力
- `api/login_auth.rs:44` 中已有 `ConnectInfo(addr): ConnectInfo<SocketAddr>` 的用法，可参照扩展
- `middleware.rs` 中已有 Tower `AsyncRequireAuthorizationLayer` 的实践，新中间件沿用同一模式
- `error.rs` 中的 `ApiError` 枚举可直接扩展，添加限流错误类型

### 中间件执行顺序（现有）
```
Request
  └─ NormalizePathLayer          (trim trailing slash)
  └─ CorsLayer                   (跨域)
  └─ TraceLayer                  (链路追踪)
  └─ DefaultBodyLimit            (body 大小限制)
  └─ TimeoutLayer                (超时 60s)
  └─ AsyncRequireAuthorizationLayer (JWT 认证)
  └─ Router (业务路由)
```

### 新中间件插入位置
```
Request
  └─ NormalizePathLayer
  └─ [NEW] IpGuardLayer          ← 插在最外层，最先执行，最早拦截
  └─ CorsLayer
  └─ TraceLayer
  └─ DefaultBodyLimit
  └─ TimeoutLayer
  └─ AsyncRequireAuthorizationLayer
  └─ Router
```

IP 拦截必须在 JWT 认证之前执行，避免被封禁的 IP 消耗认证计算资源。

---

## 3. 技术选型

### 并发计数存储：`dashmap`
- `DashMap` 是分片式并发 HashMap，读写性能远优于 `RwLock<HashMap>`
- 适合高并发场景下的 IP 计数器和黑名单存储
- 无需引入 Redis 等外部依赖，降低部署复杂度

```toml
# Cargo.toml
dashmap = "6"
```

### 限流算法：滑动窗口计数器（Sliding Window Counter）

选择滑动窗口而非固定窗口，原因：
- 固定窗口在窗口边界处允许 2 倍流量突破（窗口末尾 + 下一窗口开始各打满），不够精准
- 滑动窗口记录每次请求的时间戳，统计最近 N 秒内的实际请求数，更精准

实现方式：每个 IP 维护一个 `VecDeque<Instant>`，存储最近的请求时间戳，
每次请求时先清理过期时间戳，再统计窗口内数量。

---

## 4. 核心数据结构设计

```rust
// src/ip_guard/store.rs

use dashmap::DashMap;
use std::collections::VecDeque;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 单个 IP 的访问记录
struct IpRecord {
    /// 滑动窗口内的请求时间戳队列
    timestamps: VecDeque<Instant>,
}

/// 黑名单条目
struct BlacklistEntry {
    /// 封禁到期时间，None 表示永久封禁
    ban_until: Option<Instant>,
    /// 封禁原因
    reason: String,
}

/// IP 访问控制共享状态（Arc 包裹，跨线程共享）
pub struct IpGuardState {
    /// IP 请求计数器：IP -> 请求时间戳队列
    counters: DashMap<IpAddr, IpRecord>,
    /// 黑名单：IP -> 封禁条目
    blacklist: DashMap<IpAddr, BlacklistEntry>,
    /// IP 白名单（永不封禁）
    whitelist: Vec<IpAddr>,
    /// 配置
    config: IpGuardConfig,
}

/// 配置参数
#[derive(Clone)]
pub struct IpGuardConfig {
    /// 滑动窗口大小
    pub window: Duration,
    /// 窗口内最大请求数
    pub max_requests: usize,
    /// 封禁时长，None 为永久
    pub ban_duration: Option<Duration>,
}

impl Default for IpGuardConfig {
    fn default() -> Self {
        Self {
            window: Duration::from_secs(60),
            max_requests: 200,
            ban_duration: Some(Duration::from_secs(3600)),
        }
    }
}
```

---

## 5. 文件结构设计

```
src/
├── ip_guard/                     # 新增模块
│   ├── mod.rs                    # 模块入口，导出 IpGuardLayer、IpGuardState
│   ├── config.rs                 # IpGuardConfig 配置结构
│   ├── store.rs                  # IpGuardState：计数器 + 黑名单存储逻辑
│   ├── layer.rs                  # Tower Layer/Service 实现（中间件核心）
│   └── cleaner.rs                # 后台清理任务（定时清理过期条目）
├── api/
│   ├── mod.rs                    # 注册 /admin/ip 管理路由
│   └── ip_admin.rs               # 新增：黑名单管理 API
├── application.rs                # 修改：AppState 中新增 IpGuardState，启动清理任务
├── error.rs                      # 修改：新增 RateLimitError
└── lib.rs                        # 修改：声明 ip_guard 模块
```

---

## 6. Tower 中间件实现

### 6.1 Layer 定义

```rust
// src/ip_guard/layer.rs
use std::sync::Arc;
use tower::{Layer, Service};
use crate::ip_guard::store::IpGuardState;

/// Tower Layer，用于注册中间件
#[derive(Clone)]
pub struct IpGuardLayer {
    state: Arc<IpGuardState>,
}

impl IpGuardLayer {
    pub fn new(state: Arc<IpGuardState>) -> Self {
        Self { state }
    }
}

impl<S> Layer<S> for IpGuardLayer {
    type Service = IpGuardService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        IpGuardService {
            inner,
            state: Arc::clone(&self.state),
        }
    }
}
```

### 6.2 Service 实现（核心逻辑）

```rust
// src/ip_guard/layer.rs（续）
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, Response, StatusCode};
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::Service;

#[derive(Clone)]
pub struct IpGuardService<S> {
    inner: S,
    state: Arc<IpGuardState>,
}

impl<S> Service<Request<Body>> for IpGuardService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        // 从 ConnectInfo 扩展中提取客户端 IP
        let ip = req
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ci| ci.0.ip());

        let state = Arc::clone(&self.state);
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let Some(ip) = ip else {
                // 无法提取 IP，直接放行（不应发生，因为已配置 ConnectInfo）
                return inner.call(req).await;
            };

            // 1. 白名单检查：白名单 IP 直接放行
            if state.is_whitelisted(&ip) {
                return inner.call(req).await;
            }

            // 2. 黑名单检查：已封禁则直接拒绝
            if let Some(reason) = state.check_blacklist(&ip) {
                tracing::warn!("Blocked blacklisted IP: {}, reason: {}", ip, reason);
                return Ok(build_blocked_response(ip, reason));
            }

            // 3. 频率检测：记录请求并判断是否超阈值
            if state.record_and_check(&ip) {
                // 超过阈值，加入黑名单
                let reason = format!(
                    "Rate limit exceeded: >{} requests in {}s",
                    state.config().max_requests,
                    state.config().window.as_secs()
                );
                state.add_to_blacklist(ip, reason.clone());
                tracing::warn!("IP {} added to blacklist: {}", ip, reason);
                return Ok(build_blocked_response(ip, reason));
            }

            // 4. 正常放行
            inner.call(req).await
        })
    }
}

/// 构建被拒绝请求的响应
fn build_blocked_response(ip: std::net::IpAddr, reason: String) -> Response<Body> {
    use axum::http::header::CONTENT_TYPE;
    use serde_json::json;

    let body = serde_json::to_string(&json!({
        "code": 429,
        "msg": format!("Access denied for IP {}: {}", ip, reason),
        "data": null
    }))
    .unwrap_or_default();

    Response::builder()
        .status(StatusCode::TOO_MANY_REQUESTS)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap()
}
```

### 6.3 核心计数逻辑

```rust
// src/ip_guard/store.rs（实现部分）
use std::net::IpAddr;
use std::time::Instant;

impl IpGuardState {
    /// 记录一次请求，并判断是否超过频率阈值
    /// 返回 true 表示已超过阈值，应封禁
    pub fn record_and_check(&self, ip: &IpAddr) -> bool {
        let now = Instant::now();
        let window = self.config.window;
        let max = self.config.max_requests;

        let mut entry = self.counters.entry(*ip).or_default();
        let record = entry.value_mut();

        // 清理窗口外的过期时间戳
        let cutoff = now - window;
        while let Some(&front) = record.timestamps.front() {
            if front < cutoff {
                record.timestamps.pop_front();
            } else {
                break;
            }
        }

        // 记录本次请求
        record.timestamps.push_back(now);

        // 判断是否超阈值
        record.timestamps.len() > max
    }

    /// 检查 IP 是否在黑名单中
    /// 返回封禁原因；None 表示未封禁（或已到期自动解封）
    pub fn check_blacklist(&self, ip: &IpAddr) -> Option<String> {
        if let Some(entry) = self.blacklist.get(ip) {
            match entry.ban_until {
                // 永久封禁
                None => return Some(entry.reason.clone()),
                // 检查是否到期
                Some(until) if Instant::now() < until => return Some(entry.reason.clone()),
                // 已到期，惰性移除
                _ => {}
            }
        }
        // 惰性清理已过期条目
        self.blacklist.remove(ip);
        None
    }

    /// 将 IP 加入黑名单
    pub fn add_to_blacklist(&self, ip: IpAddr, reason: String) {
        let ban_until = self.config.ban_duration.map(|d| Instant::now() + d);
        self.blacklist.insert(ip, BlacklistEntry { ban_until, reason });
        // 封禁后清除计数器，节省内存
        self.counters.remove(&ip);
    }

    /// 从黑名单中移除 IP（手动解封）
    pub fn remove_from_blacklist(&self, ip: &IpAddr) -> bool {
        self.blacklist.remove(ip).is_some()
    }

    pub fn is_whitelisted(&self, ip: &IpAddr) -> bool {
        self.whitelist.contains(ip)
    }

    pub fn config(&self) -> &IpGuardConfig {
        &self.config
    }
}
```

---

## 7. 后台清理任务

计数器和过期黑名单需要定期清理，避免内存泄漏。

```rust
// src/ip_guard/cleaner.rs
use std::sync::Arc;
use std::time::{Duration, Instant};
use crate::ip_guard::store::IpGuardState;

/// 启动后台清理任务
/// 每隔 `interval` 清理一次过期的计数器和黑名单条目
pub fn start_cleanup_task(state: Arc<IpGuardState>, interval: Duration) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            let removed_counters = cleanup_counters(&state);
            let removed_blacklist = cleanup_blacklist(&state);
            if removed_counters > 0 || removed_blacklist > 0 {
                tracing::debug!(
                    "IP Guard cleanup: removed {} counters, {} blacklist entries",
                    removed_counters,
                    removed_blacklist
                );
            }
        }
    });
}

fn cleanup_counters(state: &IpGuardState) -> usize {
    let now = Instant::now();
    let window = state.config().window;
    let cutoff = now - window;
    let mut removed = 0;

    state.counters.retain(|_, record| {
        // 清理窗口内时间戳
        while let Some(&front) = record.timestamps.front() {
            if front < cutoff { record.timestamps.pop_front(); } else { break; }
        }
        // 如果队列为空则移除整个条目
        if record.timestamps.is_empty() { removed += 1; false } else { true }
    });
    removed
}

fn cleanup_blacklist(state: &IpGuardState) -> usize {
    let now = Instant::now();
    let mut removed = 0;
    state.blacklist.retain(|_, entry| {
        match entry.ban_until {
            Some(until) if now >= until => { removed += 1; false }
            _ => true,
        }
    });
    removed
}
```

---

## 8. 注册到 AppState 与 application.rs

```rust
// src/application.rs（修改）
use crate::ip_guard::store::{IpGuardConfig, IpGuardState};
use crate::ip_guard::layer::IpGuardLayer;
use crate::ip_guard::cleaner::start_cleanup_task;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub ip_guard: Arc<IpGuardState>,   // 新增
}

impl AppState {
    fn new(db: DatabaseConnection) -> Self {
        let ip_guard = Arc::new(IpGuardState::new(IpGuardConfig::default()));
        Self { db, ip_guard }
    }
}

// 在 Server::create_routes 中，将 IpGuardLayer 加在最外层：
fn create_routes(&self, state: AppState, router: Router<AppState>) -> Router {
    // 启动后台清理任务（每 5 分钟清理一次）
    start_cleanup_task(Arc::clone(&state.ip_guard), Duration::from_secs(300));

    let ip_guard_layer = IpGuardLayer::new(Arc::clone(&state.ip_guard));

    Router::new()
        .merge(router)
        .layer(ip_guard_layer)     // ← 最外层，最先执行
        .layer(timeout)
        .layer(body_limit)
        .layer(tracing)
        .layer(cors)
        .layer(normalize_path)
        .with_state(state)
}
```

---

## 9. 黑名单管理 API

提供管理接口，支持查看、手动封禁、解封操作。建议此路由单独加认证或仅内网访问。

```rust
// src/api/ip_admin.rs

use axum::{extract::State, routing::{get, post, delete}, Router, Json};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use crate::application::AppState;
use crate::response::ApiResponse;

#[derive(Deserialize)]
pub struct BanIpRequest {
    pub ip: IpAddr,
    pub reason: Option<String>,
    /// 封禁时长（秒），None 表示永久
    pub duration_secs: Option<u64>,
}

#[derive(Serialize)]
pub struct BlacklistItem {
    pub ip: String,
    pub reason: String,
    pub ban_until: Option<String>,  // ISO 8601 时间字符串
}

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/blacklist", get(list_blacklist))
        .route("/blacklist", post(ban_ip))
        .route("/blacklist/{ip}", delete(unban_ip))
}

/// 查看当前黑名单列表
async fn list_blacklist(
    State(state): State<AppState>,
) -> ApiResponse<Vec<BlacklistItem>> {
    let items = state.ip_guard.list_blacklist();
    ApiResponse::success("ok", Some(items))
}

/// 手动封禁 IP
async fn ban_ip(
    State(state): State<AppState>,
    Json(req): Json<BanIpRequest>,
) -> ApiResponse<()> {
    let reason = req.reason.unwrap_or_else(|| "Manually banned".to_string());
    state.ip_guard.add_to_blacklist_with_duration(
        req.ip,
        reason,
        req.duration_secs.map(std::time::Duration::from_secs),
    );
    tracing::warn!("Manually banned IP: {}", req.ip);
    ApiResponse::success("IP banned successfully", None)
}

/// 解封 IP
async fn unban_ip(
    State(state): State<AppState>,
    axum::extract::Path(ip): axum::extract::Path<IpAddr>,
) -> ApiResponse<()> {
    let removed = state.ip_guard.remove_from_blacklist(&ip);
    if removed {
        tracing::info!("Manually unbanned IP: {}", ip);
        ApiResponse::success("IP unbanned successfully", None)
    } else {
        ApiResponse::error(format!("IP {} not found in blacklist", ip))
    }
}
```

在 `api/mod.rs` 中注册管理路由：
```rust
// /admin/ip 路由，建议加额外的 AdminOnly 认证层
.nest("/admin/ip", ip_admin::routes())
```

---

## 10. error.rs 扩展

```rust
// src/error.rs（新增一个变体）
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    // ...现有变体...

    #[error("Rate Limit Exceeded: {0}")]
    RateLimitError(String),   // 新增
}

impl ApiError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            // ...现有匹配...
            ApiError::RateLimitError(_) => StatusCode::TOO_MANY_REQUESTS,  // 新增
        }
    }
}
```

---

## 11. 配置外化

将限流参数移入 `config/dev.yaml` 和 `config/prod.yaml`，支持不同环境使用不同阈值：

```yaml
# config/dev.yaml
ip_guard:
  window_secs: 60
  max_requests: 1000    # 开发环境放宽
  ban_duration_secs: 300
  whitelist:
    - "127.0.0.1"
    - "::1"

# config/prod.yaml
ip_guard:
  window_secs: 60
  max_requests: 200     # 生产环境收紧
  ban_duration_secs: 3600
  whitelist:
    - "127.0.0.1"
```

对应 `src/config/` 中新增：
```rust
// src/config/ip_guard.rs
#[derive(Debug, Deserialize, Clone)]
pub struct IpGuardConfig {
    pub window_secs: u64,
    pub max_requests: usize,
    pub ban_duration_secs: Option<u64>,
    pub whitelist: Vec<String>,
}
```

---

## 12. 请求流程图

```
客户端请求
    │
    ▼
IpGuardLayer（Tower Middleware）
    │
    ├─── 提取客户端 IP（ConnectInfo<SocketAddr>）
    │
    ├─── [白名单检查] ──── 在白名单中 ──────────────────────────► 直接放行
    │
    ├─── [黑名单检查] ──── 在黑名单且未过期 ──► 返回 429（不进入业务层）
    │         │
    │         └─ 条目已过期 ──► 惰性删除，继续检查
    │
    ├─── [频率检测] ─────── 滑动窗口计数
    │         │
    │         ├─ 未超阈值 ──────────────────────────────────────► 放行，进入后续中间件
    │         │
    │         └─ 超过阈值 ──► 加入黑名单 ──► 返回 429
    │
    ▼
CorsLayer → TraceLayer → TimeoutLayer → JWTAuthLayer → Router（业务）
```

---

## 13. 实施步骤

### Phase 1：基础拦截（核心功能）
1. `Cargo.toml` 添加 `dashmap = "6"`
2. 创建 `src/ip_guard/` 模块，实现 `IpGuardState`、`IpGuardLayer`、`IpGuardService`
3. `AppState` 中集成 `Arc<IpGuardState>`
4. 在 `application.rs` 的 `create_routes` 中注册 `IpGuardLayer`（最外层）
5. 验证：压测工具（如 `wrk`/`hey`）模拟同一 IP 高频请求，确认触发封禁并返回 429

### Phase 2：维护与管理
6. 实现后台清理任务 `cleaner.rs`，防止内存泄漏
7. 实现 `/admin/ip` 管理 API（查看黑名单、手动封禁、解封）
8. `error.rs` 新增 `RateLimitError`

### Phase 3：配置与优化
9. 将限流参数移入 `config/*.yaml`，支持环境差异化配置
10. 为 `/auth/login` 单独配置更严格的限流规则（登录接口最易被暴力破解）
11. 添加 `tracing` 指标统计（当前黑名单数量、总封禁次数）
12. 编写单元测试（覆盖：白名单放行、黑名单拦截、频率超限封禁、到期自动解封）

---

## 14. 新增依赖汇总

```toml
# Cargo.toml
dashmap = "6"    # 并发 HashMap，用于计数器和黑名单存储
```

> 仅需 1 个新依赖。`tokio`、`tower`、`axum`、`serde_json`、`tracing` 均已在项目中存在。
