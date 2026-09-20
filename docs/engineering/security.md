# Security

Security is a first-class requirement. This app runs on a workstation and talks to physical doors. A leak can expose device passwords, biometric templates, and who is allowed through a door.

## Boundaries

| Boundary | Rule |
|---|---|
| UI ↔ Rust | Only Tauri commands. Never trust the UI for authorization. |
| Rust ↔ PostgreSQL | Parameterized SQLx only. No string-built SQL with input. |
| Rust ↔ Matrix device | Only `matrix/`. Device basic-auth never returned to React. |
| USB license | Independent of Matrix. License secrets never in the UI. |
| Git | No `.env`, passwords, keys, templates, or real `DATABASE_URL`. |

This product is a **single local database**, not a cloud multi-tenant SaaS. Still: never trust client-supplied identity, roles, or “I own this row” flags. When desktop auth exists, identity comes from the **server-side session** in Rust.

## Authentication vs authorization

- **Authentication:** who is the workstation administrator? (`domains/auth`, **PLANNED**). Distinct from HTTP basic auth **to a COSEC device**.
- **Authorization:** is that administrator allowed to run this command? Enforce in Rust. Hiding a button is not security.

Deny by default. Every new command must declare whether it is public (today only foundation status commands) or auth-gated.

## Untrusted input

Validate at boundaries: Tauri command arguments, `.env`, Matrix responses, imported files, device host/port strings.

Assume Matrix responses are malformed, partial, or hostile. Do not pass them straight into SQL or the UI.

## Device URLs (SSRF)

Administrators will enter device IPs. Treat that as dangerous:

- Prefer IPv4/IPv6 literals or DNS you have validated for **doors**, not arbitrary URLs.
- Do not fetch `file://`, cloud metadata, or unexpected schemes.
- Timeouts on every device HTTP call.
- Do not let the UI supply a full URL that the client fetches; Rust builds the CGI URL inside `matrix::client`.

## Secrets and credentials

Never commit, log, or return:

- PostgreSQL passwords
- Matrix device usernames/passwords
- USB/license material
- biometric templates or raw samples
- session tokens (when they exist)

Passwords for **application login** must be hashed (not reversible). Device passwords need a designed secret store — not plaintext in a command payload.

## Biometrics

Minimize storage. Prefer vendor templates over raw images. Never log or send templates to React. Encrypt at rest when that design lands. Matching is expected **on the device**; do not assume this app performs live biometric match ([architecture overview](../architecture/overview.md)).

## Injection

- SQL: bind parameters only.
- No shelling out with user strings.
- No path traversal on uploads (when added): server-side names, size/type limits.

## Concurrency and replay

Enrollment, sync jobs, and device commands can be duplicated. Use unique constraints and idempotent job keys. Do not use `if not exists { insert }` alone.

## Logging

Structured tracing is already used. Log **action + resource id**, not secrets, templates, or Authorization headers.

## Frontend

React is untrusted. It must not:

- store device passwords
- enforce license checks
- decide tenant/admin identity
- call Matrix or PostgreSQL
