# Code review checklist

Complete this before considering a change done. Skip items that cannot apply (and say why).

## Process

- [ ] Requirement understood; owning module identified
- [ ] Relevant `docs/engineering/` and `docs/architecture/` read
- [ ] Existing commands, types, and repositories searched
- [ ] Smallest change that preserves boundaries
- [ ] One domain service at a time (no “also add Matrix HTTP + events”)

## Correctness

- [ ] Behavior matches the request
- [ ] Edge cases: missing DB, offline device, duplicate request
- [ ] Matrix behavior is documented or marked for verification (not invented)

## Security

- [ ] UI cannot authorize itself
- [ ] Input validated at the command/adapter boundary
- [ ] No secrets in git, logs, or Tauri responses
- [ ] No SQL concatenation; no CGI outside `matrix/`
- [ ] Device host/port cannot be turned into arbitrary SSRF
- [ ] Biometric/template data not sent to React
- [ ] Unique constraints / idempotency where retries exist

## Reliability

- [ ] Typed errors; nothing swallowed
- [ ] Retries only if safe
- [ ] Partial failure leaves explicit job/session state
- [ ] Transactions for multi-row desired-state updates

## Maintainability

- [ ] No new god module or `utils` dump
- [ ] No unrelated refactor
- [ ] Pattern used only because it earned its keep

## Tests and quality

- [ ] Unit or IPC tests for new behavior
- [ ] `cargo fmt`, `cargo test`, `clippy -D warnings`, `npm run build`

## Compatibility

- [ ] Existing Tauri command payloads unchanged or migrated
- [ ] Migrations reviewed if schema changed

## Report

What changed, why, files, security notes, tests, DB/command changes, limitations.
