-- Align devices with the current-flow device entity.
-- `status` is the software record (active/inactive).
-- `connection_status` stays the reachability result of a connection test.
-- `device_model` stays empty until a later discovery step can fill it.
-- `mac_address` is optional and stored only in canonical form.

ALTER TABLE devices RENAME COLUMN name TO device_name;
ALTER TABLE devices RENAME CONSTRAINT devices_name_not_blank TO devices_device_name_not_blank;
ALTER TABLE devices RENAME CONSTRAINT devices_name_length TO devices_device_name_length;

ALTER TABLE devices
    ADD COLUMN mac_address TEXT NULL,
    ADD COLUMN device_model TEXT NULL,
    ADD COLUMN status TEXT NOT NULL DEFAULT 'active';

ALTER TABLE devices
    ADD CONSTRAINT devices_mac_address_format CHECK (
        mac_address IS NULL
        OR mac_address ~ '^([0-9A-F]{2}:){5}[0-9A-F]{2}$'
    ),
    ADD CONSTRAINT devices_device_model_length CHECK (
        device_model IS NULL
        OR (btrim(device_model) <> '' AND char_length(device_model) <= 100)
    ),
    ADD CONSTRAINT devices_status_allowed CHECK (status IN ('active', 'inactive'));
