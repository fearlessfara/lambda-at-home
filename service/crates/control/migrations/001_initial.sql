-- Create functions table
CREATE TABLE IF NOT EXISTS functions (
    function_id TEXT PRIMARY KEY,
    function_name TEXT UNIQUE NOT NULL,
    runtime TEXT NOT NULL,
    role TEXT,
    handler TEXT NOT NULL,
    code_sha256 TEXT NOT NULL,
    description TEXT,
    timeout INTEGER NOT NULL,
    memory_size INTEGER NOT NULL,
    environment TEXT NOT NULL DEFAULT '{}',
    last_modified TEXT NOT NULL,
    code_size INTEGER NOT NULL,
    version TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'Pending',
    state_reason TEXT,
    state_reason_code TEXT
);

-- Create versions table
CREATE TABLE IF NOT EXISTS versions (
    version_id TEXT PRIMARY KEY,
    function_id TEXT NOT NULL,
    version TEXT NOT NULL,
    description TEXT,
    code_sha256 TEXT NOT NULL,
    last_modified TEXT NOT NULL,
    code_size INTEGER NOT NULL,
    FOREIGN KEY (function_id) REFERENCES functions (function_id) ON DELETE CASCADE
);

-- Create aliases table
CREATE TABLE IF NOT EXISTS aliases (
    alias_id TEXT PRIMARY KEY,
    function_id TEXT NOT NULL,
    name TEXT NOT NULL,
    function_version TEXT NOT NULL,
    description TEXT,
    routing_config TEXT,
    revision_id TEXT NOT NULL,
    last_modified TEXT NOT NULL,
    FOREIGN KEY (function_id) REFERENCES functions (function_id) ON DELETE CASCADE,
    UNIQUE (function_id, name)
);

-- Create executions table
CREATE TABLE IF NOT EXISTS executions (
    execution_id TEXT PRIMARY KEY,
    function_id TEXT NOT NULL,
    function_version TEXT NOT NULL,
    aws_request_id TEXT NOT NULL,
    container_id TEXT,
    start_time TEXT NOT NULL,
    end_time TEXT,
    duration_ms INTEGER,
    billed_ms INTEGER,
    memory_used_mb INTEGER,
    error_type TEXT,
    status TEXT NOT NULL DEFAULT 'Pending',
    FOREIGN KEY (function_id) REFERENCES functions (function_id) ON DELETE CASCADE
);

-- Create indexes
CREATE INDEX IF NOT EXISTS idx_functions_name ON functions (function_name);
CREATE INDEX IF NOT EXISTS idx_versions_function_id ON versions (function_id);
CREATE INDEX IF NOT EXISTS idx_versions_version ON versions (function_id, version);
CREATE INDEX IF NOT EXISTS idx_aliases_function_id ON aliases (function_id);
CREATE INDEX IF NOT EXISTS idx_aliases_name ON aliases (function_id, name);
CREATE INDEX IF NOT EXISTS idx_executions_function_id ON executions (function_id);
CREATE INDEX IF NOT EXISTS idx_executions_start_time ON executions (start_time);
CREATE INDEX IF NOT EXISTS idx_executions_status ON executions (status);
