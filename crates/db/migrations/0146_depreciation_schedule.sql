-- Extend depreciation methods to include sum-of-years-digits

ALTER TABLE fixed_assets
    DROP CONSTRAINT fixed_assets_depreciation_method_check;

ALTER TABLE fixed_assets
    ADD CONSTRAINT fixed_assets_depreciation_method_check
        CHECK (depreciation_method IN (
            'straight_line',
            'declining_balance',
            'sum_of_years_digits'
        ));

-- Index to support bulk-depreciation queries (active assets per org)
CREATE INDEX IF NOT EXISTS idx_fixed_assets_org_active
    ON fixed_assets (organization_id)
    WHERE status = 'active';
