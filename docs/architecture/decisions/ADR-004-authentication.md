# ADR-004: Authentication

- **Status:** Accepted direction; implementation **PLANNED**
- **Date:** 2026-09-20

## Context

Two different “logins” exist in this product space: workstation administrators vs HTTP basic auth **to a Matrix device**.

## Decision

- `domains/auth` authenticates **desktop administrators** only. Identity for authorization comes from a Rust-side session, never from client-supplied user/role fields.
- Device basic-auth is stored and used only in `matrix` / device records. It is not the app login.
- No cloud IdP, no public JWT API for the UI in the MVP. If tokens are added later, validate signature, issuer, audience, exp, and algorithm with established libraries — never home-grown crypto.
- Frontend authorization (hiding buttons) is not sufficient.

## Consequences

- Do not mix `auth` with device passwords.
- New Tauri commands default to deny-until-auth exists; foundation status commands are the current exception.
- Tenant/SaaS isolation is **out of scope** for this local single-database app; do not build `tenant_id` cloud plumbing unless a future ADR says the product became multi-site SaaS.
