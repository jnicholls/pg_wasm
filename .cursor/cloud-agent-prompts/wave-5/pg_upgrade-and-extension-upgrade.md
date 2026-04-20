# Wave-5 Cloud Agent: `pg_upgrade-and-extension-upgrade`

**Branch**: `wave-5/pg_upgrade-and-extension-upgrade` (base: `main`)
**PR title**: `[wave-5] pg_upgrade-and-extension-upgrade: compat-hash gate and upgrade scaffolding`

Read `@.cursor/cloud-agent-prompts/wave-5/README.md` and
`@.cursor/cloud-agent-prompts/wave-1/README.md` first.

## Task (copied verbatim)

> Verify artifacts survive `pg_upgrade`. Implement the
> `Engine::precompile_compatibility_hash` +
> `Engine::detect_precompiled_file` gate in `artifacts.rs`: on cold
> attach, if the hash stored alongside `module.cwasm` does not match
> the running engine's hash (or if `detect_precompiled_file` returns
> `None`/`Some(Precompiled::Module)` when we expect `Component`),
> delete the stale artifact and recompile from `module.wasm`. Add
> `sql/pg_wasm--X.Y--X.Z.sql` scaffolding; `catalog::migrations`
> validates shape on `_PG_init`.

Design ref: `docs/architecture.md` §§ "Extension upgrade and
pg_upgrade".

## Files you own

- `pg_wasm/src/artifacts.rs` — add the compat-hash gate. **Append**
  new helpers; do not rewrite existing ones.
- `pg_wasm/sql/pg_wasm--0.1.0--0.1.1.sql` (new scaffolding, even if
  empty) — demonstrates the upgrade-script convention.

## Files you must not touch

- Everything else. Specifically: do not edit `runtime::component` or
  `lifecycle/*` — those callers are already designed to consume the
  helpers you add here.
- `Cargo.toml`, `pg_wasm.control`, `pg_wasm/src/lib.rs`.

## Implementation notes

- **Hash sidecar**: each module dir already contains
  `module.cwasm`. Add a sibling `compat_hash` file (plain text hex)
  written atomically alongside `module.cwasm` by a new helper:
  ```rust
  pub(crate) fn write_compat_hash(module_dir: &Path, hash: &[u8]) -> io::Result<()>;
  pub(crate) fn read_compat_hash(module_dir: &Path) -> io::Result<Option<Vec<u8>>>;
  ```
- **Gate helper**:
  ```rust
  pub(crate) enum CompatCheck { Ok, StaleRecompile, MissingRecompile }
  pub(crate) fn check_compat(
      module_dir: &Path,
      engine: &wasmtime::Engine,
      expected_kind: ExpectedKind, // Component | Core
  ) -> Result<CompatCheck, PgWasmError>;
  ```
  - If `module.cwasm` is absent → `MissingRecompile`.
  - If `Engine::detect_precompiled_file(cwasm_path)` returns `None`
    or a kind mismatch → `StaleRecompile`.
  - If the sidecar hash does not match
    `engine.precompile_compatibility_hash()` → `StaleRecompile`.
  - Otherwise → `Ok`.
- **Stale-artifact cleanup**:
  ```rust
  pub(crate) fn invalidate_cwasm(module_dir: &Path) -> io::Result<()>;
  ```
  Removes `module.cwasm` and `compat_hash`; leaves `module.wasm` and
  `world.wit` intact so callers can recompile.
- **`catalog::migrations::validate_shape`** is authored by Wave-1;
  this task does **not** edit it. If the upgrade-script scaffolding
  requires new columns in the future, that belongs to a real 0.1.0
  → 0.1.1 bump when it happens. For this PR, the upgrade SQL file is
  intentionally empty (or a single-line comment) — demonstrating
  the convention only.
- **pg_upgrade notes**: document in a top-of-file comment that:
  - `$PGDATA/pg_wasm/<module_id>/` is copied to the new cluster as
    part of normal `pg_upgrade` data-dir hard-links.
  - On first backend start against the new Postgres major, the
    first invocation will trigger `check_compat` → `StaleRecompile`
    if the Wasmtime engine has changed, and recompile from
    `module.wasm`.
  - Catalog rows survive via the standard extension
    dump-and-restore; the `wit_world` text column is authoritative
    for reconstituting UDT shape.

## Validation expectations

- `cargo check -p pg_wasm` passes.
- Host-only `#[test]`s:
  - Hash sidecar round-trip.
  - `invalidate_cwasm` removes the expected files and preserves the
    others.
  - Mock `detect_precompiled_file` behavior is out of scope —
    substitute with a real `Engine` building a trivial component,
    precompiling, writing, then flipping the hash file and asserting
    `check_compat` returns `StaleRecompile`.
- A narrow `#[pg_test]` confirming `pg_wasm--0.1.0--0.1.1.sql` is
  installed (touched by `ALTER EXTENSION pg_wasm UPDATE`).

## Final commit

Flip `pg_upgrade-and-extension-upgrade`'s `status:` line to `completed`.
