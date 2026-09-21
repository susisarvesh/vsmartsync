# ADR-013: device_users Matrix identity mapping

- **Status:** Accepted
- **Date:** 2026-09-21

## Context

Sync must know which Matrix `user-id` / `ref-user-id` represents an application user on a given device. Those identifiers are device-scoped (guide: alphanumeric `user-id` ≤15, numeric `ref-user-id` ≤8 digits). They must not live on `users`, and must not be provisioned until Sync successfully calls `set_user`.

## Decision

### Tables

1. **`device_users`** — one Matrix identity mapping per `(user_id, device_id)`.
2. **`device_id_sequences`** — per-device counters for allocating ids (not columns on `devices`).

### Identity allocation

- `matrix_user_id`: device-local `VS######` (e.g. `VS000001`). Not derived from person name.
- `matrix_ref_user_id`: device-local sequence starting at `10000001`, stored as `BIGINT` with `CHECK (0 … 99999999)`.
- Same application user may receive the same *shape* of ids on different devices; uniqueness is enforced **per device**.

### Lifecycle

```text
ensure_mapping → row exists, provisioned_at NULL
Sync set_user success → mark_provisioned → provisioned_at set
```

Allocation ≠ provisioned. Sync jobs (future) own execution/errors — not this table.

### Constraints

- `UNIQUE (user_id, device_id)`
- `UNIQUE (device_id, matrix_user_id)`
- `UNIQUE (device_id, matrix_ref_user_id)`
- FKs to `users` / `devices` with `ON DELETE RESTRICT`

### Out of scope for this ADR’s slice

- Tauri/React UI
- Sync jobs, Matrix HTTP, Card/Face
- Changing enrollments / credentials schemas

## Consequences

- Sync will call `ensure_mapping` before `set_user`, then `mark_provisioned`.
- Deleting a user or device that still has mappings fails until mappings are removed by an explicit workflow (later).
