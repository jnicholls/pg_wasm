# Wave-3 Cloud Agent: `invocation-path`

**Branch**: `wave-3/invocation-path` (base: `main`)
**PR title**: `[wave-3] invocation-path: full trampoline borrow-pool-call-unmarshal path`

Read `@.cursor/cloud-agent-prompts/wave-3/README.md` and
`@.cursor/cloud-agent-prompts/wave-1/README.md` first.

## Task (copied verbatim)

> Flesh out `trampoline::pg_wasm_udf_trampoline` to borrow a pooled
> instance, build per-call `StoreLimits` via
> `wasmtime::StoreLimitsBuilder` and attach with `Store::limiter`, set
> fuel via `Store::set_fuel` (and read with `Store::get_fuel`
> afterwards for metrics), set the epoch deadline via
> `Store::set_epoch_deadline` (ticks = deadline_ms / epoch_tick_ms),
> marshal args, call the typed export, unmarshal, and update shmem
> counters. Downcast `wasmtime::Error` via
> `err.downcast_ref::<wasmtime::Trap>()`: `Trap::Interrupt` ->
> `ERRCODE_QUERY_CANCELED`, `Trap::OutOfFuel` ->
> `ERRCODE_PROGRAM_LIMIT_EXCEEDED`, other `Trap` variants ->
> `PgWasmError::Trap { kind }` with
> `ERRCODE_EXTERNAL_ROUTINE_EXCEPTION`. Wrap in
> `std::panic::catch_unwind`.

Design ref: `docs/architecture.md` §§ "Invocation path", "Per-call
Store and limits", "Trap downcasting".

## Files you own

- `pg_wasm/src/trampoline.rs` — replaces the Wave-1 stub body with the
  full path. The file is still "owned" by this task; keep the public
  C entry point symbol name (`pg_wasm_udf_trampoline`) unchanged for
  compatibility with `proc_reg`.

## Files you must not touch

- All files listed in `@.cursor/cloud-agent-prompts/wave-3/README.md`.
- `pg_wasm/src/registry.rs` — Wave-1 stub traits (`GenerationSource`,
  `CatalogSource`). You may replace the default stubs with
  real-sourced impls here in `trampoline.rs`, but leave `registry.rs`
  itself unchanged. Adding new methods to `Registry` via extension
  traits in `trampoline.rs` is ok.
- `pg_wasm/src/runtime/pool.rs` — sibling Wave-2; read-only.
- `pg_wasm/src/mapping/*` — read-only.

## Implementation notes

- **Top-level flow** (keep the function short; dispatch to helpers):
  1. `catch_unwind` around the whole body. Map panics to
     `ERRCODE_INTERNAL_ERROR`.
  2. Resolve `fn_oid` → `RegistryEntry`. If miss →
     `registry::refresh_from_catalog()` then retry. Second miss →
     `PgWasmError::NotFound`.
  3. Load the `EffectivePolicy` from the cached plan (or re-resolve
     via `policy::resolve(GucSnapshot::take(), stored_overrides,
     stored_limits)` each call — cheap per the `policy-resolve`
     design).
  4. `runtime::pool::acquire(module_id)` → `PooledInstance`. On drop
     (RAII), the pool reclaims.
  5. Build per-call `StoreLimits` via
     `wasmtime::StoreLimitsBuilder::new().memory_size(policy.max_memory_pages
     * 65536).instances(1).build()`. Attach with `Store::limiter(|s|
     &mut s.limits)`.
  6. If `policy.fuel_per_invocation` is `Some(n)`:
     `store.set_fuel(n)?`. Record `fuel_before` for metrics.
  7. `epoch_tick_ms = crate::guc::EPOCH_TICK_MS.get()` (non-zero
     fallback to 10). `deadline_ticks = max(1,
     policy.invocation_deadline_ms.saturating_div(epoch_tick_ms))`.
     `store.set_epoch_deadline(deadline_ticks)`.
  8. Marshal args: walk the cached `MarshalPlan` (stored on the
     `RegistryEntry`) calling `mapping::{scalars,composite,list}::
     datum_to_val`.
  9. Invoke:
     - Component path: `func.call(&mut store, &args_val,
       &mut result_val)?; func.post_return(&mut store)?;`
     - Core path: `typed_func.call(&mut store, args_tuple)?;`
  10. Unmarshal the result back to a `Datum` via the same
      `MarshalPlan` (reverse direction).
  11. Metrics: read `store.get_fuel()`; compute `fuel_used =
      fuel_before - fuel_after` if fuel is enabled. Call
      `shmem::incr_export_counter(module_id, export_index,
      Counter::Invocations)` and a fuel-used counter.
  12. Return the `Datum` through `fcinfo`.
- **Trap downcast helper**:
  ```rust
  fn map_wasmtime_err(e: wasmtime::Error) -> PgWasmError {
      if let Some(trap) = e.downcast_ref::<wasmtime::Trap>() {
          return match trap {
              wasmtime::Trap::Interrupt => PgWasmError::Timeout(
                  "invocation interrupted by epoch deadline".into()),
              wasmtime::Trap::OutOfFuel => PgWasmError::ResourceLimitExceeded(
                  "fuel exhausted".into()),
              other => PgWasmError::Trap { kind: format!("{other}") },
          };
      }
      PgWasmError::Internal(format!("{e:#}"))
  }
  ```
  Add `PgWasmError::Trap { kind: String }` as a new variant at the
  bottom of the enum (append-only) with `sqlstate` mapping to
  `ERRCODE_EXTERNAL_ROUTINE_EXCEPTION`.
- **`post_return` safety**: call it before `store` is reused. If the
  typed call errors, do **not** call `post_return`; discard the
  `PooledInstance` (do not return it to the pool). Extend
  `runtime::pool::PooledInstance` with a `poison()` method for this;
  if that API is missing, add a narrow method on the sibling
  `runtime::pool` file — and call this a minor cross-task edit,
  permitted only if unavoidable. Prefer flagging the missing API in
  the PR description and using `drop(pooled)` with a TODO.

## Validation expectations

- `cargo check -p pg_wasm` passes.
- Host-only `#[test]`: trap downcast helper covers each `Trap` variant
  relevant to v43.
- `#[pg_test]` + a fixture component that:
  - Returns normally → counters increment, fuel-used metric non-zero
    when fuel is enabled.
  - Infinite-loop export → interrupted by epoch deadline →
    `ERRCODE_QUERY_CANCELED`.
  - Exhausts fuel → `ERRCODE_PROGRAM_LIMIT_EXCEEDED`.
  - Intentional `unreachable!` → `ERRCODE_EXTERNAL_ROUTINE_EXCEPTION`
    with the trap kind in DETAIL.

## Final commit

Flip `invocation-path`'s `status:` line to `completed`.
