CREATE TABLE IF NOT EXISTS users (
    id CHAR(36) NOT NULL PRIMARY KEY,
    username VARCHAR(200) NOT NULL,
    status VARCHAR(16) NOT NULL,
    matrix_user_id VARCHAR(15) NULL,
    short_name VARCHAR(15) NULL,
    full_name VARCHAR(200) NULL,
    reference_id BIGINT NULL,
    created_at DATETIME(6) NOT NULL,
    updated_at DATETIME(6) NOT NULL,
    CONSTRAINT users_username_not_blank CHECK (TRIM(username) <> ''),
    CONSTRAINT users_username_length CHECK (CHAR_LENGTH(username) <= 200),
    CONSTRAINT users_status_allowed CHECK (status IN ('active', 'inactive')),
    CONSTRAINT users_matrix_user_id_unique UNIQUE (matrix_user_id),
    CONSTRAINT users_reference_id_unique UNIQUE (reference_id),
    CONSTRAINT users_matrix_user_id_shape CHECK (
        matrix_user_id IS NULL
        OR (CHAR_LENGTH(matrix_user_id) BETWEEN 1 AND 15 AND matrix_user_id REGEXP '^[A-Za-z0-9]+$')
    ),
    CONSTRAINT users_short_name_length CHECK (
        short_name IS NULL OR CHAR_LENGTH(TRIM(short_name)) BETWEEN 1 AND 15
    ),
    CONSTRAINT users_full_name_length CHECK (
        full_name IS NULL OR CHAR_LENGTH(full_name) <= 200
    ),
    CONSTRAINT users_reference_id_range CHECK (
        reference_id IS NULL OR (reference_id >= 1 AND reference_id <= 99999999)
    )
);

CREATE TABLE IF NOT EXISTS devices (
    id CHAR(36) NOT NULL PRIMARY KEY,
    device_name VARCHAR(200) NOT NULL,
    host VARCHAR(255) NOT NULL,
    port INT NOT NULL DEFAULT 80,
    mac_address VARCHAR(17) NULL,
    device_model VARCHAR(100) NULL,
    username VARCHAR(100) NOT NULL,
    password_ciphertext LONGBLOB NOT NULL,
    status VARCHAR(16) NOT NULL DEFAULT 'active',
    connection_status VARCHAR(16) NOT NULL DEFAULT 'unknown',
    last_seen_at DATETIME(6) NULL,
    created_at DATETIME(6) NOT NULL,
    updated_at DATETIME(6) NOT NULL,
    CONSTRAINT devices_device_name_not_blank CHECK (TRIM(device_name) <> ''),
    CONSTRAINT devices_host_not_blank CHECK (TRIM(host) <> ''),
    CONSTRAINT devices_username_not_blank CHECK (TRIM(username) <> ''),
    CONSTRAINT devices_port_range CHECK (port >= 1 AND port <= 65535),
    CONSTRAINT devices_status_allowed CHECK (status IN ('active', 'inactive')),
    CONSTRAINT devices_connection_status_allowed CHECK (
        connection_status IN ('unknown', 'online', 'offline')
    ),
    CONSTRAINT devices_mac_address_format CHECK (
        mac_address IS NULL
        OR mac_address REGEXP '^([0-9A-F]{2}:){5}[0-9A-F]{2}$'
    ),
    CONSTRAINT devices_host_port_unique UNIQUE (host, port)
);

CREATE TABLE IF NOT EXISTS credentials (
    id CHAR(36) NOT NULL PRIMARY KEY,
    user_id CHAR(36) NOT NULL,
    type VARCHAR(32) NOT NULL,
    value_ciphertext LONGBLOB NOT NULL,
    value_digest LONGBLOB NOT NULL,
    display_hint VARCHAR(16) NULL,
    card_type VARCHAR(32) NULL,
    identifier_type VARCHAR(16) NULL,
    status VARCHAR(16) NOT NULL DEFAULT 'active',
    created_at DATETIME(6) NOT NULL,
    updated_at DATETIME(6) NOT NULL,
    card_digest_unique VARBINARY(64)
        GENERATED ALWAYS AS (CASE WHEN type = 'card' THEN value_digest ELSE NULL END) STORED,
    pin_user_unique CHAR(36)
        GENERATED ALWAYS AS (CASE WHEN type = 'pin' THEN user_id ELSE NULL END) STORED,
    CONSTRAINT credentials_user_fk FOREIGN KEY (user_id) REFERENCES users (id),
    CONSTRAINT credentials_type_allowed CHECK (
        type IN (
            'card', 'pin', 'read_only_card', 'smart_card', 'finger', 'face',
            'duress_finger', 'biometric_card'
        )
    ),
    CONSTRAINT credentials_status_allowed CHECK (status IN ('active', 'inactive')),
    CONSTRAINT credentials_card_value_digest_unique UNIQUE (card_digest_unique),
    CONSTRAINT credentials_one_pin_per_user UNIQUE (pin_user_unique)
);

CREATE TABLE IF NOT EXISTS enrollments (
    id CHAR(36) NOT NULL PRIMARY KEY,
    user_id CHAR(36) NOT NULL,
    credential_id CHAR(36) NOT NULL,
    device_id CHAR(36) NOT NULL,
    status VARCHAR(16) NOT NULL DEFAULT 'pending',
    identifier_digest LONGBLOB NULL,
    cancelled_at DATETIME(6) NULL,
    revoked_at DATETIME(6) NULL,
    activated_at DATETIME(6) NULL,
    created_at DATETIME(6) NOT NULL,
    updated_at DATETIME(6) NOT NULL,
    open_pair VARCHAR(80)
        GENERATED ALWAYS AS (
            CASE
                WHEN status IN ('pending', 'active', 'failed')
                THEN CONCAT(credential_id, ':', device_id)
                ELSE NULL
            END
        ) STORED,
    active_card_key VARBINARY(64)
        GENERATED ALWAYS AS (
            CASE
                WHEN status = 'active' AND identifier_digest IS NOT NULL THEN identifier_digest
                ELSE NULL
            END
        ) STORED,
    CONSTRAINT enrollments_user_fk FOREIGN KEY (user_id) REFERENCES users (id),
    CONSTRAINT enrollments_credential_fk FOREIGN KEY (credential_id) REFERENCES credentials (id),
    CONSTRAINT enrollments_device_fk FOREIGN KEY (device_id) REFERENCES devices (id),
    CONSTRAINT enrollments_status_allowed CHECK (
        status IN ('pending', 'active', 'failed', 'cancelled', 'revoked')
    ),
    CONSTRAINT enrollments_open_credential_device_unique UNIQUE (open_pair),
    CONSTRAINT enrollments_active_card_identifier_unique UNIQUE (device_id, active_card_key)
);

CREATE TABLE IF NOT EXISTS device_id_sequences (
    device_id CHAR(36) NOT NULL PRIMARY KEY,
    next_matrix_user_seq BIGINT NOT NULL DEFAULT 1,
    next_ref_user_id BIGINT NOT NULL DEFAULT 10000001,
    updated_at DATETIME(6) NOT NULL,
    CONSTRAINT device_id_sequences_device_fk FOREIGN KEY (device_id) REFERENCES devices (id)
);

CREATE TABLE IF NOT EXISTS device_users (
    id CHAR(36) NOT NULL PRIMARY KEY,
    user_id CHAR(36) NOT NULL,
    device_id CHAR(36) NOT NULL,
    matrix_user_id VARCHAR(15) NOT NULL,
    matrix_ref_user_id BIGINT NOT NULL,
    provisioned_at DATETIME(6) NULL,
    created_at DATETIME(6) NOT NULL,
    updated_at DATETIME(6) NOT NULL,
    CONSTRAINT device_users_user_fk FOREIGN KEY (user_id) REFERENCES users (id),
    CONSTRAINT device_users_device_fk FOREIGN KEY (device_id) REFERENCES devices (id),
    CONSTRAINT device_users_matrix_user_id_shape CHECK (matrix_user_id REGEXP '^[A-Za-z0-9]+$'),
    CONSTRAINT device_users_user_device_unique UNIQUE (user_id, device_id),
    CONSTRAINT device_users_device_matrix_user_unique UNIQUE (device_id, matrix_user_id),
    CONSTRAINT device_users_device_matrix_ref_unique UNIQUE (device_id, matrix_ref_user_id)
);

CREATE TABLE IF NOT EXISTS user_devices (
    id CHAR(36) NOT NULL PRIMARY KEY,
    user_id CHAR(36) NOT NULL,
    device_id CHAR(36) NOT NULL,
    created_at DATETIME(6) NOT NULL,
    CONSTRAINT user_devices_user_fk FOREIGN KEY (user_id) REFERENCES users (id),
    CONSTRAINT user_devices_device_fk FOREIGN KEY (device_id) REFERENCES devices (id),
    CONSTRAINT user_devices_pair_unique UNIQUE (user_id, device_id)
);

CREATE TABLE IF NOT EXISTS enrollment_sessions (
    id CHAR(36) NOT NULL PRIMARY KEY,
    user_id CHAR(36) NOT NULL,
    device_id CHAR(36) NOT NULL,
    enroll_type VARCHAR(32) NOT NULL,
    status VARCHAR(32) NOT NULL,
    credential_id CHAR(36) NULL,
    error_code VARCHAR(64) NULL,
    card_type VARCHAR(32) NULL,
    identifier_type VARCHAR(16) NULL,
    identifier_unavailable TINYINT(1) NOT NULL DEFAULT 0,
    capture_factor VARCHAR(16) NULL,
    started_at DATETIME(6) NOT NULL,
    completed_at DATETIME(6) NULL,
    created_at DATETIME(6) NOT NULL,
    updated_at DATETIME(6) NOT NULL,
    CONSTRAINT enrollment_sessions_user_fk FOREIGN KEY (user_id) REFERENCES users (id),
    CONSTRAINT enrollment_sessions_device_fk FOREIGN KEY (device_id) REFERENCES devices (id),
    CONSTRAINT enrollment_sessions_credential_fk FOREIGN KEY (credential_id) REFERENCES credentials (id),
    CONSTRAINT enrollment_sessions_status_allowed CHECK (
        status IN (
            'starting', 'waiting_for_card', 'processing', 'verifying', 'saving',
            'success', 'failed', 'cancelled', 'timeout', 'busy', 'persistence_failed'
        )
    ),
    CONSTRAINT enrollment_sessions_enroll_type_allowed CHECK (
        enroll_type IN (
            'read_only_card', 'smart_card', 'biometric', 'biometric_then_card',
            'face', 'duress_finger', 'card_then_face', 'face_then_card'
        )
    ),
    CONSTRAINT enrollment_sessions_capture_factor_allowed CHECK (
        capture_factor IS NULL OR capture_factor IN ('card', 'face')
    )
);

CREATE TABLE IF NOT EXISTS device_event_cursors (
    device_id CHAR(36) NOT NULL PRIMARY KEY,
    roll_over_count INT NOT NULL,
    seq_number BIGINT NOT NULL,
    updated_at DATETIME(6) NOT NULL,
    CONSTRAINT device_event_cursors_device_fk FOREIGN KEY (device_id) REFERENCES devices (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS events (
    id CHAR(36) NOT NULL PRIMARY KEY,
    device_id CHAR(36) NOT NULL,
    device_name VARCHAR(200) NOT NULL,
    roll_over_count INT NOT NULL,
    seq_number BIGINT NOT NULL,
    event_id VARCHAR(64) NULL,
    device_date VARCHAR(32) NOT NULL,
    device_time VARCHAR(32) NOT NULL,
    user_id CHAR(36) NULL,
    person_name VARCHAR(200) NULL,
    ref_user_id BIGINT NULL,
    detail_1 TEXT NULL,
    detail_2 TEXT NULL,
    detail_3 TEXT NULL,
    detail_4 TEXT NULL,
    detail_5 TEXT NULL,
    ingested_at DATETIME(6) NOT NULL,
    CONSTRAINT events_device_fk FOREIGN KEY (device_id) REFERENCES devices (id) ON DELETE CASCADE,
    CONSTRAINT events_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE SET NULL,
    CONSTRAINT events_device_seq_unique UNIQUE (device_id, roll_over_count, seq_number),
    CONSTRAINT events_rollover_range CHECK (roll_over_count >= 0 AND roll_over_count <= 65535),
    CONSTRAINT events_seq_range CHECK (seq_number >= 0 AND seq_number <= 500000)
);

CREATE INDEX credentials_user_id_idx ON credentials (user_id);
CREATE INDEX enrollments_user_created_idx ON enrollments (user_id, created_at);
CREATE INDEX device_users_device_id_idx ON device_users (device_id);
CREATE INDEX user_devices_device_id_idx ON user_devices (device_id);
CREATE INDEX enrollment_sessions_device_status_idx ON enrollment_sessions (device_id, status, created_at);
CREATE INDEX events_device_seq_idx ON events (device_id, roll_over_count, seq_number);
CREATE INDEX events_recent_idx ON events (roll_over_count, seq_number);
