-- Add migration script here
-- spots 表新增联系方式字段
ALTER TABLE spots
    ADD COLUMN IF NOT EXISTS phone   TEXT,   -- 联系电话，可为空
    ADD COLUMN IF NOT EXISTS website TEXT;   -- 网站网址，可为空
