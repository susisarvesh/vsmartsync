# ADR-005: Synchronization

- **Status:** Accepted direction; implementation **PLANNED**
- **Date:** 2026-09-20

## Context

PostgreSQL holds desired state. Each door holds its own users/credentials. The COSEC Devices API guide has **no** single “sync everything” endpoint.

## Decision

Synchronization is an **application** module (`domains/synchronization`): planner + durable jobs + worker that calls the Matrix adapter. Jobs are in-process in the desktop app, persisted in PostgreSQL. Not a vendor sync call, not Redis, not a separate worker service.

Reconcile and retry explicitly so an offline door does not block the administrator’s edits.

## Consequences

- Implement Users/Devices/Credentials (data) before a full sync worker; a thin adapter ping during Devices is allowed.
- Jobs must be idempotent where device commands may be retried.
- See [synchronization.md](../../synchronization.md).
