CREATE TABLE aggregate_signature_messages (
                       id uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
                       proposal_id "ProposalID" NOT NULL references proposals(id),
                       signature_data JSONB NOT NULL,
                       created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                       updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Indices
CREATE INDEX index_aggregate_signature_messages_uuid ON aggregate_signature_messages (id);
CREATE INDEX index_aggregate_signature_messages_proposal_id ON aggregate_signature_messages (proposal_id);
