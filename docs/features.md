# Features

**Status:** All product features below are **PLANNED** unless marked otherwise. The running application today only shows foundation metadata.

Do not treat this list as a claim that screens or APIs already exist.

## Feature status legend

| Band | Meaning |
|---|---|
| **MVP** | Required for the first usable local product |
| **Post-MVP** | After the first reviewable vertical slices work |
| **Future / model-specific** | Depends on device variant or later product scope |

Matrix behavior: architecture targets all guide-listed families; individual features remain model-specific. **Documented support ≠ hardware-verified support.** Always **REQUIRES MATRIX DOCUMENTATION VERIFICATION** / lab verification per path.

---

## MVP

### 1. User Management

| | |
|---|---|
| **What** | Create, view, update username, activate, deactivate, and assign one person to many devices. |
| **Why** | PostgreSQL is the desired-state source. Devices receive a projection of that state. |
| **UI** | Users page: list, view, create, edit username, activate, deactivate, delete, assign to many devices. Delete removes the person locally and from every Matrix device that already has them. Assign to device (sidebar, under Users): select one device, then select users to assign to it. |
| **Backend** | User service validates input, assigns internal UUIDs, writes via the user repository. Status changes only through activate and deactivate. Assignment is a separate `user_devices` service: it checks that the user and device exist, then stores the pair. Adding the person on the hardware is the synchronization step, which then writes `device_users`. |
| **Database** | `users` (`id`, `username`, `status`, `created_at`, `updated_at`). Assignment is `user_devices` (`user_id`, `device_id`), unique per pair. |
| **Matrix** | Not called from this module. Push/update/delete via `/device.cgi/users` happens **when syncing a device**. |

Device-side users use alphanumeric `user-id` (max 15) and numeric `ref-user-id` (max 8 digits). Mapping is **`device_users`** (**IMPLEMENTED**, ADR-013): device-local `VS######` / `10000001…`. Schema for Sync jobs remains **PLANNED**.

### 2. Device Management

| | |
|---|---|
| **What** | Register a COSEC device, store network details and optional MAC, keep software status, test reachability, show last seen. Model stays empty until discovery. |
| **Why** | All later sync, enrollment, and events need a known device record. |
| **UI** | Add/edit/view device, activate/deactivate, connection test. Password is write-only. |
| **Backend** | Device service stores connection data; never returns device passwords to the UI. |
| **Database** | `devices` (`device_name`, host, port, `mac_address`, `device_model`, `status`, `connection_status`, `last_seen_at`). |
| **Matrix** | Reachability probe via `/device.cgi/device-basic-config` (**IMPLEMENTED**). `reader-config` and `enroll-options` are the later discovery step and are not called yet. |

### 3. Basic Credential Management

| | |
|---|---|
| **What** | Associate credential *records* with a user (card identifiers, PINs). Store values encrypted at rest. Biometric/face templates are **out of scope** for this slice. |
| **Why** | Enrollment/sync later provision credentials onto doors from this system of record. |
| **UI** | Credentials page: list/filter, create Card/PIN, replace value, activate/deactivate. Enrollment is a section on this page, not a separate screen. Write-only secrets. |
| **Backend** | Credential service; AES-256-GCM via the shared application master key. |
| **Database** | `credentials` table (**IMPLEMENTED**). |
| **Matrix** | Not called from this slice. Future: `/device.cgi/credential` via Enrollment/Sync. |

Distinguish **credential provisioning** (HTTP set/get of templates or card numbers) from an **enrollment session** (device prompts for a live finger/card/face). See [system-flow.md](system-flow.md).

### 4. Enrollment (desired assignment)

| | |
|---|---|
| **What** | Assign a credential to a device, or enroll on the hardware when the device configuration reports that type. |
| **Why** | Enrollment has to match the reader that will capture the credential, then stay recorded for that user. |
| **UI** | Inside Credentials: Enroll on device, plus local assignment (user + credential + device), list/filter, cancel, retry, revoke. |
| **Backend** | `enroll_on_device` checks device configuration, syncs the user, calls `enrolluser`, and stores an active enrollment on that user. Face enrollment sets `enable-fr=1`. A face-only door is set to Card & Face before a card enrollment so the card prompt can start. Add Enrollment stays local. See ADR-010. |
| **Database** | `enrollments` table (**IMPLEMENTED**). |
| **Matrix** | Before enrollment, read `device-basic-config`, `reader-config`, and `enroll-options`. Then `users?action=set` and `enrolluser?action=enroll` for a type those documents report. Template bytes are not stored. |

Hardware enrollment is `enroll_on_device`. The older Add Enrollment action remains a local assignment and does not call the device.

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
| **What** | Copy desired PostgreSQL state onto devices. This slice adds assigned users on the device. Credentials, deletes, and a durable job queue remain later. |
| **Why** | There is **no single Matrix “sync” endpoint**. The application owns reconciliation. |
| **UI** | Assign to device: Assign and sync, and Sync assigned. The user Devices action also adds that person on the device. |
| **Backend** | For each assigned user: allocate `device_users`, call `set_user`, then `mark_provisioned`. One failure does not stop the other users. |
| **Database** | Uses `user_devices` and `device_users`. Planned `sync_jobs` are not created yet. |
| **Matrix** | `GET /device.cgi/users?action=set` with `user-id`, `ref-user-id`, `user-active`, and `name` when the username fits the guide (15 alphanumeric). |

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
