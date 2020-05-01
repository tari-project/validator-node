CREATE TABLE digital_assets (
                       id uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
                       template_type TEXT NOT NULL,
                       committee_mode TEXT NOT NULL,
                       node_threshold OID NULL,
                       minimum_collateral BIGINT NULL,
                       consensus_strategy OID NULL,
                       fqdn varchar(255) NULL,
                       digital_asset_template_id BIGINT NOT NULL,
                       raid_id char(15) NULL,
                       created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                       updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE digital_assets
    ADD CONSTRAINT public_committee_required_fields CHECK
    (NOT ((node_threshold IS NULL OR minimum_collateral IS NULL OR consensus_strategy IS NULL) AND committee_mode = 'Public'));

-- Indices
CREATE INDEX index_digital_assets_uuid ON digital_assets (id);
CREATE INDEX index_digital_assets_template_type ON digital_assets (template_type);
CREATE INDEX index_digital_assets_committee_mode ON digital_assets (committee_mode);
CREATE INDEX index_digital_assets_consensus_strategy ON digital_assets (consensus_strategy);
CREATE INDEX index_digital_assets_fqdn ON digital_assets (fqdn);
CREATE INDEX index_digital_assets_digital_asset_template_id ON digital_assets (digital_asset_template_id);
CREATE INDEX index_digital_assets_raid_id ON digital_assets (raid_id);
