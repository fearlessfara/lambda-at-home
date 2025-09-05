-- Per-function reserved concurrency
CREATE TABLE IF NOT EXISTS function_concurrency (
    function_id TEXT PRIMARY KEY,
    reserved_concurrent_executions INTEGER NULL,
    updated_at TEXT NOT NULL
);

