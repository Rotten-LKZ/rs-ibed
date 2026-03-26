CREATE TABLE IF NOT EXISTS images (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    hash         TEXT NOT NULL UNIQUE,             -- BLAKE3 指纹 (64 hex chars)
    display_name TEXT NOT NULL DEFAULT '',         -- 用户可修改的显示名
    file_name    TEXT NOT NULL,                    -- 上传时的原始文件名
    extension    TEXT NOT NULL,                    -- 真实后缀 (jpg, webp 等)
    mime_type    TEXT NOT NULL,                    -- image/jpeg 等
    size         INTEGER NOT NULL,                 -- 字节数
    width        INTEGER NOT NULL,                 -- 像素宽
    height       INTEGER NOT NULL,                 -- 像素高

    user_id      TEXT,                             -- 预留所有者 ID
    is_deleted   BOOLEAN NOT NULL DEFAULT 0,
    created_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_images_hash ON images(hash);
CREATE INDEX IF NOT EXISTS idx_images_display_name ON images(display_name);
CREATE INDEX IF NOT EXISTS idx_images_created_at ON images(created_at);
