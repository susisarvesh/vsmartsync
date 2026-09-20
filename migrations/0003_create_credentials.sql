-- Application credential system of record (not Matrix objects).
-- Types for this slice: card, pin. No biometric templates.
CREATE TABLE credentials (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id),
    type TEXT NOT NULL,
    value_ciphertext BYTEA NOT NULL,
    value_digest BYTEA NOT NULL,
    display_hint TEXT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT credentials_type_allowed CHECK (type IN ('card', 'pin')),
    CONSTRAINT credentials_status_allowed CHECK (status IN ('active', 'inactive')),
    CONSTRAINT credentials_display_hint_length CHECK (
        display_hint IS NULL OR char_length(display_hint) <= 16
    )
);

-- List/filter by owning user (Credentials page filters).
CREATE INDEX credentials_user_id_idx ON credentials (user_id);

-- Filter by application status.
CREATE INDEX credentials_status_idx ON credentials (status);

-- Filter by credential type.
CREATE INDEX credentials_type_idx ON credentials (type);

-- One physical card number cannot belong to two users.
CREATE UNIQUE INDEX credentials_card_value_digest_unique
    ON credentials (value_digest)
    WHERE type = 'card';

-- At most one PIN credential per user in this MVP.
CREATE UNIQUE INDEX credentials_one_pin_per_user
    ON credentials (user_id)
    WHERE type = 'pin';
