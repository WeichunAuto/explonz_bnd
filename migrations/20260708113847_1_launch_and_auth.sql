-- Add migration script here

-- =============================================================================
-- Explonz — 启动 & 身份认证 数据库 Schema
-- 来源：docs/architecture/launch-and-authentication/TDS.md v1.2
-- =============================================================================


-- ---------------------------------------------------------------------------
-- 扩展
-- ---------------------------------------------------------------------------

CREATE EXTENSION IF NOT EXISTS "pgcrypto";   -- 提供 crypt()（密码哈希验证）


-- ---------------------------------------------------------------------------
-- 枚举类型
-- ---------------------------------------------------------------------------

-- 登录方式枚举，与 Flutter 端 AuthProvider { email, google, facebook } 保持一致
CREATE TYPE auth_provider_type AS ENUM ('email', 'google', 'facebook');

-- 性别枚举；NULL 表示用户未填写
CREATE TYPE gender_type AS ENUM ('male', 'female', 'prefer_not_to_say');


-- ---------------------------------------------------------------------------
-- 表：users（用户主表）
--
-- 存储用户的基本信息。
-- - email 允许为 NULL：部分 Social 登录（如 Facebook 受限权限）可能不返回邮箱
-- - nickname / avatar_url 首次 Social 登录时由 OAuth 提供方填充，用户后续可自行修改
-- ---------------------------------------------------------------------------

CREATE TABLE users (
    -- UUIDv7：前 48 位为毫秒时间戳，天然有序；PostgreSQL 17+ 内置，18.3 直接可用
    id          UUID        PRIMARY KEY DEFAULT uuidv7(),
    nickname    TEXT        NOT NULL,                               -- 用户昵称，首次登录取自 OAuth 提供方
    avatar_url  TEXT,                                              -- 头像 URL，可为空
    email       TEXT,                                              -- 邮箱，可为空；有值时全局唯一
    gender      gender_type,                                       -- 性别，可为空（用户未填写时为 NULL）
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),                -- 账号创建时间
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),                -- 最后更新时间，由触发器自动维护

    -- 邮箱唯一约束：多个 NULL 值不互相冲突，有值时不允许重复
    -- NULLS NOT DISTINCT 是 PostgreSQL 15+ 语法
    CONSTRAINT uq_users_email UNIQUE NULLS NOT DISTINCT (email)
);

-- 触发器函数：每次 UPDATE 时自动更新 updated_at 字段
CREATE OR REPLACE FUNCTION fn_set_updated_at()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION fn_set_updated_at();


-- ---------------------------------------------------------------------------
-- 表：user_auth_providers（用户登录方式绑定表）
--
-- 每个用户每种登录方式对应一行，支持同一用户绑定多种登录方式：
--   • 邮箱/密码   → provider = 'email'，password_hash 有值
--   • Google 登录 → provider = 'google'，provider_user_id 为 Google UID
--   • Facebook 登录 → provider = 'facebook'，provider_user_id 为 Facebook UID
--
-- 账号合并逻辑（由后端静默处理，前端无感知，见 TDS §5.0 TQ2）：
--   当 OAuth 登录返回的邮箱与已有 users.email 匹配时，
--   后端将新的 provider 行绑定到已有用户，而不是创建新账号。
-- ---------------------------------------------------------------------------

CREATE TABLE user_auth_providers (
    id               UUID               PRIMARY KEY DEFAULT uuidv7(),
    user_id          UUID               NOT NULL REFERENCES users(id) ON DELETE CASCADE,  -- 关联用户
    provider         auth_provider_type NOT NULL,                                          -- 登录方式

    -- OAuth 提供方的外部用户 ID（email 登录时为 NULL）
    provider_user_id TEXT,

    -- OAuth 提供方返回的邮箱，用于账号合并查找
    -- 可能与 users.email 不同（用户后续修改了主邮箱）
    provider_email   TEXT,

    -- 密码哈希（bcrypt），仅 provider = 'email' 时填写，Social 登录行为 NULL
    password_hash    TEXT,

    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),  -- 绑定时间

    -- 同一用户不能重复绑定同一登录方式
    CONSTRAINT uq_user_auth_providers_user_provider
        UNIQUE (user_id, provider),

    -- 同一 OAuth 外部 ID 不能绑定到多个账号（防止身份冲突）
    CONSTRAINT uq_user_auth_providers_provider_external
        UNIQUE (provider, provider_user_id)
);

-- 账号合并核心查找路径：根据 OAuth 返回邮箱查找已有用户
CREATE INDEX idx_user_auth_providers_provider_email
    ON user_auth_providers (provider_email)
    WHERE provider_email IS NOT NULL;

-- 按用户查询其所有绑定的登录方式
CREATE INDEX idx_user_auth_providers_user_id
    ON user_auth_providers (user_id);


-- ---------------------------------------------------------------------------
-- 表：refresh_tokens（Refresh Token 存储表）
--
-- 实现 Refresh Token 轮换机制（见 TDS §6.1、§10.3）：
--   1. 每次登录成功后插入一条新记录
--   2. 每次 POST /auth/refresh：
--      a. 根据 token_hash 验证 Token 有效性
--      b. 将旧记录的 revoked_at 标记为当前时间
--      c. 插入新记录（新 token_hash + 新 expires_at）
--   3. 退出登录时将对应记录的 revoked_at 置为当前时间
--
-- 安全说明：
--   token_hash 存储客户端 Token 的 SHA-256 哈希值，原始 Token 不入库。
--
-- Token 有效期（由 TDS TQ3 确认，服务端以 Unix 时间戳秒返回给客户端）：
--   Access Token  — 1 小时（短效，由客户端持有，不入库）
--   Refresh Token — 30 天（expires_at = created_at + INTERVAL '30 days'）
-- ---------------------------------------------------------------------------

CREATE TABLE refresh_tokens (
    id           UUID        PRIMARY KEY DEFAULT uuidv7(),
    user_id      UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,  -- 关联用户

    -- 客户端 Token 的 SHA-256 哈希值（十六进制字符串），原始值不存储
    token_hash   TEXT        NOT NULL UNIQUE,

    -- Token 绝对过期时间；与返回给客户端的 refresh_token_expires_at（Unix 秒）一致
    expires_at   TIMESTAMPTZ NOT NULL,

    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),  -- Token 签发时间

    -- NULL    → Token 有效（可使用）
    -- 非 NULL → Token 已吊销（轮换或主动退出登录）
    revoked_at   TIMESTAMPTZ
);

-- 按用户查询其所有 Token（如强制全端下线时使用）
CREATE INDEX idx_refresh_tokens_user_id
    ON refresh_tokens (user_id);

-- 有效 Token 专用局部索引，跳过已吊销记录，提升验证性能
CREATE INDEX idx_refresh_tokens_active
    ON refresh_tokens (token_hash, expires_at)
    WHERE revoked_at IS NULL;


-- ---------------------------------------------------------------------------
-- 视图：v_user_providers
--
-- 聚合每个用户已绑定的登录方式，直接对应 GET /users/me 接口的 providers 数组：
--   { "id": "...", "providers": ["email", "google"] }
-- ---------------------------------------------------------------------------

CREATE VIEW v_user_providers AS
SELECT
    u.id,
    u.nickname,
    u.avatar_url,
    u.email,
    -- 将该用户所有绑定的 provider 聚合为数组，按绑定时间升序排列
    COALESCE(
        ARRAY_AGG(uap.provider::TEXT ORDER BY uap.created_at),
        '{}'::TEXT[]
    ) AS providers,
    u.created_at,
    u.updated_at
FROM users u
LEFT JOIN user_auth_providers uap ON uap.user_id = u.id
GROUP BY u.id;


-- ---------------------------------------------------------------------------
-- 函数：fn_purge_expired_refresh_tokens（清理过期 Token）
--
-- 删除所有已过期或已吊销的 Refresh Token，返回删除行数。
-- 建议通过 pg_cron 每天凌晨定时执行：
--   SELECT cron.schedule('0 3 * * *', 'SELECT fn_purge_expired_refresh_tokens()');
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION fn_purge_expired_refresh_tokens()
RETURNS INTEGER LANGUAGE plpgsql AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM refresh_tokens
    WHERE expires_at < NOW()   -- 已超过有效期
       OR revoked_at IS NOT NULL;  -- 已被主动吊销

    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$;
