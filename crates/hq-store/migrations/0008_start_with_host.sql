-- `interrupted_at` now marks a sidebar session to start again when the host starts
-- (add-inactive-sessions D1). Hosts before this marked nothing on a stop, so the
-- sessions the last stop ended (within 30 s of the latest end) are marked, and older
-- marks are cleared: those sessions were already stopped and stay inactive.
UPDATE sessions SET interrupted_at = CASE
    WHEN ended_at >= (SELECT MAX(ended_at) FROM sessions
                      WHERE sidebar_added_at IS NOT NULL AND status = 'inactive') - 30000
    THEN ended_at
    ELSE NULL
END
WHERE sidebar_added_at IS NOT NULL AND status = 'inactive';
