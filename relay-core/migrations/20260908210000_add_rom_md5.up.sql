-- MD5 is needed alongside the existing CRC32 for RetroAchievements hash matching, which
-- identifies by MD5, not CRC32. Additive migration (not squashed into the initial schema) since
-- that one's already applied on real devices.
ALTER TABLE roms ADD COLUMN md5 TEXT;
