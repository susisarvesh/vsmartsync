# ADR-002: Local PostgreSQL

- **Status:** Accepted
- **Date:** 2026-09-20

## Context

The application needs a durable system of record for desired access-control state. Docker was rejected for this product.

## Decision

Use **PostgreSQL 16+ installed on the workstation**, host port **5432**. Access via SQLx (`runtime-tokio`, rustls). Schema changes only through versioned files in `migrations/`. The process runs `connect_and_migrate` at startup; the window still opens if Postgres is down (setup UI).

## Consequences

- Operators install Postgres themselves; the UI shows OS-specific steps.
- `.env` / `DATABASE_URL` are local secrets (gitignored).
- Default tests stay offline; live tests are `--ignored`.
- Do not add a second database or an ORM alongside SQLx without a new ADR.
