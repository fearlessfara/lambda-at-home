-- API Routes table to map HTTP paths/methods to function names
CREATE TABLE IF NOT EXISTS api_routes (
    route_id TEXT PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    method TEXT NULL,
    function_name TEXT NOT NULL,
    created_at TEXT NOT NULL
);

