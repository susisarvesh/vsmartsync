# ADR-003: Matrix integration boundary

- **Status:** Accepted (thin connectivity probe implemented; broader CGI not implemented)
- **Date:** 2026-09-20

## Context

COSEC Devices API is HTTP CGI on each door (`device.cgi`), model-specific, with device basic-auth. If domain modules inlined those URLs, tests would need hardware and credentials would leak across layers.

## Decision

All Matrix communication stays in `src-tauri/src/matrix/` (`adapter`, `client`, `models`). Domain services depend on application-facing operations (upsert user, start enroll, fetch events), not on CGI paths.

**Devices v1** implements only:

`GET /device.cgi/device-basic-config?action=get`

as a reachability/auth probe with explicit timeout, no redirects, host/port validated upstream, and no credential logging. No other CGI endpoints in this slice.

Facts about the vendor API come only from **COSEC Devices API User Guide v28** (and later verified model docs). Unknown behavior is marked **REQUIRES MATRIX DOCUMENTATION VERIFICATION**. Do not invent endpoints.

## Consequences

- Domain tests mock/map probe outcomes; HTTP tests use wiremock.
- Device passwords never leave Rust toward React.
- USB license failures are not confused with device HTTP failures.
- Changing a door firmware mapping is an adapter change, not a users-module rewrite.
