CREATE TABLE asset_states (
                       id uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
                       name varchar(64) NOT NULL DEFAULT '',
                       description varchar(216) NOT NULL DEFAULT '',
                       limit_per_wallet OID NULL,
                       allow_transfers bool DEFAULT 't',
                       asset_issuer_pub_key TEXT NOT NULL,
                       authorized_signers TEXT[] NOT NULL DEFAULT '{}',
                       expiry_date TIMESTAMPTZ NULL,
                       superseded_by uuid NULL references asset_states(id),
                       initial_permission_bitflag BIGINT NOT NULL DEFAULT 0,
                       initial_data_json JSONB NOT NULL DEFAULT '{}',
                       asset_id char(64) NOT NULL UNIQUE,
                       digital_asset_id UUID references digital_assets(id),
                       blocked_until TIMESTAMPTZ NOT NULL DEFAULT now(),
                       created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                       updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Indices
CREATE INDEX index_asset_states_uuid ON asset_states (id);
CREATE INDEX index_asset_states_name ON asset_states (name);
CREATE INDEX index_asset_states_superseded_by ON asset_states (superseded_by);
CREATE INDEX index_asset_states_expiry_date ON asset_states (expiry_date);
CREATE UNIQUE INDEX index_asset_states_asset_id ON asset_states (asset_id);
