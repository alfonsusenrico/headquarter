-- Session index. Metadata only: there is deliberately no column for terminal output,
-- prompts, or model output. Times are Unix milliseconds.
CREATE TABLE IF NOT EXISTS sessions (
    id               TEXT PRIMARY KEY,
    agent            TEXT NOT NULL CHECK (agent IN ('claude', 'codex', 'agy')),
    cwd              TEXT NOT NULL,
    name             TEXT NOT NULL,
    agent_session_id TEXT,
    status           TEXT NOT NULL
                     CHECK (status IN ('working', 'needs-you', 'done-unread', 'done', 'inactive')),
    needs_reason     TEXT,
    created_at       INTEGER NOT NULL,
    last_activity_at INTEGER NOT NULL,
    ended_at         INTEGER
);

CREATE INDEX IF NOT EXISTS sessions_by_cwd ON sessions (cwd, last_activity_at);
