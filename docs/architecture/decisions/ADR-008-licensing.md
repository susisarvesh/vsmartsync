# ADR-008: USB licensing

- **Status:** Accepted direction; implementation **PLANNED**
- **Date:** 2026-09-20

## Context

The product intends to require a USB license for protected features. License state must not be confused with “door is offline.”

## Decision

`domains/licensing` validates the USB key in Rust. React must not enforce licensing. License secrets never go to the UI or git. Losing the key is a license failure, not a Matrix failure. Vendor SDK, key format, and OS USB details are chosen when that service is built — not invented in foundation docs.

Until then, `npm start` does not require a dongle.

## Consequences

- Do not block Postgres setup on USB absence.
- See [licensing.md](../../licensing.md).
