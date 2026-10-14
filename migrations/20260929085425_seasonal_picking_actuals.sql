-- Add migration script here
CREATE TABLE seasonal_picking_actuals (
    id           UUID        PRIMARY KEY DEFAULT uuidv7(),
    picking_id   UUID        NOT NULL REFERENCES seasonal_pickings(id) ON DELETE CASCADE,
    year         SMALLINT    NOT NULL,               -- 对应年份，如 2026
    actual_start DATE        NOT NULL,               -- 实际开放起始日，如 2026-11-08
    actual_end   DATE        NOT NULL,               -- 实际开放截止日，如 2027-02-16
    notes        TEXT        NOT NULL DEFAULT '',    -- 管理员备注，如 "Confirmed by farm owner"
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_seasonal_picking_actuals_picking_year UNIQUE (picking_id, year),
    CONSTRAINT chk_actual_dates      CHECK (actual_end >= actual_start),
    CONSTRAINT chk_actual_start_year CHECK (EXTRACT(YEAR FROM actual_start) = year),
    CONSTRAINT chk_actual_end_year   CHECK (
        EXTRACT(YEAR FROM actual_end) = year
        OR EXTRACT(YEAR FROM actual_end) = year + 1
    )
);

CREATE TRIGGER trg_seasonal_picking_actuals_updated_at
    BEFORE UPDATE ON seasonal_picking_actuals
    FOR EACH ROW EXECUTE FUNCTION fn_set_updated_at();

CREATE INDEX idx_seasonal_picking_actuals_picking_id
    ON seasonal_picking_actuals (picking_id);

CREATE INDEX idx_seasonal_picking_actuals_year
    ON seasonal_picking_actuals (year);