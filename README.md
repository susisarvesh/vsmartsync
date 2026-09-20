# Matrixcosec

Local desktop access-control application for Matrix hardware.

**Current status: project foundation only.** Domain services, database tables, Matrix API calls, authentication, and USB licensing are not implemented.

## Purpose

Matrixcosec is an enterprise-grade local desktop application for access control. The UI runs in Tauri/React. Business logic, PostgreSQL access, and Matrix device integration run in Rust.

## Architecture

```
Tauri
  ├── React + TypeScript (Vite)
  └── Rust modular monolith (src-tauri)
        ├── auth, users, devices, credentials, enrollments
        ├── synchronization, events, audit, licensing
        ├── matrix (client / adapter / models)
        ├── database (config, pool, models, repositories)
        └── common
```

Rules already encoded in this foundation:

- React talks to Rust only through Tauri commands (`src/services`).
- React never talks to PostgreSQL or Matrix devices.
- Matrix credentials must never be sent to the frontend.
- PostgreSQL access goes through the Rust database layer.
- Matrix HTTP access will go only through the Matrix adapter (not implemented yet).
- SQL migrations are version-controlled in `migrations/`.

This repository uses the standard Tauri 2 layout (`src/` frontend, `src-tauri/` backend) rather than nesting both under a single `src/` folder.

## Prerequisites

- Node.js 20+
- Rust stable (edition 2021)
- Docker and Docker Compose (local PostgreSQL)
- Tauri system dependencies for your OS: https://v2.tauri.app/start/prerequisites/

## Environment configuration

1. Copy `.env.example` to `.env`.
2. Change the placeholder password before any non-local use.
3. Keep `DATABASE_URL` in sync with the `POSTGRES_*` variables.

`.env` is gitignored. The Rust process loads `.env` from the project root or from `src-tauri/` (the usual `tauri dev` working directory).

## Start PostgreSQL

```bash
docker compose up -d
docker compose ps
```

PostgreSQL is published on host port **5433** by default (`127.0.0.1:5433` → container `5432`) so it does not collide with a local PostgreSQL on 5432. Override `POSTGRES_PORT` in `.env` if needed.

Stop it with `docker compose down`. Data is stored in the named volume `matrixcosec_postgres_data`.

## Run the Tauri application

```bash
npm install
cp .env.example .env
docker compose up -d
npm run tauri dev
```

The window should show application metadata from the Rust command `get_app_info()`.

Frontend-only (no Rust window):

```bash
npm run dev
```

That Vite preview cannot invoke Tauri commands.

## Current implementation status

Implemented:

- Tauri 2 + React + TypeScript + Vite scaffold
- Rust modular-monolith module layout
- SQLx PostgreSQL pool, configuration loading, and migration runner
- Docker Compose PostgreSQL
- One test command: `get_app_info`

Not implemented:

- Users, devices, credentials, enrollments, synchronization, events, audit
- Authentication
- USB licensing
- Matrix HTTP client, adapter, or API models
- Application database tables
- Domain repositories
