-- Created inside the import transaction; the old public.products stays intact until publication.
CREATE TABLE public.products_json_import (
    id bigint PRIMARY KEY CHECK (id > 0),
    cod_good text,
    name text NOT NULL,
    marketing_description text,
    url_key text,
    price numeric,
    currency text,
    brand jsonb CHECK (brand IS NULL OR jsonb_typeof(brand) = 'object'),
    collection jsonb CHECK (collection IS NULL OR jsonb_typeof(collection) = 'object'),
    country jsonb CHECK (country IS NULL OR jsonb_typeof(country) = 'object'),
    categories jsonb CHECK (categories IS NULL OR jsonb_typeof(categories) = 'object'),
    unit text,
    description text,
    weight numeric,
    characteristics jsonb CHECK (characteristics IS NULL OR jsonb_typeof(characteristics) = 'array')
);
