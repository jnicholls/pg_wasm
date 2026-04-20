# Wave-7 Cloud Agent: `integration-tests`

**Branch**: `wave-7/integration-tests` (base: `main`)
**PR title**: `[wave-7] integration-tests: workspace tests crate with tokio-postgres`

Read `@.cursor/cloud-agent-prompts/wave-7/README.md` and
`@.cursor/cloud-agent-prompts/wave-1/README.md` first.

## Task (copied verbatim)

> Add workspace `tests/` crate using `tokio-postgres`. Cover
> concurrent-backend load visibility via generation bumps, backend
> restart recovery, query cancellation via epoch interruption, fuel
> exhaustion, memory-pages limit, WASI policy denials.

Design ref: `AGENTS.md` §§ "Integration", `docs/architecture.md` §§
"Concurrency model", "Policy enforcement".

## Files you own

- `tests/Cargo.toml` (new)
- `tests/src/**` (new)
- `tests/README.md` (new) — document how to run
  (`DATABASE_URL=postgres://localhost:28813/postgres cargo test -p tests`
  assuming `pgrx start` on pg13; bump port per PG major = `28800 +
  major`).
- Workspace `Cargo.toml` — **one** append to `members`:
  ```toml
  [workspace]
  members = [
      "pg_wasm",
      "tests",
  ]
  ```
  No other workspace edits.

## Files you must not touch

- `pg_wasm/src/**`.
- `pg_wasm/Cargo.toml`, `pg_wasm.control`.
- `pg_wasm/tests/**`, `pg_wasm/fixtures/**`.
- `pg_wasm/sql/**`.

## Implementation notes

- **Crate metadata**:
  ```toml
  [package]
  name = "tests"
  edition.workspace = true
  version.workspace = true
  authors.workspace = true
  publish = false

  [dependencies]
  anyhow.workspace = true
  tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
  tokio-postgres = "0.7"
  ```
  `tokio` and `tokio-postgres` are new workspace-level deps; add them
  as `[dev-dependencies]` on the `tests` crate only, or add to
  `[workspace.dependencies]`. **If you need to add them to
  `[workspace.dependencies]`**, this is the one allowed workspace
  Cargo.toml edit beyond the `members` entry — keep it alphabetical.
  Otherwise keep all new deps scoped to `tests/Cargo.toml`.
- **No `pgrx` in this crate**. The extension is a `cdylib`; this
  client-only crate must not link `pgrx`. All interactions are SQL.
- **Bootstrap per test**:
  - Connect via `DATABASE_URL`.
  - `DROP EXTENSION IF EXISTS pg_wasm CASCADE; CREATE EXTENSION pg_wasm;`
    at test start.
  - Clean up afterwards.
  - Serialize tests (`#[tokio::test(flavor = "current_thread")]`) to
    avoid cross-test interference, or run concurrently and use
    separate schemas / module-name prefixes per test.
- **Required test cases** (one `#[tokio::test]` per bullet):
  1. **Concurrent-backend load visibility**: backend A loads a
     module; backend B calls the resulting function right after; B
     observes the new function (generation propagation).
  2. **Backend restart recovery**: load a component; call it;
     restart Postgres (use a test helper that shells out to
     `cargo pgrx stop/start`); reconnect; call the function again
     — assert it still works (cold-attach via `.cwasm`).
  3. **Query cancellation**: load a module with an infinite loop;
     call it; cancel the query (`pg_cancel_backend` from another
     connection); assert the client sees `ERRCODE_QUERY_CANCELED`
     within a bounded time.
  4. **Fuel exhaustion**: load with tight
     `pg_wasm.fuel_per_invocation` override; invoke an expensive
     export; assert `ERRCODE_PROGRAM_LIMIT_EXCEEDED`.
  5. **Memory-pages limit**: assert `max_memory_pages` triggers
     trap.
  6. **WASI policy denials**: load with
     `pg_wasm.allow_wasi_fs = off`; module that attempts a WASI
     `path_open` errors with `PgWasmError::PermissionDenied` mapped
     to `ERRCODE_INSUFFICIENT_PRIVILEGE`.
- **Helpers**: put shared setup in `tests/src/common/mod.rs`. Each
  test file lives in `tests/src/<name>.rs` (or
  `tests/tests/<name>.rs` — follow pgrx 0.18 integration conventions
  and document the choice).

## Validation expectations

- `cargo test -p tests` passes locally against a `cargo pgrx start`ed
  pg13 (port 28813 by default; parameterize via `DATABASE_URL`).
- Add a CI recipe — a short shell snippet in `tests/README.md` is
  fine; updating CI YAML is out of scope.

## Final commit

Flip `integration-tests`'s `status:` line to `completed`.
