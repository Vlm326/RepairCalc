-- Isolated from public.products, which is replaced by the JSON importer.
CREATE SCHEMA catalog;
CREATE TABLE catalog.categories (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name text NOT NULL CHECK (btrim(name) <> ''),
    description text NOT NULL DEFAULT ''
);
CREATE TABLE catalog.products (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    sku text NOT NULL UNIQUE CHECK (btrim(sku) <> ''),
    name text NOT NULL CHECK (btrim(name) <> ''),
    description text NOT NULL DEFAULT '',
    unit text NOT NULL CHECK (btrim(unit) <> ''),
    package_quantity numeric(20,6) NOT NULL CHECK (package_quantity > 0 AND package_quantity <> 'NaN')
);
CREATE TABLE catalog.product_categories (
    product_id bigint REFERENCES catalog.products ON DELETE CASCADE,
    category_id bigint REFERENCES catalog.categories ON DELETE CASCADE,
    PRIMARY KEY (product_id, category_id)
);
CREATE INDEX product_categories_category_idx ON catalog.product_categories(category_id);
CREATE TABLE catalog.characteristics (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    code text NOT NULL UNIQUE CHECK (btrim(code) <> ''),
    name text NOT NULL CHECK (btrim(name) <> ''),
    unit text
);
CREATE TABLE catalog.product_characteristics (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    product_id bigint NOT NULL REFERENCES catalog.products ON DELETE CASCADE,
    characteristic_id bigint NOT NULL REFERENCES catalog.characteristics ON DELETE RESTRICT,
    value text NOT NULL,
    UNIQUE (product_id, characteristic_id)
);
CREATE INDEX product_characteristics_definition_idx ON catalog.product_characteristics(characteristic_id);
-- One current price per product/region/currency; amount is per sales package.
CREATE TABLE catalog.prices (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    product_id bigint NOT NULL REFERENCES catalog.products ON DELETE CASCADE,
    region text NOT NULL CHECK (btrim(region) <> ''),
    currency text NOT NULL CHECK (currency ~ '^[A-Z]{3}$'),
    amount numeric(18,2) NOT NULL CHECK (amount >= 0 AND amount <> 'NaN'),
    UNIQUE (product_id, region, currency)
);
-- quantity counts sales packages; may be fractional for goods sold by weight/length.
CREATE TABLE catalog.stocks (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    product_id bigint NOT NULL REFERENCES catalog.products ON DELETE CASCADE,
    region text NOT NULL CHECK (btrim(region) <> ''),
    warehouse text NOT NULL CHECK (btrim(warehouse) <> ''),
    quantity numeric(20,6) NOT NULL CHECK (quantity >= 0 AND quantity <> 'NaN'),
    UNIQUE (product_id, region, warehouse)
);
