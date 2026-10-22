-- Whether the owner named the session. An owner's name is shown instead of the agent's
-- own title; a generated `<agent>-<n>` name is not.
ALTER TABLE sessions ADD COLUMN owner_named INTEGER NOT NULL DEFAULT 0 CHECK (owner_named IN (0, 1));

-- Sessions renamed before this column existed. Generated names start with the agent
-- (`claude-3`, `codex-1a2b3c4d`); any other name was given by the owner.
UPDATE sessions SET owner_named = 1 WHERE name NOT GLOB agent || '-*';
