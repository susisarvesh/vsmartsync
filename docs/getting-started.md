# Getting started

**Status:** These steps start the **IMPLEMENTED** foundation. They do not enable planned domain features.

This is a **native desktop application**. Do not open `http://127.0.0.1:1420` in a browser.

## Setup (any OS)

From the project root:

```bash
npm install && npm start
```

That single command:

1. Installs npm packages
2. Creates `.env` from `.env.example` if `.env` is missing
3. Creates the local `vsmart_sync` PostgreSQL role and database when `psql` can reach a local server
4. Opens the native **Vsmart Sync** window

If PostgreSQL is running, the app connects automatically. If it is not, the window still opens and shows install steps for this OS, with a **Try again** button.

No Docker. No `cp` / `Copy-Item`. No separate `createuser` step on the happy path.

PostgreSQL must already be installed and running on `127.0.0.1:5432`.

After a successful start you should see the foundation screen calling Rust `get_app_info`. The first compile can take a minute.

## Prerequisites

| Tool | Why |
|---|---|
| Node.js 20+ | `npm install && npm start` |
| Rust stable (edition 2021) | Tauri / `src-tauri` |
| PostgreSQL 16+ | Local database on port **5432** |
| Tauri OS libraries | [Prerequisites](https://v2.tauri.app/start/prerequisites/) |

- **macOS:** `xcode-select --install`; `brew install postgresql@16` and `brew services start postgresql@16`
- **Windows:** MSVC Build Tools, [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/), [PostgreSQL](https://www.postgresql.org/download/windows/)
- **Linux (Debian/Ubuntu):** WebKitGTK 4.1 packages from the Tauri page, plus `postgresql`

`.env` uses `postgres://<username>:<password>@127.0.0.1:5432/<database>`. Defaults in `.env.example` are user/database `vsmart_sync` on port **5432**.

## Verify / build

```bash
cargo fmt --all --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --all-targets --manifest-path src-tauri/Cargo.toml -- -D warnings
npm run build
```

Live Postgres: `cargo test --manifest-path src-tauri/Cargo.toml -- --ignored --nocapture`

Packaged app: `npm run tauri build`

Frontend-only (no Rust window): `npm run dev`

## Common problems

| Symptom | What to do |
|---|---|
| `connection refused` on 5432 | Start local PostgreSQL (not Docker) |
| Role or database missing | Run `npm start` again so `scripts/setup.mjs` can create them, or check `psql` is on `PATH` |
| Missing `.env` | `npm start` creates it; do not commit `.env` |
| Tauri / webview compile errors | Install OS packages from the Tauri prerequisites page |
| Browser tab instead of a window | Close the browser; use the Vsmart Sync desktop window |

## What you should see today

A foundation window that loads application metadata from Rust. User, device, enrollment, and Matrix features are **PLANNED**.
