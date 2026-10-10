-- Identity taken from an Excel import. Manual create leaves these null.
-- The same ID and reference id are copied onto device_users when the person
-- is assigned to a device.

ALTER TABLE users
    ADD COLUMN matrix_user_id TEXT,
    ADD COLUMN short_name TEXT,
    ADD COLUMN full_name TEXT,
    ADD COLUMN reference_id BIGINT;

ALTER TABLE users
    ADD CONSTRAINT users_matrix_user_id_unique UNIQUE (matrix_user_id);

ALTER TABLE users
    ADD CONSTRAINT users_reference_id_unique UNIQUE (reference_id);

ALTER TABLE users
    ADD CONSTRAINT users_matrix_user_id_shape CHECK (
        matrix_user_id IS NULL
        OR (
            char_length(matrix_user_id) BETWEEN 1 AND 15
            AND matrix_user_id ~ '^[A-Za-z0-9]+$'
        )
    );

ALTER TABLE users
    ADD CONSTRAINT users_short_name_length CHECK (
        short_name IS NULL
        OR (
            char_length(btrim(short_name)) BETWEEN 1 AND 15
            AND char_length(short_name) <= 15
        )
    );

ALTER TABLE users
    ADD CONSTRAINT users_full_name_length CHECK (
        full_name IS NULL OR char_length(full_name) <= 200
    );

ALTER TABLE users
    ADD CONSTRAINT users_reference_id_range CHECK (
        reference_id IS NULL
        OR (reference_id >= 1 AND reference_id <= 99999999)
    );
