# ADR-009: Device password at rest

- **Status:** Accepted
- **Date:** 2026-09-20

## Context

Devices store Matrix COSEC HTTP basic-auth passwords. Plaintext in PostgreSQL is unacceptable. An environment variable as the permanent workstation encryption key is a weak fit for a desktop app (easy to copy, often logged, shared across processes).

## Decision

1. Encrypt device passwords **and** user credential secrets (card numbers, PINs) with **AES-256-GCM** (`aes-gcm` crate). Ciphertext lives in PostgreSQL.
2. Persist a **single** AES master key in a file the application creates: `~/.vsmart-sync/master.key` (`HOME`, or `USERPROFILE` when `HOME` is unset). 32 random bytes, created once. Reuse that key for Devices and Credentials — do not create a second master key. Do not use the macOS Keychain, Windows Credential Manager, or Linux Secret Service.
3. Plaintext enters Rust only on write paths (or device probe decrypt). List/get responses never include plaintext, ciphertext, or digests. Cards may expose a stored last-4 display hint; PINs expose nothing.
4. Do not invent a custom cipher. Do not use env vars as the long-term master key unless a later ADR explicitly overrides this.

## Consequences

- Moving a database to another workstation without `master.key` cannot decrypt passwords.
- A missing or unreadable key file surfaces as `DEVICE_SECRET_UNAVAILABLE`. A key file that is not 32 bytes surfaces as `DEVICE_SECRET_CORRUPT`.
- Amended 2026-09-27: the master key is the application file above. The OS credential store is not used.
- React password fields are write-only and never prepopulated.
