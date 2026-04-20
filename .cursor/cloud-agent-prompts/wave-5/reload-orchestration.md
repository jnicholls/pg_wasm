# Wave-5 Cloud Agent: `reload-orchestration`

**Branch**: `wave-5/reload-orchestration` (base: `main`)
**PR title**: `[wave-5] reload-orchestration: OID-preserving reload with breaking-change gate`

Read `@.cursor/cloud-agent-prompts/wave-5/README.md` and
`@.cursor/cloud-agent-prompts/wave-1/README.md` first.

## Task (copied verbatim)

> Implement `lifecycle::reload` that preserves `fn_oid` /
> `pg_type.oid` when signatures/definitions are unchanged, issues
> `ALTER TYPE` where possible, and errors on breaking changes unless
> `options.breaking_changes_allowed`. Atomic module.wasm swap via
> temp + rename.

Design ref: `docs/architecture.md` §§ "Reload lifecycle", "OID
preservation", "Breaking-change semantics".

## Files you own

- `pg_wasm/src/lifecycle/reload.rs` (new). Declare in
  `pg_wasm/src/lifecycle/mod.rs` (`pub mod reload;`).

## Files you must not touch

- `pg_wasm/src/lifecycle/{load,unload,reconfigure}.rs` — read-only;
  call into them via `pub(crate)` APIs.
- All other files per the Wave-5 README.

## Implementation notes

- **SQL entry point**:
  ```rust
  #[pg_extern]
  pub fn reload(
      module_name: &str,
      bytes_or_path: pgrx::Json,
      options: default!(Option<pgrx::Json>, NULL),
  ) -> bool
  ```
  Accepts the same `breaking_changes_allowed` option as `load`.
- **Flow**:
  1. AuthZ + resolve `module_id`.
  2. Read new bytes (same helper as load).
  3. Validate + classify (`abi::validate` + `abi::detect`). The ABI
     (core vs component) must match the current row — if not, hard
     error unless `breaking_changes_allowed`.
  4. Resolve new WIT world + plan types (component path).
  5. Compare plans:
     - **Exports**: per old export, find the new export by name.
       - Missing export in new plan → breaking change.
       - Signature change → breaking change.
       - Otherwise: reuse `fn_oid`.
     - New exports (present in new plan, absent in old):
       `proc_reg::register` normally.
     - Orphaned exports: `proc_reg::unregister` unless
       `breaking_changes_allowed` is false, in which case error
       before any mutation.
     - **WIT types**: delegate to `wit::udt::register_type_plan` which
       already handles safe `ALTER TYPE` transitions and errors on
       unsafe ones.
   6. **Atomic wasm swap**:
     - Write new `module.wasm` to a temp path and rename over the old
       one (via `artifacts::write_atomic`). Same for `module.cwasm`
       and `world.wit`.
     - If any step fails, delete the temp files; leave the old
       artifact untouched.
   7. Update `pg_wasm.modules.digest`, `wit_world`, `policy_json`,
      `limits_json`.
   8. Drain + rebuild the instance pool for this module:
      `runtime::pool::drain(module_id)`. New acquisitions will
      instantiate against the new `Component`.
   9. `shmem::bump_generation(module_id)`. In-flight invocations
      using the old plan complete naturally (the pool drain is
      cooperative — instances currently in use are reclaimed on
      return, not forcibly killed).
   10. Invoke `on-reconfigure` (reload is a superset of reconfigure).
- **Transaction boundary**: catalog mutations + DDL inside the
  caller's transaction via SPI. On rollback, the temp+rename swap is
  already atomic but irrevocable — register an xact callback that
  reverts the wasm file by writing the old bytes back from a
  temporary backup. Simpler alternative: keep the old `module.wasm`
  as `module.wasm.prev` until commit, then remove on commit. Pick
  whichever pattern has the clearest semantics; document the choice
  in the module comment at the top of `reload.rs`.
- **Breaking-change taxonomy** (centralize in a
  `BreakingChange { Enum, Message }`):
  - Export removed
  - Export signature changed
  - Record field removed or reordered
  - Enum value removed
  - ABI switched (core ↔ component)
  Enum variants map 1-1 to `DETAIL`/`HINT` strings surfaced in the
  final `PgWasmError`.

## Validation expectations

- `cargo check -p pg_wasm` passes.
- `cargo pgrx regress` `reload.sql` suite:
  - Load → reload with identical bytes → no DDL; assert `fn_oid` and
    `pg_type.oid` unchanged.
  - Load → reload with added record field → `ALTER TYPE ADD
    ATTRIBUTE` issued; OID preserved.
  - Load → reload with renamed export and `breaking_changes_allowed
    = false` → error; catalog unchanged.
  - Load → reload with renamed export and `breaking_changes_allowed
    = true` → unregister old + register new.
- `#[pg_test]` rollback: begin transaction, reload, ROLLBACK; assert
  artifact + catalog both revert.

## Final commit

Flip `reload-orchestration`'s `status:` line to `completed`.
