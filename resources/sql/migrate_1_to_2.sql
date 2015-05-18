ALTER TABLE demos ADD COLUMN mtime INT;
UPDATE demos SET mtime = strftime('%s','now');
DROP TABLE playerdemos;
