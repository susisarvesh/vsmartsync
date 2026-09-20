# ADR-009: Device password at rest

- **Status:** Accepted
- **Date:** 2026-09-20

## Context

Devices store Matrix COSEC HTTP basic-auth passwords. Plaintext in PostgreSQL is unacceptable. An environment variable as the permanent workstation encryption key is a weak fit for a desktop app (easy to copy, often logged, shared across processes).

## Decision

1. Encrypt device passwords with **AES-256-GCM** (`aes-gcm` crate). Ciphertext (`nonce || ciphertext`) is stored in `devices.password_ciphertext` (`BYTEA`).
2. Persist the AES master key in the **OS credential store** via the `keyring` crate (macOS Keychain / Windows Credential Manager / Linux Secret Service).
3. Plaintext passwords enter Rust only through write commands (`create_device`, `set_device_password`) or during a connection probe decrypt. They are never returned on list/get, never logged, and never sent to React.
4. Do not invent a custom cipher. Do not use env vars as the long-term master key unless a later ADR explicitly overrides this.

## Consequences

- Moving a database to another workstation without the keyring entry cannot decrypt passwords.
- Keyring unavailability surfaces as `DEVICE_SECRET_UNAVAILABLE`.
- React password fields are write-only and never prepopulated.
