# ARGO FACE Digest authentication — implementation reference

**Status:** The Digest handshake in this note is implemented in the Matrix HTTP client (ADR-011). The rest of this file remains a reference for later endpoints. Do not invent CGI paths from it.

**Lab device this reference is written against:**

| Field | Value |
|---|---|
| Model | Matrix COSEC ARGO FACE |
| Firmware | V01R25.02 |
| Lab address | `192.168.0.11:80` (test fixture only; never hardcode in source) |
| CGI root | `/device.cgi/...` |

**Guide vs device:** COSEC Devices API User Guide v28 says the device requests **HTTP Basic Authentication**. On this ARGO FACE unit the observed challenge is **HTTP Digest** (`algorithm=MD5`, `qop=auth`). That observation conflicts with the guide, with [ADR-003](architecture/decisions/ADR-003-matrix-integration.md), with [ADR-011](architecture/decisions/ADR-011-matrix-client-foundation.md), and with the client in `src-tauri/src/matrix/client/` (`reqwest` `basic_auth`).

The client now answers a Digest challenge with Digest and a Basic challenge with Basic (ADR-011). Unknown endpoints stay **REQUIRES MATRIX DOCUMENTATION VERIFICATION**. Do not invent CGI paths.

## What already exists (integrate; do not duplicate)

| Concern | Existing module | Rule for this work |
|---|---|---|
| HTTP client | `src-tauri/src/matrix/client/` — `MatrixHttpClient` | Extend this client. Do not add a second Matrix client. |
| Adapter | `src-tauri/src/matrix/adapter/` | Application-facing ops stay here. CGI syntax stays out of domains and React. |
| Probe | `GET /device.cgi/device-basic-config?action=get` | Already implemented with Basic Auth and `Response-Code=0`. |
| Saved-device test | `devices::test_device_connection` → Tauri `test_device_connection(id)` | Decrypts the vault password in Rust, probes, updates `connection_status` / `last_seen_at`. Keep this path for saved devices. |
| Device record | `domains/devices` `Device` | `id`, `name`, `host`, `port`, `username`, `connection_status`, `last_seen_at`, `created_at`, `updated_at`. No plaintext password on the UI model. |
| Password at rest | [ADR-009](architecture/decisions/ADR-009-device-password-at-rest.md) | AES-256-GCM ciphertext in PostgreSQL. Master key is the application file `~/.vsmart-sync/master.key`. Do not store plaintext passwords in PostgreSQL. Do not use the OS credential store. Do not invent a second vault. |
| Users / PIN on device | Adapter `set_user`, `set_pin` | Already on Basic Auth. A Digest change must cover these calls too, in a later slice, not by copying URL builders. |
| Crates | `reqwest` 0.12 (rustls, no default features), `thiserror` 2, `tracing` | `tokio` 1 is a dev-dependency today (the Tauri runtime supplies it in the app). `digest_auth` is **not** in `Cargo.toml`. Add it only in the implementation slice, at a version compatible with reqwest 0.12. Do not upgrade reqwest or tokio to make Digest fit. |

React talks to Rust only through Tauri commands in `src/services`. There is no `src/services/matrix.ts` yet. React must not call `http://<device>/device.cgi`, must not build a Digest header, and must not keep the device password except in the write-only connection form.

## Conflicts to resolve before coding

1. **Basic vs Digest.** Implemented client sends `Authorization: Basic`. This device answered `401` with `WWW-Authenticate: Digest`. An ADR must say whether ARGO FACE uses Digest only, whether other families stay on Basic, or whether the client tries Digest after a Digest challenge and keeps Basic only where a device still demands it. Do not send `Authorization: Basic` to a device that challenged with Digest. Do not use browser cookies.
2. **Client retries.** ADR-011 forbids retries inside the client. This reference allows a bounded Digest handshake (below) and still forbids Sync-style retries. That exception needs the ADR update.
3. **Success rule.** ADR-011: HTTP 2xx **and** body `Response-Code=0`. The XML sample below has no `Response-Code` element. Keep the `Response-Code` rule for the text body until a live `format=xml` body is captured. Do not drop it because this sample omits it.
4. **Timeouts.** Current client: 5s connect and 5s request. This reference: 5s connect, 30s request, no redirects (redirect policy already `none`).
5. **Where the password lives.** Current client takes username/password per call and does not store them. This reference allows them on the client for the life of one connection. Either way: no `Debug` of the password, no logs, no Tauri return value.
6. **Probe query.** Current probe is `action=get` with no `format`. This reference’s first screen asks for `action=get&format=xml`.

## Observed authentication

Do not implement HTTP Basic Authentication for this ARGO FACE unit.

The generic guide mentions Basic Authentication. The challenge observed from firmware V01R25.02 is Digest. Conceptually:

```http
HTTP/1.1 401 Unauthorized
WWW-Authenticate: Digest
  realm="Authenticate Yourself",
  nonce="...",
  algorithm=MD5,
  opaque="...",
  qop=auth
```

Challenge fields the device sends: `realm`, `nonce`, `opaque`, `algorithm`, `qop`. Optionally `stale=true` on a later 401.

These belong on the **client** `Authorization` header, not on the server challenge: `username`, `uri`, `response`, `nc`, `cnonce`. The Digest calculation uses the request URI **including the query string**, for example:

```text
/device.cgi/device-basic-config?action=get&format=xml
```

Required: `algorithm=MD5`, `qop=auth`. Parse the challenge with the `digest_auth` crate. Do not hand-roll MD5 Digest.

## Target call path

```text
React
  → Tauri invoke
  → Tauri command
  → DeviceService (existing devices domain)
  → MatrixAdapter
  → Matrix HTTP client
  → reqwest
  → HTTP Digest (MD5, qop=auth)
  → ARGO FACE /device.cgi
```

Do not expose the HTTP client to JavaScript. Username and password stay in Rust. The password is never returned to React, never logged, and never copied into an API response.

## Client behavior

Extend `MatrixHttpClient`. Do not add `src/matrix/` at the repository root. Suggested shape once the ADR allows it:

- `reqwest::Client` (connect timeout 5s, request timeout 30s, redirects off)
- host, port, username
- password stored so `Debug` does not print it

`new(host, port, username, password)` validates host, port, username, and password before any socket open. Validation stays consistent with `domains/devices` (`MAX_HOST_LENGTH`, port range, non-empty username/password).

Generic request, path not baked into the auth code:

```text
request(method, path, query, body)
```

1. Build `http://{host}:{port}{path}` with the query string encoded.
2. Send **without** `Authorization` when no Digest session exists.
3. HTTP 2xx: return status and body (then apply the existing `Response-Code` rule for text bodies).
4. Not 401: map to a typed error (timeout, unreachable, 404, 400, other HTTP).
5. 401: read `WWW-Authenticate`. If it is not a usable Digest challenge, return `InvalidDigestChallenge`. Do not fall back to Basic on this device.
6. Build the Digest context with username, password, request URI (path + query), and method.
7. Send the same request once with `Authorization: Digest ...`.
8. 2xx: return it.
9. 401 with `stale=true`: refresh the challenge and retry **once**.
10. 401 after that: `AuthenticationFailed`. Stop.

Maximum authentication attempts: **2** retries after the unauthenticated request (one Digest response, one stale refresh). No loop.

```text
request
  → 401
  → Digest
  → request
  → 401 stale=true
  → refresh challenge
  → request
  → failure
```

`DigestAuthState` may cache `realm`, `nonce`, `opaque`, `qop`, `algorithm`, `nonce_count`, and `cnonce` when the device accepts a reused challenge. A fresh 401 or `stale=true` refreshes that state. Do not reuse a challenge across devices.

Convenience methods must call the same `request` path:

```text
get("/device.cgi/users?action=get&format=xml")
get("/device.cgi/credential?action=get&format=xml&user-id=123")
get("/device.cgi/events?action=getevent&format=xml")
```

Paths above that are not already implemented stay behind the slice order below. Several are verified in [matrix-integration.md](matrix-integration.md). Others still need the guide. Do not invent names.

## Logging

Allowed: method, device IP, API path, HTTP status, elapsed time.

```text
Matrix API GET /device.cgi/device-basic-config -> 200
```

Never log: password, `Authorization`, nonce combined with `response`, `cnonce`, credential values, biometric templates, or raw query strings that contain secrets (PIN, card data).

## Errors

Extend `MatrixClientError`. Do not replace it with a parallel enum. Map to safe command strings. React must be able to tell these apart:

| Situation | Safe message |
|---|---|
| Unreachable / timeout | Unable to connect to Matrix device. |
| 401 after Digest | Matrix authentication failed. |
| 404 | Matrix API endpoint was not found. |
| 400 | Matrix rejected the request parameters. |
| 2xx and Matrix success | Request successful. |
| 2xx but Matrix API error | Existing `ApiError { code }` / `MATRIX_API_ERROR:{code}` |

Suggested variants, mapped onto the existing names where they already exist: `InvalidUrl` / `InvalidTarget`, `ConnectionFailed` / `Unreachable`, `Timeout`, `AuthenticationFailed` / `AuthFailed`, `InvalidDigestChallenge`, `HttpError`, `InvalidResponse` / `BadResponse`, `UnsupportedApi`, `DeviceOffline`, `ParseError`, `RequestFailed`.

No Digest nonce, opaque, or `Authorization` value in the Tauri error string.

## First implementation slice (only this, after the ADR)

1. Digest handshake inside the existing Matrix HTTP client.
2. Tauri connection test that runs one documented GET.
3. Parse device basic configuration.
4. React test screen.
5. Unit tests.
6. One ignored live-device test.

**Request:**

```text
GET /device.cgi/device-basic-config?action=get&format=xml
```

```text
http://192.168.0.11:80/device.cgi/device-basic-config?action=get&format=xml
```

Use the host and port the operator entered. The lab IP is an example, not a constant.

**Sample body** (name is not stable; do not assert a specific device name in unit tests):

```xml
<COSEC_API>
    <app>1</app>
    <name>Typredo10 Out</name>
    <asc-code>0</asc-code>
    <generate-invalid-user-events>0</generate-invalid-user-events>
    <generate-exit-switch-events>0</generate-exit-switch-events>
    <manual-door-mode-selection>0</manual-door-mode-selection>
    <mifare-custom-key-enable>0</mifare-custom-key-enable>
    <hid-iclass-custom-key-enable>0</hid-iclass-custom-key-enable>
    <card-custom-key-auto-update>0</card-custom-key-auto-update>
    <max-faces>9</max-faces>
</COSEC_API>
```

Parse at least `name` and `max-faces`.

**Command:** prefer extending the devices probe over a second public client. If the UI must test **before** a row is saved, a command may accept `host`, `port`, `username`, and `password` from the form. It still must not return the password, `Authorization`, nonce, or other Digest internals.

Result fields for that unsaved test (camelCase at the Tauri boundary, matching `Device`):

| Field | Success | Auth failure |
|---|---|---|
| `success` | `true` | `false` |
| `statusCode` | `200` | `401` |
| `deviceName` | parsed `name` | null |
| `maxFaces` | parsed integer | null |
| `rawResponse` | XML body | null |
| `error` | null | `Matrix authentication failed` |

`rawResponse` is the device XML for this config GET only. Do not attach it to auth failures.

**UI:** operator enters IP, port, username, and password, then presses Test Device. Show connection, HTTP status, device name, maximum faces, and the XML. Password field is write-only. Do not put the password in React state after the command returns. Saved devices keep using `test_device_connection(id)` and ADR-009.

**Screen copy on success:**

- Connection: Connected
- HTTP status: 200
- Device: value of `<name>` (example from the lab unit: Typredo10 Out)
- Maximum faces: value of `<max-faces>` (example: 9)

## Tests (first slice)

Unit tests, no live device and no password in the repo:

1. URL construction
2. Query encoding (URI used for Digest includes the query)
3. Digest challenge parsing
4. Authorization header generation (`MD5`, `qop=auth`)
5. 401 then Digest then success
6. `stale=true` refresh, one extra attempt, then failure
7. Successful authenticated request
8. Authentication failure (second 401, not stale)
9. Timeout
10. Device unreachable
11. Malformed body

Use wiremock (already a dev-dependency). Do not log or snapshot `Authorization` values in assertions that print secrets; compare the header in the mock and assert the test password never appears in error strings.

Live test, ignored by default, credentials from the environment only:

```rust
#[ignore]
#[tokio::test]
async fn test_real_argo_face_connection() {
    // MATRIX_DEVICE_HOST, MATRIX_DEVICE_PORT, MATRIX_DEVICE_USERNAME, MATRIX_DEVICE_PASSWORD
}
```

`cargo test` must pass with this test skipped. Do not claim the lab device works until this ignored test has been run and succeeded.

## Later slices (do not build with the first slice)

Order: Users, then Credentials, then Enrollment, then Events, then Commands. Each slice uses the same Digest `request` path. Endpoint strings stay in the Matrix module so a wrong path is fixed without touching auth.

Documented paths already listed in [matrix-integration.md](matrix-integration.md) may be used. Anything not in that file or the guide is **REQUIRES MATRIX DOCUMENTATION VERIFICATION**.

### Device configuration

Centralize path constants. Guide-backed examples already in this repo’s Matrix doc include `device-basic-config`, `access-setting`, `reader-config`, `enroll-options`, `alarm`. Further configuration areas (function keys, finger/palm parameters, date/time, door features, timers, special functions, card formats, display, LED/buzzer, custom message) are added only when the guide gives the exact request-type. Do not guess the CGI name.

### Users

Adapter methods: `get_users`, `get_user`, `create_user`, `update_user`, `delete_user`. Verified path: `/device.cgi/users`. Create already requires `user-id` and `ref-user-id` ([ADR-012](architecture/decisions/ADR-012-matrix-adapter-set-user-pin.md)). `delete` removes the user and credentials on the device. `set_user` / `set_pin` already exist; do not reimplement them beside the adapter.

### Credentials

`/device.cgi/credential` with `set` / `get` / `delete`. Guide type codes include finger `1`, card `2`, palm `3`, palm template with guide mode `4`, face template `5`, face image `6`. Types 5 and 6 apply to ARGO FACE. Do not invent template encodings. `set_card` stays blocked until a verified card payload exists (ADR-011).

Keep these separate:

| Kind | Meaning | Path |
|---|---|---|
| Enrollment command | Device starts live capture | `/device.cgi/enrolluser?action=enroll` |
| Credential get/set/delete | Read or write credential data | `/device.cgi/credential` |

Face enrollment type `7` is the documented enroll type for face. It is not the same call as uploading a face template.

### Events

HTTP pull and TCP push stay separate. Do not mix PUSH API behavior into the HTTP client.

| Call | Path (verified in the integration doc) |
|---|---|
| `get_events` | `/device.cgi/events?action=getevent` with `roll-over-count` and `seq-number` |
| `get_event_count` | `/device.cgi/command?action=geteventcount` |
| `get_tcp_events` | `/device.cgi/tcp-events?action=getevent` |

TCP is not supported on every model (Direct Door V2 is called out in the guide). Confirm ARGO FACE before enabling it.

### Device commands

`/device.cgi/command?action=...`. Guide actions include lock, unlock, normalize, clear alarm, acknowledge alarm, deny user, `getusercount`, `getcount` (per user, not a global inventory), `geteventcount`. The guide marks lock/unlock/normalize/open door as **not applicable for ARGO FACE200T**. Confirm this firmware before exposing them.

UI rule when a command is dangerous (open door and anything that changes door or alarm state): explicit confirmation in React. Do not put that button on the first connection screen.

## React service (after the first slice)

`src/services/matrix.ts` (or the existing devices service, if the screen already calls `test_device_connection`) uses `invoke` only. Later functions: `getUsers`, `getUser`, `createUser`, `deleteUser`, `getCredential`, `setCredential`, `deleteCredential`, `enrollUser`, `getEvents`, `getEventCount`, `openDoor`. No `fetch` to the device. No Digest math in TypeScript.

## Done for the first slice

- `cargo check` and `cargo test` pass.
- Ignored live test is not required for `cargo test`.
- Do not report that the ARGO FACE integration works until that ignored test has succeeded against the lab device.
