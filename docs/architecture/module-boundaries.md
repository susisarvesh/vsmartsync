# Modules

**Status:** Module **folders exist** under `src-tauri/src/`. Domain services live in `src-tauri/src/domains/`. **IMPLEMENTED:** `common` (app info, device password vault), `database` (config/pool/migrate/users+devices repositories), `commands` (`get_app_info`, database status, users, devices), `domains/users`, `domains/devices`, thin `matrix` connectivity probe. Other domains are **PLANNED**.

Each domain is a module in **one** Rust crate. That is a modular monolith, not microservices.

Domain services live under `src-tauri/src/domains/` (`crate::domains::users`, and so on). Matrix HTTP stays in `src-tauri/src/matrix/`. PostgreSQL stays in `src-tauri/src/database/`.

## Cross-cutting rules

- Domain modules **must not** build Matrix `device.cgi` URLs.
- Domain modules persist through `database` repositories, not ad-hoc SQL in the UI layer.
- React **must not** import these modules; it only calls Tauri commands.
- Commands stay a thin façade over services.

---

## `auth`

| | |
|---|---|
| **Responsibility** | Authenticate **desktop administrators** (application login). |
| **Owns** | Session/auth policy for the workstation app. |
| **Must not own** | Matrix device basic-auth passwords; USB license secrets. |
| **Depends on** | `database`, `common`, later `audit`. |

**PLANNED.** Placeholder only. Distinct from HTTP basic auth to a COSEC device.

---

## `users`

| | |
|---|---|
| **Responsibility** | Application user records (people who get access). |
| **Owns** | Create, list, rename, deactivate (`active` / `inactive`). |
| **Must not own** | HTTP to `/device.cgi/users`; credential templates; Matrix `user-id` mapping. |
| **Depends on** | `database` user repository. |

**IMPLEMENTED** for this slice. Deactivate is a domain operation; the UI cannot set status via update. No Matrix, credentials, or extra unique keys.

---

## `devices`

| | |
|---|---|
| **Responsibility** | Registered COSEC devices and how this app reaches them. |
| **Owns** | Address, port, display name, encrypted device password (Rust-side), `connection_status` / `last_seen_at` as **application reachability** data. |
| **Must not own** | CGI client implementation beyond calling `matrix` abstractions; event ingest; enrollment; sync. |
| **Depends on** | `database` device repository, `common` password vault; reachability via `matrix` adapter. |

Never return device passwords or ciphertext to Tauri/React.

**IMPLEMENTED** for this slice: create, list, update metadata, set password, test connection (`GET /device.cgi/device-basic-config?action=get`). `connection_status` is application reachability only (`unknown` / `online` / `offline`), not physical device health.

---

## `credentials`

| | |
|---|---|
| **Responsibility** | Credential records and provisioning data for users. |
| **Owns** | Types (card, finger, palm, face — as models allow), set/get/delete orchestration. |
| **Must not own** | Live “place finger on reader” sessions (`enrollments`); Matrix HTTP. |
| **Depends on** | `users`, `database`, `matrix` (through interfaces). |

---

## `enrollments`

| | |
|---|---|
| **Responsibility** | Enrollment **sessions** on a chosen device. |
| **Owns** | Start session, correlate device events to a session, session status. |
| **Must not own** | Generic credential storage; treating enrolluser as a template download. |
| **Depends on** | `users`, `devices`, `credentials`, `events` (outcome), `matrix`. |

---

## `synchronization`

| | |
|---|---|
| **Responsibility** | Turn desired PostgreSQL state into per-device jobs and run them. |
| **Owns** | Planner, job state machine, retry/backoff policy, per-device concurrency. |
| **Must not own** | CGI details; UI rendering. |
| **Depends on** | All domain repositories it must push; `matrix` adapter; `audit`. |

There is no Matrix “sync module.” This module **is** the sync system. See [synchronization.md](../synchronization.md).

---

## `events`

| | |
|---|---|
| **Responsibility** | Ingest, recover, deduplicate, and query device events. |
| **Owns** | Sequence/rollover tracking per device, HTTP gap fill, history queries. |
| **Must not own** | Access-control decisions; pretending TCP recovery uses TCP. |
| **Depends on** | `devices`, `database`, `matrix`. |

See [events.md](../events.md).

---

## `audit`

| | |
|---|---|
| **Responsibility** | Administrator and system actions **inside this application**. |
| **Owns** | Audit records (who, what, when UTC). |
| **Must not own** | Device event logs (those are `events`). |
| **Depends on** | `database`, `common`. |

---

## `licensing`

| | |
|---|---|
| **Responsibility** | USB key presence and cryptographic validation. |
| **Owns** | License manager, grace/lock policy (**PLANNED**). |
| **Must not own** | Matrix communication; secrets in React. |
| **Depends on** | OS USB APIs / vendor SDK (**not chosen yet**), `common`. |

**NOT implemented.** See [licensing.md](../licensing.md).

---

## `matrix`

This is the **integration boundary**. All COSEC Devices HTTP (and TCP event listen) stays here.

| Submodule | Role |
|---|---|
| `matrix::adapter` | Application-facing operations: upsert user on device, start enroll, fetch events, send command. |
| `matrix::client` | HTTP (and TCP) transport, basic auth, timeouts, text/XML parsing. |
| `matrix::models` | DTOs for CGI arguments/responses — **not** domain entities. |

| | |
|---|---|
| **Owns** | `/device.cgi/...` knowledge, device auth headers, vendor error codes. |
| **Must not own** | PostgreSQL schema; “should this user be allowed through the door?” product policy. |
| **Depends on** | No domain module should depend on `client` internals. Services depend on **adapter traits**. |

The rest of the application talks to **domain/application abstractions** (for example `DeviceGateway::upsert_user`). Only the adapter implements those by calling Matrix endpoints.

Do not invent undocumented endpoints in this module. Unknown behavior is marked **REQUIRES MATRIX DOCUMENTATION VERIFICATION**.

---

## `database`

| | |
|---|---|
| **Responsibility** | PostgreSQL access. |
| **Owns** | `DatabaseConfig`, pool, `connect_and_migrate`, future models and repositories. |
| **Must not own** | Domain invariants (those stay in services); Matrix types. |
| **Depends on** | SQLx, `migrations/` at repo root. |

**IMPLEMENTED:** config, pool, migrator. **PLANNED:** tables and real repositories (`models` / `repositories` are placeholders).

---

## `common`

| | |
|---|---|
| **Responsibility** | Shared, non-domain helpers. |
| **Owns** | App metadata (`get_app_info` payload), shared error/logging patterns as they appear. |
| **Must not own** | User/device business rules; Matrix client. |
| **Depends on** | crate metadata / serde. |

**IMPLEMENTED:** `AppInfo`.

---

## `commands` (Tauri surface)

Not a domain, but the IPC boundary (`src-tauri/src/commands.rs`).

| | |
|---|---|
| **Responsibility** | Expose a **small** set of commands to React. |
| **Owns** | IPC signatures. |
| **Must not own** | SQL, Matrix HTTP, license crypto. |
| **Depends on** | Domain services (when they exist). |

**IMPLEMENTED:** `get_app_info` only.
