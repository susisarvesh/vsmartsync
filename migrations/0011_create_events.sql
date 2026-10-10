-- Device access history. The door decides entry. This table stores what the
-- device already reported. Names are copied at ingest so a later rename does
-- not change the log.

CREATE TABLE device_event_cursors (
    device_id UUID PRIMARY KEY REFERENCES devices (id) ON DELETE CASCADE,
    roll_over_count INTEGER NOT NULL,
    seq_number BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT device_event_cursors_rollover_range CHECK (
        roll_over_count >= 0 AND roll_over_count <= 65535
    ),
    CONSTRAINT device_event_cursors_seq_range CHECK (
        seq_number >= 0 AND seq_number <= 500000
    )
);

CREATE TABLE events (
    id UUID PRIMARY KEY,
    device_id UUID NOT NULL REFERENCES devices (id) ON DELETE CASCADE,
    device_name TEXT NOT NULL,
    roll_over_count INTEGER NOT NULL,
    seq_number BIGINT NOT NULL,
    event_id TEXT NULL,
    device_date TEXT NOT NULL,
    device_time TEXT NOT NULL,
    user_id UUID NULL REFERENCES users (id) ON DELETE SET NULL,
    person_name TEXT NULL,
    ref_user_id BIGINT NULL,
    detail_1 TEXT NULL,
    detail_2 TEXT NULL,
    detail_3 TEXT NULL,
    detail_4 TEXT NULL,
    detail_5 TEXT NULL,
    ingested_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT events_device_seq_unique UNIQUE (device_id, roll_over_count, seq_number),
    CONSTRAINT events_rollover_range CHECK (
        roll_over_count >= 0 AND roll_over_count <= 65535
    ),
    CONSTRAINT events_seq_range CHECK (
        seq_number >= 0 AND seq_number <= 500000
    ),
    CONSTRAINT events_device_name_not_blank CHECK (btrim(device_name) <> ''),
    CONSTRAINT events_ref_range CHECK (
        ref_user_id IS NULL OR (ref_user_id >= 0 AND ref_user_id <= 99999999)
    )
);

CREATE INDEX events_device_seq_idx
    ON events (device_id, roll_over_count DESC, seq_number DESC);
