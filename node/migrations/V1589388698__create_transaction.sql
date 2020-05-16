CREATE TABLE contract_transactions (
                       id uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
                       asset_state_id uuid NOT NULL references asset_states(id),
                       token_id uuid NULL references tokens(id),
                       template_id BIGINT NOT NULL,
                       contract_name TEXT NOT NULL,
                       status TEXT NOT NULL,
                       params JSONB NOT NULL DEFAULT '{}',
                       result JSONB NOT NULL DEFAULT '{}',
                       created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                       updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Indices
CREATE INDEX index_contract_transactions_uuid ON contract_transactions (id);
CREATE INDEX index_contract_transactions_template_contract_name ON contract_transactions (template_id, contract_name);
CREATE INDEX index_contract_transactions_asset_state_id ON contract_transactions (asset_state_id);
CREATE INDEX index_contract_transactions_token_id ON contract_transactions (token_id);
CREATE INDEX index_contract_status ON contract_transactions (status);
