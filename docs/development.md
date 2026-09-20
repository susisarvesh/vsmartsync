# Development

**Status:** Process for building the product **one domain service at a time**. Domain services themselves are **not** implemented in this documentation pass.

## Principle

> Build one domain service at a time and review it before moving to the next.

Do not implement Users, Devices, Matrix HTTP, and Events in a single change set. The human developer reviews each service before work continues.

## Suggested implementation sequence

Do **not** implement these now. Order for later prompts:

1. Database foundation (first real migrations / entities as designed)
2. Users
3. Devices
4. Credentials
5. Enrollments
6. Matrix Adapter (real client, verified endpoints only)
7. Synchronization
8. Events
9. Monitoring
10. Audit
11. Licensing
12. Access Control expansion

Matrix adapter is listed after the first domain records exist so there is something meaningful to push. A thin “ping device” adapter spike during Devices is acceptable if it stays behind the adapter module.

## Verification commands (actual repo)

From the repository root:

```bash
cargo fmt --all --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --all-targets --manifest-path src-tauri/Cargo.toml -- -D warnings
npm run build
```

Desktop:

```bash
npm install && npm start
```

Live Postgres test:

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- --ignored --nocapture
```

Packaged app (run on the OS you want to ship):

```bash
npm run tauri build
```

CI (`.github/workflows/ci.yml`) runs `cargo fmt`, Clippy, `cargo test`, and `npm run build` on macOS, Windows, and Ubuntu.

Frontend-only (no Tauri):

```bash
npm run dev
```

## Standards

### Rust

- Format with `cargo fmt --all` before review.
- Tests for implemented business logic (config parsing, job state transitions, mapping functions, command handlers).
- Clippy `--all-targets -- -D warnings` must stay clean.
- Errors: explicit types, no silent unwrap in library paths.
- Logging: structured tracing (JSON subscriber is already wired). Log **actions**, not passwords, templates, or license secrets.
- UTC for timestamps; UUIDs for internal ids (when tables exist).

### TypeScript / React

- `npm run build` (`tsc && vite build`) must succeed.
- UI talks only to `src/services` → Tauri invoke.
- No PostgreSQL, no `device.cgi`, no license crypto in the frontend.

### SQLx / PostgreSQL

- New tables **only** via versioned files in `migrations/`.
- Do not silently change schema in a feature that did not ask for it.
- Transactions where multiple rows must commit together.

### Git

- Feature branches; small commits; code review before merge.
- **No secrets in Git:** `.env`, device passwords, USB secrets, production `DATABASE_URL`.
- `.env.example` may contain placeholders only.
- Do not commit `node_modules/`, `target/`, or `dist/`.

### Documentation

- Update `docs/` when a service lands if behavior is now **IMPLEMENTED**.
- Keep **IMPLEMENTED** vs **PLANNED** vs **REQUIRES MATRIX DOCUMENTATION VERIFICATION** honest.
- Do not invent Matrix endpoints. If unknown, write that verification is required.

### Testing expectations

| Layer | Expectation |
|---|---|
| Pure Rust logic | Unit tests in-module |
| Tauri commands | Tests like existing `get_app_info` IPC test |
| PostgreSQL | Ignored live tests against a local instance — do not require PostgreSQL for default `cargo test` |
| Matrix | Mock the adapter in domain tests; hardware tests optional and explicit |

## Architecture change rule

If a change needs a new crate, a queue product, or microservices, **explain why first**. Default is the modular monolith in [architecture.md](architecture.md).

## Local desktop reminder

Ship and develop as a **Tauri application**. Do not instruct users to “open localhost in Chrome” as the product.
