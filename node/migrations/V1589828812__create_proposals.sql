CREATE DOMAIN "ProposalID" AS UUID;

CREATE TABLE proposals (
                       id "ProposalID" PRIMARY KEY NOT NULL,
                       new_view JSONB NOT NULL,
                       asset_id char(64) NOT NULL,
                       node_id BYTEA NOT NULL,
                       status TEXT NOT NULL DEFAULT 'Pending',
                       created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                       updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE instructions
    ADD proposal_id "ProposalID" NULL references proposals(id);

ALTER TABLE views
    ADD proposal_id "ProposalID" NULL references proposals(id);

-- Indices
CREATE INDEX index_proposals_uuid ON proposals (id);
CREATE INDEX index_proposals_asset_id ON proposals (asset_id);
CREATE INDEX index_proposals_status ON proposals (status);



CREATE INDEX index_instructions_proposal_id ON instructions (proposal_id);
CREATE INDEX index_views_proposal_id ON views (proposal_id);
