CREATE TABLE users (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT users_name_not_blank CHECK (btrim(name) <> ''),
    CONSTRAINT users_name_length CHECK (char_length(name) <= 200),
    CONSTRAINT users_status_allowed CHECK (status IN ('active', 'inactive'))
);
