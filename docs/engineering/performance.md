# Performance

Do not micro-optimize. Do treat obvious waste as a defect.

## Default rules

- Bound every list returned to the UI (page size + max).
- Avoid N+1 SQL.
- Pool PostgreSQL (already in the database layer). Do not open a new connection per command.
- Device HTTP has timeouts. Do not block the UI thread; Tauri commands are async.
- Long work (bulk sync, bulk enroll) belongs in **durable jobs**, not one giant command the user cannot cancel. Jobs are in-process in this monolith, not a Redis queue ([ADR-001](../architecture/decisions/ADR-001-modular-monolith.md)).

## Caching

Do not cache until you can answer: what, TTL, invalidation, behavior if cache is cold, and whether stale data could authorize the wrong person. There is no Redis in the MVP.

## Measurement

Identify the bottleneck (SQL, device RTT, CPU) before adding complexity. Connection pooling and batch adapter calls beat premature thread pools.

## Packaging

Release builds use LTO in `Cargo.toml`. Do not disable that for convenience. `npm run package` is per-OS; do not cross-compile as a surprise requirement.
