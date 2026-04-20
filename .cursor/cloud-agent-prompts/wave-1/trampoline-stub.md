# Wave-1 Cloud Agent: `trampoline-stub`

**Branch**: `wave-1/trampoline-stub` (base: `main`)
**PR title**: `[wave-1] trampoline-stub: C trampoline and generation-aware fn_oid registry`

Before doing anything else, read `@.cursor/cloud-agent-prompts/wave-1/README.md`
in full and comply with every shared rule in it.

## Task (copied verbatim from the plan todo)

> Add `trampoline::pg_wasm_udf_trampoline` C entry point that resolves
> `fn_oid` through `registry::FN_OID_MAP`. Initially returns a constant;
> wire `registry` with a generation-aware cache that refreshes from
> catalog on miss.

Authoritative design sections:
`docs/architecture.md` §§ "One trampoline, many pg_proc rows",
"Generation-aware registry cache", "Invocation path".

## Files you own

- `pg_wasm/src/trampoline.rs`
- `pg_wasm/src/registry.rs`

## Files you must not touch

- `pg_wasm/src/lib.rs`.
- Every other `pg_wasm/src/*.rs` file (catalog CRUD is owned by
  `catalog-schema`; shmem generation is owned by `shmem-and-generation`).
- `Cargo.toml`, `pg_wasm/Cargo.toml`, `pg_wasm/pg_wasm.control`.
- Any line of the plan file other than your own todo's `status:` line.

## Implementation notes

- **Trampoline symbol**: `pg_wasm_udf_trampoline` — the single C entry
  point pgrx wires every `pg_wasm`-owned `pg_proc` row to. Signature:
  ```rust
  #[pg_guard]
  pub unsafe extern "C-unwind" fn pg_wasm_udf_trampoline(
      fcinfo: pgrx::pg_sys::FunctionCallInfo,
  ) -> pgrx::pg_sys::Datum
  ```
  For this task the body just:
  1. Resolves `flinfo->fn_oid` via `registry::resolve_fn_oid`.
  2. If the map has no entry, calls `registry::refresh_from_catalog()`
     and retries once.
  3. Returns a placeholder `Datum::from(0i32)` (we'll replace this with
     real invocation in `invocation-path` in Wave 4).
- **`registry::FN_OID_MAP`**: per-process cache. Use a
  `parking_lot`-free pattern (no new deps):
  `std::sync::OnceLock<RwLock<RegistryInner>>`. `RegistryInner` holds:
  - `generation: u64`
  - `by_fn_oid: HashMap<Oid, RegistryEntry>`
  - `by_module_id: HashMap<u64, ModuleEntry>`
- **`RegistryEntry`** (Wave-1 stub shape): `{ module_id: u64,
  export_index: u32, fn_oid: pgrx::pg_sys::Oid }`. Downstream Wave-2
  tasks will extend with the compiled plan; keep the Wave-1 type
  extensible.
- **Generation check**: every public accessor reads
  `shmem::read_generation()` (from the `shmem-and-generation` agent's
  output) and, if it differs from the cached one, takes a write lock and
  rebuilds from `pg_wasm.exports` via `catalog::exports::list()` (from
  the `catalog-schema` agent's output).
- **Cross-agent coupling**: both `shmem::read_generation` and
  `catalog::exports::list` are being added by other Wave-1 PRs. While
  those aren't merged, provide **temporary stubs** at the call sites
  behind a narrow trait so your PR builds on its own:
  ```rust
  trait GenerationSource { fn read(&self) -> u64; }
  trait CatalogSource    { fn list_exports(&self) -> Vec<RegistryEntry>; }
  ```
  with default `pub(crate) struct DefaultSources;` whose impls currently
  return `0` and `vec![]` respectively. Add a TODO comment referencing
  the `shmem-and-generation` and `catalog-schema` todo ids; a later PR
  will swap in the real sources. This keeps your PR green against
  `main` without requiring a merge order.
- `pgrx::pg_guard` + `#[pg_extern]` attribute macros: use `pgrx::pg_sys::Datum`
  and `pgrx::pg_sys::FunctionCallInfo` directly. Wrap unsafe FFI in
  `std::panic::catch_unwind` so a panic becomes an `ereport` rather
  than an `abort` (error mapping will be finalized later; for now a
  `log::error!`-equivalent `ereport(NOTICE, ...)` is fine).

## Validation expectations

- `cargo check -p pg_wasm` passes.
- Host-only `#[test]`s:
  - `registry::resolve_fn_oid` miss → returns `None`; hit after inserting
    an entry → returns `Some(_)`.
  - Generation bump via a mock `GenerationSource` causes a refresh.
- A narrow `#[pg_test]` that asserts calling the trampoline through a
  synthetic `FunctionCallInfo` returns the expected placeholder (this is
  optional; if too fiddly to construct, skip and note in PR).

## Final commit

Flip the `status:` line for `trampoline-stub` in
`.cursor/plans/pg_wasm_extension_implementation_v2.plan.md` from `pending`
to `completed`. No other edits to that file.
