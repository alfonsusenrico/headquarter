-- How each session entered the index: started by Headquarter ('hosted') or imported
-- from an agent's own history ('history'). The conversation index is not unique on
-- purpose: an existing index may already hold duplicates; code matches before insert.
ALTER TABLE sessions ADD COLUMN origin TEXT NOT NULL DEFAULT 'hosted'
    CHECK (origin IN ('hosted', 'history'));

CREATE INDEX IF NOT EXISTS sessions_by_conversation ON sessions (agent, agent_session_id);
