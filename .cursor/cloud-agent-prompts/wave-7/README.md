# Wave-7 Cloud Agent prompts

Launch **after every Wave-6 PR has merged into `main`**. The two
Wave-7 agents run in parallel on disjoint directories.

Shared rules: see `.cursor/cloud-agent-prompts/wave-1/README.md`.

## Ownership matrix (Wave 7)

| Prompt                           | Branch                                | Files owned |
|----------------------------------|---------------------------------------|-------------|
| `test-corpus-and-pg_regress.md`  | `wave-7/test-corpus-and-pg_regress`   | `pg_wasm/fixtures/**` (net-new additions), `pg_wasm/tests/pg_regress/**` (new suites) |
| `integration-tests.md`           | `wave-7/integration-tests`            | `tests/` (new workspace crate), workspace `Cargo.toml` (append `tests` to `members`) |

## "Do not touch"

- `pg_wasm/src/**` — both Wave-7 tasks only add tests/fixtures, not
  code. If you find a bug that needs a code fix, STOP and note it in
  the PR description; open a follow-up issue or PR.
- `pg_wasm.control`, `pg_wasm/Cargo.toml` (untouched here).
- `integration-tests` is the **only** task permitted to edit the
  workspace `Cargo.toml`, and only to append one entry to `members`.

## Scheduling note

Both tasks are standalone. Run in parallel. Expect a minor conflict
on workspace `Cargo.toml` only if `integration-tests` and some
future (out-of-plan) task touch it together; Wave-7 alone is safe.
