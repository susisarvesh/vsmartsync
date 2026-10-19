# Testing

Every meaningful behavior change needs tests. Tests verify **behavior**, not private internals.

## Pyramid

1. **Unit** — pure Rust: config parsing, job state transitions, mapping, authorization decisions. Fast. Default `cargo test`.
2. **IPC** — Tauri command tests like existing `get_app_info` (no live devices).
3. **Postgres** — ignored tests (`-- --ignored`) against a **separate** database. Never require Postgres for CI `cargo test`. Writing tests read `TEST_DATABASE_URL` only. The database name must end with `_test` and must not be the application database (`DATABASE_URL` / `POSTGRES_DB`, normally `vsmart_sync`). A missing or matching URL fails before any insert. Each test deletes only the user and device ids it registered, including when the test panics. Cleanup also removes the device id sequence for those devices. A cleanup failure fails the test. Test encryption keys are used only against that database.
4. **Matrix** — domain tests mock the adapter. Hardware tests are optional, explicit, and never the only coverage.
5. **Frontend** — `npm run build` (typecheck + Vite). Add UI tests when screens exist; do not make every check an E2E desktop run.

## Minimum cases for a feature

- Happy path
- Validation failure
- Unauthorized / unauthenticated (when auth exists)
- Not found
- Duplicate / retry (idempotency)
- External (Matrix) timeout or error
- Obvious concurrency where uniqueness matters

Security-sensitive code also tests: command invoked without a session, access to another administrator’s resources if that model exists, injection-ish command payloads, secret fields absent from JSON.

## Rules

- Mock `matrix` in domain tests. Do not hit a real door in default tests.
- Do not commit `.env` or real device passwords in fixtures.
- Do not assert on log line wording as an API.
- Clippy and `cargo fmt --check` in CI are part of the quality bar.

## Commands

```bash
cargo test --manifest-path src-tauri/Cargo.toml
# Create the isolated database once, then point tests at it. Do not use the application database.
# The application login may not be allowed to create databases. Create it with a role that can,
# and grant that login use of the schema:
# CREATE DATABASE vsmart_sync_test;
# GRANT USAGE, CREATE ON SCHEMA public TO PUBLIC;  -- in vsmart_sync_test
# TEST_DATABASE_URL=postgres://USER:PASSWORD@127.0.0.1:5432/vsmart_sync_test
cargo test --manifest-path src-tauri/Cargo.toml -- --ignored --nocapture
npm run build
```
