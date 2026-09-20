# ADR-006: Device events

- **Status:** Accepted direction; implementation **PLANNED**
- **Date:** 2026-09-20

## Context

Doors emit events (HTTP fetch and/or TCP, per vendor guide). Sequence numbers and rollover are used to detect gaps. This application is not assumed to perform every live biometric match.

## Decision

`domains/events` owns ingest, HTTP gap recovery, deduplication, and query. Persistence uses vendor keys as documented (device + rollover + seq) **after** schema design. TCP listen, if implemented, stays behind `matrix`, not in React.

Access decisions remain **on the device**. The app stores and displays what devices report.

## Consequences

- Do not treat `audit` rows as door events.
- Do not invent TCP vs HTTP recovery behavior; verify against the model and API guide.
- See [events.md](../../events.md).
