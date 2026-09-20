# ADR-009: Device password at rest

- **Status:** Accepted
- **Date:** 2026-09-20

## Context

Devices store Matrix COSEC HTTP basic-auth passwords. Plaintext in PostgreSQL is unacceptable. An environment variable as the permanent workstation encryption key is a weak fit for a desktop app (easy to copy, often logged, shared across processes).

## Decision

1. Encrypt device passwords **and** user credential secrets (card numbers, PINs) with **AES-256-GCM** (`aes-gcm` crate). Ciphertext lives in PostgreSQL.
2. Persist a **single** AES master key in the **OS credential store** via the `keyring` crate (macOS Keychain / Windows Credential Manager / Linux Secret Service). Reuse the same key for Devices and Credentials — do not create a second master key.
3. Plaintext enters Rust only on write paths (or device probe decrypt). List/get responses never include plaintext, ciphertext, or digests. Cards may expose a stored last-4 display hint; PINs expose nothing.
4. Do not invent a custom cipher. Do not use env vars as the long-term master key unless a later ADR explicitly overrides this.

## Consequences

- Moving a database to another workstation without the keyring entry cannot decrypt passwords.
- Keyring unavailability surfaces as `DEVICE_SECRET_UNAVAILABLE`.
- React password fields are write-only and never prepopulated.
