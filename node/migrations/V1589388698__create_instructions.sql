CREATE DOMAIN "InstructionID" AS UUID;

CREATE TABLE instructions (
                       id "InstructionID" PRIMARY KEY NOT NULL,
                       parent_id "InstructionID" NULL DEFAULT NULL references instructions(id),
                       initiating_node_id BYTEA NOT NULL,
                       signature TEXT NOT NULL,
                       asset_id char(64) NOT NULL references asset_states(asset_id),
                       token_id char(96) NULL references tokens(token_id),
                       template_id BIGINT NOT NULL,
                       contract_name TEXT NOT NULL,
                       status TEXT NOT NULL DEFAULT 'Scheduled',
                       params JSONB NOT NULL DEFAULT '{}',
                       result JSONB NOT NULL DEFAULT '{}',
                       created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                       updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Indices
CREATE INDEX index_instructions_uuid ON instructions (id);
CREATE INDEX index_instructions_template_contract_name ON instructions (template_id, contract_name);
CREATE INDEX index_instructions_asset_id ON instructions (asset_id);
CREATE INDEX index_instructions_token_id ON instructions (token_id);
CREATE INDEX index_instructions_status ON instructions (status);
CREATE INDEX index_instructions_parent_id ON instructions (parent_id);
