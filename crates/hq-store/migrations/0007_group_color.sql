-- The colour the owner picked for a group, by palette name; NULL is automatic
-- (add-group-colors).
ALTER TABLE groups ADD COLUMN color TEXT;
