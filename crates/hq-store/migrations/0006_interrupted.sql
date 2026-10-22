-- When start-up recovery found the session still running: the host before it stopped
-- without ending it (a crash, a reboot, a kill). NULL when it was not interrupted, or
-- once it was restarted or the owner chose "Not now".
ALTER TABLE sessions ADD COLUMN interrupted_at INTEGER;
