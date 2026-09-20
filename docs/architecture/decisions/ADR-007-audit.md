# ADR-007: Audit logging

- **Status:** Accepted direction; implementation **PLANNED**
- **Date:** 2026-09-20

## Context

Operators need a record of what **this application** did (who enrolled whom, who registered a device), distinct from door hardware event logs.

## Decision

`domains/audit` records administrator/system actions in PostgreSQL: actor, action, resource, UTC time, result. No secrets, no biometric payloads. Core workflows call audit explicitly (or via a dedicated port), not via a hidden global event bus.

## Consequences

- Device `events` module must not be used as the admin audit trail.
- UI may list audit rows only through Tauri commands that strip secrets.
