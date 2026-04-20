# Wave-2 Cloud Agent: `udt-registration`

**Branch**: `wave-2/udt-registration` (base: `main`)
**PR title**: `[wave-2] udt-registration: CREATE TYPE/DOMAIN/ENUM for WIT type plans`

Read `@.cursor/cloud-agent-prompts/wave-2/README.md` and
`@.cursor/cloud-agent-prompts/wave-1/README.md` first.

## Task (copied verbatim)

> Implement `wit::udt::register_type_plan` that issues `CREATE TYPE`,
> `CREATE DOMAIN`, `CREATE ENUM` DDL via SPI and records rows in
> `pg_wasm.wit_types` with `recordDependencyOn`. Idempotent for
> reload-compatible definitions; updates OIDs in-place when definitions
> match.

Design ref: `docs/architecture.md` §§ "UDT registration and reload
compatibility", "Stable type keys".

## Files you own

- `pg_wasm/src/wit/udt.rs` (new) — declare `pub mod udt;` in
  `pg_wasm/src/wit/mod.rs`. This is the only edit to `wit/mod.rs` you
  are permitted — do not change anything else in that file.

## Files you must not touch

- All files listed in `@.cursor/cloud-agent-prompts/wave-2/README.md`.
- `pg_wasm/src/wit/world.rs`, `pg_wasm/src/wit/typing.rs` — Wave-1
  outputs.
- Catalog internals beyond calling into `catalog::wit_types::{upsert,
  get_by_type_key, delete}` helpers (added by `catalog-schema`). Do
  not modify those files.

## Implementation notes

- **`register_type_plan(plan: &wit::typing::TypePlan, module_id: u64,
  extension_oid: Oid) -> Result<RegisteredTypes, PgWasmError>`**
  walks `TypePlan` in dependency order and for each type:
  1. Look up an existing row in `pg_wasm.wit_types` by `type_key`.
  2. If missing: issue the appropriate DDL via SPI
     (`SPI::run_with_args` or equivalent pgrx 0.18 API); resolve the
     resulting OID with `regtype::from_name`; insert a new catalog row;
     `recordDependencyOn(object=<type oid>,
     depender=<extension oid>, DEPENDENCY_EXTENSION)`.
  3. If present and the definition **matches** (same fields/cases in
     the same order and types): keep the existing OID; update only
     `last_seen_generation` and return it. No DDL issued.
  4. If present and definition **differs** but transition is legal
     (`ALTER TYPE ADD ATTRIBUTE`, `ALTER TYPE ADD VALUE`,
     `ALTER TYPE RENAME ATTRIBUTE` when safe): issue the `ALTER`,
     update catalog row. Preserve OID.
  5. If present and transition is **illegal** (reordered, removed, or
     type-changed attributes; new required field on an existing
     record; removed enum value): return
     `PgWasmError::InvalidConfiguration(_)` hint-ing
     `options.breaking_changes_allowed` even though this task doesn't
     honor that flag yet — `reload-orchestration` (Wave 5) will. A
     clean error message here suffices.
- **Per-WIT-kind DDL**:
  - `record` → `CREATE TYPE <name> AS (...)`.
  - `variant` → `CREATE TYPE <name> AS (discriminant text, payload
    jsonb)` — or tagged composite per the design doc's chosen
    representation. Follow whatever `wit::typing::TypePlan` emits as
    the target PG shape.
  - `enum` → `CREATE TYPE <name> AS ENUM (...)`.
  - `flags` → `CREATE TYPE <name> AS (...)` of booleans, or a `bit(n)`
    domain. Again follow the plan's declared shape.
  - `option<T>` and `result<T,E>` → typically handled as nullable +
    tagged composite per the plan; do not invent; implement what the
    plan says.
  - Unsigned-int domains (`uN`) → `CREATE DOMAIN <name> AS intN CHECK
    (VALUE >= 0 AND VALUE <= <max>)`.
- **Idempotency** is critical: calling `register_type_plan` twice with
  the same plan must be a no-op (beyond `last_seen_generation` bump).
- **Unregister path**: add `unregister_module_types(module_id)` that
  deletes `pg_wasm.wit_types` rows for the module and issues `DROP
  TYPE` / `DROP DOMAIN` for each, respecting the `pg_wasm.dependencies`
  table (if another module references the type, refuse unless
  `cascade: true` — the cascade path belongs to
  `unload-orchestration`, so here just return an error enum the
  caller can map).

## Validation expectations

- `cargo check -p pg_wasm` passes.
- `#[pg_test]`:
  - Register a plan containing one record + one enum + one domain;
    assert the PG types exist with the right shape and that
    `pg_wasm.wit_types` has three rows.
  - Re-register the same plan; assert zero DDL was issued (capture
    `pg_stat_xact_*` or just observe that OIDs did not change).
  - Register a modified plan adding a new record field; assert
    `ALTER TYPE ADD ATTRIBUTE` succeeded and the OID is preserved.
  - Register a breaking plan change; assert expected error.
  - `unregister_module_types` drops the types and catalog rows.

## Final commit

Flip `udt-registration`'s `status:` line to `completed`.
