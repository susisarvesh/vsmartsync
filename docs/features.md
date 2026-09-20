# Features

**Status:** All product features below are **PLANNED** unless marked otherwise. The running application today only shows foundation metadata.

Do not treat this list as a claim that screens or APIs already exist.

## Feature status legend

| Band | Meaning |
|---|---|
| **MVP** | Required for the first usable local product |
| **Post-MVP** | After the first reviewable vertical slices work |
| **Future / model-specific** | Depends on device variant or later product scope |

Matrix behavior always **REQUIRES MATRIX DOCUMENTATION VERIFICATION** for the target door model. The COSEC Devices API guide is general; variants may omit features.

---

## MVP

### 1. User Management

| | |
|---|---|
| **What** | Create, update, deactivate, and delete people who may receive access on doors. |
| **Why** | PostgreSQL is the desired-state source. Devices receive a projection of that state. |
| **UI** | Administrator maintains a user list and user detail form. |
| **Backend** | User service validates input, assigns internal UUIDs, writes via the user repository. |
| **Database** | Persist application users (planned `users` entity). |
| **Matrix** | Push/update/delete via `/device.cgi/users` **when syncing a device** — not from the React form directly. |

Device-side users use alphanumeric `user-id` and numeric `ref-user-id` (COSEC Devices API). Mapping between application UUIDs and those identifiers is **PLANNED** and must be designed with the schema. Exact field limits **REQUIRE MATRIX DOCUMENTATION VERIFICATION** per model.

### 2. Device Management

| | |
|---|---|
| **What** | Register a COSEC device (address, port, name, credentials stored only in Rust), test reachability, show basic identity. |
| **Why** | All later sync, enrollment, and events need a known device record. |
| **UI** | Add/edit device, connection test, status. |
| **Backend** | Device service stores connection data; never returns device passwords to the UI. |
| **Database** | Persist planned `devices` entity. |
| **Matrix** | Optional read of `/device.cgi/device-basic-config` to confirm the device. |

### 3. Basic Credential Management

| | |
|---|---|
| **What** | Associate credential *records* with a user (card identifiers, PINs). Store values encrypted at rest. Biometric/face templates are **out of scope** for this slice. |
| **Why** | Enrollment/sync later provision credentials onto doors from this system of record. |
| **UI** | Credentials page: list/filter, create Card/PIN, replace value, activate/deactivate. Write-only secrets. |
| **Backend** | Credential service; AES-256-GCM via shared OS-keychain vault. |
| **Database** | `credentials` table (**IMPLEMENTED**). |
| **Matrix** | Not called from this slice. Future: `/device.cgi/credential` via Enrollment/Sync. |

Distinguish **credential provisioning** (HTTP set/get of templates or card numbers) from an **enrollment session** (device prompts for a live finger/card/face). See [system-flow.md](system-flow.md).

### 4. Enrollment

| | |
|---|---|
| **What** | Start an enrollment session on a chosen device so the person presents a card, finger, palm, or face at the hardware. |
| **Why** | Many credentials cannot be typed in; the device must capture them. |
| **UI** | Pick user, device, credential type; show in-progress / success / failure from events or polling. Exact UX **PLANNED**. |
| **Backend** | Enrollment service starts a session, records outcome. |
| **Database** | Planned `enrollments` entity. |
| **Matrix** | `/device.cgi/enrolluser?action=enroll`. The API guide states this **only starts enrollment**; retrieving the credential later uses get/set credential. |

Enrollment options and reader group selection are **model-specific** (**REQUIRES MATRIX DOCUMENTATION VERIFICATION**).

### 5. Basic Access Configuration

| | |
|---|---|
| **What** | Apply a small set of door access settings (for example working days / work hours on the device). |
| **Why** | MVP needs more than “user exists”; doors need a basic policy. |
| **UI** | Simple access settings per device or shared profile — exact model **PLANNED**. |
| **Backend** | Access configuration as part of device or a dedicated service; applied through sync. |
| **Database** | Store desired access settings (schema not finalized). |
| **Matrix** | `/device.cgi/access-setting`. Reader modes live under `/device.cgi/reader-config` and are often **post-MVP / model-specific**. |

Do not assume advanced access routes, visitor flows, or cafeteria features in MVP.

### 6. Synchronization

| | |
|---|---|
| **What** | Copy desired PostgreSQL state onto devices via jobs (users, credentials, access settings, deletes). |
| **Why** | There is **no single Matrix “sync” endpoint**. The application owns reconciliation. |
| **UI** | Per-device sync status, last success, failures, retry. |
| **Backend** | Planner + worker; statuses pending / processing / success / failed / retry. |
| **Database** | Planned `sync_jobs`. |
| **Matrix** | Adapter calls the individual CGI APIs required for that job. |

See [synchronization.md](synchronization.md).

### 7. Event History

| | |
|---|---|
| **What** | Persist device events and show them to the administrator. |
| **Why** | Access happened **on the device**; the desktop app is the history store. |
| **UI** | Filterable event list (time, device, user, event id). |
| **Backend** | Event service: ingest, deduplicate, recover gaps. |
| **Database** | Planned `events`. |
| **Matrix** | HTTP `/device.cgi/events?action=getevent` and TCP `/device.cgi/tcp-events?action=getevent`. Missed TCP events are recovered via the **HTTP** event API (verified in the API guide). |

### 8. Device Monitoring

| | |
|---|---|
| **What** | Online / offline / last seen for each registered device. |
| **Why** | Administrators need to know if a door is reachable before sync or enrollment. |
| **UI** | Status badges on the device list. |
| **Backend** | Periodic reachability (method **PLANNED**; may use basic-config get, geteventcount, or TCP liveness). |
| **Database** | Status fields on `devices` or a related table (not finalized). |
| **Matrix** | Lightweight HTTP to the device. Exact health probe **REQUIRES MATRIX DOCUMENTATION VERIFICATION**. |

### 9. Basic Audit Logging

| | |
|---|---|
| **What** | Record who did what in **this application** (create user, start sync, change device). |
| **Why** | Distinct from device events. Device events are hardware history; audit is administrator actions. |
| **UI** | Read-only audit trail. |
| **Backend** | Audit service called by other services. |
| **Database** | Planned `audit_logs`. |
| **Matrix** | None (unless a device command itself is audited as an application action). |

### 10. USB Licensing architecture

| | |
|---|---|
| **What** | Reserve a license manager that validates a USB hardware key in Rust and can lock protected operations. |
| **Why** | Product is licensed per workstation, independent of Matrix. |
| **UI** | License missing / present messaging only. No secrets. |
| **Backend** | License manager in the `licensing` module. |
| **Database** | Optional license metadata — **not designed yet**. |
| **Matrix** | None. |

**Full USB licensing is NOT implemented.** MVP includes the **architecture and module boundary**, not necessarily complete cryptographic enforcement in the first slice. See [licensing.md](licensing.md) and [development.md](development.md) for sequence.

---

## Post-MVP

These are **PLANNED** after the MVP slices are reviewed:

| Feature | Intent | Matrix notes |
|---|---|---|
| Authentication of administrators | Login to the desktop app | Not a Matrix API |
| Richer access control | Groups, levels, routes | Many settings are device- and model-specific |
| Reader / enroll option configuration | `/device.cgi/reader-config`, `/device.cgi/enroll-options` | **REQUIRES MATRIX DOCUMENTATION VERIFICATION** |
| Door commands | lock / unlock / normalize / open | `/device.cgi/command?action=…`; several **not applicable** on some models (e.g. ARGO FACE200T per the API guide) |
| Alarm handling | `/device.cgi/alarm`; commands `clearalarm`, `acknowledgealarm` | Model-specific |
| `denyuser` command | Device command | Extra-info fields required by the API guide |
| Photo / face image handling | `/device.cgi/userphoto`; face credential types | VEGA/NGT/ARGO FACE as documented |
| Special cards, card read/write | `/device.cgi/enrollspcard`, `/device.cgi/card-read-write` | Not applicable on some models |
| Deeper monitoring | Door held, tamper, alarms as events | Event IDs in the API appendix |

## Future / model-specific

Do **not** promise these in MVP. They appear in the COSEC Devices API guide but depend on hardware:

- Palm templates (e.g. PVR)
- Face templates/images (ARGO FACE)
- OSDP / ARC reader groups
- Wiegand, smart-card format, cafeteria APIs
- Temperature / face-mask compulsion
- Cafeteria reset-recharge
- Verify biometric and open door
- Multi-language files, LED/buzzer cadence, FR settings

Each of these **REQUIRES MATRIX DOCUMENTATION VERIFICATION** against the installed model.

## Explicitly out of foundation scope today

**IMPLEMENTED:** none of the MVP product features above.

The UI only verifies React → Tauri → Rust with `get_app_info`.
