# Coding standards

Stack: **Rust** in `src-tauri/`, **TypeScript + React** in `src/`. Do not add Python or a second backend.

## Rust

- `cargo fmt --all --manifest-path src-tauri/Cargo.toml` must be clean.
- Clippy `--all-targets -- -D warnings` must stay clean.
- No `unwrap` / `expect` on library paths (commands, domain, database, matrix). Tests may use `expect` with a reason.
- Prefer explicit error types (`thiserror`) over `String` except at the Tauri command boundary, where a **safe** `String` for the UI is allowed.
- Public module docs state **owns / must not own**.
- UTC for timestamps. UUIDs for internal ids when tables exist.
- Keep `common/` small. Do not dump domain logic there.
- Domain code lives in `src-tauri/src/domains/<name>/`. Matrix HTTP lives in `src-tauri/src/matrix/`.

## TypeScript / React

- `npm run build` (`tsc && vite build`) must succeed.
- Strict TypeScript. Avoid `any`.
- UI calls only `src/services` → Tauri `invoke`. No `fetch` to devices, no Postgres clients, no license crypto.
- Functional components. Hooks for reusable client state. Keep pages thin.
- Do not put business invariants in components (enrollment rules, sync planning, access policy).

## Naming

- Tauri commands: `snake_case`, verb-first (`get_app_info`, `connect_database`).
- Rust modules: crate layout already defined. Do not create `utils.rs` / `helpers.rs` dumping grounds.
- Prefer domain words: `Enrollment`, `SyncJob`, `Device`, not `Manager` / `CommonService`.

## Comments

Explain **why**, security constraints, and Matrix/API tradeoffs. Do not narrate the next line of code.

```rust
// Do not retry this command blindly: the device may have executed it
// before the HTTP timeout returned to us.
```

## Configuration

- Secrets and connection strings come from `.env` / environment, never source.
- Fail clearly when required config is missing. Do not silently use insecure production defaults.
- `.env.example` holds placeholders only.

## Dependencies

Do not add crates or npm packages casually. Prefer the standard library and libraries already in `Cargo.toml` / `package.json`. Never roll custom cryptography.
