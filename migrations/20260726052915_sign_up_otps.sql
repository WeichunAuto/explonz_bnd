-- Add migration script here
CREATE TABLE sign_up_otps (
    id          UUID PRIMARY KEY DEFAULT uuidv7(),
    email       TEXT        NOT NULL,
    code        CHAR(6),                    -- Step 1 生成，Step 2 验证后置 NULL
    otp_token   UUID,                       -- Step 2 生成，Step 3 验证后删除整行
    attempts    SMALLINT    NOT NULL DEFAULT 0,
    sent_count  SMALLINT    NOT NULL DEFAULT 0,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX sign_up_otps_email_key ON sign_up_otps (email);
CREATE UNIQUE INDEX ON sign_up_otps (otp_token) WHERE otp_token IS NOT NULL;
