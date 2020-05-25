CREATE TABLE signed_proposals (
                       id uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
                       node_id BYTEA NOT NULL,
                       signature TEXT NOT NULL,
                       status TEXT NOT NULL DEFAULT 'Pending',
                       proposal_id "ProposalID" references proposals(id),
                       created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                       updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Indices
CREATE INDEX index_signed_proposals_uuid ON signed_proposals (id);
CREATE INDEX index_signed_proposals_proposal_id ON signed_proposals (proposal_id);
