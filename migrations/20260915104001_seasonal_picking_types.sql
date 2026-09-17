-- Add migration script here
CREATE TABLE seasonal_picking_types (
    id          UUID        PRIMARY KEY DEFAULT uuidv7(),
    name        TEXT        NOT NULL,
    description TEXT        NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_seasonal_picking_types_name UNIQUE (name)
);

ALTER TABLE seasonal_pickings
ADD COLUMN picking_type_id UUID NOT NULL
    REFERENCES seasonal_picking_types(id)
    ON DELETE RESTRICT;

CREATE INDEX idx_seasonal_pickings_picking_type_id
    ON seasonal_pickings (picking_type_id);

CREATE UNIQUE INDEX uq_seasonal_pickings_spot_picking_type
    ON seasonal_pickings (spot_id, picking_type_id);


INSERT INTO seasonal_picking_types (name)
VALUES
    ('Strawberry Picking'),
    ('Blueberry Picking'),
    ('Raspberry Picking'),
    ('Boysenberry Picking'),
    ('Blackberry Picking'),
    ('Cherry Picking'),
    ('Currant Picking'),
    ('Tayberry Picking'),
    ('Haskap Picking'),
    ('Apple Picking'),
    ('Pear Picking'),
    ('Lavender Picking'),
    ('Vegetable Picking'),

    ('Apricot Picking'),
    ('Peach Picking'),
    ('Nectarine Picking'),
    ('Plum Picking'),
    ('Grape Picking'),
    ('Feijoa Picking'),
    ('Persimmon Picking'),
    ('Fig Picking'),
    ('Gooseberry Picking'),
    ('Goji Berry Picking'),
    ('Elderberry Picking'),
    ('Quince Picking'),
    ('Walnut Picking'),
    ('Chestnut Picking'),
    ('Hazelnut Picking'),
    ('Corn Picking'),
    ('Pumpkin Picking'),
    ('Tomato Picking'),
    ('Pea Picking'),
    ('Bean Picking'),
    ('Sunflower Picking'),
    ('Christmas Tree Picking')
ON CONFLICT (name) DO NOTHING;