-- Device-scoped Matrix identity mapping and id allocation sequences.
-- See ADR-013. No Sync, Matrix HTTP, or UI in this migration.

CREATE TABLE device_id_sequences (
    device_id UUID PRIMARY KEY REFERENCES devices (id) ON DELETE RESTRICT,
    next_matrix_user_seq BIGINT NOT NULL DEFAULT 1,
    next_ref_user_id BIGINT NOT NULL DEFAULT 10000001,
    updated_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT device_id_sequences_user_seq_positive CHECK (next_matrix_user_seq >= 1),
    CONSTRAINT device_id_sequences_ref_range CHECK (
        next_ref_user_id >= 0
        AND next_ref_user_id <= 99999999
    )
);

CREATE TABLE device_users (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    device_id UUID NOT NULL REFERENCES devices (id) ON DELETE RESTRICT,
    matrix_user_id TEXT NOT NULL,
    matrix_ref_user_id BIGINT NOT NULL,
    provisioned_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT device_users_matrix_user_id_not_blank CHECK (btrim(matrix_user_id) <> ''),
    CONSTRAINT device_users_matrix_user_id_length CHECK (char_length(matrix_user_id) <= 15),
    CONSTRAINT device_users_matrix_user_id_alnum CHECK (matrix_user_id ~ '^[A-Za-z0-9]+$'),
    CONSTRAINT device_users_matrix_ref_range CHECK (
        matrix_ref_user_id >= 0
        AND matrix_ref_user_id <= 99999999
    ),
    CONSTRAINT device_users_user_device_unique UNIQUE (user_id, device_id),
    CONSTRAINT device_users_device_matrix_user_unique UNIQUE (device_id, matrix_user_id),
    CONSTRAINT device_users_device_matrix_ref_unique UNIQUE (device_id, matrix_ref_user_id)
);

CREATE INDEX device_users_device_id_idx ON device_users (device_id);
CREATE INDEX device_users_user_id_idx ON device_users (user_id);
