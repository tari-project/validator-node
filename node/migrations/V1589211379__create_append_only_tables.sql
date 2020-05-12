DROP AGGREGATE IF EXISTS jsonb_merge(jsonb);
CREATE AGGREGATE jsonb_merge(jsonb) (
    SFUNC = jsonb_concat(jsonb, jsonb),
    STYPE = jsonb,
    INITCOND = '{}'
);

CREATE TABLE token_state_append_only (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
    token_id uuid NOT NULL references tokens(id),
    state_instruction JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX index_token_state_append_only_uuid ON token_state_append_only (id);
CREATE INDEX index_token_state_append_only_token_id_created_at ON token_state_append_only (token_id, created_at);

CREATE OR REPLACE VIEW tokens_view AS
SELECT
    t.*,
    current_token_state.additional_data_json as additional_data_json
FROM
  tokens t
JOIN
(
    SELECT
        t.id,
        t.initial_data_json || COALESCE(jsonb_merge(tsao.state_instruction), '{}') as additional_data_json
    FROM
        tokens t
    LEFT JOIN
        token_state_append_only tsao
    ON
        tsao.token_id = t.id
    GROUP BY
        t.id
) current_token_state
ON
    t.id = current_token_state.id;

CREATE TABLE asset_state_append_only (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
    asset_state_id uuid NOT NULL references asset_states(id),
    state_instruction JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX index_asset_state_append_only_uuid ON asset_state_append_only (id);
CREATE INDEX index_asset_state_append_only_asset_state_id_created_at ON asset_state_append_only (asset_state_id, created_at);

CREATE OR REPLACE VIEW asset_states_view AS
SELECT
    ast.*,
    current_asset_state.additional_data_json as additional_data_json
FROM
  asset_states ast
JOIN
(
    SELECT
        ast.id,
        ast.initial_data_json || COALESCE(jsonb_merge(asao.state_instruction), '{}') as additional_data_json
    FROM
        asset_states ast
    LEFT JOIN
        asset_state_append_only asao
    ON
        asao.asset_state_id = ast.id
    GROUP BY
        ast.id
) current_asset_state
ON
    ast.id = current_asset_state.id;
