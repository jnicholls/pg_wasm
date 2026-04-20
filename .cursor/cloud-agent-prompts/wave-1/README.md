# Wave-1 Cloud Agent prompts

These files are ready-to-paste prompts for 10 **Cursor Cloud Agents** that
together implement Wave 1 of the v2 plan
(`.cursor/plans/pg_wasm_extension_implementation_v2.plan.md`). Each prompt is
self-contained; launch one Cloud Agent per file.

## Dispatch checklist

For each of the 10 files `wave-1/<id>.md`:

1. Open Cursor's Cloud Agent picker (`cursor-agent -c` or `Cmd+E` in the IDE).
2. Select the model the user prefers (e.g. `Codex 5.3 High`).
3. Set the base branch to `main`.
4. Paste the entire contents of the prompt file.
5. Launch.

All 10 agents run **in parallel**. They edit disjoint files and each open
their own PR `wave-1/<id>` targeting `main`.

## Shared rules (referenced by every prompt)

Every Wave-1 Cloud Agent must obey these rules. Each individual prompt also
re-asserts the non-negotiable points inline.

### Branch, base, PR

- Work on branch `wave-1/<id>` off `main`.
- Open a PR titled `[wave-1] <id>: <one-line summary>` targeting `main`.
- PR body must include: link to the plan file, your todo id, a short summary,
  and the testing you performed.

### Non-negotiable constraints

1. Follow `AGENTS.md` and every file under `.cursor/rules/*.mdc`. Read them
   before writing code.
2. Dependencies are already pinned at the workspace level: **wasmtime 43**
   (with `component-model`), **wasmtime-wasi 43**, **wasmtime-wasi-http 43**,
   **wasmparser 0.247**, **wit-component 0.247**, **wit-parser 0.247**, plus
   `anyhow`, `serde_json`, `sha2`, `thiserror`, `pgrx 0.18`. Do **not** add
   new dependencies or bump versions. If you believe a new dep is truly
   required, STOP and call it out in the PR description rather than add it.
3. Edit **only** the files listed under "Files you own" in your prompt. Do
   **not** edit files listed under "Files you must not touch".
4. `pg_wasm/src/lib.rs` is scaffolded. `_PG_init` already calls
   `shmem::init()`, `runtime::init()`, and `catalog::init()`. The owners of
   those three modules fill in those functions inside their own files. No
   other Wave-1 agent should touch `lib.rs`.
5. Only the `errors-and-guc` and `bootstrap-layout` todos are already
   `completed`. As the **final commit** of your PR, flip your own todo's
   `status: pending` → `status: completed` in
   `.cursor/plans/pg_wasm_extension_implementation_v2.plan.md`. That one
   line is the **only** line of that file you are allowed to change.
6. `cargo check -p pg_wasm` must pass before you push.
7. Testing layers (see `.cursor/rules/pg-wasm-pgrx-testing.mdc`):
   - Host-only `#[test]` for pure Rust code that does not call into a
     Postgres backend.
   - `#[pg_test]` (run via `cargo pgrx test -p pg_wasm`) only when the code
     needs a loaded backend.
   - pg_regress goldens for SQL-visible behavior.

### Universal "do not touch"

- `pg_wasm/src/lib.rs`
- `Cargo.toml` and `pg_wasm/Cargo.toml`
- `pg_wasm/pg_wasm.control`
- Other Wave-1 owners' files as listed per-task
- Any line of the plan file other than your own todo's `status:` line

### Merge-conflict policy on the plan file

If another Wave-1 PR merges first and flips its own todo's status, a rebase
will conflict on the YAML list in the plan. To resolve: accept incoming
changes from `main` AND re-apply your own `status: completed` flip for your
own todo id. No other edits.

### Reference material

- Plan: `.cursor/plans/pg_wasm_extension_implementation_v2.plan.md`
- Design doc: `docs/architecture.md` (authoritative for design decisions;
  includes detailed sections referenced by each prompt)
- Workspace rules: `AGENTS.md`, `.cursor/rules/rust-coding-standards.mdc`,
  `.cursor/rules/dependency-api-docs.mdc`,
  `.cursor/rules/pg-wasm-pgrx-testing.mdc`
- Pinned crate API docs (use these exact version paths, not `/latest/`):
  - wasmtime 43: https://docs.rs/wasmtime/43.0.0/wasmtime/
  - wasmtime-wasi 43: https://docs.rs/wasmtime-wasi/43.0.0/wasmtime_wasi/
  - wasmtime-wasi-http 43: https://docs.rs/wasmtime-wasi-http/43.0.0/wasmtime_wasi_http/
  - wasmparser 0.247: https://docs.rs/wasmparser/0.247.0/wasmparser/
  - wit-component 0.247: https://docs.rs/wit-component/0.247.0/wit_component/
  - wit-parser 0.247: https://docs.rs/wit-parser/0.247.0/wit_parser/
  - pgrx 0.18: https://docs.rs/pgrx/0.18

## Ownership matrix

| Prompt file                       | Branch                          | Files owned |
|-----------------------------------|---------------------------------|-------------|
| `catalog-schema.md`               | `wave-1/catalog-schema`         | `pg_wasm/src/catalog.rs` (convertible to `catalog/` dir), `pg_wasm/sql/*.sql` |
| `shmem-and-generation.md`         | `wave-1/shmem-and-generation`   | `pg_wasm/src/shmem.rs` |
| `artifacts-layout.md`             | `wave-1/artifacts-layout`       | `pg_wasm/src/artifacts.rs` |
| `policy-resolve.md`               | `wave-1/policy-resolve`         | `pg_wasm/src/config.rs`, `pg_wasm/src/policy.rs` |
| `abi-detect.md`                   | `wave-1/abi-detect`             | `pg_wasm/src/abi.rs` |
| `engine-and-epoch-ticker.md`      | `wave-1/engine-and-epoch-ticker`| `pg_wasm/src/runtime.rs` (convertible to `runtime/` dir) |
| `trampoline-stub.md`              | `wave-1/trampoline-stub`        | `pg_wasm/src/trampoline.rs`, `pg_wasm/src/registry.rs` |
| `proc-reg-ddl.md`                 | `wave-1/proc-reg-ddl`           | `pg_wasm/src/proc_reg.rs` |
| `wit-type-resolver.md`            | `wave-1/wit-type-resolver`      | `pg_wasm/src/wit.rs` (convertible to `wit/` dir) |
| `docs-and-readme.md`              | `wave-1/docs-and-readme`        | `README.md`, `docs/guc.md` (new), `docs/wit-mapping.md` (new), small edit to `docs/architecture.md` |

## Expected Wave-1 outcomes

After all 10 PRs merge, `main` will have:

- Full catalog schema, migrations, and roles (`catalog-schema`).
- Shared-memory generation counters + CatalogLock LWLock (`shmem-and-generation`).
- `$PGDATA/pg_wasm/` artifact layout with atomic write/fsync/sha256
  (`artifacts-layout`).
- `config::{LoadOptions, PolicyOverrides, Limits}` and
  `policy::{EffectivePolicy, resolve}` with narrowing semantics and tests
  (`policy-resolve`).
- `abi::detect` using `wasmparser::Parser::parse_all` + `validate` with
  unit tests (`abi-detect`).
- Lazily-built shared `wasmtime::Engine` + epoch-ticker thread hooked from
  `_PG_init` (`engine-and-epoch-ticker`).
- Trampoline C entry point + generation-aware registry cache stub
  (`trampoline-stub`).
- `proc_reg::{register, unregister}` with `pg_proc` wiring + extension
  dependency recording (`proc-reg-ddl`).
- WIT resolver producing a stable type plan, world text via `WitPrinter`
  (`wit-type-resolver`).
- Refreshed README plus `docs/guc.md` and `docs/wit-mapping.md`
  (`docs-and-readme`).

Wave 2 can then start.
