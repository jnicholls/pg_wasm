# Wave-1 Cloud Agent: `proc-reg-ddl`

**Branch**: `wave-1/proc-reg-ddl` (base: `main`)
**PR title**: `[wave-1] proc-reg-ddl: ProcedureCreate/RemoveFunctionById wrappers with extension deps`

Before doing anything else, read `@.cursor/cloud-agent-prompts/wave-1/README.md`
in full and comply with every shared rule in it.

## Task (copied verbatim from the plan todo)

> Implement `proc_reg::{register, unregister}` wrapping `ProcedureCreate`
> / `RemoveFunctionById` and `recordDependencyOn(DEPENDENCY_EXTENSION)`.
> Validate name collision handling per `options.replace_exports`.

Authoritative design sections:
`docs/architecture.md` §§ "pg_proc registration", "Extension
dependencies", "Name collision semantics".

## Files you own

- `pg_wasm/src/proc_reg.rs`

## Files you must not touch

- `pg_wasm/src/lib.rs`.
- `pg_wasm/src/trampoline.rs` — the trampoline symbol and its pgrx glue
  are owned by the `trampoline-stub` agent. You **reference** the
  trampoline symbol by name when populating `prosrc` / `probin`, but you
  do not define it or edit the trampoline file.
- Every other `pg_wasm/src/*.rs` file.
- `Cargo.toml`, `pg_wasm/Cargo.toml`, `pg_wasm/pg_wasm.control`.
- Any line of the plan file other than your own todo's `status:` line.

## Implementation notes

- **Public API**:
  ```rust
  pub(crate) struct ProcSpec {
      pub schema: String,
      pub name: String,
      pub arg_types: Vec<pgrx::pg_sys::Oid>,
      pub arg_names: Vec<String>,
      pub arg_modes: Vec<ProcArgMode>, // in, out, inout, variadic
      pub ret_type: pgrx::pg_sys::Oid,
      pub returns_set: bool,
      pub volatility: Volatility,
      pub strict: bool,
      pub parallel: Parallel,
      pub cost: Option<f32>,
  }

  pub(crate) fn register(
      spec: &ProcSpec,
      extension_oid: pgrx::pg_sys::Oid,
      replace_exports: bool,
  ) -> Result<pgrx::pg_sys::Oid, PgWasmError>;

  pub(crate) fn unregister(fn_oid: pgrx::pg_sys::Oid) -> Result<(), PgWasmError>;
  ```
- **Implementation**:
  - `register` calls `pgrx::pg_sys::ProcedureCreate` (or the pgrx-0.18
    safe wrapper if one exists) with:
    - `prolang = ClanguageId` (so PG routes to the trampoline C symbol).
    - `prosrc` = `"pg_wasm_udf_trampoline"`.
    - `probin` = the extension's `.so` path (pgrx auto-resolves via
      `PG_MODULE_MAGIC`; re-use the conventional extension name).
    - `replace = replace_exports`.
  - On success call
    `pgrx::pg_sys::recordDependencyOn(new_object, extension_object, DEPENDENCY_EXTENSION)`
    so `DROP EXTENSION pg_wasm` cleans the procs up.
  - Name-collision: if the proc already exists and `replace_exports` is
    false, return `PgWasmError::InvalidConfiguration("function
    <schema>.<name>(args) already exists; set replace_exports := true to
    overwrite")`. If `replace_exports` is true, `ProcedureCreate` handles
    the replace path; ensure the dependency record stays correct.
- `unregister` calls `pgrx::pg_sys::RemoveFunctionById(fn_oid)` inside
  the current SPI transaction. Extension dependency is automatically
  cleared by PG.
- **Do not** invoke any SPI here beyond what pgrx wraps for the FFI
  calls. Transaction control is the caller's responsibility (the future
  `lifecycle::load` task owns the overall transaction).

## Validation expectations

- `cargo check -p pg_wasm` passes.
- `#[pg_test]`s (this task is inherently backend-bound):
  - `register` a synthetic proc pointing at the trampoline; confirm
    `pg_proc` has a row with `prosrc = 'pg_wasm_udf_trampoline'` and
    `prolang = C`.
  - `register` with `replace_exports = false` against an existing name
    → error with expected SQLSTATE.
  - `register` with `replace_exports = true` overwrites.
  - `unregister` removes the row; a subsequent `SELECT` from `pg_proc`
    by oid returns 0 rows.
  - After `register`, `pg_depend` has an entry with
    `deptype = 'e' (DEPENDENCY_EXTENSION)` pointing at pg_wasm's
    extension oid.

## Final commit

Flip the `status:` line for `proc-reg-ddl` in
`.cursor/plans/pg_wasm_extension_implementation_v2.plan.md` from `pending`
to `completed`. No other edits to that file.
