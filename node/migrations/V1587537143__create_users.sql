CREATE TABLE access (
                       id uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
                       pub_key TEXT NOT NULL UNIQUE,
                       deleted_at TIMESTAMPTZ NULL,
                       created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                       updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Indices
CREATE INDEX index_access_uuid ON access (id);
CREATE INDEX index_access_pub_key ON access (pub_key);