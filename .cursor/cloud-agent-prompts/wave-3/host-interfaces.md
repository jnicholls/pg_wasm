# Wave-3 Cloud Agent: `host-interfaces`

**Branch**: `wave-3/host-interfaces` (base: `main`)
**PR title**: `[wave-3] host-interfaces: pg_wasm:host/log and pg_wasm:host/query`

Read `@.cursor/cloud-agent-prompts/wave-3/README.md` and
`@.cursor/cloud-agent-prompts/wave-1/README.md` first.

## Task (copied verbatim)

> Implement `pg_wasm:host/log` (maps to `ereport(NOTICE/INFO/WARNING)`)
> and `pg_wasm:host/query` (SPI read-only by default, gated by
> `pg_wasm.allow_spi`). Provide WIT text in `pg_wasm/wit/host.wit` and
> wire into the component `Linker`.

Design ref: `docs/architecture.md` §§ "Host interfaces", "SPI gating".

Pinned API:
- `wasmtime::component::Linker::root()` / `instance(name)`:
  https://docs.rs/wasmtime/43.0.0/wasmtime/component/struct.Linker.html
- pgrx SPI (0.18): https://docs.rs/pgrx/0.18

## Files you own

- `pg_wasm/wit/host.wit` (new) — the WIT definition for the host
  interfaces. Package `pg-wasm:host`; interfaces `log` and `query`.
- `pg_wasm/src/runtime/host.rs` (new) — implements the linker wiring.
  Declare in `pg_wasm/src/runtime/mod.rs` (`pub mod host;`).

## Files you must not touch

- All files listed in `@.cursor/cloud-agent-prompts/wave-3/README.md`
  "Do not touch" section.
- `pg_wasm/src/runtime/component.rs` (Wave-2) — it should already have
  left a `TODO(wave-3: host-interfaces)` hook. If the hook is a simple
  "call `runtime::host::add_to_linker(&mut linker, &policy)?`" line,
  add that one line only; no other edits to `component.rs`.

## Implementation notes

- **`host.wit`** (exact WIT, keep stable — `reload`-friendly):
  ```wit
  package pg-wasm:host@0.1.0;

  interface log {
      enum level { info, notice, warning }
      log: func(level: level, message: string);
  }

  interface query {
      variant value {
          null,
          bool(bool),
          int(s64),
          float(float64),
          text(string),
          bytea(list<u8>),
      }
      record row {
          columns: list<value>,
      }
      record result-set {
          column-names: list<string>,
          rows: list<row>,
      }
      read: func(sql: string, params: list<value>) -> result<result-set, string>;
  }

  world host-only {
      export log;
      export query;
  }
  ```
  Export semantics: the **host** exports these; the guest imports them.
- **`runtime::host::add_to_linker(linker: &mut Linker<StoreCtx>, policy:
  &EffectivePolicy) -> Result<(), PgWasmError>`**:
  - Always add `pg-wasm:host/log` → dispatch to `ereport(level, msg)`
    using the `level` enum; `info→INFO`, `notice→NOTICE`,
    `warning→WARNING`. Truncate messages at a sane cap (e.g. 1 MiB)
    with a suffix.
  - If `policy.allow_spi` is true, add `pg-wasm:host/query` → dispatch
    to pgrx SPI `read-only`. Convert each row to the WIT `value`
    variant by inspecting `pg_type.oid` / `typtype`. Unsupported types
    → variant `text(...)` with `pg_type_to_text(value)` fallback; note
    this in the WIT doc comment.
  - If `policy.allow_spi` is false, skip the query interface entirely
    so instantiation fails fast with a clear "host/query not linked"
    error when the guest imports it (instead of silent no-op). Surface
    as `PgWasmError::PermissionDenied` with a hint referencing
    `pg_wasm.allow_spi`.
- **Safety**: SPI must execute in the parent connection's transaction.
  Use `pgrx::spi::Spi::connect_sync` / equivalent pgrx 0.18 pattern.
  Never start your own transaction.
- **Param binding**: translate `value` → PG `Datum` via the
  `mapping::scalars` helpers from Wave 2. For `bytea`, pass the list's
  raw bytes. For unsupported cases, return `result::err("unsupported
  param type at index N")`.

## Validation expectations

- `cargo check -p pg_wasm` passes.
- Host-only `#[test]`:
  - `host.wit` parses via `wit_parser::Resolve::push_path`.
- `#[pg_test]`:
  - Component that imports `pg-wasm:host/log` and calls
    `log.log(level::notice, "hi")` → `NOTICE` visible in test output.
  - Component that imports `pg-wasm:host/query` and runs
    `SELECT 1, 'x'` → receives two columns; with
    `pg_wasm.allow_spi = off` instantiation of that component fails
    with a permission-denied error mentioning the GUC.
  - Verify write SQL through `query.read` is rejected (SPI read-only).

## Final commit

Flip `host-interfaces`'s `status:` line to `completed`.
