# Engineering guidelines

**Status:** Binding for all implementation work. Product features themselves may still be **PLANNED**.

This folder is the coding constitution. Architecture (what the system is) lives in `docs/architecture/`. Why a choice was made lives in `docs/architecture/decisions/`. How Cursor must work is in `/AGENTS.md`.

## Priorities (in order)

1. Correctness
2. Security
3. Maintainability
4. Reliability
5. Testability
6. Observability
7. Performance
8. Clear architecture
9. Compatibility of existing commands and schema
10. Small, reviewable changes

Do not optimize for the smallest patch if it breaks a boundary. Do not add layers to appear “enterprise.”

## This codebase

Vsmart Sync is a **local workstation** product:

- One desktop process (Tauri)
- One PostgreSQL database on the same machine
- Direct HTTP to Matrix COSEC devices
- USB license check **PLANNED**, independent of Matrix connectivity

It is **not** Matrix COSEC Web, Centra, or a multi-tenant cloud. Do not add Redis, Kafka, Kubernetes, or a public REST API for the UI unless an ADR says so.

## Before you write code

1. Name the owning module under `src-tauri/src/domains/` (or `matrix/`, `database/`, `commands.rs`, or `src/`).
2. Read the module notes in [module-boundaries.md](../architecture/module-boundaries.md).
3. Read the matching file in this folder (security, database, testing, …).
4. Search for an existing command, type, or helper. Reuse it.
5. If Matrix HTTP is involved, read [matrix-integration.md](../matrix-integration.md). Do not invent CGI.

## After you write code

Run the [code-review-checklist.md](code-review-checklist.md). Default verification:

```bash
cargo fmt --all --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --all-targets --manifest-path src-tauri/Cargo.toml -- -D warnings
npm run build
```

## Index

| Document | Use when |
|---|---|
| [coding-standards.md](coding-standards.md) | Rust / TypeScript style |
| [design-patterns.md](design-patterns.md) | Repositories, adapters, use cases |
| [security.md](security.md) | Auth, secrets, devices, biometrics |
| [database.md](database.md) | SQLx, migrations, constraints |
| [api-design.md](api-design.md) | Tauri commands and Matrix CGI |
| [error-handling.md](error-handling.md) | Errors to UI vs logs |
| [testing.md](testing.md) | What to test and how |
| [observability.md](observability.md) | Logs, audit, tracing |
| [performance.md](performance.md) | Queries, jobs, caching |
| [ui-design.md](ui-design.md) | Enterprise desktop UI (light theme, shadcn) |
| [code-review-checklist.md](code-review-checklist.md) | Final gate |
