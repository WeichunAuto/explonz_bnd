# 登录验证：使用单条 SQL（pgcrypto crypt）一步完成

## 前提条件

此方案要求密码哈希在**写入时**就使用 PostgreSQL `pgcrypto` 扩展的 `crypt()` 存储，而不是 Rust 的 `bcrypt`。

如果你当前用 `src/common.rs` 的 `hash_password()` 存储的是 bcrypt 哈希，则必须同步修改注册流程，否则此方案无法工作。

---

## 单条 SQL 的思路

```sql
SELECT u.id, u.nickname, u.email
FROM users u
JOIN user_auth_providers uap ON uap.user_id = u.id
WHERE u.email = $1
  AND uap.provider = 'email'
  AND uap.password_hash = crypt($2, uap.password_hash)
```

- `crypt($2, uap.password_hash)` 用存储的哈希作为 salt 对输入密码重新哈希，再与存储值比对。
- 仅当 email 匹配**且**密码正确时才返回行。
- 返回空行意味着"用户不存在或密码错误"，无法区分两种情况（见安全说明）。

---

## 修改点一：注册时改用 pgcrypto 存储密码

注册时不再调用 Rust 的 `hash_password()`，改为在 INSERT SQL 里用 `crypt(password, gen_salt('bf'))` 存储：

```sql
-- 注册示例（在 register handler 里用 raw SQL 执行）
INSERT INTO user_auth_providers (user_id, provider, password_hash)
VALUES ($1, 'email', crypt($2, gen_salt('bf')));
```

或者在 Rust 里通过 SeaORM 的 `query_one` 执行带 `crypt` 的 raw SQL。

---

## 修改点二：login handler 改用单条 raw SQL

SeaORM 支持 `sea_orm::Statement` 执行 raw SQL：

```rust
use sea_orm::{ConnectionTrait, DbBackend, Statement, FromQueryResult};
use serde::Deserialize;

#[derive(Debug, FromQueryResult)]
struct LoginUser {
    id: uuid::Uuid,
    nickname: String,
    email: String,
}

#[debug_handler]
#[tracing::instrument(name = "login", skip_all, fields(account = %email, IP = %addr))]
pub async fn login(
    State(AppState { db }): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    BValidJson(LoginParams { email, password }): BValidJson<LoginParams>,
) -> ApiResult<LoginResponse> {
    tracing::info!("start login, account: {}", email);

    // 单条 SQL：JOIN + pgcrypto crypt 验证，一步完成
    let sql = r#"
        SELECT u.id, u.nickname, u.email
        FROM users u
        JOIN user_auth_providers uap ON uap.user_id = u.id
        WHERE u.email = $1
          AND uap.provider = 'email'
          AND uap.password_hash = crypt($2, uap.password_hash)
        LIMIT 1
    "#;

    let user = LoginUser::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        [email.clone().into(), password.into()],
    ))
    .one(&db)
    .await?;

    let user = user.ok_or_else(|| {
        tracing::warn!("login failed (user not found or wrong password), account: {}", email);
        ApiError::BizError("user or password is not correct".to_string())
    })?;

    // 生成 JWT
    let principal = Principal {
        id: user.id.to_string(),
        name: user.nickname,
        email: user.email,
    };
    let access_token = get_jwt().encode(principal)?;

    tracing::info!("login success, IP: {}, user: {}", addr, email);

    Ok(ApiResponse::success(
        "login success",
        Some(LoginResponse { access_token }),
    ))
}
```

需要在文件顶部补充 import：

```rust
use sea_orm::{ConnectionTrait, DbBackend, Statement, FromQueryResult};
```

---

## 与两步方案的对比

| 维度 | 两步方案（login_verify.md） | 单 SQL 方案（本文件） |
|------|-----------------------------|-----------------------|
| 密码哈希算法 | Rust bcrypt（现有代码） | pgcrypto crypt/bf（需迁移） |
| DB 查询次数 | 2 次 | 1 次 |
| 区分"用户不存在"vs"密码错误" | 可以（分两步） | 不能（单次结果只有有/无） |
| 需改注册流程 | 否 | **是** |
| 需要 pgcrypto 扩展 | 否 | **是**（`CREATE EXTENSION pgcrypto`） |
| 维护复杂度 | 略高（多一次查询） | 较低（逻辑全在 SQL） |

---

## 安全说明

单 SQL 方案无法区分"用户不存在"和"密码错误"，统一返回相同错误消息。
这实际上是**更安全的做法**——防止攻击者通过不同错误信息枚举已注册邮箱。
真实原因通过 `tracing::warn!` 记录在日志中，便于内部排查。

---

## 需要修改的文件汇总

| 文件 | 改动 |
|------|------|
| `src/api/login_auth.rs` | 替换 login handler，用 raw SQL + `FromQueryResult` |
| 注册 handler（register） | 改用 `crypt($password, gen_salt('bf'))` 存储密码 |
| 数据库迁移 | 确保已启用 `CREATE EXTENSION IF NOT EXISTS pgcrypto` |
| `src/common.rs` | `hash_password` / `verify_password` 可从 login 流程中移除（注册也要对应去掉） |
