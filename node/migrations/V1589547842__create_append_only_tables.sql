CREATE TABLE token_state_append_only (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
    token_id char(96) NOT NULL references tokens(token_id),
    instruction_id "InstructionID" NOT NULL references instructions(id),
    status TEXT NOT NULL DEFAULT 'Available',
    state_data_json JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX index_token_state_append_only_uuid ON token_state_append_only (id);
CREATE INDEX index_token_state_instruction_id ON token_state_append_only (instruction_id);
CREATE INDEX index_token_state_append_only_token_id_created_at ON token_state_append_only (token_id, created_at);

CREATE OR REPLACE VIEW tokens_view AS
SELECT
    t.*,
    COALESCE(tsao.state_data_json, t.initial_data_json) as additional_data_json,
    COALESCE(tsao.status,'Available') as status
FROM
  tokens t
LEFT JOIN
(
    SELECT DISTINCT ON(tsao.token_id) tsao.*
    FROM token_state_append_only AS tsao
    ORDER BY tsao.token_id, tsao.created_at DESC
) tsao
ON
    t.token_id = tsao.token_id;

CREATE TABLE asset_state_append_only (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
    asset_id char(64) NOT NULL,
    instruction_id "InstructionID" NOT NULL references instructions(id),
    status TEXT NOT NULL DEFAULT 'Active',
    state_data_json JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX index_asset_state_append_only_uuid ON asset_state_append_only (id);
CREATE INDEX index_asset_state_append_only_instruction_id ON asset_state_append_only (instruction_id);
CREATE INDEX index_asset_state_append_only_asset_state_id_created_at ON asset_state_append_only (asset_id, created_at);

CREATE OR REPLACE VIEW asset_states_view AS
SELECT
    ast.*,
    COALESCE(asao.state_data_json, ast.initial_data_json) as additional_data_json,
    COALESCE(asao.status, 'Active') as status
FROM
  asset_states ast
LEFT JOIN
(
    SELECT DISTINCT ON(asao.asset_id) asao.*
    FROM asset_state_append_only AS asao
    ORDER BY asao.asset_id, asao.created_at DESC
) asao
ON
    ast.asset_id = asao.asset_id;
