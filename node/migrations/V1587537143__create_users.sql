CREATE TABLE users (
                       id uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
                       username TEXT NULL UNIQUE,
                       pub_key TEXT NOT NULL,
                       created_at TIMESTAMP NOT NULL DEFAULT now(),
                       updated_at TIMESTAMP NOT NULL DEFAULT now()
);

-- Indices
CREATE INDEX index_users_uuid ON users (id);
CREATE INDEX index_users_username ON users (username);