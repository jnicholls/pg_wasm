# Wave-5 Cloud Agent: `concurrency-safety`

**Branch**: `wave-5/concurrency-safety` (base: `main`)
**PR title**: `[wave-5] concurrency-safety: CatalogLock around lifecycle + stress test`

Read `@.cursor/cloud-agent-prompts/wave-5/README.md` and
`@.cursor/cloud-agent-prompts/wave-1/README.md` first.

## Task (copied verbatim)

> Add `pg_wasm.CatalogLock` (LWLock tranche) held during
> load/unload/reload/reconfigure catalog mutation and shmem
> generation bumps. Confirm in-flight invocations complete against
> the old handle under reload. Stress-test with an integration test
> issuing concurrent loads + calls.

Design ref: `docs/architecture.md` §§ "Concurrency model",
"Generation-driven reload safety".

## Files you own

This is a **cross-cutting** PR. You edit four lifecycle files and one
test file. Keep the per-file diff **narrow** — one
`shmem::with_catalog_lock_exclusive(|| { ... })` wrapper per entry
point plus a comment. Do not rewrite unrelated code.

- `pg_wasm/src/lifecycle/load.rs`
- `pg_wasm/src/lifecycle/unload.rs`
- `pg_wasm/src/lifecycle/reload.rs`
- `pg_wasm/src/lifecycle/reconfigure.rs`
- `pg_wasm/tests/pg_regress/sql/concurrency.sql` + expected (new), or
  a workspace integration test if `tests/` exists by this wave —
  pick one and document the choice in the PR.

## Files you must not touch

- All non-lifecycle `pg_wasm/src/*` files.
- `pg_wasm/src/shmem.rs` — the `CatalogLock` and
  `with_catalog_lock_*` helpers already exist from Wave-1
  `shmem-and-generation`. Do not expand that API here; use what is
  there.
- `Cargo.toml`, `pg_wasm.control`, `pg_wasm/src/lib.rs`.

## Implementation notes

- The `CatalogLock` is an LWLock. Held Exclusive during any
  mutator's catalog writes + `shmem::bump_generation`. Released
  before returning control to the caller. Example wrapper:
  ```rust
  pub fn load(...) -> Result<(), PgWasmError> {
      shmem::with_catalog_lock_exclusive(|| do_load_inner(...))
  }
  ```
- **Do not** hold the lock during the network-style steps of load
  (reading a file, compiling a component). Only the final
  "catalog-writes + generation bump" phase needs it. Factor the
  function accordingly: `prepare(...) -> PreparedLoad` outside the
  lock, `commit(prep) -> Result<(), _>` under the lock. Apply the
  same pattern to `unload`, `reload`, `reconfigure`.
- **Reader path** (invocation):
  - Invocation path (trampoline) does **not** take CatalogLock; it
    reads generation via `shmem::read_generation()` (lock-free
    `Relaxed`) and only takes its own local registry lock.
  - Therefore in-flight invocations can run concurrently with a
    reload's `prepare(...)` phase. They also continue running during
    the `commit` phase because they use the old `Component` via a
    strong `Arc<Component>` still held by their pool slot — the new
    `Component` replaces the pool's factory but does not revoke
    existing handles.
- **Stress test**:
  - Fixture: small component with one export that sleeps ~50ms
    internally. Load it; spawn N background sessions calling the
    export in a loop; in the foreground, issue alternating
    `pg_wasm.reload(...)` with identical bytes. Assert: zero failed
    calls, generation monotonically increases, and `pg_wasm.stats()`
    invocation counter equals the sum of successful calls.
  - Use `tokio-postgres` if this lands as an integration test;
    otherwise pg_regress + `pg_background` for the foreground+
    background concurrency (pg_regress is typically deterministic
    and concurrent-test-unfriendly, so the workspace integration
    crate is preferred if it exists by this wave).

## Validation expectations

- `cargo check -p pg_wasm` passes.
- Stress test passes in CI (all platforms the plan targets).

## Final commit

Flip `concurrency-safety`'s `status:` line to `completed`.
