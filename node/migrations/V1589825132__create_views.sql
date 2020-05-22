CREATE TABLE views (
                       id uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
                       asset_id char(64) NOT NULL,
                       initiating_node_id "NodeID" NOT NULL,
                       signature TEXT NOT NULL,
                       instruction_set uuid [] NOT NULL,
                       invalid_instruction_set uuid [] NOT NULL,
                       asset_state_append_only JSONB NOT NULL DEFAULT '{}',
                       token_state_append_only JSONB NOT NULL DEFAULT '{}',
                       status TEXT NOT NULL DEFAULT 'Prepare',
                       created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                       updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Indices
CREATE INDEX index_views_uuid ON views (id);
CREATE INDEX index_views_asset_id ON views (asset_id);
CREATE INDEX index_views_status ON views (status);
