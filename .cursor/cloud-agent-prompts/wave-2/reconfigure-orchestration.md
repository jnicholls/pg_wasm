# Wave-2 Cloud Agent: `reconfigure-orchestration`

**Branch**: `wave-2/reconfigure-orchestration` (base: `main`)
**PR title**: `[wave-2] reconfigure-orchestration: update policy/limits + generation bump`

Read `@.cursor/cloud-agent-prompts/wave-2/README.md` and
`@.cursor/cloud-agent-prompts/wave-1/README.md` first.

## Task (copied verbatim)

> Implement `lifecycle::reconfigure` that updates `policy` / `limits`
> rows, calls `on-reconfigure` hook, and bumps generation. Confirm
> `StoreLimits` and epoch deadlines pick up the new values on next call
> via integration test.

Design ref: `docs/architecture.md` §§ "Reconfigure lifecycle",
"Generation-driven reload".

## Files you own

- `pg_wasm/src/lifecycle/reconfigure.rs` (new). If the Wave-1 output
  left `pg_wasm/src/lifecycle.rs` as a flat file, convert it to
  `pg_wasm/src/lifecycle/mod.rs` and declare `pub mod reconfigure;`.
  Do not add sibling lifecycle submodules here — those belong to
  Wave 3/4.

## Files you must not touch

- All files listed in `@.cursor/cloud-agent-prompts/wave-2/README.md`.
- `pg_wasm/src/lifecycle/load.rs` / `unload.rs` / `reload.rs` — none
  exist yet, but if another concurrent Wave-3/4 agent added them
  later, do not edit them from this PR.
- `pg_wasm/src/policy.rs`, `pg_wasm/src/config.rs` — Wave-1 outputs;
  read-only.

## Implementation notes

- **SQL entry point** (only the Rust side here; the actual `CREATE
  FUNCTION pg_wasm.reconfigure(...)` lands via pgrx
  `#[pg_extern]`):
  ```rust
  #[pg_extern]
  pub fn reconfigure(
      module_name: &str,
      policy: Option<pgrx::Json>, // serde_json::Value with PolicyOverrides shape
      limits: Option<pgrx::Json>,
  ) -> bool
  ```
- **Flow**:
  1. AuthZ: require `pg_wasm_loader` role membership (or superuser).
     Use `pg_has_role` via SPI or `pgrx::pg_sys::has_privs_of_role`.
  2. Resolve `module_id` from `module_name` via
     `catalog::modules::get_by_name`.
  3. Deserialize `policy` into `config::PolicyOverrides`, `limits`
     into `config::Limits` (via `serde_json::from_value`).
  4. Compute new effective policy: `policy::resolve(GucSnapshot::take(),
     overrides, limits)` — fails closed on any widening attempt.
  5. Update `pg_wasm.modules.policy_json` and
     `pg_wasm.modules.limits_json` atomically within the caller's
     transaction (SPI).
  6. If the module exports an `on-reconfigure` hook, invoke it with a
     serialized blob describing the new effective policy. **Stub**
     the invocation: add a `TODO(wave-4: hooks)` comment since the
     hooks machinery doesn't exist yet; for now, if
     `catalog::exports::get_hook(module_id, "on-reconfigure")`
     returns `Some(_)`, emit a `NOTICE` and skip. Do **not** fail if
     the hook is absent.
  7. `shmem::bump_generation(module_id)`. Other backends will drop
     cached plans on their next trampoline entry (already implemented
     in `registry`).
- **`StoreLimits` pickup** is automatic because `invocation-path`
  (Wave 3) reads the freshly-resolved `EffectivePolicy` per call. No
  pool invalidation needed for reconfigure (instances themselves are
  stateless re: policy in v43 pgrx — per-call `Store` carries the
  limits). Document this in a comment so future readers know the
  invariant.

## Validation expectations

- `cargo check -p pg_wasm` passes.
- `#[pg_test]`:
  - Load a stub module row (can bypass real compile via direct catalog
    insert for this test).
  - Call `pg_wasm.reconfigure(...)` twice — once narrowing, once
    attempting to widen (`allow_wasi_http = true` when GUC is false).
  - Assert the narrow path updates `pg_wasm.modules` and bumps
    generation; the widen path returns a
    `ERRCODE_INSUFFICIENT_PRIVILEGE` error with a helpful message.
- Integration-style test (host + pgrx): assert that after reconfigure,
  a subsequent invocation path's `StoreLimits` shows the new values.
  If the invocation path isn't wired yet (Wave 3 dependency), write a
  unit that at least asserts `EffectivePolicy::resolve` returns the
  new numbers after catalog row changes, plus a `TODO(wave-3)` note
  to extend the check post-merge.

## Final commit

Flip `reconfigure-orchestration`'s `status:` line to `completed`.
