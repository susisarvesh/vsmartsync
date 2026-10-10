# ADR-014: Excel reader for user import

- **Status:** Accepted
- **Date:** 2026-10-10

## Context

Operators add people from an `.xls` or `.xlsx` list, and they can also register one person in the app. The desktop app cannot parse those files with the crates it already has. React must not open PostgreSQL, and it must not be the place that decides which values are valid.

## Decision

Add `calamine` to the Rust crate and use it only to read the first worksheet of an uploaded user list. The users domain validates the rows and writes them in one transaction, or writes nothing. The file is not sent to a Matrix device.

Imported and manually registered `ID` and `Reference ID` are stored on the person. Assigning that person to a device copies those values into `device_users`. Manual registration uses the same fields and uniqueness rules as one spreadsheet row. People created by the older username-only command still receive the per-device ids from ADR-013.

## Consequences

- One new dependency, used for spreadsheet bytes only.
- A sheet with any invalid row is rejected as a whole.
