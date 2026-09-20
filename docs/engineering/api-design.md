# API design

There are **two** APIs. Do not mix them.

## 1. UI API — Tauri commands

React never uses REST against this app. The contract is `invoke("command_name", payload)`.

Rules:

- Keep the command surface **small**. Commands are a façade: validate input, call a use case, map a **safe** result.
- Commands must not contain SQL, CGI, or license crypto.
- Explicit request/response types (serde). Do not send database row types or Matrix DTOs to the UI.
- Never return password hashes, device passwords, templates, tokens, or connection strings.
- Errors: stable codes or short safe messages. Not `Display` of `sqlx::Error`.
- Every new command documents: auth requirement, validation, what is stored, audit need.
- Breaking command payloads requires searching all `src/services` callers and tests.

Existing commands (foundation + users + devices + credentials): `get_app_info`, `get_database_status`, `connect_database`, `create_user`, `list_users`, `update_user` (name only), `deactivate_user`, `create_device`, `list_devices`, `update_device` (metadata only), `set_device_password`, `test_device_connection`, `create_credential`, `list_credentials`, `get_credential`, `update_credential` (optional value replace), `set_credential_status`. Device and credential responses never include plaintext secrets or ciphertext.

## 2. Device API — Matrix COSEC CGI

Owned only by `src-tauri/src/matrix/`.

Verified shape (COSEC Devices API User Guide v28):

```text
http://<deviceIP:deviceport>/device.cgi/<request-type>?<argument>=<value>
```

- Default device port in that guide is **80**. Auth is HTTP basic auth **to the device**.
- Do not invent request-types. If the guide or model docs do not establish behavior, mark **REQUIRES MATRIX DOCUMENTATION VERIFICATION**.
- Domain code depends on an adapter trait (`upsert_user`, `start_enroll`, …), not on query strings.
- Timeouts, mapping of vendor error text/XML, and retries (only when idempotent) live in the adapter/client.

Details: [matrix-integration.md](../matrix-integration.md).

## Compatibility

Before renaming a command or field: find all invocations, tests, and UI types. Prefer additive changes.

There is no public HTTP JSON API for administrators in the MVP. Do not add Axum/Actix “for convenience.”
