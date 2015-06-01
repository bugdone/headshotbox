ALTER TABLE demos ADD COLUMN type VARCHAR(20);
UPDATE demos SET type = 'valve';
