CREATE TABLE tokens (
                       id uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
                       issue_number BIGINT NOT NULL,
                       owner_pub_key TEXT NOT NULL,
                       status TEXT NOT NULL DEFAULT 'Active',
                       token_id char(96) NOT NULL UNIQUE,
                       asset_state_id uuid NOT NULL references asset_states(id),
                       additional_data_json JSONB NOT NULL DEFAULT '{}',
                       created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                       updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE FUNCTION set_issue_number()
RETURNS trigger AS $$
BEGIN
  IF NEW.issue_number IS NULL THEN
    NEW.issue_number = (SELECT COALESCE(MAX(issue_number), 0) + 1 FROM tokens WHERE asset_state_id = NEW.asset_state_id);
  END IF;
  RETURN NEW;
END
$$ LANGUAGE 'plpgsql';

CREATE TRIGGER set_issue_number_trigger
BEFORE INSERT ON tokens
FOR EACH ROW
EXECUTE PROCEDURE set_issue_number();

-- Indices
CREATE INDEX index_tokens_uuid ON tokens (id);
CREATE INDEX index_tokens_owner_pub_key ON tokens (owner_pub_key);
CREATE INDEX index_tokens_status ON tokens (status);
CREATE INDEX index_tokens_asset_state_id ON tokens (asset_state_id);
