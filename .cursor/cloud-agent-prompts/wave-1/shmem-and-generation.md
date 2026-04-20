# Wave-1 Cloud Agent: `shmem-and-generation`

**Branch**: `wave-1/shmem-and-generation` (base: `main`)
**PR title**: `[wave-1] shmem-and-generation: per-cluster shmem segment with CatalogLock`

Before doing anything else, read `@.cursor/cloud-agent-prompts/wave-1/README.md`
in full and comply with every shared rule in it.

## Task (copied verbatim from the plan todo)

> Implement `shmem.rs` with a per-cluster segment sized by fixed
> compile-time constants (module slots and export slots). Provide
> `bump_generation(module_id)`, `read_generation()`, and atomic per-export
> counters. Protect mutators with `pg_wasm.CatalogLock` (LWLock). Wire into
> `shmem_request_hook` and `shmem_startup_hook`.

Authoritative design sections:
`docs/architecture.md` §§ "Generation counters and CatalogLock",
"Shared-memory sizing", "Observability".

## Files you own

- `pg_wasm/src/shmem.rs` (add bodies; the two slot-count constants are
  already defined — reuse them).

`_PG_init` already calls `shmem::init()`. That is where you install
`shmem_request_hook` and `shmem_startup_hook` and register the
`pg_wasm.CatalogLock` LWLock tranche.

## Files you must not touch

- `pg_wasm/src/lib.rs` (scaffolding already wires `shmem::init()`).
- Every other `pg_wasm/src/*.rs` file (they belong to other Wave-1 agents or
  later waves).
- `Cargo.toml`, `pg_wasm/Cargo.toml`, `pg_wasm/pg_wasm.control`.
- Any line of the plan file other than your own todo's `status:` line.

## Implementation notes

- Use pgrx 0.18's shared-memory and LWLock APIs
  (https://docs.rs/pgrx/0.18 — `pgrx::pg_sys::shmem_request_hook`,
  `shmem_startup_hook`, `RequestAddinShmemSpace`, `ShmemInitStruct`,
  `LWLockRegisterTranche`, `LWLockPadded`, `GetNamedLWLockTranche`). Prefer
  pgrx wrappers when they exist.
- **Layout**: one `pg_wasm::SharedState` struct in shmem containing:
  - `generation: AtomicU64`
  - `module_slots: [ModuleSlot; SHMEM_MODULE_SLOTS]` — each slot has
    `module_id: u64`, per-export atomic counters (invocations, traps, etc.),
    and enough space for aggregate stats per module.
  - `export_slots: [ExportSlot; SHMEM_EXPORT_SLOTS]` — atomic counters
    keyed by (module_id, export_index).
  - A single `LWLock` handle for `pg_wasm.CatalogLock`.
- **API surface** (minimum):
  - `pub(crate) fn bump_generation(module_id: u64) -> u64` — takes
    CatalogLock Exclusive, increments generation, returns new value.
  - `pub(crate) fn read_generation() -> u64` — lock-free `Relaxed` load.
  - `pub(crate) fn incr_export_counter(module_id, export_index, kind)` —
    lock-free atomic increments on counter enums.
  - `pub(crate) fn allocate_slots(module_id, n_exports) -> Result<SlotRefs, ShmemOverflow>`
    — reserves module + export slots under CatalogLock.
  - `pub(crate) fn free_slots(module_id)` — releases them on unload.
  - `pub(crate) fn with_catalog_lock_exclusive<F, T>(f: F) -> T` /
    `_shared<F, T>` helpers if they're useful for other call sites.
- **Overflow behavior**: if a load exceeds `SHMEM_MODULE_SLOTS` or the sum
  of exports exceeds `SHMEM_EXPORT_SLOTS`, return `ShmemOverflow` rather
  than panic. Callers will set `shared := false` in `pg_wasm.modules` and
  fall back to process-local `AtomicU64`s (that fallback logic itself is
  out of scope for this task — just emit an error variant).
- Wire `shmem_request_hook` to call `RequestAddinShmemSpace` with
  `mem::size_of::<SharedState>()` + tranche request. Wire
  `shmem_startup_hook` to `ShmemInitStruct` + initialize the lock.
- Keep the prior hook chain intact: save the previous hook pointer,
  restore it inside your hook, invoke it before returning (standard pgrx
  hook pattern — follow any neighboring examples).

## Validation expectations

- `cargo check -p pg_wasm` passes.
- `cargo pgrx test -p pg_wasm` — add at least one `#[pg_test]` that:
  - Calls `read_generation()`, then `bump_generation(some_id)`, then
    `read_generation()` and asserts it increased by 1.
  - Exercises `incr_export_counter` concurrently (use
    `std::thread::scope`) and asserts final count is correct.
- Host-only `#[test]`s for any pure helpers (e.g. slot index math).

## Final commit

Flip the `status:` line for `shmem-and-generation` in
`.cursor/plans/pg_wasm_extension_implementation_v2.plan.md` from `pending`
to `completed`. No other edits to that file.
