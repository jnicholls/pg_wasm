# Wave-6 Cloud Agent: `error-mapping` (RUN FIRST IN WAVE 6)

**Branch**: `wave-6/error-mapping` (base: `main`)
**PR title**: `[wave-6] error-mapping: final PgWasmError → ereport with SQLSTATE/DETAIL/HINT`

Read `@.cursor/cloud-agent-prompts/wave-6/README.md` and
`@.cursor/cloud-agent-prompts/wave-1/README.md` first.

## Task (copied verbatim)

> Finalize `errors::PgWasmError` -> `ereport` conversion, including
> SQLSTATE, MESSAGE, DETAIL (module_id, export_id, wasmtime_version),
> HINT (policy hints on denials).

Design ref: `docs/architecture.md` §§ "Error taxonomy", "Error
mapping".

## Files you own

- `pg_wasm/src/errors.rs` — may rewrite extensively.
- Every call site that currently does
  `ereport!(ERROR, err.sqlstate(), err.to_string())` or similar: switch
  to `err.report()` (new method on `PgWasmError`). You may touch any
  file to make this change, **but only that change**. Do not edit
  unrelated lines; keep diffs narrow.
- Optional: `pg_wasm/tests/pg_regress/{sql,expected}/errors.sql`.

## Files you must not touch

- Anything except to perform the narrow "use the new reporter"
  substitution.
- `Cargo.toml`, `pg_wasm.control`, `pg_wasm/src/lib.rs` (beyond
  whatever call-site edit is needed — typically nothing).

## Implementation notes

- **Final variant set**: take the union of everything other waves
  appended. Keep variants in the order they appear; do not reorder.
- **Add structured context**:
  ```rust
  pub(crate) struct ErrorContext {
      pub module_id: Option<u64>,
      pub export_index: Option<u32>,
      pub wasmtime_version: &'static str,
  }
  ```
  Default `wasmtime_version = env!("WASMTIME_VERSION")` resolved via a
  tiny build.rs **only if necessary** — or inline a hard-coded
  `"43.0.0"` constant, given the workspace pin. Prefer the constant to
  avoid adding a build.rs.
- **`report(self, ctx: ErrorContext) -> !`** on `PgWasmError`: call
  `pgrx::pg_sys::ereport` with:
  - `elevel = ERROR`
  - `errcode = self.sqlstate()`
  - `errmsg = self.to_string()`
  - `errdetail = format!("module_id={}, export_index={}, wasmtime={}",
    ...)`
  - `errhint = self.hint()` where `hint()` returns `Option<&str>` for
    policy-denial variants (e.g. `PermissionDenied` suggests the GUC
    to flip) and `None` otherwise.
- **Convenience**: keep the existing
  `Result<T> = core::result::Result<T, PgWasmError>` alias.
- **Call-site migration**: define a tiny extension trait:
  ```rust
  pub(crate) trait IntoReport {
      fn or_report(self, ctx: ErrorContext) -> Self::Ok
      where Self: Sized;
      type Ok;
  }
  impl<T> IntoReport for Result<T> {
      type Ok = T;
      fn or_report(self, ctx: ErrorContext) -> T {
          match self {
              Ok(v) => v,
              Err(e) => e.report(ctx),
          }
      }
  }
  ```
  Swap call sites from ad-hoc `ereport!(ERROR, ...)` to
  `result.or_report(ctx)`. Skip sites that only log at lower levels
  (NOTICE/WARNING) — those are intentional and not errors.
- **Regress goldens**: `errors.sql` triggers at least one failure per
  SQLSTATE class and asserts the DETAIL string includes the expected
  module_id.

## Validation expectations

- `cargo check -p pg_wasm` passes.
- `cargo pgrx regress` existing suites still green (the
  DETAIL/HINT changes affect output; rebaseline goldens as needed).
  Keep the rebase narrow and justify each change.
- Host-only `#[test]`: `hint()` returns `Some` only for
  `PermissionDenied` and any other variant that has a natural hint
  (see the task description — "policy hints on denials").

## Final commit

Flip `error-mapping`'s `status:` line to `completed`.
