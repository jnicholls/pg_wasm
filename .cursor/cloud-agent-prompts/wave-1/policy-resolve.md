# Wave-1 Cloud Agent: `policy-resolve`

**Branch**: `wave-1/policy-resolve` (base: `main`)
**PR title**: `[wave-1] policy-resolve: policy narrowing with EffectivePolicy::resolve`

Before doing anything else, read `@.cursor/cloud-agent-prompts/wave-1/README.md`
in full and comply with every shared rule in it.

## Task (copied verbatim from the plan todo)

> Define `config::{LoadOptions, PolicyOverrides, Limits}` and
> `policy::{EffectivePolicy, resolve}`. Enforce narrowing semantics
> (overrides can only deny/tighten). Cover with host-only unit tests for
> every combination.

Authoritative design sections:
`docs/architecture.md` §§ "Policy narrowing", "Per-module overrides",
"Limits", "Sandbox configuration".

## Files you own

- `pg_wasm/src/config.rs`
- `pg_wasm/src/policy.rs`

## Files you must not touch

- `pg_wasm/src/lib.rs`.
- `pg_wasm/src/guc.rs` — every GUC is already defined by `errors-and-guc`.
  You **read** those GUCs from `config::` / `policy::` but do not modify
  `guc.rs`.
- Every other `pg_wasm/src/*.rs` file.
- `Cargo.toml`, `pg_wasm/Cargo.toml`, `pg_wasm/pg_wasm.control`.
- Any line of the plan file other than your own todo's `status:` line.

## Implementation notes

- **`config::LoadOptions`** — one struct mirroring the `pg_wasm.load(...)`
  function's JSON argument (the public SQL API that later lifecycle tasks
  will expose). At minimum: `abi: Option<Abi>`, `replace_exports: bool`,
  `on_load_hook: bool`, `limits: Option<Limits>`, `overrides:
  Option<PolicyOverrides>`, `cascade: Option<bool>`,
  `breaking_changes_allowed: bool`. Derive `Clone, Debug, Default,
  Deserialize, Serialize` (deriv order alphabetical per coding standards).
- **`config::PolicyOverrides`** — per-module overrides. Mirrors the GUC
  flags: `allow_wasi`, `allow_wasi_{stdio,env,fs,net,http}`,
  `wasi_preopens`, `allowed_hosts`, `allow_spi`. Use `Option<bool>` for
  booleans and `Option<Vec<String>>` / `Option<BTreeMap<String,String>>`
  for list-valued ones. `None` = inherit GUC.
- **`config::Limits`** — `max_memory_pages`, `instances_per_module`,
  `fuel_per_invocation`, `invocation_deadline_ms`. Same `Option<_>` +
  `None = inherit` pattern.
- **`policy::EffectivePolicy`** — fully resolved, no `Option<_>` left.
  Fields are concrete values in the same shape as `PolicyOverrides` +
  `Limits`.
- **`policy::resolve(guc_snapshot, overrides, limits) -> EffectivePolicy`**
  — applies narrowing:
  - Booleans: `effective = guc && override.unwrap_or(guc)`. Override can
    flip `true` → `false` but never `false` → `true`.
  - `wasi_preopens`: must be a subset of the GUC's preopens.
  - `allowed_hosts`: must be a subset of the GUC's allowed hosts.
  - Numeric limits: `effective = min(guc_ceiling,
    override.unwrap_or(guc_ceiling))`. Override can only shrink.
  - If any override attempts to widen, return
    `PgWasmError::PermissionDenied` with a concrete message naming the
    field.
- **`guc_snapshot`** — define a `pub(crate) struct GucSnapshot` here that
  packages the current GUC values into an immutable bundle. Read pgrx
  `GucSetting` values via `guc::<name>.get()` (see `pg_wasm/src/guc.rs`
  for the definitions already registered). Do not cache across calls —
  take the snapshot per `resolve`.

## Validation expectations

- `cargo check -p pg_wasm` passes.
- Host-only `#[test]`s (**required** per plan wording "for every
  combination"):
  - GUC permits, override denies → effective denies.
  - GUC denies, override permits → `PermissionDenied` with field name.
  - GUC denies, override denies → denies.
  - GUC permits, override permits → permits.
  - `wasi_preopens` subset/superset/equal/disjoint.
  - `allowed_hosts` subset/superset/equal/disjoint.
  - Limits: override above ceiling → `PermissionDenied`; below →
    effective = override; absent → effective = ceiling.
- Use a `GucSnapshot` constructor (or test-only `new_for_test`) that lets
  you construct snapshots without touching real GUCs.

## Final commit

Flip the `status:` line for `policy-resolve` in
`.cursor/plans/pg_wasm_extension_implementation_v2.plan.md` from `pending`
to `completed`. No other edits to that file.
