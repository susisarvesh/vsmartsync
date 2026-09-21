-- Durable desired assignment: user + credential + device.
-- Matrix sync and physical enrolluser sessions are separate concerns.
CREATE TABLE enrollments (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id),
    credential_id UUID NOT NULL REFERENCES credentials (id),
    device_id UUID NOT NULL REFERENCES devices (id),
    status TEXT NOT NULL DEFAULT 'pending',
    cancelled_at TIMESTAMPTZ NULL,
    revoked_at TIMESTAMPTZ NULL,
    activated_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT enrollments_status_allowed CHECK (
        status IN ('pending', 'active', 'failed', 'cancelled', 'revoked')
    )
);

-- At most one open enrollment per credential+device.
CREATE UNIQUE INDEX enrollments_open_credential_device_unique
    ON enrollments (credential_id, device_id)
    WHERE status IN ('pending', 'active', 'failed');

-- List enrollments for a user (paginated).
CREATE INDEX enrollments_user_created_idx
    ON enrollments (user_id, created_at DESC);

-- List / queue by device and status.
CREATE INDEX enrollments_device_status_created_idx
    ON enrollments (device_id, status, created_at DESC);

-- List by credential.
CREATE INDEX enrollments_credential_id_idx
    ON enrollments (credential_id);

-- Pending/failed work scans for future Sync.
CREATE INDEX enrollments_status_updated_idx
    ON enrollments (status, updated_at);
