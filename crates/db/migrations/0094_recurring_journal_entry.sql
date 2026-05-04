-- Extend recurring_schedules.template_type to support journal_entry templates.
ALTER TABLE recurring_schedules
    DROP CONSTRAINT recurring_schedules_template_type_check;

ALTER TABLE recurring_schedules
    ADD CONSTRAINT recurring_schedules_template_type_check
    CHECK (template_type IN ('invoice', 'bill', 'journal_entry'));
