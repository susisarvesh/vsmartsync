# Testing

Every meaningful behavior change needs tests. Tests verify **behavior**, not private internals.

## Pyramid

1. **Unit** — pure Rust: config parsing, job state transitions, mapping, authorization decisions. Fast. Default `cargo test`.
2. **IPC** — Tauri command tests like existing `get_app_info` (no live devices).
3. **Postgres** — ignored tests (`-- --ignored`) against local 5432. Never require Postgres for CI `cargo test`.
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
cargo test --manifest-path src-tauri/Cargo.toml -- --ignored --nocapture
npm run build
```
