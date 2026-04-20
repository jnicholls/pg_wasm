# Wave-3 Cloud Agent prompts

Launch **after every Wave-2 PR has merged into `main`**. The four Wave-3
agents run in parallel on disjoint files.

Shared rules: see
`.cursor/cloud-agent-prompts/wave-1/README.md` — still apply verbatim.

## Ownership matrix (Wave 3)

| Prompt                       | Branch                               | Files owned |
|------------------------------|--------------------------------------|-------------|
| `unload-orchestration.md`    | `wave-3/unload-orchestration`        | `pg_wasm/src/lifecycle/unload.rs` (new) |
| `host-interfaces.md`         | `wave-3/host-interfaces`             | `pg_wasm/wit/host.wit` (new), `pg_wasm/src/runtime/host.rs` (new) |
| `invocation-path.md`         | `wave-3/invocation-path`             | `pg_wasm/src/trampoline.rs` (extends Wave-1 stub) |
| `metrics-and-views.md`       | `wave-3/metrics-and-views`           | `pg_wasm/src/views.rs`, `pg_wasm/sql/pg_wasm--0.1.0.sql` (view DDL append-only) |

## "Do not touch" highlights

- Other Wave-3 owners' files.
- All Wave-1 + Wave-2 files except the ones your prompt explicitly
  lists as "owned". Everything else is read-only.
- `pg_wasm/src/lib.rs`, `Cargo.toml` variants, `pg_wasm.control`.
- Any line of the plan file other than your own todo's `status:` line.

## Scheduling note

`invocation-path` and `metrics-and-views` both read shared-memory
atomics. They edit different files so parallel is fine, but if one
breaks the shmem counter API shape, the other will need a rebase. The
shmem API shipped in Wave 1 (`shmem-and-generation`); do not change it
here.
