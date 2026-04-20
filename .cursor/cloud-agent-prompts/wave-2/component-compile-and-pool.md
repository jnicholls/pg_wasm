# Wave-2 Cloud Agent: `component-compile-and-pool`

**Branch**: `wave-2/component-compile-and-pool` (base: `main`)
**PR title**: `[wave-2] component-compile-and-pool: AOT component compile + per-module instance pool`

Read `@.cursor/cloud-agent-prompts/wave-2/README.md` and
`@.cursor/cloud-agent-prompts/wave-1/README.md` first.

## Task (copied verbatim)

> Implement `runtime::component` to compile a
> `wasmtime::component::Component` (via `Component::from_binary`),
> AOT-precompile a `.cwasm` to disk via
> `Engine::precompile_component(bytes)`, and record the artifact's
> `Engine::precompile_compatibility_hash` alongside it. On cold backends
> reload with `Engine::detect_precompiled_file` + the unsafe
> `Component::deserialize_file`. Stand up a
> `wasmtime::component::Linker` and wire WASI via
> `wasmtime_wasi::p2::add_to_linker_sync` (the v43 path; the older
> `wasmtime_wasi::preview2::*` module was renamed to `p2`) with a
> per-store `WasiCtx` built from `wasmtime_wasi::WasiCtxBuilder` and a
> `WasiView` impl returning `WasiCtxView { ctx, table }`. Wire HTTP
> (when enabled) via `wasmtime_wasi_http::p2::add_to_linker_sync`.
> Implement `runtime::pool` with a per-module bounded instance pool
> sized by `pg_wasm.instances_per_module` (new GUC).

Design ref: `docs/architecture.md` §§ "Component compile and pool",
"WASI wiring", "Per-backend instance pool".

Pinned API:
- `component::Component::{from_binary, deserialize_file}`
- `Engine::{precompile_component, precompile_compatibility_hash,
  detect_precompiled_file}`
- `wasmtime_wasi::p2::add_to_linker_sync`,
  `wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView, WasiCtxView}`
- `wasmtime_wasi_http::p2::add_to_linker_sync`,
  `wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView, WasiHttpCtxView}`

All links in `.cursor/plans/pg_wasm_extension_implementation_v2.plan.md`
→ "References" section.

## Files you own

- `pg_wasm/src/runtime/component.rs` (new)
- `pg_wasm/src/runtime/pool.rs` (new)

Declare both in `pg_wasm/src/runtime/mod.rs` — the only edit to that
file you may make is adding `pub mod component;` and `pub mod pool;`.

## Files you must not touch

- All files listed in `@.cursor/cloud-agent-prompts/wave-2/README.md`.
- `pg_wasm/src/runtime/engine.rs` — Wave-1 output; read-only.
- `pg_wasm/src/runtime/core.rs` (sibling).
- `pg_wasm/src/mapping/*` (sibling Wave-2 territory).

## Implementation notes

- **Compile**:
  - `compile(engine, bytes) -> Result<Component, PgWasmError>` wraps
    `Component::from_binary`.
  - `precompile_to(engine, bytes, out_path) -> Result<[u8;32],
    PgWasmError>`:
    - Call `engine.precompile_component(bytes)` → `Vec<u8>`.
    - Write to `out_path` via `artifacts::write_atomic`.
    - Compute `engine.precompile_compatibility_hash()` and return it to
      the caller to persist alongside (the caller decides whether to
      store in a sidecar file or on the catalog row).
- **Cold-backend reload**:
  - `load_precompiled(engine, path, expected_hash) -> Result<Component,
    PgWasmError>`:
    - Call `Engine::detect_precompiled_file(path)`. If it returns
      `None` or `Some(Precompiled::Module)` (we expect `Component`),
      return `PgWasmError::InvalidModule(_)` with a `stale_cache`
      hint.
    - Compare the engine's current `precompile_compatibility_hash()`
      to `expected_hash`; mismatch → stale hint.
    - If both checks pass: call the **unsafe**
      `Component::deserialize_file`. Keep the `unsafe` block small
      and document the safety argument (we verified the source file
      and hash; file is under `$PGDATA/pg_wasm/` which only pgrx
      owns).
- **Linker** (`build_linker(engine, policy) -> Result<Linker<StoreCtx>,
  PgWasmError>`):
  - Wire `wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?` always.
  - If `policy.allow_wasi_http` is true, wire
    `wasmtime_wasi_http::p2::add_to_linker_sync(&mut linker)?`.
  - Build a `WasiView`/`WasiHttpView`-implementing `StoreCtx` struct
    that owns `WasiCtx`, `wasmtime::component::ResourceTable`, and
    `WasiHttpCtx` (all opt-in via policy). On calls the view returns
    the proper `*CtxView` references.
  - **Host interfaces** (`pg_wasm:host/log`, `pg_wasm:host/query`) are
    owned by the later `host-interfaces` task; do not add them here,
    but leave a clear `TODO(wave-3: host-interfaces)` comment with a
    hook into the linker so that task can slot in.
- **Per-store `WasiCtx`**: `WasiCtxBuilder::new()` + preopens from
  `policy.wasi_preopens` + env from GUC when `allow_wasi_env` + stdio
  routed to `/dev/null`-style by default; policy flags gate each.
- **Instance pool** (`runtime::pool::InstancePool`):
  - Per-module bounded queue of (pre-instantiated, optionally warmed)
    `wasmtime::Store<StoreCtx>` + `wasmtime::component::Instance`
    pairs.
  - Size = `policy.instances_per_module` (GUC
    `pg_wasm.instances_per_module` — already registered by Wave-1
    `errors-and-guc`; read via `crate::guc::INSTANCES_PER_MODULE.get()`).
  - `acquire(module_id) -> PooledInstance` lazily creates or blocks
    until one is free (`std::sync::Mutex` + `Condvar`, no async). On
    backpressure beyond a short timeout return
    `PgWasmError::ResourceLimitExceeded`.
  - `release(pooled)` returns the slot. Call
    `Func::post_return` on the caller side, not the pool.
  - **Lifecycle**: on module unload, drain and drop all pooled
    instances. Expose `drain(module_id)`.

## Validation expectations

- `cargo check -p pg_wasm` passes.
- Host-only `#[test]`:
  - Build a tiny component at test time (via
    `wit_component::ComponentEncoder`) that imports nothing;
    precompile, round-trip via `load_precompiled`, instantiate.
  - Pool: acquire/release concurrency test with `N > pool_size`
    threads; assert waiters unblock in order and none acquire
    simultaneously beyond the cap.
- A narrow `#[pg_test]` that exercises the `$PGDATA`-anchored precompile
  path via `artifacts::write_atomic`.

## Final commit

Flip `component-compile-and-pool`'s `status:` line to `completed`.
