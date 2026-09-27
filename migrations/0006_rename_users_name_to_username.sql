-- Align the users table with the current-flow user entity field `username`.
-- Storage values for status stay `active` / `inactive`.

ALTER TABLE users RENAME COLUMN name TO username;
ALTER TABLE users RENAME CONSTRAINT users_name_not_blank TO users_username_not_blank;
ALTER TABLE users RENAME CONSTRAINT users_name_length TO users_username_length;
