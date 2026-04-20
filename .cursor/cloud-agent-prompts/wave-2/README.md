# Wave-2 Cloud Agent prompts

Launch **after every Wave-1 PR has merged into `main`**. The five
Wave-2 agents run in parallel on disjoint files.

## Shared rules

Every prompt assumes you have read and will comply with:

- `.cursor/cloud-agent-prompts/wave-1/README.md` — the shared ruleset
  there (branch conventions, workspace rules, pinned dep versions,
  plan-file status flip, merge-conflict policy) applies verbatim to
  every later wave.
- `.cursor/plans/pg_wasm_extension_implementation_v2.plan.md` — your
  todo spec.
- `docs/architecture.md` — authoritative design.
- `AGENTS.md`, `.cursor/rules/rust-coding-standards.mdc`,
  `.cursor/rules/dependency-api-docs.mdc`,
  `.cursor/rules/pg-wasm-pgrx-testing.mdc`.

## Ownership matrix (Wave 2)

| Prompt                            | Branch                              | Files owned |
|-----------------------------------|-------------------------------------|-------------|
| `core-module-scalar-path.md`      | `wave-2/core-module-scalar-path`    | `pg_wasm/src/runtime/core.rs` (new), `pg_wasm/src/mapping/scalars.rs` (new), `pg_wasm/fixtures/core/*`, `pg_wasm/tests/pg_regress/sql/core_scalar.sql` + expected |
| `udt-registration.md`             | `wave-2/udt-registration`           | `pg_wasm/src/wit/udt.rs` (new) |
| `component-compile-and-pool.md`   | `wave-2/component-compile-and-pool` | `pg_wasm/src/runtime/component.rs` (new), `pg_wasm/src/runtime/pool.rs` (new) |
| `component-marshal-dynamic.md`    | `wave-2/component-marshal-dynamic`  | `pg_wasm/src/mapping/composite.rs` (new), `pg_wasm/src/mapping/list.rs` (new) |
| `reconfigure-orchestration.md`    | `wave-2/reconfigure-orchestration`  | `pg_wasm/src/lifecycle/reconfigure.rs` (new; may need to convert `lifecycle.rs` → dir) |

## "Do not touch" highlights for Wave 2

- Other Wave-2 owners' files as listed above.
- `pg_wasm/src/lib.rs` — already wires `_PG_init`; no changes needed.
- `pg_wasm/src/{shmem,trampoline,registry,proc_reg,abi,artifacts,policy,config,guc}.rs`
  — Wave-1 outputs; read-only for Wave 2.
- `pg_wasm/src/wit/{world,typing}.rs` — Wave-1 outputs; read-only.
- `pg_wasm/src/runtime/engine.rs` (or `runtime.rs` if flat) — Wave-1 output;
  read-only.
- `Cargo.toml`, `pg_wasm/Cargo.toml`, `pg_wasm/pg_wasm.control` — do not
  edit.
- Any line of the plan file other than your own todo's `status:` line.

## Module-tree assumptions

Depending on how Wave-1 agents shaped their modules, two directory
layouts are possible:

- `pg_wasm/src/runtime.rs` (flat) **or** `pg_wasm/src/runtime/mod.rs`
  (dir). Wave-2 agents who introduce new `runtime::<sub>` files must
  convert the flat file into a directory if it still is one: move the
  existing content to `runtime/mod.rs` (or a `runtime/engine.rs` that
  it already became) and declare `pub mod <sub>;` there. Preserve all
  prior `pub(crate)` visibility.
- `pg_wasm/src/wit/mod.rs` (dir) is expected to already exist after
  `wit-type-resolver` landed.
- `pg_wasm/src/mapping.rs` is still flat from Wave 1. Convert to
  `pg_wasm/src/mapping/mod.rs` when adding the first submodule; whoever
  gets there first owns the conversion, but all three Wave-2 mapping
  submodule owners are disjoint so a harmless conversion merge is ok.
- `pg_wasm/src/lifecycle.rs` likewise converts to
  `pg_wasm/src/lifecycle/mod.rs` when `reconfigure-orchestration`
  lands.
