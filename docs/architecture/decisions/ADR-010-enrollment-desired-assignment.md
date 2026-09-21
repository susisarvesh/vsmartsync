# ADR-010: Enrollment as durable desired assignment

- **Status:** Accepted
- **Date:** 2026-09-21

## Context

Users, credentials, and devices exist as separate system-of-record entities. The product must express that a credential belonging to a user should be associated with a device, at scale (~40k users), without coupling that intent to Matrix HTTP or sync execution.

Existing docs sometimes describe Enrollment as a live `enrolluser` capture session. That is a different concern from durable assignment.

## Decision

1. `enrollments` rows are **durable desired assignments**: User + Credential + Device.
2. Lifecycle statuses: `pending`, `active`, `failed`, `cancelled`, `revoked` with a fixed transition graph. Sync (future) moves `pending` ↔ `failed` / → `active`; Enrollment UI creates, cancels, retries, revokes.
3. Physical Matrix capture (`/device.cgi/enrolluser`) is **not** an enrollment row; it may become `enrollment_sessions` later.
4. Sync attempts, retries, workers, and execution errors belong in a future Sync domain — not on `enrollments`.
5. Enrollment never calls Matrix CGI.

## Consequences

- Partial unique index enforces one open enrollment per `(credential_id, device_id)`.
- Offline devices may still receive `pending` assignments.
- Synchronization and Matrix client slices consume these rows later.
