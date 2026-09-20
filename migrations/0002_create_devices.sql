CREATE TABLE devices (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    host TEXT NOT NULL,
    port INTEGER NOT NULL DEFAULT 80,
    username TEXT NOT NULL,
    password_ciphertext BYTEA NOT NULL,
    connection_status TEXT NOT NULL DEFAULT 'unknown',
    last_seen_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT devices_name_not_blank CHECK (btrim(name) <> ''),
    CONSTRAINT devices_name_length CHECK (char_length(name) <= 200),
    CONSTRAINT devices_host_not_blank CHECK (btrim(host) <> ''),
    CONSTRAINT devices_host_length CHECK (char_length(host) <= 255),
    CONSTRAINT devices_username_not_blank CHECK (btrim(username) <> ''),
    CONSTRAINT devices_username_length CHECK (char_length(username) <= 100),
    CONSTRAINT devices_port_range CHECK (port >= 1 AND port <= 65535),
    CONSTRAINT devices_connection_status_allowed CHECK (
        connection_status IN ('unknown', 'online', 'offline')
    ),
    CONSTRAINT devices_host_port_unique UNIQUE (host, port)
);
