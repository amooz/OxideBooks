-- Account sub-type for more granular COA classification
-- (parent_id already exists; just add sub_type)
ALTER TABLE accounts ADD COLUMN sub_type TEXT;
