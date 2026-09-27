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
5. Enrollment never calls Matrix CGI for the assignment row itself. A separate command, `enroll_on_device`, may call the Matrix adapter after reading `device-basic-config`, `reader-config`, and `enroll-options`. It then calls `users?action=set` and `enrolluser?action=enroll`, and stores an active enrollment. Biometric template bytes are not stored.

## Consequences

- Partial unique index enforces one open enrollment per `(credential_id, device_id)`.
- Offline devices may still receive `pending` assignments.
- Amended 2026-09-27: hardware enrollment checks documented device configuration, starts capture with `enrolluser`, syncs the user onto that device, and stores the local link. `enroll-on-device=0` does not block `enrolluser`.
