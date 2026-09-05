-- Add migration script here

-- ---------------------------------------------------------------------------
-- 表：spot_labels（标签库主表）
-- ---------------------------------------------------------------------------

CREATE TABLE spot_labels (
    id          UUID        PRIMARY KEY DEFAULT uuidv7(),
    name        TEXT        NOT NULL,
    description TEXT        NOT NULL,
    icon        TEXT        NOT NULL DEFAULT 'Tag',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_spot_labels_name UNIQUE (name)
);

CREATE TRIGGER trg_spot_labels_updated_at
    BEFORE UPDATE ON spot_labels
    FOR EACH ROW EXECUTE FUNCTION fn_set_updated_at();

-- ---------------------------------------------------------------------------
-- 表：spot_label_assignments（Spot 与标签多对多关联）
-- ---------------------------------------------------------------------------

CREATE TABLE spot_label_assignments (
    spot_id  UUID NOT NULL REFERENCES spots(id)       ON DELETE CASCADE,
    label_id UUID NOT NULL REFERENCES spot_labels(id) ON DELETE CASCADE,

    PRIMARY KEY (spot_id, label_id)
);

CREATE INDEX idx_spot_label_assignments_spot_id  ON spot_label_assignments (spot_id);
CREATE INDEX idx_spot_label_assignments_label_id ON spot_label_assignments (label_id);
