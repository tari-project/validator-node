CREATE TABLE instructions (
                       id uuid PRIMARY KEY NOT NULL,
                       asset_state_id uuid NOT NULL references asset_states(id),
                       token_id uuid NULL references tokens(id),
                       template_id BIGINT NOT NULL,
                       contract_name TEXT NOT NULL,
                       status TEXT NOT NULL DEFAULT 'Pending',
                       params JSONB NOT NULL DEFAULT '{}',
                       result JSONB NOT NULL DEFAULT '{}',
                       created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                       updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Indices
CREATE INDEX index_instructions_uuid ON instructions (id);
CREATE INDEX index_instructions_template_contract_name ON instructions (template_id, contract_name);
CREATE INDEX index_instructions_asset_state_id ON instructions (asset_state_id);
CREATE INDEX index_instructions_token_id ON instructions (token_id);
CREATE INDEX index_instructions_status ON instructions (status);
