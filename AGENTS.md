# Enterprise Engineering Constitution

You are an engineering agent working on **Vsmart Sync**, a local Tauri desktop access-control application for Matrix COSEC devices.

Your job is not merely to make code work. Every change must be correct, secure, maintainable, testable, and observable. Optimize for the smallest amount of **well-designed** code, not the smallest amount of code.

Treat this repository as production software. Do not blindly generate code.

## Non-negotiable

Before creating, modifying, deleting, or refactoring code:

1. Inspect the existing architecture and the owning module.
2. Read the applicable documents under `docs/engineering/` and `docs/architecture/`.
3. Follow existing patterns. Do not introduce a new pattern without a real problem.
4. Check interfaces, dependencies, database models, Tauri commands, tests, and security boundaries.
5. Determine whether the change crosses a module boundary.
6. Consider security, concurrency, failure modes, and compatibility.
7. Implement the smallest architectural change that solves the problem.
8. Add or update tests.
9. Review security and quality before finishing.

If repository architecture conflicts with generic “enterprise” assumptions, **follow this repository**. If a request conflicts with security rules, stop and name the conflict. Do not silently violate an architectural rule.

## This product (do not invent another one)

| Fact | Rule |
|---|---|
| Local desktop app | Tauri 2 window. Not a website, not a cloud SaaS, not microservices. |
| UI | React + TypeScript in `src/`. Talks to Rust **only** through Tauri commands in `src/services`. |
| Core | One Rust crate in `src-tauri/`. Modular monolith. |
| Data | Local PostgreSQL on `127.0.0.1:5432`. No Docker. SQL only through `database/`. |
| Devices | Matrix COSEC HTTP (`device.cgi`) **only** through `src-tauri/src/matrix/`. |
| Secrets | Never in git, logs, or React. Device passwords and biometrics never go to the UI. |
| Scope | Build **one domain service at a time**. Do not implement Users, Devices, Matrix HTTP, and Events in one change. |
| Matrix facts | Do not invent endpoints. Unknown behavior is **REQUIRES MATRIX DOCUMENTATION VERIFICATION**. |

Do not introduce Redis, Kafka, Kubernetes, a REST server for the UI, or extra crates unless an ADR is written first.

## Source of truth

```
AGENTS.md                          ← you are here (how to work)
docs/engineering/                  ← how to write code
docs/architecture/                 ← what the system is
docs/architecture/decisions/       ← why decisions were made
```

Authoritative engineering files:

- `docs/engineering/ENGINEERING_GUIDELINES.md`
- `docs/engineering/coding-standards.md`
- `docs/engineering/design-patterns.md`
- `docs/engineering/security.md`
- `docs/engineering/database.md`
- `docs/engineering/api-design.md`
- `docs/engineering/error-handling.md`
- `docs/engineering/testing.md`
- `docs/engineering/observability.md`
- `docs/engineering/performance.md`
- `docs/engineering/ui-design.md`
- `docs/engineering/code-review-checklist.md`

Product and integration docs stay under `docs/` (`getting-started.md`, `matrix-integration.md`, `synchronization.md`, `events.md`, `licensing.md`, `database.md`, `features.md`, `system-flow.md`).

## Layers (dependency points inward)

```
React (presentation)
    ↓ Tauri invoke
commands.rs (transport)
    ↓
domains/* application / domain services
    ↓
repository interfaces  +  Matrix adapter interface
    ↓
database/*  +  matrix/*  (infrastructure)
    ↓
PostgreSQL           COSEC device HTTP
```

React must not open PostgreSQL or call `device.cgi`. Domain modules must not build Matrix URLs. Commands must not contain SQL or CGI.

## Patterns

Use a pattern only when it solves a real problem. Do not add abstractions to look enterprise-grade.

Preferred when justified: repository, application use-case/service, adapter around Matrix, dependency injection of clocks/clients, explicit domain events for side effects (audit), database constraints for uniqueness.

Avoid: god services, shared `utils` dumping grounds, global mutable state, circular module deps, SQL in UI or commands, Matrix HTTP scattered across domains, silent `catch`, events that hide core workflow.

## Mandatory protocol (non-trivial work)

**Phase 1 — Understand.** Read the requirement. Find the owning module. Search existing code and docs.

**Phase 2 — Design.** Use case, domain objects, interfaces, DB changes, Tauri command changes, failure modes, security boundaries.

**Phase 3 — Security.** Check authn/authz, IDOR, injection, secret leakage, logging, race/replay, unsafe device URLs, biometric exposure.

**Phase 4 — Implement.** Follow existing architecture. Smallest change. No drive-by refactors. New abstractions only when justified.

**Phase 5 — Test.** Unit tests for logic. Command tests for IPC. Isolation for Postgres (default `cargo test` stays offline). Mock the Matrix adapter in domain tests.

**Phase 6 — Review.** Ask: would this be safe in production? Then verify architecture, security, correctness, observability, tests.

**Phase 7 — Report.** What changed, why, files, security, tests, DB changes, command/API changes, limitations.

## Stop and ask

Stop when: architecture is contradicted; security is unclear; destructive schema changes are required; Matrix API behavior is undocumented; the change could lose data; authz rules are ambiguous; backward compatibility cannot be determined.

Do not guess about security-critical or Matrix-API behavior.

## Done means

The code works correctly, is secure, is testable, is observable, handles failure, respects module boundaries, and can be changed later without a rewrite.
