# Design patterns

Use a pattern **only when it solves a real problem**. Prefer simple composition and explicit dependencies. Do not introduce interfaces, factories, or event buses to look enterprise-grade.

## Repository

Use when domain/application logic must not know SQL.

```
UserRepository
DeviceRepository
EnrollmentRepository
```

Repositories persist and load. They do not decide enrollment policy or build `device.cgi` URLs.

SQL lives under `src-tauri/src/database/` (models + repositories). Domain modules call repository traits or functions, not `sqlx` ad hoc.

## Application use case / service

One type or function per meaningful operation:

```
enroll_user
register_device
plan_device_sync
assign_access
```

Avoid `CommonService`, `UtilityService`, `ManagerService`, `AppHelper`.

Business rules belong here (or in small domain types), not in Tauri commands or React.

## Adapter (required for Matrix)

```
domain use case
    → MatrixDeviceGateway (application interface)
        → matrix::adapter
            → matrix::client
                → device.cgi
```

The rest of the crate must not know CGI paths, basic-auth headers, or vendor XML/text parsing. See [ADR-003](../architecture/decisions/ADR-003-matrix-integration.md).

## Strategy / factory

Use only when behavior really varies at runtime (for example multiple enrollment capture modes **after** Matrix docs confirm them). Do not create a Strategy for a single implementation. Do not factory-wrap simple structs.

## Dependency injection

Pass repositories, Matrix gateway, clock, and config into services. No global mutable pools besides what Tauri `State` already holds.

## Events

Use application events for **side effects** (audit) after the core operation is explicit. Do not hide “enroll the user” inside a pub/sub chain. There is **no** product-wide message bus in the MVP ([ADR-001](../architecture/decisions/ADR-001-modular-monolith.md)).

## Anti-patterns

God modules, circular domain imports, SQL in commands, Matrix HTTP in `users`/`devices`, leaking database row types to React, giant `shared/utils`, microservices, Redis, “just this once” CGI from a command handler.
