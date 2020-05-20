CREATE TABLE token_state_append_only (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
    token_id uuid NOT NULL references tokens(id),
    instruction_id uuid NOT NULL references instructions(id),
    status TEXT NOT NULL DEFAULT 'Active',
    state_data_json JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX index_token_state_append_only_uuid ON token_state_append_only (id);
CREATE INDEX index_token_state_append_only_token_id_status_created_at ON token_state_append_only (token_id, instruction_id, created_at);

CREATE OR REPLACE VIEW tokens_view AS
SELECT
    t.*,
    COALESCE(tsao.state_data_json, t.initial_data_json) as additional_data_json,
    COALESCE(tsao.status,'Active') as status
FROM
  tokens t
LEFT JOIN
(
    SELECT DISTINCT ON(tsao.token_id) tsao.*
    FROM token_state_append_only AS tsao
    ORDER BY tsao.token_id, tsao.created_at DESC
) tsao
ON
    t.id = tsao.token_id;

CREATE TABLE asset_state_append_only (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
    asset_state_id uuid NOT NULL references asset_states(id),
    instruction_id uuid NOT NULL references instructions(id),
    status TEXT NOT NULL DEFAULT 'Active',
    state_data_json JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX index_asset_state_append_only_uuid ON asset_state_append_only (id);
CREATE INDEX index_asset_state_append_only_asset_state_id_status_created_at ON asset_state_append_only (asset_state_id, instruction_id, created_at);

CREATE OR REPLACE VIEW asset_states_view AS
SELECT
    ast.*,
    COALESCE(asao.state_data_json, ast.initial_data_json) as additional_data_json,
    COALESCE(asao.status, 'Active') as status
FROM
  asset_states ast
LEFT JOIN
(
    SELECT DISTINCT ON(asao.asset_state_id) asao.*
    FROM asset_state_append_only AS asao
        INNER JOIN instructions as ct
        ON asao.instruction_id = ct.id
    WHERE ct.status = 'Commit'
    ORDER BY asao.asset_state_id, asao.created_at DESC
) asao
ON
    ast.id = asao.asset_state_id;
