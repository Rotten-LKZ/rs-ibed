CREATE TABLE IF NOT EXISTS storage_endpoints (
    name             TEXT     PRIMARY KEY,
    description      TEXT     NOT NULL DEFAULT '',
    capacity_bytes   INTEGER  NOT NULL,
    used_size        INTEGER  NOT NULL DEFAULT 0,
    priority         INTEGER  NOT NULL DEFAULT 1,
    status           TEXT     NOT NULL DEFAULT 'active',
    created_at       DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at       DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_storage_endpoints_status ON storage_endpoints(status);
