# Wave-1 Cloud Agent: `catalog-schema`

**Branch**: `wave-1/catalog-schema` (base: `main`)
**PR title**: `[wave-1] catalog-schema: durable catalog tables, roles, and migrations`

Before doing anything else, read `@.cursor/cloud-agent-prompts/wave-1/README.md`
in full and comply with every shared rule in it.

## Task (copied verbatim from the plan todo)

> Add `pg_wasm.modules`, `pg_wasm.exports`, `pg_wasm.wit_types`,
> `pg_wasm.dependencies` tables in versioned SQL. Implement
> `catalog::{modules,exports,wit_types}` CRUD via SPI. Set up
> `pg_wasm_loader` and `pg_wasm_reader` roles with minimal grants. Add
> `catalog::migrations` that validates shape on `_PG_init`.

Authoritative design sections:
`docs/architecture.md` §§ "Catalog schema", "Generation counters and
CatalogLock", "Role model", "Lifecycle operations".

## Files you own

- `pg_wasm/src/catalog.rs` — flat file today; you may convert it to
  `pg_wasm/src/catalog/mod.rs` plus the following submodules (pick either
  layout, but keep everything under `catalog::`):
  - `catalog::modules`
  - `catalog::exports`
  - `catalog::wit_types`
  - `catalog::migrations`
- `pg_wasm/sql/*.sql` — versioned schema SQL (pgrx 0.18 conventions:
  `pg_wasm--0.1.0.sql` for initial, and the upgrade-script scaffolding
  `pg_wasm--X.Y--X.Z.sql` can be added empty for now).

`_PG_init` already calls `catalog::init()`. Fill that function in — it
should call `catalog::migrations::validate_shape()` (or the equivalent
name you pick) so a mis-shaped catalog errors loudly at backend start.

## Files you must not touch

- `pg_wasm/src/lib.rs` (scaffolding already wires `catalog::init()`).
- Every other `pg_wasm/src/*.rs` file (they belong to other Wave-1 agents or
  later waves).
- `Cargo.toml`, `pg_wasm/Cargo.toml`, `pg_wasm/pg_wasm.control`.
- Any line of the plan file other than your own todo's `status:` line.

## Implementation notes

- **Schema**: single SQL file defining the `pg_wasm` schema plus four tables:
  `modules`, `exports`, `wit_types`, `dependencies`. Match the column set in
  `docs/architecture.md` (module_id, digest, origin, wit_world, policy,
  limits, etc.; per-export `fn_oid`, `arg_types`, `ret_type`, etc.).
- **Roles**: create `pg_wasm_loader` (can invoke load/unload/reload,
  read+write catalog) and `pg_wasm_reader` (can read catalog and
  observability views). Grant minimally: USAGE on the schema, SELECT on
  tables for reader, SELECT+INSERT+UPDATE+DELETE for loader.
- **CRUD via SPI**: pgrx 0.18 has `pgrx::spi`. Expose CRUD helpers that
  other modules (lifecycle, views, registry) will call. Do not embed policy
  logic here — just type-safe row access.
- **`catalog::migrations::validate_shape`**: light-weight runtime check.
  Query `pg_catalog` (via SPI) for the expected tables/columns; raise
  `PgWasmError::InvalidConfiguration` via `ereport` if shape drifts.
- **Do not** exercise catalog CRUD from `_PG_init`. It only runs on first
  use (lifecycle entry points). `catalog::init()` should be cheap and
  idempotent — reserve heavy work for per-call paths.
- **Extension dependency recording**: do not record DEPENDENCY_EXTENSION
  here; that belongs in `proc_reg-ddl` and `udt-registration` (later waves).

## Validation expectations

- `cargo check -p pg_wasm` passes.
- `cargo pgrx test -p pg_wasm` — add a `#[pg_test]` that confirms the four
  tables exist with the expected columns after `CREATE EXTENSION pg_wasm;`
  and that `pg_wasm_reader` / `pg_wasm_loader` roles have the right grants.
- Optional host-only `#[test]` for any pure Rust helpers you add (e.g. row
  serde).

## Final commit

Flip the following line in
`.cursor/plans/pg_wasm_extension_implementation_v2.plan.md`:

```
  - id: catalog-schema
    content: ...
    status: pending
```

to `status: completed`. That is the **only** line of the plan file you may
edit.
