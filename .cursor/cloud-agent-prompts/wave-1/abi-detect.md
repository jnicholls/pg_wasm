# Wave-1 Cloud Agent: `abi-detect`

**Branch**: `wave-1/abi-detect` (base: `main`)
**PR title**: `[wave-1] abi-detect: classify component vs core with wasmparser`

Before doing anything else, read `@.cursor/cloud-agent-prompts/wave-1/README.md`
in full and comply with every shared rule in it.

## Task (copied verbatim from the plan todo)

> Implement `abi::detect` using `wasmparser` 0.247 to classify bytes as
> `Component` or `Core`. Drive it off `wasmparser::Parser::parse_all` and
> match the first `Payload::Version { encoding: Encoding::Component |
> Encoding::Module }`. Also run `wasmparser::validate(bytes)` for full
> validation before handing bytes to Wasmtime. Honor `options.abi` only to
> force `core` parsing; reject unknown encodings with
> `PgWasmError::ValidationFailed`. Add host-only unit tests with
> hand-crafted binaries.

Authoritative design sections:
`docs/architecture.md` §§ "ABI detection and invocation shape",
"Validation".

API references (pinned to 0.247):
- `wasmparser::Parser::parse_all`:
  https://docs.rs/wasmparser/0.247.0/wasmparser/struct.Parser.html
- `wasmparser::Payload`:
  https://docs.rs/wasmparser/0.247.0/wasmparser/enum.Payload.html
- `wasmparser::Encoding`:
  https://docs.rs/wasmparser/0.247.0/wasmparser/enum.Encoding.html
- `wasmparser::validate`:
  https://docs.rs/wasmparser/0.247.0/wasmparser/fn.validate.html

## Files you own

- `pg_wasm/src/abi.rs`

## Files you must not touch

- `pg_wasm/src/lib.rs`.
- `pg_wasm/src/errors.rs` — see note below about `ValidationFailed`.
- Every other `pg_wasm/src/*.rs` file (they belong to other Wave-1 agents
  or later waves).
- `Cargo.toml`, `pg_wasm/Cargo.toml`, `pg_wasm/pg_wasm.control`.
- Any line of the plan file other than your own todo's `status:` line.

## Implementation notes

- **Public API** (minimum):
  - `pub(crate) enum Abi { Component, Core }`
  - `pub(crate) enum AbiOverride { Auto, ForceCore }` (accepted via the
    later `LoadOptions`; this task just defines the enum and honors it).
  - `pub(crate) fn detect(bytes: &[u8], override_: AbiOverride) -> Result<Abi, PgWasmError>`
  - Optionally `pub(crate) fn validate(bytes: &[u8]) -> Result<(), PgWasmError>`
    wrapping `wasmparser::validate` with a pg-friendly error.
- **Classification**: iterate `Parser::new(0).parse_all(bytes)`. On the
  first `Ok(Payload::Version { encoding, .. })`:
  - `Encoding::Component` → `Abi::Component` (rejected if
    `AbiOverride::ForceCore`).
  - `Encoding::Module` → `Abi::Core`.
  - Any other/unknown variant or a parse error before the `Version`
    payload → `PgWasmError::ValidationFailed`.
- **Full validation**: after classification, call
  `wasmparser::validate(bytes)`. Propagate failure as
  `PgWasmError::ValidationFailed(format!("{:#}", err))`.
- **`PgWasmError::ValidationFailed` is not yet defined.** You may add it
  to `pg_wasm/src/errors.rs` in **append-only** form (add a new variant
  at the bottom of the enum, add its `sqlstate()` arm mapping to
  `ERRCODE_INVALID_BINARY_REPRESENTATION`). This is the single permitted
  edit outside your owned files; no other `errors.rs` changes, no
  reordering of existing variants. If two Wave-1 agents both need to
  append variants, whichever PR merges second just rebases and appends
  after the first.

## Validation expectations

- `cargo check -p pg_wasm` passes.
- Host-only `#[test]`s with hand-crafted bytes:
  - Minimal valid core module header: `[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]` → `Abi::Core`.
  - Minimal valid component header (same magic, component-model version
    word — see the component-model binary format spec): `Abi::Component`.
  - `AbiOverride::ForceCore` applied to component bytes → error.
  - Truncated magic (e.g. first 4 bytes only) → `ValidationFailed`.
  - Valid magic but invalid body (a section with bogus length) →
    `ValidationFailed` via `wasmparser::validate`.

## Final commit

Flip the `status:` line for `abi-detect` in
`.cursor/plans/pg_wasm_extension_implementation_v2.plan.md` from `pending`
to `completed`. No other edits to that file.
