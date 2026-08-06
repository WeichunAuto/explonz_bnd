-- Add migration script here
-- =============================================================================
-- Explonz — Home (Discover Feed + Spot Detail) 数据库 Schema
-- 来源：docs/architecture/home/TDS.md v1.0
-- 前置依赖：docs/sql/launch-and-authentication.sql（users 表必须已存在）
-- 执行顺序：launch-and-authentication.sql → 本文件
-- =============================================================================


-- ---------------------------------------------------------------------------
-- 函数：fn_set_updated_at（幂等）
--
-- 首次定义于 launch-and-authentication.sql。
-- 此处使用 CREATE OR REPLACE 确保本文件可单独执行，重复定义无副作用。
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION fn_set_updated_at()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;


-- ---------------------------------------------------------------------------
-- 枚举类型
-- ---------------------------------------------------------------------------

-- 帖子类型，与 Flutter 端 PostType { standard, comment, repost } 保持一致
CREATE TYPE post_type_enum AS ENUM ('standard', 'comment', 'repost');


-- 用户徽章类型，与 Flutter 端 UserBadgeType { localLegend, explorer, contributor } 保持一致
CREATE TYPE user_badge_type_enum AS ENUM ('local_legend', 'explorer', 'contributor');


-- ---------------------------------------------------------------------------
-- 扩展 users 表：新增用户徽章字段
-- （users 表已定义于 docs/sql/launch-and-authentication.sql）
-- ---------------------------------------------------------------------------

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS badge_type  user_badge_type_enum,   -- 徽章类型，NULL 表示无徽章
    ADD COLUMN IF NOT EXISTS badge_label TEXT;                    -- 徽章显示文案，如 "Local Legend"


-- ---------------------------------------------------------------------------
-- 表：spots（地点主表）
--
-- 不存储的字段（运行时计算）：
--   drive_duration — 由用户当前 GPS 位置 + 本表 latitude/longitude 在客户端实时计算
--   is_open        — 由 fn_is_spot_open() 根据当前时间 + spot_opening_hours 实时计算
--   cover_image_url — 多张封面图存于 spot_photos 表
-- ---------------------------------------------------------------------------

CREATE TABLE spots (
    id          UUID         PRIMARY KEY DEFAULT uuidv7(),
    name        TEXT         NOT NULL,
    -- 0.0–5.0 星；NUMERIC(2,1) 可表示 0.0 到 9.9，加 CHECK 约束限定上限
    rating      NUMERIC(2,1) NOT NULL DEFAULT 0.0
                    CHECK (rating >= 0.0 AND rating <= 5.0),
    location    TEXT         NOT NULL,           -- 人类可读地址，如 "Kumeu, Auckland"
    latitude    DOUBLE PRECISION NOT NULL,        -- WGS-84 纬度，客户端计算驾车时长使用
    longitude   DOUBLE PRECISION NOT NULL,        -- WGS-84 经度
    description TEXT         NOT NULL DEFAULT '',
    -- 封面图 URL 数组，index 0 为主图（列表卡片使用），由管理员整体维护
    photo_urls  TEXT[]       NOT NULL DEFAULT '{}',
    -- 属性标签，由管理员后台整体维护，随地点一起读取，无需独立查询
    -- 格式：[{"type": "family_friendly", "label": "Family Friendly"}, ...]
    attributes  JSONB        NOT NULL DEFAULT '[]',
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE TRIGGER trg_spots_updated_at
    BEFORE UPDATE ON spots
    FOR EACH ROW EXECUTE FUNCTION fn_set_updated_at();

-- Nearby Tab 地理范围查询（经纬度范围过滤）
CREATE INDEX idx_spots_lat_lng ON spots (latitude, longitude);



-- ---------------------------------------------------------------------------
-- 表：spot_opening_hours（地点营业时间，结构化存储）
--
-- day_of_week 遵循 PostgreSQL EXTRACT(DOW) 规范：0=Sunday … 6=Saturday
-- is_closed = TRUE 表示当天休息（open_time / close_time 忽略）
-- 支持跨午夜营业（如酒吧 22:00 ~ 02:00）：close_time < open_time 时视为跨天
-- ---------------------------------------------------------------------------

CREATE TABLE spot_opening_hours (
    id           UUID     PRIMARY KEY DEFAULT uuidv7(),
    spot_id      UUID     NOT NULL REFERENCES spots(id) ON DELETE CASCADE,
    day_of_week  SMALLINT NOT NULL CHECK (day_of_week BETWEEN 0 AND 6),
    is_closed    BOOLEAN  NOT NULL DEFAULT FALSE,
    is_open_24h  BOOLEAN  NOT NULL DEFAULT FALSE,
    open_time    TIME,    -- is_closed=FALSE 且 is_open_24h=FALSE 时必填
    close_time   TIME,    -- is_closed=FALSE 且 is_open_24h=FALSE 时必填

    CONSTRAINT uq_spot_opening_hours_day UNIQUE (spot_id, day_of_week),

    -- is_closed 与 is_open_24h 互斥
    CONSTRAINT chk_opening_hours_exclusive
        CHECK (NOT (is_closed = TRUE AND is_open_24h = TRUE)),

    -- 普通营业模式下必须提供具体时间
    CONSTRAINT chk_opening_hours_times
        CHECK (
            is_closed = TRUE
            OR is_open_24h = TRUE
            OR (open_time IS NOT NULL AND close_time IS NOT NULL)
        )
);

CREATE INDEX idx_spot_opening_hours_spot_id ON spot_opening_hours (spot_id);


-- ---------------------------------------------------------------------------
-- 函数：fn_is_spot_open — 根据当前时间判断地点是否营业中
--
-- 判断逻辑（按顺序）：
--   1. 若当日无记录（未配置营业时间）         → FALSE
--   2. 若 is_closed = TRUE（当日休息）         → FALSE
--   3. 若 is_open_24h = TRUE（全天营业）       → TRUE
--   4. 若 open_time <= close_time（正常区间）  → 当前时间是否在 [open, close] 内
--   5. 若 open_time >  close_time（跨午夜）   → 当前时间 >= open 或 <= close
--
-- 示例：
--   SELECT fn_is_spot_open('spot-uuid');
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION fn_is_spot_open(p_spot_id UUID)
RETURNS BOOLEAN
LANGUAGE plpgsql STABLE AS $$
DECLARE
    v_dow   SMALLINT := EXTRACT(DOW FROM NOW())::SMALLINT;
    v_time  TIME     := NOW()::TIME;
    v_hours RECORD;
BEGIN
    SELECT is_closed, is_open_24h, open_time, close_time
    INTO v_hours
    FROM spot_opening_hours
    WHERE spot_id = p_spot_id AND day_of_week = v_dow;

    -- 未配置当日营业时间，或当日休息
    IF NOT FOUND OR v_hours.is_closed THEN
        RETURN FALSE;
    END IF;

    -- 24 小时营业
    IF v_hours.is_open_24h THEN
        RETURN TRUE;
    END IF;

    -- 正常区间（如 09:00 ~ 17:00）
    IF v_hours.open_time <= v_hours.close_time THEN
        RETURN v_time BETWEEN v_hours.open_time AND v_hours.close_time;
    END IF;

    -- 跨午夜区间（如 22:00 ~ 02:00）
    RETURN v_time >= v_hours.open_time OR v_time <= v_hours.close_time;
END;
$$;




-- ---------------------------------------------------------------------------
-- 函数：fn_is_in_season — 根据当前日期判断是否处于时令季中
--
-- 支持跨年季节（如南半球草莓季 11 月 ~ 2 月）：
--   当 start_mmdd > end_mmdd 时，视为跨年区间，今日在 [start, 12/31] 或 [1/1, end] 均为当季
--
-- 示例：
--   SELECT fn_is_in_season(11, 1, 2, 28);   -- 11/1 ~ 2/28，南半球草莓季
--   SELECT fn_is_in_season(6, 1, 8, 31);    -- 6/1  ~ 8/31，北半球夏季
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION fn_is_in_season(
    start_month SMALLINT,
    start_day   SMALLINT,
    end_month   SMALLINT,
    end_day     SMALLINT
)
RETURNS BOOLEAN
LANGUAGE plpgsql STABLE AS $$
DECLARE
    today_mmdd INT := EXTRACT(MONTH FROM NOW())::INT * 100
                    + EXTRACT(DAY   FROM NOW())::INT;
    start_mmdd INT := start_month * 100 + start_day;
    end_mmdd   INT := end_month   * 100 + end_day;
BEGIN
    IF start_mmdd <= end_mmdd THEN
        -- 同年区间，如 6/1 ~ 8/31
        RETURN today_mmdd BETWEEN start_mmdd AND end_mmdd;
    ELSE
        -- 跨年区间，如 11/1 ~ 2/28
        RETURN today_mmdd >= start_mmdd OR today_mmdd <= end_mmdd;
    END IF;
END;
$$;


-- ---------------------------------------------------------------------------
-- 表：seasonal_pickings（时令活动推荐）
--
-- 每条记录对应一个地点的时令活动。
-- season_start_month / season_start_day — 每年开始月日（如 11, 1 = 11 月 1 日）
-- season_end_month   / season_end_day   — 每年结束月日（如 2, 28 = 2 月 28 日）
--
-- is_in_season 由 fn_is_in_season() 实时计算，后端查询时调用该函数返回给客户端，
-- 不存储在表中，避免需要定时任务维护状态。
-- ---------------------------------------------------------------------------

CREATE TABLE seasonal_pickings (
    id                 UUID        PRIMARY KEY DEFAULT uuidv7(),
    spot_id            UUID        NOT NULL REFERENCES spots(id) ON DELETE CASCADE,
    season_start_month SMALLINT    NOT NULL CHECK (season_start_month BETWEEN 1 AND 12),
    season_start_day   SMALLINT    NOT NULL CHECK (season_start_day   BETWEEN 1 AND 31),
    season_end_month   SMALLINT    NOT NULL CHECK (season_end_month   BETWEEN 1 AND 12),
    season_end_day     SMALLINT    NOT NULL CHECK (season_end_day     BETWEEN 1 AND 31),
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER trg_seasonal_pickings_updated_at
    BEFORE UPDATE ON seasonal_pickings
    FOR EACH ROW EXECUTE FUNCTION fn_set_updated_at();

CREATE INDEX idx_seasonal_pickings_spot_id ON seasonal_pickings (spot_id);


-- ---------------------------------------------------------------------------
-- 表：posts（帖子主表，统一承载三种类型）
--
-- type = 'standard'  → 原创帖子；original_post_id 为 NULL
-- type = 'comment'   → 评论帖子；original_post_id 指向被评论的原帖
-- type = 'repost'    → 转发帖子；original_post_id 指向被转发的原帖
--
-- like_count / comment_count / repost_count 为冗余计数列，
-- 由各自触发器自动维护，避免每次查询实时聚合。
-- ---------------------------------------------------------------------------

CREATE TABLE posts (
    id               UUID           PRIMARY KEY DEFAULT uuidv7(),
    type             post_type_enum NOT NULL DEFAULT 'standard',
    author_id        UUID           NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title            TEXT           NOT NULL DEFAULT '',
    body             TEXT           NOT NULL DEFAULT '',
    -- spot_id 可为空：帖子可不关联特定地点
    spot_id          UUID           REFERENCES spots(id) ON DELETE SET NULL,
    -- comment / repost 时指向原帖；original_post 被删除时置 NULL（SET NULL）
    original_post_id UUID           REFERENCES posts(id) ON DELETE SET NULL,
    like_count       INT            NOT NULL DEFAULT 0 CHECK (like_count >= 0),
    comment_count    INT            NOT NULL DEFAULT 0 CHECK (comment_count >= 0),
    repost_count     INT            NOT NULL DEFAULT 0 CHECK (repost_count >= 0),
    created_at       TIMESTAMPTZ    NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ    NOT NULL DEFAULT NOW(),

    -- comment / repost 类型必须有 original_post_id
    CONSTRAINT chk_posts_original_required
        CHECK (type = 'standard' OR original_post_id IS NOT NULL)
);

CREATE TRIGGER trg_posts_updated_at
    BEFORE UPDATE ON posts
    FOR EACH ROW EXECUTE FUNCTION fn_set_updated_at();

-- Feed 全局时间排序（For You / Trending Tab）
CREATE INDEX idx_posts_created_at       ON posts (created_at DESC);

-- Nearby Tab：按地点 + 时间查询
CREATE INDEX idx_posts_spot_created_at  ON posts (spot_id, created_at DESC)
    WHERE spot_id IS NOT NULL;

-- 按作者查询其所有帖子
CREATE INDEX idx_posts_author_id        ON posts (author_id, created_at DESC);

-- 按原帖查询所有评论 / 转发
CREATE INDEX idx_posts_original_post_id ON posts (original_post_id)
    WHERE original_post_id IS NOT NULL;

-- 同一用户对同一帖子只能转发一次
CREATE UNIQUE INDEX uq_posts_user_repost
    ON posts (author_id, original_post_id)
    WHERE type = 'repost';


-- ---------------------------------------------------------------------------
-- 表：post_photos（帖子图片，与 posts 一对多，展示顺序有意义）
-- ---------------------------------------------------------------------------

CREATE TABLE post_photos (
    id          UUID     PRIMARY KEY DEFAULT uuidv7(),
    post_id     UUID     NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    url         TEXT     NOT NULL,
    order_index SMALLINT NOT NULL DEFAULT 0,   -- 从 0 开始；前端按此字段排序

    CONSTRAINT uq_post_photos_post_order UNIQUE (post_id, order_index)
);

CREATE INDEX idx_post_photos_post_id ON post_photos (post_id, order_index);



-- ---------------------------------------------------------------------------
-- 表：post_likes（帖子点赞）
--
-- Toggle 语义：插入 = 点赞；删除 = 取消点赞
-- 由触发器 trg_post_likes_count 自动维护 posts.like_count
-- ---------------------------------------------------------------------------

CREATE TABLE post_likes (
    user_id    UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    post_id    UUID        NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (user_id, post_id)
);

CREATE INDEX idx_post_likes_post_id ON post_likes (post_id);

CREATE OR REPLACE FUNCTION fn_post_likes_count()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE posts SET like_count = like_count + 1 WHERE id = NEW.post_id;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE posts SET like_count = GREATEST(like_count - 1, 0) WHERE id = OLD.post_id;
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER trg_post_likes_count
    AFTER INSERT OR DELETE ON post_likes
    FOR EACH ROW EXECUTE FUNCTION fn_post_likes_count();


-- ---------------------------------------------------------------------------
-- 表：post_bookmarks（帖子书签/收藏，与 post_likes 结构对称）
--
-- Toggle 语义：插入 = 收藏；删除 = 取消收藏
-- 无对应计数列（前端只需 is_bookmarked 状态，不显示收藏总数）
-- ---------------------------------------------------------------------------

CREATE TABLE post_bookmarks (
    user_id    UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    post_id    UUID        NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (user_id, post_id)
);

CREATE INDEX idx_post_bookmarks_post_id ON post_bookmarks (post_id);
CREATE INDEX idx_post_bookmarks_user_id ON post_bookmarks (user_id);   -- 查询用户收藏列表


-- ---------------------------------------------------------------------------
-- 触发器：posts.comment_count 自动维护
-- type='comment' 的帖子插入 / 删除时，更新被评论帖子的 comment_count
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION fn_post_comment_count()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' AND NEW.type = 'comment' AND NEW.original_post_id IS NOT NULL THEN
        UPDATE posts SET comment_count = comment_count + 1 WHERE id = NEW.original_post_id;
    ELSIF TG_OP = 'DELETE' AND OLD.type = 'comment' AND OLD.original_post_id IS NOT NULL THEN
        UPDATE posts SET comment_count = GREATEST(comment_count - 1, 0) WHERE id = OLD.original_post_id;
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER trg_post_comment_count
    AFTER INSERT OR DELETE ON posts
    FOR EACH ROW EXECUTE FUNCTION fn_post_comment_count();


-- ---------------------------------------------------------------------------
-- 触发器：posts.repost_count 自动维护
-- type='repost' 的帖子插入 / 删除时，更新被转发帖子的 repost_count
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION fn_post_repost_count()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' AND NEW.type = 'repost' AND NEW.original_post_id IS NOT NULL THEN
        UPDATE posts SET repost_count = repost_count + 1 WHERE id = NEW.original_post_id;
    ELSIF TG_OP = 'DELETE' AND OLD.type = 'repost' AND OLD.original_post_id IS NOT NULL THEN
        UPDATE posts SET repost_count = GREATEST(repost_count - 1, 0) WHERE id = OLD.original_post_id;
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER trg_post_repost_count
    AFTER INSERT OR DELETE ON posts
    FOR EACH ROW EXECUTE FUNCTION fn_post_repost_count();


-- ---------------------------------------------------------------------------
-- 表：tips（地点贴士）
--
-- helpful_count 为冗余计数列，由 trg_tip_votes_count 自动维护。
-- is_top_tip 标记当前地点最高赞的置顶贴士，每个地点最多一条（局部唯一索引）。
-- ---------------------------------------------------------------------------

CREATE TABLE tips (
    id            UUID        PRIMARY KEY DEFAULT uuidv7(),
    author_id     UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    spot_id       UUID        NOT NULL REFERENCES spots(id) ON DELETE CASCADE,
    body          TEXT        NOT NULL,
    helpful_count INT         NOT NULL DEFAULT 0 CHECK (helpful_count >= 0),
    is_top_tip    BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER trg_tips_updated_at
    BEFORE UPDATE ON tips
    FOR EACH ROW EXECUTE FUNCTION fn_set_updated_at();

-- 地点贴士列表：按 helpful_count 降序 + 时间降序
CREATE INDEX idx_tips_spot_id ON tips (spot_id, helpful_count DESC, created_at DESC);
CREATE INDEX idx_tips_author_id ON tips (author_id);

-- 每个地点最多一条置顶贴士
CREATE UNIQUE INDEX uq_tips_top_tip_per_spot ON tips (spot_id)
    WHERE is_top_tip = TRUE;


-- ---------------------------------------------------------------------------
-- 表：tip_votes（贴士「有帮助」投票）
--
-- vote = TRUE  → 认为有帮助
-- vote = FALSE → 认为没帮助
-- 取消投票（前端发送 "vote": null）→ 后端删除对应行
--
-- 由触发器 trg_tip_votes_count 自动维护 tips.helpful_count：
--   仅统计 vote = TRUE 的行数
-- ---------------------------------------------------------------------------

CREATE TABLE tip_votes (
    user_id    UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tip_id     UUID        NOT NULL REFERENCES tips(id)  ON DELETE CASCADE,
    vote       BOOLEAN     NOT NULL,   -- TRUE = 有帮助；FALSE = 没帮助
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (user_id, tip_id)
);

CREATE INDEX idx_tip_votes_tip_id ON tip_votes (tip_id);

CREATE OR REPLACE FUNCTION fn_tip_votes_count()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.vote = TRUE THEN
            UPDATE tips SET helpful_count = helpful_count + 1 WHERE id = NEW.tip_id;
        END IF;

    ELSIF TG_OP = 'UPDATE' THEN
        -- FALSE → TRUE：加 1
        IF OLD.vote = FALSE AND NEW.vote = TRUE THEN
            UPDATE tips SET helpful_count = helpful_count + 1 WHERE id = NEW.tip_id;
        -- TRUE → FALSE：减 1
        ELSIF OLD.vote = TRUE AND NEW.vote = FALSE THEN
            UPDATE tips SET helpful_count = GREATEST(helpful_count - 1, 0) WHERE id = NEW.tip_id;
        END IF;

    ELSIF TG_OP = 'DELETE' THEN
        IF OLD.vote = TRUE THEN
            UPDATE tips SET helpful_count = GREATEST(helpful_count - 1, 0) WHERE id = OLD.tip_id;
        END IF;
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER trg_tip_votes_count
    AFTER INSERT OR UPDATE OR DELETE ON tip_votes
    FOR EACH ROW EXECUTE FUNCTION fn_tip_votes_count();


-- ---------------------------------------------------------------------------
-- 表：user_saved_spots（用户收藏地点）
--
-- Toggle 语义：插入 = 收藏；删除 = 取消收藏
-- 对应 API：POST /spots/:id/save → { "is_saved": true/false }
-- ---------------------------------------------------------------------------

CREATE TABLE user_saved_spots (
    user_id    UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    spot_id    UUID        NOT NULL REFERENCES spots(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (user_id, spot_id)
);

CREATE INDEX idx_user_saved_spots_user_id ON user_saved_spots (user_id);
CREATE INDEX idx_user_saved_spots_spot_id ON user_saved_spots (spot_id);


-- ---------------------------------------------------------------------------
-- 视图：v_feed_posts
--
-- 供 GET /feed 和 GET /spots/:id/discoveries 接口使用。
-- 预先 JOIN users、spots 和原帖摘要，组装 photo_urls 数组，
-- 并内联最近 3 条评论（recent_comments），减少后端 N+1 查询。
--
-- 注意：is_liked / is_bookmarked 为用户级字段，需由后端在查询时
-- LEFT JOIN post_likes / post_bookmarks 按 user_id 补充，不在本视图中。
--
-- recent_comments 字段：
--   类型：JSON 数组，最多 3 条，按 created_at DESC 排序
--   每条结构：
--     { "id": "...", "body": "...", "created_at": "...",
--       "author": { "id": "...", "display_name": "...", "avatar_url": "..." } }
-- ---------------------------------------------------------------------------

CREATE OR REPLACE VIEW v_feed_posts AS
SELECT
    p.id,
    p.type::TEXT                                               AS type,

    -- 作者信息
    p.author_id,
    u.nickname                                                 AS author_display_name,
    u.avatar_url                                               AS author_avatar_url,
    u.badge_type::TEXT                                         AS author_badge_type,
    u.badge_label                                              AS author_badge_label,

    -- 帖子内容
    p.title,
    p.body,
    p.spot_id,
    s.name                                                     AS spot_name,

    -- 图片 URL 数组（按 order_index 升序）
    COALESCE(
        (SELECT ARRAY_AGG(pp.url ORDER BY pp.order_index)
         FROM post_photos pp
         WHERE pp.post_id = p.id),
        '{}'::TEXT[]
    )                                                          AS photo_urls,

    -- 互动计数
    p.like_count,
    p.comment_count,
    p.repost_count,

    -- 原帖摘要（type = 'comment' / 'repost' 时有值）
    p.original_post_id,
    op.title                                                   AS original_post_title,
    op.body                                                    AS original_post_body,
    op.author_id                                               AS original_post_author_id,
    op_u.nickname                                              AS original_post_author_name,
    op_u.avatar_url                                            AS original_post_author_avatar,

    -- 最近 3 条评论（type='comment' 的 posts，按时间倒序）
    -- 内层子查询先按 created_at DESC LIMIT 3，外层 JSON_AGG 保留该顺序
    COALESCE(
        (
            SELECT JSON_AGG(
                JSON_BUILD_OBJECT(
                    'id',         c.id,
                    'body',       c.body,
                    'created_at', c.created_at,
                    'author',     JSON_BUILD_OBJECT(
                        'id',           cu.id,
                        'display_name', cu.nickname,
                        'avatar_url',   cu.avatar_url
                    )
                )
                ORDER BY c.created_at DESC
            )
            FROM (
                SELECT pc.id, pc.body, pc.created_at, pc.author_id
                FROM posts pc
                WHERE pc.original_post_id = p.id
                  AND pc.type = 'comment'
                ORDER BY pc.created_at DESC
                LIMIT 3
            ) c
            JOIN users cu ON cu.id = c.author_id
        ),
        '[]'::JSON
    )                                                          AS recent_comments,

    p.created_at

FROM posts p
JOIN  users u    ON u.id   = p.author_id
LEFT JOIN spots s     ON s.id   = p.spot_id
LEFT JOIN posts op    ON op.id  = p.original_post_id
LEFT JOIN users op_u  ON op_u.id = op.author_id;


-- ---------------------------------------------------------------------------
-- 函数：fn_recalc_post_counts（数据修复工具）
--
-- 在数据不一致时（如批量导入、手动修改）重算单条帖子的计数。
-- 使用方式：SELECT fn_recalc_post_counts('post-uuid-here');
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION fn_recalc_post_counts(p_post_id UUID)
RETURNS VOID LANGUAGE plpgsql AS $$
BEGIN
    UPDATE posts
    SET
        like_count    = (SELECT COUNT(*) FROM post_likes  WHERE post_id = p_post_id),
        comment_count = (SELECT COUNT(*) FROM posts       WHERE original_post_id = p_post_id AND type = 'comment'),
        repost_count  = (SELECT COUNT(*) FROM posts       WHERE original_post_id = p_post_id AND type = 'repost')
    WHERE id = p_post_id;
END;
$$;


-- ---------------------------------------------------------------------------
-- 函数：fn_recalc_tip_helpful_count（数据修复工具）
--
-- 重算单条贴士的有帮助计数（仅统计 vote = TRUE 的行）。
-- 使用方式：SELECT fn_recalc_tip_helpful_count('tip-uuid-here');
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION fn_recalc_tip_helpful_count(p_tip_id UUID)
RETURNS VOID LANGUAGE plpgsql AS $$
BEGIN
    UPDATE tips
    SET helpful_count = (
        SELECT COUNT(*) FROM tip_votes WHERE tip_id = p_tip_id AND vote = TRUE
    )
    WHERE id = p_tip_id;
END;
$$;
