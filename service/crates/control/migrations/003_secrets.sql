-- Secrets store (values are not exposed via API; stored as base64-encoded strings)
CREATE TABLE IF NOT EXISTS secrets (
    name TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    created_at TEXT NOT NULL
);

