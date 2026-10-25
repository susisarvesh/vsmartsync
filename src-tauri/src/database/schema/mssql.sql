CREATE TABLE users (
    id UNIQUEIDENTIFIER NOT NULL PRIMARY KEY,
    username NVARCHAR(200) NOT NULL,
    status NVARCHAR(16) NOT NULL,
    matrix_user_id NVARCHAR(15) NULL,
    short_name NVARCHAR(15) NULL,
    full_name NVARCHAR(200) NULL,
    reference_id BIGINT NULL,
    created_at DATETIMEOFFSET NOT NULL,
    updated_at DATETIMEOFFSET NOT NULL,
    CONSTRAINT users_username_not_blank CHECK (LTRIM(RTRIM(username)) <> N''),
    CONSTRAINT users_status_allowed CHECK (status IN (N'active', N'inactive')),
    CONSTRAINT users_matrix_user_id_shape CHECK (
        matrix_user_id IS NULL OR matrix_user_id NOT LIKE N'%[^0-9A-Za-z]%'
    ),
    CONSTRAINT users_reference_id_range CHECK (
        reference_id IS NULL OR (reference_id >= 1 AND reference_id <= 99999999)
    )
);

CREATE UNIQUE INDEX users_matrix_user_id_unique
    ON users (matrix_user_id)
    WHERE matrix_user_id IS NOT NULL;

CREATE UNIQUE INDEX users_reference_id_unique
    ON users (reference_id)
    WHERE reference_id IS NOT NULL;

CREATE TABLE devices (
    id UNIQUEIDENTIFIER NOT NULL PRIMARY KEY,
    device_name NVARCHAR(200) NOT NULL,
    host NVARCHAR(255) NOT NULL,
    port INT NOT NULL CONSTRAINT devices_port_default DEFAULT 80,
    mac_address NVARCHAR(17) NULL,
    device_model NVARCHAR(100) NULL,
    username NVARCHAR(100) NOT NULL,
    password_ciphertext VARBINARY(MAX) NOT NULL,
    status NVARCHAR(16) NOT NULL CONSTRAINT devices_status_default DEFAULT N'active',
    connection_status NVARCHAR(16) NOT NULL CONSTRAINT devices_connection_default DEFAULT N'unknown',
    last_seen_at DATETIMEOFFSET NULL,
    created_at DATETIMEOFFSET NOT NULL,
    updated_at DATETIMEOFFSET NOT NULL,
    CONSTRAINT devices_device_name_not_blank CHECK (LTRIM(RTRIM(device_name)) <> N''),
    CONSTRAINT devices_host_not_blank CHECK (LTRIM(RTRIM(host)) <> N''),
    CONSTRAINT devices_port_range CHECK (port >= 1 AND port <= 65535),
    CONSTRAINT devices_status_allowed CHECK (status IN (N'active', N'inactive')),
    CONSTRAINT devices_connection_status_allowed CHECK (
        connection_status IN (N'unknown', N'online', N'offline')
    ),
    CONSTRAINT devices_host_port_unique UNIQUE (host, port)
);

CREATE TABLE credentials (
    id UNIQUEIDENTIFIER NOT NULL PRIMARY KEY,
    user_id UNIQUEIDENTIFIER NOT NULL,
    type NVARCHAR(32) NOT NULL,
    value_ciphertext VARBINARY(MAX) NOT NULL,
    value_digest VARBINARY(MAX) NOT NULL,
    display_hint NVARCHAR(16) NULL,
    card_type NVARCHAR(32) NULL,
    identifier_type NVARCHAR(16) NULL,
    status NVARCHAR(16) NOT NULL CONSTRAINT credentials_status_default DEFAULT N'active',
    created_at DATETIMEOFFSET NOT NULL,
    updated_at DATETIMEOFFSET NOT NULL,
    CONSTRAINT credentials_user_fk FOREIGN KEY (user_id) REFERENCES users (id),
    CONSTRAINT credentials_type_allowed CHECK (
        type IN (
            N'card', N'pin', N'read_only_card', N'smart_card', N'finger', N'face',
            N'duress_finger', N'biometric_card'
        )
    ),
    CONSTRAINT credentials_status_allowed CHECK (status IN (N'active', N'inactive'))
);

CREATE UNIQUE INDEX credentials_card_value_digest_unique
    ON credentials (value_digest)
    WHERE type = N'card';

CREATE UNIQUE INDEX credentials_one_pin_per_user
    ON credentials (user_id)
    WHERE type = N'pin';

CREATE TABLE enrollments (
    id UNIQUEIDENTIFIER NOT NULL PRIMARY KEY,
    user_id UNIQUEIDENTIFIER NOT NULL,
    credential_id UNIQUEIDENTIFIER NOT NULL,
    device_id UNIQUEIDENTIFIER NOT NULL,
    status NVARCHAR(16) NOT NULL CONSTRAINT enrollments_status_default DEFAULT N'pending',
    identifier_digest VARBINARY(MAX) NULL,
    cancelled_at DATETIMEOFFSET NULL,
    revoked_at DATETIMEOFFSET NULL,
    activated_at DATETIMEOFFSET NULL,
    created_at DATETIMEOFFSET NOT NULL,
    updated_at DATETIMEOFFSET NOT NULL,
    CONSTRAINT enrollments_user_fk FOREIGN KEY (user_id) REFERENCES users (id),
    CONSTRAINT enrollments_credential_fk FOREIGN KEY (credential_id) REFERENCES credentials (id),
    CONSTRAINT enrollments_device_fk FOREIGN KEY (device_id) REFERENCES devices (id),
    CONSTRAINT enrollments_status_allowed CHECK (
        status IN (N'pending', N'active', N'failed', N'cancelled', N'revoked')
    )
);

CREATE UNIQUE INDEX enrollments_open_credential_device_unique
    ON enrollments (credential_id, device_id)
    WHERE status IN (N'pending', N'active', N'failed');

CREATE UNIQUE INDEX enrollments_active_card_identifier_unique
    ON enrollments (device_id, identifier_digest)
    WHERE status = N'active' AND identifier_digest IS NOT NULL;

CREATE TABLE device_id_sequences (
    device_id UNIQUEIDENTIFIER NOT NULL PRIMARY KEY,
    next_matrix_user_seq BIGINT NOT NULL CONSTRAINT device_id_sequences_user_default DEFAULT 1,
    next_ref_user_id BIGINT NOT NULL CONSTRAINT device_id_sequences_ref_default DEFAULT 10000001,
    updated_at DATETIMEOFFSET NOT NULL,
    CONSTRAINT device_id_sequences_device_fk FOREIGN KEY (device_id) REFERENCES devices (id)
);

CREATE TABLE device_users (
    id UNIQUEIDENTIFIER NOT NULL PRIMARY KEY,
    user_id UNIQUEIDENTIFIER NOT NULL,
    device_id UNIQUEIDENTIFIER NOT NULL,
    matrix_user_id NVARCHAR(15) NOT NULL,
    matrix_ref_user_id BIGINT NOT NULL,
    provisioned_at DATETIMEOFFSET NULL,
    created_at DATETIMEOFFSET NOT NULL,
    updated_at DATETIMEOFFSET NOT NULL,
    CONSTRAINT device_users_user_fk FOREIGN KEY (user_id) REFERENCES users (id),
    CONSTRAINT device_users_device_fk FOREIGN KEY (device_id) REFERENCES devices (id),
    CONSTRAINT device_users_matrix_user_id_shape CHECK (matrix_user_id NOT LIKE N'%[^0-9A-Za-z]%'),
    CONSTRAINT device_users_user_device_unique UNIQUE (user_id, device_id),
    CONSTRAINT device_users_device_matrix_user_unique UNIQUE (device_id, matrix_user_id),
    CONSTRAINT device_users_device_matrix_ref_unique UNIQUE (device_id, matrix_ref_user_id)
);

CREATE TABLE user_devices (
    id UNIQUEIDENTIFIER NOT NULL PRIMARY KEY,
    user_id UNIQUEIDENTIFIER NOT NULL,
    device_id UNIQUEIDENTIFIER NOT NULL,
    created_at DATETIMEOFFSET NOT NULL,
    CONSTRAINT user_devices_user_fk FOREIGN KEY (user_id) REFERENCES users (id),
    CONSTRAINT user_devices_device_fk FOREIGN KEY (device_id) REFERENCES devices (id),
    CONSTRAINT user_devices_pair_unique UNIQUE (user_id, device_id)
);

CREATE TABLE enrollment_sessions (
    id UNIQUEIDENTIFIER NOT NULL PRIMARY KEY,
    user_id UNIQUEIDENTIFIER NOT NULL,
    device_id UNIQUEIDENTIFIER NOT NULL,
    enroll_type NVARCHAR(32) NOT NULL,
    status NVARCHAR(32) NOT NULL,
    credential_id UNIQUEIDENTIFIER NULL,
    error_code NVARCHAR(64) NULL,
    card_type NVARCHAR(32) NULL,
    identifier_type NVARCHAR(16) NULL,
    identifier_unavailable BIT NOT NULL CONSTRAINT enrollment_sessions_unavailable_default DEFAULT 0,
    capture_factor NVARCHAR(16) NULL,
    started_at DATETIMEOFFSET NOT NULL,
    completed_at DATETIMEOFFSET NULL,
    created_at DATETIMEOFFSET NOT NULL,
    updated_at DATETIMEOFFSET NOT NULL,
    CONSTRAINT enrollment_sessions_user_fk FOREIGN KEY (user_id) REFERENCES users (id),
    CONSTRAINT enrollment_sessions_device_fk FOREIGN KEY (device_id) REFERENCES devices (id),
    CONSTRAINT enrollment_sessions_credential_fk FOREIGN KEY (credential_id) REFERENCES credentials (id),
    CONSTRAINT enrollment_sessions_status_allowed CHECK (
        status IN (
            N'starting', N'waiting_for_card', N'processing', N'verifying', N'saving',
            N'success', N'failed', N'cancelled', N'timeout', N'busy', N'persistence_failed'
        )
    ),
    CONSTRAINT enrollment_sessions_enroll_type_allowed CHECK (
        enroll_type IN (
            N'read_only_card', N'smart_card', N'biometric', N'biometric_then_card',
            N'face', N'duress_finger', N'card_then_face', N'face_then_card'
        )
    ),
    CONSTRAINT enrollment_sessions_capture_factor_allowed CHECK (
        capture_factor IS NULL OR capture_factor IN (N'card', N'face')
    )
);

CREATE TABLE device_event_cursors (
    device_id UNIQUEIDENTIFIER NOT NULL PRIMARY KEY,
    roll_over_count INT NOT NULL,
    seq_number BIGINT NOT NULL,
    updated_at DATETIMEOFFSET NOT NULL,
    CONSTRAINT device_event_cursors_device_fk FOREIGN KEY (device_id) REFERENCES devices (id) ON DELETE CASCADE
);

CREATE TABLE events (
    id UNIQUEIDENTIFIER NOT NULL PRIMARY KEY,
    device_id UNIQUEIDENTIFIER NOT NULL,
    device_name NVARCHAR(200) NOT NULL,
    roll_over_count INT NOT NULL,
    seq_number BIGINT NOT NULL,
    event_id NVARCHAR(64) NULL,
    device_date NVARCHAR(32) NOT NULL,
    device_time NVARCHAR(32) NOT NULL,
    user_id UNIQUEIDENTIFIER NULL,
    person_name NVARCHAR(200) NULL,
    ref_user_id BIGINT NULL,
    detail_1 NVARCHAR(MAX) NULL,
    detail_2 NVARCHAR(MAX) NULL,
    detail_3 NVARCHAR(MAX) NULL,
    detail_4 NVARCHAR(MAX) NULL,
    detail_5 NVARCHAR(MAX) NULL,
    ingested_at DATETIMEOFFSET NOT NULL,
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
