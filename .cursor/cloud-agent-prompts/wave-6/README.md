# Wave-6 Cloud Agent prompts — RUN SERIALLY

Launch **after every Wave-5 PR has merged into `main`**.

Both Wave-6 todos are cross-cutting and will conflict if run in
parallel. **Run `error-mapping.md` first, wait for it to merge, then
run `build-features.md`.**

Shared rules: see `.cursor/cloud-agent-prompts/wave-1/README.md`.

## Ownership matrix (Wave 6)

| Prompt             | Branch                    | Files owned |
|--------------------|---------------------------|-------------|
| `error-mapping.md` | `wave-6/error-mapping`    | `pg_wasm/src/errors.rs` (full rewrite), narrow edits across every `ereport!`/`Err(PgWasmError::...)` call site |
| `build-features.md`| `wave-6/build-features`   | `pg_wasm/Cargo.toml` (features table), narrow `#[cfg(feature = "...")]` gates across `src/**` |

## "Do not touch"

- Anything not needed to achieve the task. Keep per-file diff narrow.
- `Cargo.toml` (workspace) — only `build-features` may touch
  `pg_wasm/Cargo.toml`. The workspace `Cargo.toml` should not need
  edits.
- `pg_wasm.control`, `pg_wasm/src/lib.rs` (beyond feature gates).

## Scheduling note

Do not launch `build-features` before `error-mapping` has merged.
Rebase-after-merge is survivable but expensive given the cross-cutting
nature of both.
