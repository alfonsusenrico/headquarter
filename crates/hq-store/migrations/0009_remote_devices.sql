-- Phones paired for remote control (add-remote-gateway D8): metadata only. The id is the
-- phone's iroh endpoint id, its public key; nothing secret is stored.
CREATE TABLE IF NOT EXISTS remote_devices (
    endpoint_id  TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    paired_at    INTEGER NOT NULL,
    last_seen_at INTEGER
);
