CREATE TABLE digital_assets (
                       id uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
                       template_type OID NOT NULL,
                       committee_mode JSONB NOT NULL,
                       fqdn varchar(255) NULL,
                       raid_id char(15) NULL,
                       created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                       updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Indices
CREATE INDEX index_digital_assets_uuid ON digital_assets (id);
CREATE INDEX index_digital_assets_template_type ON digital_assets (template_type);
CREATE INDEX index_digital_assets_fqdn ON digital_assets (fqdn);
CREATE INDEX index_digital_assets_raid_id ON digital_assets (raid_id);
