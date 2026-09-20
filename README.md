# Vsmart Sync

Local desktop access-control application for Matrix hardware.

**Current status: project foundation only.** Domain services, database tables, Matrix API calls, authentication, and USB licensing are not implemented.

## Purpose

Vsmart Sync is an enterprise-grade local desktop application for access control. The UI runs in Tauri/React. Business logic, PostgreSQL access, and Matrix device integration run in Rust.

## Layout

```
src/                         React UI
  pages/                     Screens
  components/                UI pieces
  hooks/
  services/                  Tauri invoke only (never PostgreSQL or Matrix)
  types/
src-tauri/src/               Rust modular monolith
  commands.rs                IPC surface for React
  common/                    Shared helpers
  database/                  PostgreSQL config, pool, future models/repos
  domains/                   Business services (placeholders until built)
    auth, users, devices, credentials, enrollments
    synchronization, events, audit, licensing
  matrix/                    COSEC Devices API boundary
    adapter, client, models
migrations/                  SQLx SQL files
docs/
  engineering/               Binding coding constitution
  architecture/              System design, C4, ADRs
AGENTS.md                    Cursor/agent entry point
scripts/setup.mjs            First-run .env and local PostgreSQL
```

Rules already encoded in this foundation:

- React talks to Rust only through Tauri commands (`src/services`).
- React never talks to PostgreSQL or Matrix devices.
- Matrix credentials must never be sent to the frontend.
- PostgreSQL access goes through the Rust database layer.
- Matrix HTTP access will go only through the Matrix adapter (not implemented yet).
- SQL migrations are version-controlled in `migrations/`.

## Prerequisites

- Node.js 20+
- Rust stable (edition 2021)
- PostgreSQL 16+ installed and running **locally** (not Docker)
- Tauri system dependencies for **macOS, Windows, and Linux**: https://v2.tauri.app/start/prerequisites/

See [docs/getting-started.md](docs/getting-started.md) for OS-specific packages (WebView2 on Windows, WebKitGTK on Linux, Xcode CLT on macOS).

Engineering agents: read [AGENTS.md](AGENTS.md) before changing code.

## Setup (any OS)

```bash
npm install && npm start
```

That one line installs dependencies, creates `.env` if needed, prepares the local PostgreSQL database when `psql` is available, and opens the native **Vsmart Sync** window. PostgreSQL must already be installed and running on `127.0.0.1:5432`. Do not use Docker. Do not open `http://127.0.0.1:1420` in a browser.

## Packaged executable (this computer)

```bash
npm run package
```

That builds a native installer for **the OS you run it on** (macOS `.app`/`.dmg`, Windows `.msi`/`.exe`, Linux `.deb`/AppImage). Recipients do not need Node or Rust. They still need local PostgreSQL on port 5432. Run the command once per OS you want to ship; one Mac cannot produce a Windows or Linux installer.

## Current implementation status

Implemented:

- Tauri 2 + React + TypeScript + Vite scaffold
- Rust modular-monolith module layout
- SQLx PostgreSQL pool, configuration loading, and migration runner
- Local PostgreSQL (no Docker)
- One test command: `get_app_info`

Not implemented:

- Users, devices, credentials, enrollments, synchronization, events, audit
- Authentication
- USB licensing
- Matrix HTTP client, adapter, or API models
- Application database tables
- Domain repositories
