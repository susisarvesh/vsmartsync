# Error handling

## Split two audiences

| Audience | What they get |
|---|---|
| Administrator (React) | Short, safe, actionable text. No SQL, paths, stack traces, or secrets. |
| Operators / developers (logs) | Structured tracing with error type, command, resource id. Still no secrets. |

Never `return exception.to_string()` through Tauri.

## Rust

- Domain and infrastructure: typed errors (`thiserror`), `?`, no silent `let _ =`.
- Do not `catch` everything and `pass`. If you cannot recover, propagate to the command boundary and map there.
- Map `sqlx` / IO / Matrix client errors to application errors (`DeviceUnreachable`, `EnrollmentFailed`) before the UI.

Suggested stable codes (when you add them): `DEVICE_NOT_FOUND`, `DEVICE_OFFLINE`, `ENROLLMENT_FAILED`, `AUTHORIZATION_DENIED`, `DATABASE_UNAVAILABLE`.

## Matrix

Treat timeouts, HTTP 401, malformed bodies, and “unsupported on this model” as **expected** failure modes. Do not retry non-idempotent device commands blindly (the door may have already applied the change).

## Frontend

Show the safe message. Do not stringify raw invoke errors into a dump the user might copy into a ticket with secrets.

## Partial failure

Sync and enrollment: persist job/session state so a crash does not look like success. Prefer an explicit `failed` / `unknown` status over pretending the device matches PostgreSQL.
