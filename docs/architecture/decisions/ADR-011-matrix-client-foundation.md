# ADR-011: Matrix Client foundation and multi-model contract

- **Status:** Accepted
- **Date:** 2026-09-21

## Context

Vsmart Sync must talk to Matrix COSEC devices through the documented Devices API (guide v28). The product target is **all device families covered by that guide**, not a single hard-coded model. Guide applicability still varies by model: documented support ≠ hardware-verified support.

Identity mapping (`user-id` / `ref-user-id`) and Sync schema (`device_users`) are designed but must not land in the first Matrix code slice.

A lab ARGO FACE (firmware V01R25.02, `192.168.0.11`) answers `GET /device.cgi/device-basic-config?action=get` with `401` and `WWW-Authenticate: Digest` (`algorithm=MD5`, `qop=auth`). The guide still describes Basic Auth. Sending Basic to that challenge never reaches `Response-Code`.

## Decision

### Layers

| Layer | Owns | Must not own |
|---|---|---|
| **Matrix HTTP Client** | Transport, auth handshake, timeouts, no redirects, CGI URL build for documented paths, parse/`Response-Code` evaluation | Sync retries, domain rules, React types |
| **Matrix Adapter** | Application-facing ops; CGI choice (e.g. PIN via `/users`); later capability checks → `Unsupported` | SQL, enrollment SoR, password vault policy |
| **Domains / Sync** | Business rules, jobs, retries when safe | CGI paths, inventing Matrix semantics |

### Authentication

The first request carries no `Authorization` header.

- `WWW-Authenticate: Digest` with `algorithm=MD5` and `qop=auth` is answered with Digest. The request URI includes the query string. Do not send Basic to a Digest challenge.
- `WWW-Authenticate: Basic` is answered with Basic.
- A second `401` is followed only when the new Digest challenge has `stale=true`. Then stop.

That is at most two follow-up requests. It is the login handshake, not a Sync retry. Passwords, `Authorization`, and nonce material are not logged or returned to React.

### Success rule

A Matrix call succeeds only when **HTTP success and `Response-Code=0`**. Non-zero codes become `MatrixClientError::ApiError { code }`. Missing/unparsable codes are `BadResponse`.

### Multi-model

Architecture supports all guide-listed families. Do not scatter `if model == …` through domains. Capability checks belong in the adapter later; **no capability DB table in this slice**. Minimal `MatrixCapability` enum may appear when Adapter grows beyond probe.

### Identity (architecture only; schema later)

- Application `users.id` remains UUID.
- Matrix `user-id` (≤15 alnum) and `ref-user-id` (numeric ≤8 digits) are **device-scoped** mappings (`device_users`), allocated per device — not stored on `users` yet.
- Do **not** put the application UUID into `ref-user-id`.

### Explicitly out of this ADR’s implementation slice

- Broader `/credential`, events, Sync jobs (see [ADR-012](ADR-012-matrix-adapter-set-user-pin.md) for `set_user` / `set_pin`)
- `device_users` migration
- Capability framework / DB flags
- Sync retries inside the client (the auth handshake above is the only follow-up)
- `set_card` (blocked pending verified Card payload)

## Consequences

- Existing probe requires bodies with `Response-Code=0` (wiremock fixtures updated).
- Documented vs hardware-verified support must be stated in docs; do not claim every model was lab-tested.
- Next: Sync design/implementation consuming `device_users` (ADR-013); resolve Card path before `set_card`.
