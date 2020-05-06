CREATE TABLE access (
                       id uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
                       pub_key TEXT NOT NULL,
                       resource TEXT NOT NULL,
                       resource_key TEXT NULL DEFAULT NULL,
                       deleted_at TIMESTAMPTZ NULL,
                       created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                       updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                       UNIQUE (pub_key, resource, resource_key)
);

-- Indices
CREATE INDEX index_access_uuid ON access (id);
CREATE INDEX index_access_pub_key ON access (pub_key);
CREATE INDEX index_access_resource ON access (resource, resource_key);
