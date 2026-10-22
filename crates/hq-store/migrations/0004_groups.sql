-- Groups the owner makes in the sidebar, in the owner's order. Names are display text
-- the owner typed, never agent output.
CREATE TABLE IF NOT EXISTS groups (
    id         INTEGER PRIMARY KEY,
    name       TEXT NOT NULL,
    position   INTEGER NOT NULL,
    collapsed  INTEGER NOT NULL DEFAULT 0 CHECK (collapsed IN (0, 1)),
    created_at INTEGER NOT NULL
);

-- The session's group; NULL when it is in no group. Deleting a group ungroups its
-- sessions (the store turns foreign keys on for every connection).
ALTER TABLE sessions ADD COLUMN group_id INTEGER REFERENCES groups (id) ON DELETE SET NULL;

-- Folder groups are gone, and with them which folders were collapsed.
DROP TABLE IF EXISTS folders;
