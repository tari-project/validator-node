CREATE TABLE wallet (
                       id uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
                       pub_key TEXT NOT NULL UNIQUE,
                       balance BIGINT DEFAULT 0,
                       name TEXT NOT NULL,
                       created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                       updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Indices
CREATE INDEX index_wallet_uuid ON wallet (id);
CREATE INDEX index_wallet_pub_key ON wallet (pub_key);
CREATE INDEX index_wallet_name ON wallet (name);