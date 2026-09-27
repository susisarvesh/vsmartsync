-- Local assignment of one application user to many devices.
-- This is not Matrix identity (`device_users`) and does not call the device.

CREATE TABLE user_devices (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    device_id UUID NOT NULL REFERENCES devices (id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT user_devices_pair_unique UNIQUE (user_id, device_id)
);

CREATE INDEX user_devices_device_id_idx ON user_devices (device_id);
