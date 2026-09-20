# Vsmart Sync — Documentation

**Project name:** Vsmart Sync

This folder describes the **planned** Vsmart Sync desktop application and the **foundation that already exists**. It is the starting point for a developer joining the project.

Status labels used throughout:

| Label | Meaning |
|---|---|
| **IMPLEMENTED** | Present in the repository today |
| **PLANNED** | Intended for the MVP or later; not built yet |
| **REQUIRES MATRIX DOCUMENTATION VERIFICATION** | Must be confirmed against a specific Matrix device model and the COSEC Devices API guide |

Do not treat planned architecture as existing functionality.

## Project purpose

Vsmart Sync is a **local desktop access-control application** for sites that use Matrix COSEC door devices. An administrator runs it on a workstation. The application stores the desired access-control state in a local PostgreSQL database, talks to Matrix devices through the official COSEC Devices HTTP API, and keeps events, sync status, and audit history on the workstation.

It is **not** Matrix COSEC Web, COSEC Centra, or a cloud service. Device communication is intended to go **directly to COSEC devices** using the Devices API (no COSEC server required for that path).

## What the application is

- A **Tauri 2** native desktop application (not a website)
- A **React + TypeScript** administrator UI
- A **Rust modular monolith** for business logic, PostgreSQL, licensing, and Matrix integration
- A **local PostgreSQL** database as the system of record for desired state

React never talks to PostgreSQL or Matrix devices. Matrix device credentials never go to the frontend.

## Who it is intended for

| Audience | Role |
|---|---|
| Site administrators | Manage users, devices, credentials, enrollment, and review events |
| Integrators / installers | Register devices, verify connectivity, enroll credentials on hardware |
| Application developers | Implement one domain service at a time against this architecture |

## Technology stack

| Layer | Technology | Status |
|---|---|---|
| Desktop shell | Tauri 2 | **IMPLEMENTED** (foundation) |
| UI | React + TypeScript + Vite | **IMPLEMENTED** (foundation screen only) |
| Application core | Rust (edition 2021), modular monolith | **IMPLEMENTED** (module placeholders) |
| Database | PostgreSQL 16+ installed locally | **IMPLEMENTED** (no application tables yet) |
| SQL access | SQLx + versioned migrations | **IMPLEMENTED** (empty `migrations/` folder) |
| Device integration | Matrix COSEC Devices HTTP API | **PLANNED** (adapter stubs only) |
| Licensing | USB hardware key, validated in Rust | **PLANNED** |

## Current project status

**Foundation only.** The repository can start a native window, connect to PostgreSQL, run the SQLx migrator, and invoke one test command (`get_app_info`). Domain services, tables, Matrix HTTP calls, authentication, and USB licensing are **not** implemented.

### IMPLEMENTED NOW

- Tauri 2 foundation
- React + TypeScript
- Rust backend foundation (modular layout under `src-tauri/src/`, domains in `domains/`)
- PostgreSQL foundation (config, pool, migration runner)
- SQLx migration infrastructure (`migrations/` at the repository root)
- Local PostgreSQL development database (host port **5432**)
- Basic Tauri/React communication (`get_app_info`)

### PLANNED

- User Management
- Device Management
- Credential Management
- Enrollment
- Access Control
- Synchronization
- Events
- Device Monitoring
- Audit
- USB Licensing
- Matrix Integration (adapter, client, device models)

## MVP status

The **MVP is planned, not delivered**. The intended MVP is a working local desktop product that can:

1. Manage users in PostgreSQL
2. Register and talk to Matrix COSEC devices
3. Manage basic credentials
4. Run enrollment on a device
5. Apply basic access configuration
6. Synchronize desired state to devices
7. Collect and show event history
8. Show device online/offline status
9. Record basic audit logs
10. Reserve a USB licensing architecture (full enforcement may land later in the sequence)

## Desktop platforms

The foundation is a **Tauri 2** app for **macOS, Windows, and Linux**. GitHub Actions (`ci`) runs format, Clippy, tests, and the frontend build on all three.

Packaged output depends on the OS you build on (`npm run tauri build`):

- macOS: `.app` / `.dmg`
- Windows: `.msi` / NSIS `.exe` (WebView2 bootstrapper included)
- Linux: `.deb` / AppImage

See [getting-started.md](getting-started.md) for OS prerequisites.

## Documentation index

| Document | Contents |
|---|---|
| [getting-started.md](getting-started.md) | Fresh-clone setup, commands, common problems |
| [architecture.md](architecture.md) | Modular monolith, layers, and boundaries |
| [features.md](features.md) | Planned product features (MVP / post-MVP / future) |
| [system-flow.md](system-flow.md) | End-to-end flows and Mermaid diagrams |
| [modules.md](modules.md) | Planned Rust modules and ownership |
| [database.md](database.md) | Planned entities and data rules (no final schema) |
| [matrix-integration.md](matrix-integration.md) | Matrix adapter vs Matrix API (verified endpoints only) |
| [synchronization.md](synchronization.md) | Desired-state sync jobs (application-owned) |
| [events.md](events.md) | TCP events, HTTP recovery, persistence |
| [licensing.md](licensing.md) | Planned USB license architecture |
| [development.md](development.md) | Standards, verification commands, build sequence |

## Source of Matrix API facts

Matrix HTTP behavior in these docs is taken from **COSEC Devices API User Guide, Version 28 (30 October 2025)**. That guide is general documentation for multiple product variants. A given door model **may not support every API or parameter**.

Where behavior is model-specific or not established in that guide, this documentation says **REQUIRES MATRIX DOCUMENTATION VERIFICATION**.
