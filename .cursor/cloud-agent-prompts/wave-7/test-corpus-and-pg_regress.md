# Wave-7 Cloud Agent: `test-corpus-and-pg_regress`

**Branch**: `wave-7/test-corpus-and-pg_regress` (base: `main`)
**PR title**: `[wave-7] test-corpus-and-pg_regress: component + core fixtures and regress suites`

Read `@.cursor/cloud-agent-prompts/wave-7/README.md` and
`@.cursor/cloud-agent-prompts/wave-1/README.md` first.

## Task (copied verbatim)

> Build component fixtures (arith, strings, records, enums, variants,
> hooks, policy_probe, resources) and core fixtures (add_i32,
> echo_mem). Author pg_regress suites for lifecycle, WIT mappings,
> policy narrowing, error classes, metrics. Deterministic output with
> `ORDER BY` and `EXPLAIN (COSTS OFF, TIMING OFF)`.

Design ref: `docs/architecture.md` §§ "Test corpus",
`AGENTS.md` §§ "Testing".

## Files you own

- `pg_wasm/fixtures/components/arith/` (new) + sibling dirs for
  `strings`, `records`, `enums`, `variants`, `hooks`,
  `policy_probe`, `resources`. Each contains:
  - `world.wit` (source)
  - `component.wasm` **or** a `build.sh` that produces one. Prefer
    committing the built artifact so the test suite is hermetic.
    Document reproduction steps in a `README.md` per fixture.
- `pg_wasm/fixtures/core/add_i32.wat` (already added by Wave 2; do
  **not** re-add). Add `pg_wasm/fixtures/core/echo_mem.wat` if
  missing.
- `pg_wasm/tests/pg_regress/sql/` + `expected/` new suites:
  - `lifecycle.sql` — load / unload / reload / reconfigure happy and
    rollback paths.
  - `wit_mapping.sql` — every WIT primitive + composite round-trips
    through `SELECT pg_wasm.call(...)`.
  - `policy_narrow.sql` — narrowing permitted, widening denied.
  - `error_classes.sql` — one error per SQLSTATE.
  - `metrics.sql` — monotone counters.

## Files you must not touch

- `pg_wasm/src/**`.
- Any existing regress goldens unless you genuinely need to update
  them to cover a new path; if you do, explain each delta in the PR
  description.
- Workspace `Cargo.toml`, `pg_wasm/Cargo.toml`, `pg_wasm.control`.

## Implementation notes

- Keep output deterministic:
  - Every multi-row query: `ORDER BY`.
  - Every plan: `EXPLAIN (COSTS OFF, TIMING OFF)`.
  - No `now()`, `current_timestamp`, random values. Use fixed
    constants.
  - No `pg_sleep` except inside a known-bounded stress helper (and
    avoid in golden tests).
- Fixture build tooling: prefer `cargo component` or `wasm-tools
  component new` + `wit-bindgen`. If the build-time toolchain is not
  available on contributor machines, commit the resulting
  `component.wasm` and keep the source under `src/`.
- Fixture source languages can be Rust, but the fixture crates
  themselves should **not** be members of the workspace. Isolate in
  `pg_wasm/fixtures/components/<name>/` with their own `Cargo.toml`.
- **Do not** add the fixture Rust crates to the workspace
  `members` list (keeps `cargo check` fast for the main crate). If
  they must be workspace members for some reason, discuss in the PR.

## Validation expectations

- `cargo pgrx regress` passes all new suites.
- `cargo build` on each fixture crate succeeds (documented in its
  per-fixture README).

## Final commit

Flip `test-corpus-and-pg_regress`'s `status:` line to `completed`.
