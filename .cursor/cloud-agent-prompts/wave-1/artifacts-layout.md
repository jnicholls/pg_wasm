# Wave-1 Cloud Agent: `artifacts-layout`

**Branch**: `wave-1/artifacts-layout` (base: `main`)
**PR title**: `[wave-1] artifacts-layout: $PGDATA/pg_wasm/<module_id>/ artifact helpers`

Before doing anything else, read `@.cursor/cloud-agent-prompts/wave-1/README.md`
in full and comply with every shared rule in it.

## Task (copied verbatim from the plan todo)

> Implement `artifacts.rs` for `$PGDATA/pg_wasm/<module_id>/`
> (`module.wasm`, `module.cwasm`, `world.wit`). Include atomic write
> (temp + rename), directory fsync, checksum verification (sha256), and a
> `prune_stale` helper for orphaned dirs.

Authoritative design sections:
`docs/architecture.md` §§ "On-disk artifact layout", "Atomic write and
durability", "pg_upgrade / extension upgrade".

## Files you own

- `pg_wasm/src/artifacts.rs`

## Files you must not touch

- `pg_wasm/src/lib.rs`.
- Every other `pg_wasm/src/*.rs` file (they belong to other Wave-1 agents or
  later waves). In particular, do **not** implement the precompile hash
  verification in this task — that belongs to the later
  `pg_upgrade-and-extension-upgrade` todo; only provide the
  sha256-verification helper here.
- `Cargo.toml`, `pg_wasm/Cargo.toml`, `pg_wasm/pg_wasm.control`.
- Any line of the plan file other than your own todo's `status:` line.

## Implementation notes

- Resolve `$PGDATA` once via pgrx (`pgrx::pg_sys::DataDir` or
  `DataDir.to_str()`; there is also `pgrx::prelude::DataDir` helper in
  0.18) and cache it. If called off-backend (host test), allow the
  directory to be overridden via a helper (`set_data_dir_for_test`) so
  host-only unit tests can exercise real filesystem operations in a
  `tempfile::TempDir`. If you add `tempfile` only as a dev-dep via the
  workspace dev-deps table, STOP — do not add deps in this task; use
  `std::env::temp_dir()` in tests instead.
- **Module dir**: `<pgdata>/pg_wasm/<module_id>/` where `module_id` is the
  64-bit id (format hex, lowercase, 16 chars).
- **Files**:
  - `module.wasm` — raw bytes.
  - `module.cwasm` — Wasmtime precompiled artifact (written later by the
    compile step; this task just provides the write helper).
  - `world.wit` — printed WIT text (written later by the wit resolver;
    same comment).
  - `sha256` (or `checksum`) sidecar — text file with `<hex>  module.wasm`.
- **Atomic write helper** `write_atomic(path, bytes) -> io::Result<()>`:
  - Write to `<path>.tmp` in the same directory.
  - `fsync` the file descriptor.
  - `rename` to the final path.
  - `fsync` the parent directory.
- **Checksum helpers** (`sha2` is already a workspace dep):
  - `sha256_bytes(&[u8]) -> [u8; 32]`
  - `write_checksum(module_dir, sha: &[u8;32])`
  - `verify_checksum(module_dir) -> Result<(), PgWasmError>` — re-reads
    `module.wasm`, compares against the sidecar.
- **`prune_stale(active_ids: &BTreeSet<u64>) -> io::Result<usize>`** —
  walks `$PGDATA/pg_wasm/`, removes any subdirectory whose id is not in
  `active_ids`. Returns the number of directories pruned.
- Use `PgWasmError::Io` (the `From<io::Error>` is already wired) for
  bubbled errors. Add new variants only if strictly necessary, and only
  append at the end of the enum — do not reorder.

## Validation expectations

- `cargo check -p pg_wasm` passes.
- Host-only `#[test]`s (**prefer these over `#[pg_test]`** for filesystem
  work):
  - Round-trip a `write_atomic` under a temp dir; assert content and that
    the temp sibling was cleaned up.
  - sha256 round-trip.
  - `prune_stale` with a set of real and phantom module-id subdirs.
- If you need to exercise the `$PGDATA` resolution path, add a narrow
  `#[pg_test]` for that piece alone.

## Final commit

Flip the `status:` line for `artifacts-layout` in
`.cursor/plans/pg_wasm_extension_implementation_v2.plan.md` from `pending`
to `completed`. No other edits to that file.
