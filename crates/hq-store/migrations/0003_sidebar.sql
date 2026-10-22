-- When a session joined the owner's sidebar (Unix milliseconds); NULL when it is not in
-- the sidebar. Sessions Headquarter started join on their own, including earlier ones.
ALTER TABLE sessions ADD COLUMN sidebar_added_at INTEGER;

UPDATE sessions SET sidebar_added_at = created_at WHERE origin = 'hosted';

-- Folders the owner collapsed in the sidebar; a folder without a row is expanded.
CREATE TABLE IF NOT EXISTS folders (
    cwd       TEXT PRIMARY KEY,
    collapsed INTEGER NOT NULL CHECK (collapsed IN (0, 1))
);
