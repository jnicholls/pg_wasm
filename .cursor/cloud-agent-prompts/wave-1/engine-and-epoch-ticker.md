# Wave-1 Cloud Agent: `engine-and-epoch-ticker`

**Branch**: `wave-1/engine-and-epoch-ticker` (base: `main`)
**PR title**: `[wave-1] engine-and-epoch-ticker: shared Wasmtime engine and epoch ticker thread`

Before doing anything else, read `@.cursor/cloud-agent-prompts/wave-1/README.md`
in full and comply with every shared rule in it.

## Task (copied verbatim from the plan todo)

> Implement `runtime::engine::shared_engine()` returning a
> lazily-initialized `wasmtime::Engine` (v43). Configure via
> `wasmtime::Config` using only methods that exist in v43:
> `wasm_component_model(true)`, `epoch_interruption(true)`,
> `consume_fuel(pg_wasm.fuel_enabled)`, `cache(None)` (we manage our own
> on-disk cache), and `parallel_compilation(false)`. Do NOT call the
> removed/deprecated `async_support` or `cache_config_load_default`
> methods. Drive the epoch ticker thread from `_PG_init` reading
> `pg_wasm.epoch_tick_ms`; the thread holds an `EngineWeak` (from
> `Engine::weak()`) and invokes `Engine::increment_epoch()` per tick,
> calling `EngineWeak::upgrade()` each tick so the thread exits naturally
> when the last `Engine` reference is dropped.

Authoritative design sections:
`docs/architecture.md` §§ "Wasmtime configuration", "Epoch interruption
lifecycle", "Engine lifecycle".

API references (pinned to 43.0.0):
- `wasmtime::Config`:
  https://docs.rs/wasmtime/43.0.0/wasmtime/struct.Config.html
- `wasmtime::Engine`:
  https://docs.rs/wasmtime/43.0.0/wasmtime/struct.Engine.html
- `EngineWeak`:
  https://docs.rs/wasmtime/43.0.0/wasmtime/struct.EngineWeak.html

## Files you own

- `pg_wasm/src/runtime.rs` — currently a flat file with a `runtime::init()`
  stub. You may keep it flat or convert to `pg_wasm/src/runtime/mod.rs`
  plus `pg_wasm/src/runtime/engine.rs`. If you convert to a directory,
  delete the old `runtime.rs`.

`_PG_init` already calls `runtime::init()`. Fill that function in — it
should spawn the epoch-ticker thread (lazily via `OnceLock`) and register
an atexit hook that joins/stops the thread on backend shutdown.

## Files you must not touch

- `pg_wasm/src/lib.rs`.
- `pg_wasm/src/guc.rs` — the GUCs (`pg_wasm.fuel_enabled`,
  `pg_wasm.epoch_tick_ms`, etc.) are already defined; read them via
  `crate::guc::<name>.get()`, do not redefine.
- Every other `pg_wasm/src/*.rs` file.
- `Cargo.toml`, `pg_wasm/Cargo.toml`, `pg_wasm/pg_wasm.control`.
- Any line of the plan file other than your own todo's `status:` line.

## Implementation notes

- **`shared_engine()`** — `pub(crate) fn shared_engine() -> &'static wasmtime::Engine`
  backed by `OnceLock<wasmtime::Engine>`. On first call, build a
  `wasmtime::Config` using **only** the v43 methods listed in the task
  (do not call removed ones). Panic-free: if `Engine::new(cfg)` fails,
  return a `PgWasmError::Internal` via an alternate `try_shared_engine()
  -> Result<&'static Engine, PgWasmError>` rather than panicking. The
  shared path itself can call `try_shared_engine().expect(...)` only if
  you are sure it runs inside backends where any failure should be fatal;
  otherwise prefer the result form.
- **`runtime::init()`** — called from `_PG_init`. Spawn a single
  OS-thread (not tokio) that:
  - Reads `pg_wasm.epoch_tick_ms` **once** at startup. (Re-reading on
    every tick is a possible future refinement; not in scope here.)
  - Obtains `EngineWeak` via `shared_engine().weak()`.
  - In a loop: sleep `tick_ms`, call `upgrade()`. If `None`, break. Else
    call `Engine::increment_epoch()` on the upgraded `Engine`.
- **atexit hook**: register via `pgrx::pg_sys::on_proc_exit` (or the pgrx
  helper if one exists) so the backend requests the thread to exit and
  joins it. Use an `AtomicBool` shutdown flag on top of the `EngineWeak`
  drop so tight-loop termination is clean on shutdown.
- **Do not store pgrx handles in the thread.** Pass only plain Rust types
  (`Duration`, `EngineWeak`, `Arc<AtomicBool>`).
- **Do not install WASI or WASI-HTTP here.** The linker lives on
  `runtime::component` in Wave 2. This PR is just the engine + ticker.

## Validation expectations

- `cargo check -p pg_wasm` passes.
- Host-only `#[test]`:
  - Build an `Engine` via the same `Config` constructor; assert
    `Engine::is_compatible_with_module` smokes a minimal hand-rolled
    module header; assert `Engine::precompile_compatibility_hash()` is
    reproducible across two calls.
  - Manually exercise the ticker loop function (factor it out so it
    accepts an `Arc<AtomicBool>` + `EngineWeak` + tick duration) and
    confirm it exits when the flag flips or the engine drops.
- A narrow `#[pg_test]` that asserts `shared_engine()` returns the same
  pointer on two calls within the same backend.

## Final commit

Flip the `status:` line for `engine-and-epoch-ticker` in
`.cursor/plans/pg_wasm_extension_implementation_v2.plan.md` from `pending`
to `completed`. No other edits to that file.
