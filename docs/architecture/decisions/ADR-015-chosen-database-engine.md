# ADR-015: One chosen local database engine

- **Status:** Accepted
- **Date:** 2026-10-25
- **Supersedes:** the “PostgreSQL only” rule in ADR-002 and the “one PostgreSQL database” line in ADR-001

## Context

Operators install Vsmart Sync on a workstation that may already run PostgreSQL, MySQL, or Microsoft SQL Server. The product still has one desktop process and one system of record. It does not run three databases at once, and it does not use Docker.

ADR-002 forbids a second engine until a new ADR exists. The schema and repositories were written for PostgreSQL (`$1` placeholders, `RETURNING`, `ON CONFLICT`, partial indexes, `UUID`, `TIMESTAMPTZ`, `BYTEA`).

## Decision

At first run the administrator accepts the privacy notice, then chooses **one** local engine:

- PostgreSQL on `127.0.0.1:5432`
- MySQL on `127.0.0.1:3306`
- Microsoft SQL Server on `127.0.0.1:1433`

If that server accepts a local administrator connection, the app creates the `vsmart_sync` database and login and applies the schema. The administrator password is used once and is not stored. The application login password is stored only in `~/.vsmart-sync/database.json` (mode `0600` on Unix) and is never sent to the UI or written to logs.

PostgreSQL keeps the existing versioned files in `migrations/`. MySQL and SQL Server use one consolidated schema each, applied by the app, because those engines cannot run the PostgreSQL migration files.

SQL stays in `src-tauri/src/database/`. React still reaches the database only through Tauri commands.

## Consequences

- A workstation uses one engine. Switching engines does not migrate rows from the previous engine.
- Screens and repositories still execute PostgreSQL SQL. Choosing MySQL or SQL Server creates the database and tables. Those screens do not read or write that database until each repository statement has a MySQL and SQL Server form.
- Do not add a second ORM. SQLx remains the client for PostgreSQL and MySQL. SQL Server setup uses Tiberius, because SQLx 0.8 does not ship a SQL Server feature.
- Default `cargo test` stays offline.
