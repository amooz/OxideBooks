ALTER TABLE expenses ADD COLUMN project_id UUID REFERENCES projects(id) ON DELETE SET NULL;

CREATE INDEX idx_expenses_project ON expenses (project_id) WHERE project_id IS NOT NULL;
