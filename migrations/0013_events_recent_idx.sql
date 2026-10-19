-- The all-devices access log orders by sequence without filtering on device_id.
-- The device-scoped index cannot serve that sort.

CREATE INDEX events_recent_idx
    ON events (roll_over_count DESC, seq_number DESC);
