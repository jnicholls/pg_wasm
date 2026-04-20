# Wave-4 Cloud Agent: `load-orchestration`

**Branch**: `wave-4/load-orchestration` (base: `main`)
**PR title**: `[wave-4] load-orchestration: authz → read → validate → classify → resolve → compile → register`

Read `@.cursor/cloud-agent-prompts/wave-4/README.md` and
`@.cursor/cloud-agent-prompts/wave-1/README.md` first.

## Task (copied verbatim)

> Implement `lifecycle::load` running AuthZ -> read -> validate ->
> classify -> resolve WIT -> plan types -> plan exports -> resolve
> policy -> compile + persist -> register procs -> on-load hook ->
> bump generation. All DDL runs via SPI inside one transaction;
> failure rolls everything back and removes the module dir.

Design ref: `docs/architecture.md` §§ "Load lifecycle", "Transaction
boundary and rollback semantics".

## Files you own

- `pg_wasm/src/lifecycle/load.rs` (new). Declare in
  `pg_wasm/src/lifecycle/mod.rs` (`pub mod load;`).

## Files you must not touch

- All other `pg_wasm/src/*` files. This task **calls** the Wave-1/2/3
  APIs but edits none of them.
- If a helper is missing, STOP and report — do not monkey-patch in
  this PR.
- `Cargo.toml`, `pg_wasm.control`, `pg_wasm/src/lib.rs`.

## Implementation notes

- **SQL entry point**:
  ```rust
  #[pg_extern]
  pub fn load(
      module_name: &str,
      bytes_or_path: pgrx::Json, // { bytes: bytea } | { path: text }
      options: default!(Option<pgrx::Json>, NULL),
  ) -> bool
  ```
  `bytes_or_path` accepts either an inline `bytea` payload or a
  filesystem path (gated by `pg_wasm.allow_load_from_file` and
  narrowed by `pg_wasm.allowed_path_prefixes`). Follow the GUC rules
  defined in `guc.rs`.
- **Flow** (match the task description's order exactly):
  1. **AuthZ**: require `pg_wasm_loader` role or superuser.
  2. **Read bytes**:
     - `bytes`: decode directly.
     - `path`: resolve `pg_wasm.follow_symlinks` rule; verify prefix
       against `pg_wasm.allowed_path_prefixes`; enforce
       `pg_wasm.max_module_bytes`.
  3. **Validate**: `abi::validate(bytes)` (full wasmparser validate).
  4. **Classify**: `abi::detect(bytes, options.abi)` →
     `Abi::Component` or `Abi::Core`.
  5. **Resolve WIT** (component only): `wit::world::decode`. Store
     normalized WIT text for the catalog.
  6. **Plan types** (component only):
     `wit::typing::plan_types(module_prefix, &decoded)`.
  7. **Plan exports**: for each WIT export, derive a
     `proc_reg::ProcSpec` (arg/ret types from `TypePlan`, volatility
     default `VOLATILE`, strict derived from WIT option-unwrap
     conventions).
  8. **Resolve policy**: `policy::resolve(GucSnapshot::take(),
     options.overrides, options.limits)`.
  9. **Compile + persist**:
     - Component path: `runtime::component::compile` + `precompile_to`
       (writing `module.wasm`, `module.cwasm`, and `world.wit` into
       `$PGDATA/pg_wasm/<module_id>/` via `artifacts::write_atomic`).
       Record `precompile_compatibility_hash`.
     - Core path: `runtime::core::compile`. Still persist `module.wasm`
       + its sha256 to disk for `reload` compatibility checks.
  10. **Register procs**: for each `ProcSpec`, call
      `proc_reg::register(&spec, extension_oid,
      options.replace_exports)`. Collect `fn_oid` into the catalog
      row.
  11. **UDT registration** (component path):
      `wit::udt::register_type_plan(&plan, module_id, extension_oid)`.
  12. **Catalog writes**: insert rows into `pg_wasm.modules`,
      `pg_wasm.exports`, `pg_wasm.wit_types` using
      `catalog::{modules,exports,wit_types}::upsert`.
  13. **on-load hook** (component only): if the module exports
      `on-load`, invoke via `hooks::on_load(module_id, config_blob)`.
      A hook failure rolls the whole transaction back.
  14. **Bump generation**: `shmem::bump_generation(module_id)`.
  15. **Instance pool warm-up**: `runtime::pool::prewarm(module_id,
      count := 1)` is fire-and-forget; failures here are logged but
      non-fatal.
- **Transaction semantics**:
  - Steps 10–14 all run inside the caller's transaction via SPI. On
    rollback, disk artifacts must be removed — register an
    `xact_callback(XACT_EVENT_ABORT)` that calls
    `artifacts::remove_module_dir(module_id)`.
  - If the backend crashes mid-load, on next start `catalog::migrations`
    sees catalog/artifacts mismatch and `artifacts::prune_stale`
    cleans orphans.
- **Breaking-change path**: if a row for `module_name` already exists
  and `options.replace_exports = false`, fail with
  `PgWasmError::InvalidConfiguration`. If `true`, internally delegate
  to `lifecycle::reload` (Wave 5 dependency) — **for this PR**, fail
  with a clear "call pg_wasm.reload(...)" message, since `reload` is
  not yet implemented. Add a `TODO(wave-5: reload-orchestration)`
  comment.

## Validation expectations

- `cargo check -p pg_wasm` passes.
- `cargo pgrx regress` — new `load.sql` suite loads a tiny fixture
  component, asserts catalog rows, asserts `pg_wasm.modules()` view
  reports the module.
- `#[pg_test]` rollback test: `BEGIN; pg_wasm.load(...); ROLLBACK;`
  — assert catalog clean, artifact dir removed.
- `#[pg_test]` policy widening attempt → denied.

## Final commit

Flip `load-orchestration`'s `status:` line to `completed`.
