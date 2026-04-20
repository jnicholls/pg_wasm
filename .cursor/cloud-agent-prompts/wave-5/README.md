# Wave-5 Cloud Agent prompts

Launch **after every Wave-4 PR has merged into `main`**.

Shared rules: see `.cursor/cloud-agent-prompts/wave-1/README.md`.

## Ownership matrix (Wave 5)

| Prompt                               | Branch                                    | Files owned |
|--------------------------------------|-------------------------------------------|-------------|
| `reload-orchestration.md`            | `wave-5/reload-orchestration`             | `pg_wasm/src/lifecycle/reload.rs` (new) |
| `concurrency-safety.md`              | `wave-5/concurrency-safety`               | Narrow edits across `pg_wasm/src/lifecycle/*.rs` (all four files; see prompt) |
| `pg_upgrade-and-extension-upgrade.md`| `wave-5/pg_upgrade-and-extension-upgrade` | `pg_wasm/src/artifacts.rs` (hash-gate addition), `pg_wasm/sql/pg_wasm--0.1.0--0.1.1.sql` (new upgrade scaffolding) |

## Scheduling notes

- `reload-orchestration` adds a new file and calls into existing
  lifecycle helpers; it does not edit the other lifecycle files.
- `concurrency-safety` is **cross-cutting**: it wraps lifecycle entry
  points (`load`, `unload`, `reload`, `reconfigure`) with a
  CatalogLock-Exclusive acquisition. It edits four files. **Merge
  `reload-orchestration` first**, then run `concurrency-safety`; if
  you launch them at the same time, expect a light rebase on
  `lifecycle/reload.rs`.
- `pg_upgrade-and-extension-upgrade` extends `artifacts.rs` and adds
  one new SQL upgrade file; it does not touch lifecycle. Safe to run
  in parallel with either of the other two.

## "Do not touch"

- All files not listed above.
- `Cargo.toml`, `pg_wasm.control`, `pg_wasm/src/lib.rs`.
