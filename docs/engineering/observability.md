# Observability

## Logs

JSON tracing is already initialized in `lib.rs`. Use it for **actions**, not payloads.

Each important operation should make it possible to answer: what, when (UTC), which command, which resource id, success or failure. Add correlation/command id when workflows span jobs.

Never log: passwords, tokens, `DATABASE_URL`, device basic-auth, biometric templates, raw CGI query strings that include secrets.

## Audit (**PLANNED**)

Security-sensitive actions in **this application** go to `domains/audit`, not into device event history.

Examples: admin login, user create/delete, device register, enrollment start, credential change, sync enqueue, license change.

Audit fields: actor, action, resource, timestamp UTC, result, correlation id. No secrets.

Device-reported door events belong to `domains/events`.

## Sync jobs

When implemented, persist and/or log: started, completed, failed, duration, counts processed/failed, retry count. See [synchronization.md](../synchronization.md).

## Health

The foundation already exposes database connected vs setup-mode in the UI. Do not make “process is alive” depend on every door being online. A device down is not “app crashed.” USB license missing is not “Postgres down.”

## Metrics / tracing exporters

Not in the MVP. Do not add Prometheus/OpenTelemetry stacks unless an ADR says so. Structured logs first.
