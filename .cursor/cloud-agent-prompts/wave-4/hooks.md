# Wave-4 Cloud Agent: `hooks`

**Branch**: `wave-4/hooks` (base: `main`)
**PR title**: `[wave-4] hooks: optional on-load/on-unload/on-reconfigure invocations`

Read `@.cursor/cloud-agent-prompts/wave-4/README.md` and
`@.cursor/cloud-agent-prompts/wave-1/README.md` first.

## Task (copied verbatim)

> Implement `hooks::{on_load, on_unload, on_reconfigure}` invocations
> with config blob passing. Hooks are optional component exports with
> stable names; absence is not an error. on-unload failures are
> logged, not fatal.

Design ref: `docs/architecture.md` §§ "Module lifecycle hooks".

## Files you own

- `pg_wasm/src/hooks.rs` — currently a doc-comment stub. Replace its
  body with the full implementation.

## Files you must not touch

- All other files. `hooks` **calls** into `runtime::component`,
  `runtime::pool`, `mapping::composite`, and the registry; it does
  not edit them.
- `pg_wasm/src/lifecycle/*` — the lifecycle files call `hooks::*` but
  the actual `TODO(wave-4: hooks)` slots are expected to already be
  at well-defined call sites. You may **not** change lifecycle file
  bodies beyond removing the `TODO` comment line adjacent to the
  real call — limit that to a single-line deletion per occurrence.
  If removing the `TODO` requires rewriting control flow, STOP and
  note it in the PR description.

## Implementation notes

- **Hook names** (from the module's own WIT world):
  - `on-load: func(config: string) -> result<_, string>`
  - `on-unload: func() -> result<_, string>`
  - `on-reconfigure: func(config: string) -> result<_, string>`
- **Lookup** via `wasmtime::component::Instance::get_func(&mut store,
  "on-load")` etc. Absence returns `Ok(None)` — **not** an error.
- **Public API**:
  ```rust
  pub(crate) fn on_load(module_id: u64, config_blob: &serde_json::Value)
      -> Result<(), PgWasmError>;
  pub(crate) fn on_unload(module_id: u64) -> Result<(), PgWasmError>;
  pub(crate) fn on_reconfigure(module_id: u64, effective: &EffectivePolicy)
      -> Result<(), PgWasmError>;
  ```
  - Each resolves the module's `Component` (via the registry),
    instantiates or borrows a pool slot, looks up the hook, and calls
    it.
  - Config blob = serialized JSON string so Wasm side can deserialize
    without being coupled to our internal Rust types.
- **Error semantics**:
  - `on_load`: failure → `PgWasmError::InvalidConfiguration(msg)`.
    Caller (`load-orchestration`) will roll back.
  - `on_unload`: failure → `log::warn!` equivalent via
    `ereport(WARNING, ...)`; return `Ok(())`. The unload proceeds.
  - `on_reconfigure`: failure → `PgWasmError::InvalidConfiguration`.
    Caller rolls back.
- **Instance handling**: call through `runtime::pool::acquire` to
  reuse pool instances. After the hook returns (whether `Ok` or
  `Err`), call `Func::post_return` on the hook func before releasing
  the pooled instance.
- **Policy for hook calls**: use the **same** `EffectivePolicy` as
  regular invocations would (read limits/fuel/epoch from the catalog
  row). Hooks are not privileged.
- Do not swallow panics here; let `invocation-path`-style panic
  handling apply at the caller boundary (`lifecycle::*` wraps with
  `catch_unwind` already).

## Validation expectations

- `cargo check -p pg_wasm` passes.
- Host-only `#[test]` using an instance built via
  `wit_component::ComponentEncoder`:
  - Module without any hooks → all three `on_*` calls return
    `Ok(())` with no side effect.
  - Module with `on-load` that returns `err` → `on_load` returns
    `Err(InvalidConfiguration)`.
  - Module with `on-unload` that traps → `on_unload` still returns
    `Ok(())`; a `WARNING` is emitted.

## Final commit

Flip `hooks`'s `status:` line to `completed`.
