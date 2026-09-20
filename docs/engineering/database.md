# Database rules

Planned **entities** (not frozen columns) are in [docs/database.md](../database.md). This file is how you are allowed to change data.

## Engine

- PostgreSQL 16+ **local**, port **5432**, not Docker.
- Access: SQLx in `src-tauri/src/database/` only.
- Migrations: versioned SQL in repository-root `migrations/`. The app runs `connect_and_migrate` at startup.
- Default `cargo test` must not require a live server. Live checks are `cargo test --manifest-path src-tauri/Cargo.toml -- --ignored`.

## Invariants belong in the database

When a rule must always hold, use constraints: `NOT NULL`, `UNIQUE`, `FOREIGN KEY`, `CHECK`, composite uniques (for example device + vendor event seq).

Do not rely only on application `if` checks under concurrency.

## Queries

- Parameterized queries only. Never interpolate user input into SQL strings.
- No unbounded `SELECT` for UI lists. Page or cap results.
- Avoid obvious N+1 (load devices, then one query per device in a loop) without a reason.
- Indexes follow real lookups (device, user, job status, event time). Do not index every column.

## Transactions

If user + credential + sync job must appear together, use one transaction. Do not leave partial desired state.

## Identifiers

Internal primary keys: UUIDs. Vendor fields (`user-id`, `seq-number`) are **not** our primary keys. Mapping is designed per service, not copied 1:1 from Matrix XML.

## Secrets in rows

Device passwords and biometric templates: encryption and column design **before** the first table that stores them. Never commit sample secrets.

## Migrations

- One forward migration per change. Review existing data before `NOT NULL` or unique changes.
- Never hand-edit a live catalog as the way to ship schema.
- Do not invent a full schema in a feature that only asked for one entity.

## Module access

Domains do not query another module’s tables ad hoc. Cross the **repository / service interface**. No circular “users crate reads devices rows directly” once tables exist.
