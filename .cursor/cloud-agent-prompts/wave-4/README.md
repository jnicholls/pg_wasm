# Wave-4 Cloud Agent prompts

Launch **after every Wave-3 PR has merged into `main`**. The two Wave-4
agents run in parallel on disjoint files.

Shared rules: see `.cursor/cloud-agent-prompts/wave-1/README.md`.

## Ownership matrix (Wave 4)

| Prompt                     | Branch                          | Files owned |
|----------------------------|---------------------------------|-------------|
| `load-orchestration.md`    | `wave-4/load-orchestration`     | `pg_wasm/src/lifecycle/load.rs` (new) |
| `hooks.md`                 | `wave-4/hooks`                  | `pg_wasm/src/hooks.rs` |

## "Do not touch"

- All files not listed above.
- `pg_wasm/src/lib.rs`, `Cargo.toml`, `pg_wasm.control`.
- Any plan-file line other than your own todo's `status:` line.

## Scheduling note

`load-orchestration` is the big integration PR — it touches many
helpers by calling them, but does not edit their files. `hooks` only
fleshes out `pg_wasm/src/hooks.rs` and coordinates with
`load/unload/reconfigure` via function signatures that already exist.
The two PRs do not edit the same files; merge either order is fine.
