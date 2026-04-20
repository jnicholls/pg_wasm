# Wave-1 Cloud Agent: `docs-and-readme`

**Branch**: `wave-1/docs-and-readme` (base: `main`)
**PR title**: `[wave-1] docs-and-readme: component-first README, guc.md, wit-mapping.md`

Before doing anything else, read `@.cursor/cloud-agent-prompts/wave-1/README.md`
in full and comply with every shared rule in it.

## Task (copied verbatim from the plan todo)

> Update `README.md` with component-first usage. Write `docs/guc.md`
> (every GUC with default, scope, hot/cold reconfig),
> `docs/wit-mapping.md` (the full WIT → PG table with examples).
> Reference them from `docs/architecture.md`.

## Files you own

- `README.md`
- `docs/guc.md` (new)
- `docs/wit-mapping.md` (new)
- Small, scoped edit to `docs/architecture.md`: add **one** new section or
  subsection that links to `docs/guc.md` and `docs/wit-mapping.md`.
  Do not rewrite existing content, do not reorder sections, do not fix
  prose style. Append-only additions.

## Files you must not touch

- Any `pg_wasm/src/*.rs` file.
- `Cargo.toml`, `pg_wasm/Cargo.toml`, `pg_wasm/pg_wasm.control`.
- `pg_wasm/sql/*.sql`, `pg_wasm/tests/**`, `pg_wasm/fixtures/**`.
- `.cursor/rules/*.mdc`, `.cursor/plans/*` (except the one `status:`
  line), `AGENTS.md`.
- Any line of the plan file other than your own todo's `status:` line.

## Implementation notes

- **`README.md`** — replace the existing README with a component-first
  narrative:
  - Elevator pitch: run Wasm components inside Postgres as typed UDFs
    via automatic WIT → PG type mapping, managed sandbox, durable
    artifacts.
  - Install (`cargo pgrx install`), load (`CREATE EXTENSION pg_wasm;`),
    minimal `pg_wasm.load(...)` example with a tiny component.
  - Table of contents linking to `docs/architecture.md`, `docs/guc.md`,
    `docs/wit-mapping.md`.
  - Keep it honest about current status — note that this is v2, some
    todos in the plan are not yet implemented; link the plan file for
    the up-to-date status.
- **`docs/guc.md`** — read `pg_wasm/src/guc.rs` for the authoritative
  list of GUCs. For every GUC produce a row (or subsection):
  - Name
  - Type / default
  - Scope (`USERSET`, `SUSET`, `POSTMASTER`, etc.)
  - Hot vs cold: can it be `ALTER SYSTEM SET` live, or does it require a
    restart / must be set before `shared_preload_libraries` loads
  - One-sentence description of what it narrows
- **`docs/wit-mapping.md`** — the canonical WIT → PG type table, one
  section per kind:
  - primitives: `bool`, `s8/16/32/64`, `u8/16/32/64` (with domain
    wrappers), `f32/f64`, `char`, `string`, `list<u8>`
  - composites: `record`, `tuple`, `variant`, `enum`, `flags`
  - generics: `option`, `result`, `list<T>`
  - resources and handles
  - For each, give the WIT signature on the left and the PG DDL on the
    right, plus a short `SELECT` example where useful. Mirror the
    terminology in `docs/architecture.md` §§ "WIT → PostgreSQL type
    mapping".
- **`docs/architecture.md`** — append one small section referencing the
  two new docs, placed wherever it fits best (probably right after the
  "WIT → PostgreSQL type mapping" section). Do not edit anything else.
- Use GitHub-flavored Markdown. Keep examples runnable (valid SQL /
  valid WIT).

## Validation expectations

- `cargo check -p pg_wasm` passes (this task shouldn't touch Rust, but a
  final check is cheap insurance).
- Locally preview the Markdown (or trust rendering; at minimum lint
  links with `grep -R "](" docs/ README.md`).
- Spell-check the new files.

## Final commit

Flip the `status:` line for `docs-and-readme` in
`.cursor/plans/pg_wasm_extension_implementation_v2.plan.md` from `pending`
to `completed`. No other edits to that file.
