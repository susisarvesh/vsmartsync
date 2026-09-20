# ADR-001: Modular monolith

- **Status:** Accepted
- **Date:** 2026-09-20

## Context

Vsmart Sync must manage users, devices, credentials, enrollment, sync, events, audit, and licensing on a workstation, plus HTTP to Matrix doors.

## Decision

Ship **one Tauri application** and **one Rust crate** (`src-tauri`). Domains are modules (`src-tauri/src/domains/`), not microservices. One PostgreSQL database. No Redis, Kafka, or Kubernetes in the MVP.

## Consequences

- Implement and review **one domain at a time**.
- Tests run in-process.
- Cross-domain access goes through module interfaces, not another HTTP service.
- Introducing a new crate, queue product, or microservice requires a new ADR first.
