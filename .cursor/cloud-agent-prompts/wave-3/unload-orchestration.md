# Wave-3 Cloud Agent: `unload-orchestration`

**Branch**: `wave-3/unload-orchestration` (base: `main`)
**PR title**: `[wave-3] unload-orchestration: full unload flow with hook, DDL, artifact cleanup`

Read `@.cursor/cloud-agent-prompts/wave-3/README.md` and
`@.cursor/cloud-agent-prompts/wave-1/README.md` first.

## Task (copied verbatim)

> Implement `lifecycle::unload` with `on-unload` hook,
> `RemoveFunctionById`, UDT drop (respecting `pg_wasm.dependencies` and
> `options.cascade`), catalog row deletion, artifact dir removal,
> generation bump.

Design ref: `docs/architecture.md` §§ "Unload lifecycle",
"Dependency resolution".

## Files you own

- `pg_wasm/src/lifecycle/unload.rs` (new). Declare in
  `pg_wasm/src/lifecycle/mod.rs` (add `pub mod unload;`).

## Files you must not touch

- Every Wave-1 / Wave-2 file not in your "owned" list.
- `pg_wasm/src/lifecycle/{load,reload,reconfigure}.rs` (other lifecycle
  siblings — reconfigure exists from Wave 2, load/reload land later).
- `pg_wasm/src/hooks.rs` — still a stub until Wave 4.

## Implementation notes

- **SQL entry point**:
  ```rust
  #[pg_extern]
  pub fn unload(module_name: &str, cascade: default!(bool, false)) -> bool
  ```
- **Flow**:
  1. AuthZ: require `pg_wasm_loader` role.
  2. Resolve `module_id` via `catalog::modules::get_by_name`.
  3. Invoke `on-unload` export if present. **Failures here are
     logged, not fatal** — the user is unloading, so the module is
     already expected to go away. If `hooks::on_unload` isn't wired
     yet (Wave 4), emit a `NOTICE` and skip. Add
     `TODO(wave-4: hooks)`.
  4. For each row in `pg_wasm.exports` for this module, call
     `proc_reg::unregister(fn_oid)` — removes from `pg_proc` + cleans
     dependency rows.
  5. For each row in `pg_wasm.wit_types` for this module:
     - Check `pg_wasm.dependencies` for external references.
     - If referenced and `!cascade`: abort with
       `PgWasmError::InvalidConfiguration` hint-ing `cascade := true`.
     - If referenced and `cascade`: issue `DROP TYPE ... CASCADE`.
     - If unreferenced: issue `DROP TYPE ...` (or `DROP DOMAIN`).
     - Delete the catalog row.
  6. Delete the `pg_wasm.exports` and `pg_wasm.modules` rows.
  7. Drain the instance pool: `runtime::pool::drain(module_id)`.
  8. Remove the artifact dir: `artifacts::remove_module_dir(module_id)`.
     Call `artifacts::prune_stale` opportunistically afterwards.
  9. `shmem::bump_generation(module_id)`.
  10. Free shmem slots: `shmem::free_slots(module_id)`.
- **Transaction boundary**: run steps 4–6 inside the caller's
  transaction via SPI; on rollback, steps 7–10 must be undone or
  deferred. Use `pgrx::pg_sys::RegisterXactCallback(XACT_EVENT_COMMIT)`
  for pool/artifact/shmem cleanup so a rollback does not leave the
  catalog inconsistent with disk. Document the callback registration.
- **Idempotency**: `unload` of a non-existent module returns
  `PgWasmError::NotFound` with a clear message, not an exception.
- **Bulk unload**: expose `unload_all()` (superuser only) that
  iterates. Useful for tests.

## Validation expectations

- `cargo check -p pg_wasm` passes.
- `#[pg_test]`:
  - Seed a fake module via direct catalog inserts + synthetic
    `pg_proc` row; call `unload`; assert all rows removed and the
    procedure is gone.
  - UDT cascade: module A defines a record T, module B has a
    dependency row on T; `unload(A, cascade := false)` errors;
    `unload(A, cascade := true)` succeeds and drops T.
  - Artifact cleanup: seed a `$PGDATA/pg_wasm/<id>/` dir; assert it
    is removed after successful unload.
  - Rollback: start a transaction, call `unload`, `ROLLBACK`; assert
    catalog unchanged and artifact dir still present (i.e. xact
    callback correctly deferred the disk op).
  - `unload` of missing name → `NotFound`.

## Final commit

Flip `unload-orchestration`'s `status:` line to `completed`.
