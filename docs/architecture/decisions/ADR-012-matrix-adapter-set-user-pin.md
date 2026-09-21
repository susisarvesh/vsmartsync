# ADR-012: Matrix Adapter set_user and set_pin

- **Status:** Accepted
- **Date:** 2026-09-21

## Context

The Matrix Client foundation evaluates HTTP + `Response-Code=0`. The next product step is Adapter growth for user/PIN provisioning. Card remains blocked: the guide documents `card1`/`card2` on both `/users` and `/credential` (`type=2`) but provides **no worked Card set example**, so `set_card` must not be invented.

## Decision

Implement application-facing Adapter operations only:

| Operation | Matrix mapping | Notes |
|---|---|---|
| `set_user` | `GET /device.cgi/users?action=set` with `user-id`, `ref-user-id`, optional `name`, `user-active` | Establishes/updates Matrix user identity. Create requires both IDs (guide). |
| `set_pin` | `GET /device.cgi/users?action=set` with `user-id`, `user-pin` | Operates on an **existing** Matrix user. Does **not** send `ref-user-id`. Empty PIN clears (`user-pin=`). |
| `set_card` | **Deferred** | Requires hardware capture or Matrix confirmation of the Card path. |

Constraints for this slice:

- No Sync, no `device_users`, no DB migrations, no Enrollment/Users/Credentials behavior changes, no React, no retries, no capability framework, no orchestration (`set_user` → `set_pin` chaining).
- Adapter validates guide limits (user-id ≤15 alnum, ref-user-id ≤8 digits, name ≤15 alnum, pin 0 or 1–15 digits) and rejects forbidden CGI characters.
- Secrets (device password, PIN) are never logged.
- Errors: transport + `MatrixAdapterError::ApiError { code }` for non-zero `Response-Code`.

## Consequences

- Sync (later) orchestrates Ensure User → Set PIN → Set Card.
- Card implementation waits on an explicit path decision (`/users` vs `/credential type=2`).
