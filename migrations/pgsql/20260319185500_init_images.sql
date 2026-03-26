CREATE TABLE IF NOT EXISTS images (
    id           BIGSERIAL PRIMARY KEY,
    hash         TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL DEFAULT '',
    file_name    TEXT NOT NULL,
    extension    TEXT NOT NULL,
    mime_type    TEXT NOT NULL,
    size         BIGINT NOT NULL,
    width        INTEGER NOT NULL,
    height       INTEGER NOT NULL,

    user_id      TEXT,
    is_deleted   BOOLEAN NOT NULL DEFAULT FALSE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_images_hash ON images(hash);
CREATE INDEX IF NOT EXISTS idx_images_display_name ON images(display_name);
CREATE INDEX IF NOT EXISTS idx_images_created_at ON images(created_at);
