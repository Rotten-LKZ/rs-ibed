CREATE TABLE IF NOT EXISTS storage_endpoints (
    name             TEXT        PRIMARY KEY,
    description      TEXT        NOT NULL DEFAULT '',
    capacity_bytes   BIGINT      NOT NULL,
    used_size        BIGINT      NOT NULL DEFAULT 0,
    priority         INTEGER     NOT NULL DEFAULT 1,
    status           TEXT        NOT NULL DEFAULT 'active',
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_storage_endpoints_status ON storage_endpoints(status);
