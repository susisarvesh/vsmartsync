-- Live enrolluser capture. Assignment rows stay in enrollments (ADR-010).
-- Card numbers stay in credentials.value_ciphertext. This table stores status only.

ALTER TABLE credentials
    ADD COLUMN card_type TEXT NULL,
    ADD COLUMN identifier_type TEXT NULL;

ALTER TABLE credentials
    ADD CONSTRAINT credentials_card_type_allowed CHECK (
        card_type IS NULL OR card_type IN (
            'iclass_2k2',
            'iclass_16k2',
            'iclass_16k16',
            'mifare_1k',
            'mifare_4k',
            'mifare_desfire_2k',
            'mifare_desfire_4k',
            'mifare_desfire_8k',
            'read_only'
        )
    );

ALTER TABLE credentials
    ADD CONSTRAINT credentials_identifier_type_allowed CHECK (
        identifier_type IS NULL OR identifier_type IN ('csn', 'uid', 'custom')
    );

ALTER TABLE enrollments
    ADD COLUMN identifier_digest BYTEA NULL;

-- Same card identity may exist on another device. Revoke releases the number.
CREATE UNIQUE INDEX enrollments_active_card_identifier_unique
    ON enrollments (device_id, identifier_digest)
    WHERE status = 'active' AND identifier_digest IS NOT NULL;

CREATE TABLE enrollment_sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    device_id UUID NOT NULL REFERENCES devices (id) ON DELETE RESTRICT,
    enroll_type TEXT NOT NULL,
    status TEXT NOT NULL,
    credential_id UUID NULL REFERENCES credentials (id) ON DELETE RESTRICT,
    error_code TEXT NULL,
    card_type TEXT NULL,
    identifier_type TEXT NULL,
    identifier_unavailable BOOLEAN NOT NULL DEFAULT FALSE,
    started_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT enrollment_sessions_status_allowed CHECK (
        status IN (
            'starting',
            'waiting_for_card',
            'processing',
            'verifying',
            'saving',
            'success',
            'failed',
            'cancelled',
            'timeout',
            'busy',
            'persistence_failed'
        )
    ),
    CONSTRAINT enrollment_sessions_enroll_type_allowed CHECK (
        enroll_type IN (
            'read_only_card',
            'smart_card',
            'biometric',
            'biometric_then_card',
            'face',
            'duress_finger'
        )
    ),
    CONSTRAINT enrollment_sessions_card_type_allowed CHECK (
        card_type IS NULL OR card_type IN (
            'iclass_2k2',
            'iclass_16k2',
            'iclass_16k16',
            'mifare_1k',
            'mifare_4k',
            'mifare_desfire_2k',
            'mifare_desfire_4k',
            'mifare_desfire_8k',
            'read_only'
        )
    ),
    CONSTRAINT enrollment_sessions_identifier_type_allowed CHECK (
        identifier_type IS NULL OR identifier_type IN ('csn', 'uid', 'custom')
    )
);

CREATE INDEX enrollment_sessions_device_status_idx
    ON enrollment_sessions (device_id, status, created_at DESC);

CREATE INDEX enrollment_sessions_user_created_idx
    ON enrollment_sessions (user_id, created_at DESC);
