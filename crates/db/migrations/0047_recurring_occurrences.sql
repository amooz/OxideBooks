ALTER TABLE recurring_schedules ADD COLUMN max_occurrences INT;
ALTER TABLE recurring_schedules ADD COLUMN occurrences_count INT NOT NULL DEFAULT 0;
