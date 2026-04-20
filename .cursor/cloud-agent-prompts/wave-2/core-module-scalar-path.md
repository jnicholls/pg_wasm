# Wave-2 Cloud Agent: `core-module-scalar-path`

**Branch**: `wave-2/core-module-scalar-path` (base: `main`)
**PR title**: `[wave-2] core-module-scalar-path: core-module ABI with scalar I/O`

Read `@.cursor/cloud-agent-prompts/wave-2/README.md` and
`@.cursor/cloud-agent-prompts/wave-1/README.md` first; comply with every
shared rule in both.

## Task (copied verbatim from the plan todo)

> Implement `runtime::core` for core modules with scalar-only ABI
> (i32/i64/f32/f64/bool). Implement `mapping::scalars` and end-to-end
> load -> trampoline -> call on a fixture `add_i32.wat`. Verify via
> pg_regress golden output.

Design ref: `docs/architecture.md` §§ "Core module degraded path",
"Scalar-only mapping".

Pinned API:
- `wasmtime::Module`, `wasmtime::Instance`, `wasmtime::TypedFunc`,
  `wasmtime::Linker`:
  https://docs.rs/wasmtime/43.0.0/wasmtime/

## Files you own

- `pg_wasm/src/runtime/core.rs` (new) — compile + cache `wasmtime::Module`
  per module, build a per-call `Store`, resolve the export, call as a
  typed scalar function.
- `pg_wasm/src/mapping/scalars.rs` (new) — `Datum` ↔ scalar-Val converters
  for bool, int2/int4/int8 (mapping to i32/i64 with range checks),
  float4/float8 (f32/f64).
- `pg_wasm/fixtures/core/add_i32.wat` (new) — fixture with a single
  `(func (export "add") (param i32 i32) (result i32))`.
- `pg_wasm/fixtures/core/echo_mem.wat` (new) — optional round-trip
  smoke test for future linear-memory path (keep simple: no memory
  allocator; scalar echo of one i64 is acceptable).
- `pg_wasm/tests/pg_regress/sql/core_scalar.sql` + matching
  `pg_wasm/tests/pg_regress/expected/core_scalar.out`.

If Wave 1 left `pg_wasm/src/mapping.rs` flat, convert it to
`pg_wasm/src/mapping/mod.rs` and declare `pub mod scalars;` (do not
add any other submodules — those belong to
`component-marshal-dynamic`).

Similarly convert `pg_wasm/src/runtime.rs` → `runtime/mod.rs` if the
`engine-and-epoch-ticker` agent did not already do so, preserving every
prior symbol.

## Files you must not touch

- All files listed under "Do not touch" in
  `@.cursor/cloud-agent-prompts/wave-2/README.md`.
- `pg_wasm/src/runtime/engine.rs` (Wave-1 output; read-only).
- `pg_wasm/src/mapping/composite.rs` and `pg_wasm/src/mapping/list.rs`
  (sibling Wave-2 tasks).
- `pg_wasm/src/runtime/component.rs` and `pg_wasm/src/runtime/pool.rs`
  (sibling Wave-2 tasks).

## Implementation notes

- **`runtime::core::Loaded`** holds the compiled `Module` and a
  `wasmtime::Linker` with no imports (core modules are self-contained
  for this scalar path).
- **`runtime::core::compile(engine, bytes) -> Result<Loaded, PgWasmError>`**
  — uses the shared engine from `runtime::engine::shared_engine()`.
- **Per-call path** `runtime::core::invoke(loaded, export_name, args,
  policy) -> Result<Val, PgWasmError>`:
  - Build `Store<StoreCtx>` with `StoreLimits` derived from `policy`
    (`StoreLimitsBuilder` set `memory_size` from `max_memory_pages`
    when the module has memory; otherwise skip).
  - If `policy.fuel_per_invocation` is `Some`, `Store::set_fuel`.
  - Set `Store::set_epoch_deadline(deadline_ticks)` where
    `deadline_ticks = policy.invocation_deadline_ms / epoch_tick_ms`
    (both from GUCs via `crate::guc::<name>.get()`; compute using
    integer math, minimum 1).
  - Instantiate through the Linker, resolve export, downcast to the
    matching `TypedFunc<(args), (ret)>`, call.
- **`mapping::scalars`**: one small module of `from_datum`/`to_datum`
  helpers per supported PG type. Range-check when PG type is narrower
  than the Wasm type (e.g. PG `int2` → Wasm `i32`).
- **End-to-end wiring is out of scope** for this PR — we don't yet
  have `load-orchestration`. Instead, expose a temporary test-only SQL
  function `pg_wasm._core_invoke_scalar(bytes bytea, export text,
  i32args int[]) returns int` behind `#[cfg(any(test, feature =
  "pg_test"))]` that wraps the path for regress coverage. This test
  function is removed in a later wave; mark with a `// TODO(wave-4):
  replace with lifecycle::load` comment.
- **pg_regress**: load the WAT fixture via
  `pg_read_binary_file(...)` pointing at a path injected by the test
  harness, or inline the bytes as a bytea literal. `ORDER BY` outputs;
  no timing-sensitive assertions.

## Validation expectations

- `cargo check -p pg_wasm` passes.
- `cargo pgrx regress` passes the new suite.
- Host-only `#[test]` for scalar converters covering each PG type and
  over/underflow.

## Final commit

Flip `core-module-scalar-path`'s `status:` line in the plan from
`pending` to `completed`. No other plan-file edits.
