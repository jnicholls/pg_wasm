# Wave-3 Cloud Agent: `metrics-and-views`

**Branch**: `wave-3/metrics-and-views` (base: `main`)
**PR title**: `[wave-3] metrics-and-views: SRF views over catalog + shmem counters`

Read `@.cursor/cloud-agent-prompts/wave-3/README.md` and
`@.cursor/cloud-agent-prompts/wave-1/README.md` first.

## Task (copied verbatim)

> Implement `views::{modules, functions, stats, wit_types,
> policy_effective}` as SRF table functions backed by catalog rows and
> shmem atomics. Add grants so `pg_wasm_reader` can read `stats()`.
> Add regress tests asserting counter shape and monotonicity.

Design ref: `docs/architecture.md` §§ "Observability views",
"Role grants".

## Files you own

- `pg_wasm/src/views.rs`
- `pg_wasm/sql/pg_wasm--0.1.0.sql` — append-only additions for view
  registration + grants. **Do not** modify the existing catalog DDL
  authored by Wave-1 `catalog-schema`.
- Optional: `pg_wasm/tests/pg_regress/{sql,expected}/views.sql`.

## Files you must not touch

- All files listed in `@.cursor/cloud-agent-prompts/wave-3/README.md`.
- `pg_wasm/src/shmem.rs` / `catalog/*` — read-only.
- Other Wave-3 owners' files.

## Implementation notes

- **`views::modules()` → SRF** returning `module_id, name, origin,
  digest, loaded_at, policy_json, limits_json, shared`.
- **`views::functions()` → SRF** returning `module_name, export_name,
  fn_oid, arg_types, ret_type, abi, last_seen_generation`.
- **`views::wit_types()` → SRF** returning `module_name, type_key,
  kind, pg_type_oid, last_seen_generation`.
- **`views::policy_effective()` → SRF** returning the **resolved**
  policy/limits for every loaded module — i.e. after
  `policy::resolve(GucSnapshot::take(), ...)`. One row per module. Use
  JSONB for the structured shape; column names `policy_json`,
  `limits_json`.
- **`views::stats()` → SRF** returning `module_name, export_name,
  invocations, traps, fuel_used_total, last_invocation_at, shared`.
  Backed by `shmem::read_export_counters(module_id, export_index)`;
  for modules that overflowed the slot table, `shared = false` and
  the counters come from process-local atomics (fallback state is
  already emitted by Wave 1). The view does **not** attempt to merge
  counters across backends beyond what shmem already aggregates.
- **pgrx 0.18 SRF**: use `TableIterator<'_, (..., )>` with
  `#[pg_extern(parallel_safe)]` where safe. `stats()` should be
  `parallel_safe = false` (reads shmem under a lock for consistency)
  — check pgrx 0.18 conventions for the correct attribute.
- **Grants** (append-only SQL):
  ```sql
  GRANT SELECT ON pg_wasm.modules, pg_wasm.exports, pg_wasm.wit_types,
      pg_wasm.dependencies TO pg_wasm_reader;
  GRANT EXECUTE ON FUNCTION pg_wasm.modules(), pg_wasm.functions(),
      pg_wasm.wit_types(), pg_wasm.policy_effective(),
      pg_wasm.stats() TO pg_wasm_reader;
  ```
  Put in the installation SQL. No dynamic granting from Rust.

## Validation expectations

- `cargo check -p pg_wasm` passes.
- `cargo pgrx regress` new suite `views.sql`:
  - Seed a fake module + exports via catalog inserts.
  - `SELECT * FROM pg_wasm.modules() ORDER BY name` golden.
  - Call `invocation-path` a few times (using the Wave-2 test-only
    helper or a real loaded module if available by merge time);
    assert `stats()` counters strictly increase across calls
    (monotonicity).
  - Assert `pg_wasm_reader` can `SELECT` from every view (GRANT
    check).

## Final commit

Flip `metrics-and-views`'s `status:` line to `completed`.
