# Wave-1 Cloud Agent: `wit-type-resolver`

**Branch**: `wave-1/wit-type-resolver` (base: `main`)
**PR title**: `[wave-1] wit-type-resolver: decode components and map WIT types to a stable PG plan`

Before doing anything else, read `@.cursor/cloud-agent-prompts/wave-1/README.md`
in full and comply with every shared rule in it.

## Task (copied verbatim from the plan todo)

> Implement `wit::world` (parse components via `wit_component::decode`
> from wit-component 0.247, destructuring the
> `DecodedWasm::Component(Resolve, WorldId)` variant) and `wit::typing`
> with the full `wit_to_pg` mapping over `wit_parser::{Resolve, Type,
> TypeDef, TypeDefKind}` (bool, s*/u*, f32/f64, char, string, list<u8>,
> list<T>, option, result, tuple, record, variant, enum, flags,
> resource/handle). Produce a stable plan keyed by module prefix.
> Normalize world output with `wit_component::WitPrinter` for storage in
> `pg_wasm.modules.wit_world`.

Authoritative design sections:
`docs/architecture.md` §§ "WIT type resolver", "WIT → PostgreSQL type
mapping", "UDT planning and stable keys".

API references (pinned to 0.247):
- `wit_component::decode`:
  https://docs.rs/wit-component/0.247.0/wit_component/fn.decode.html
- `wit_component::DecodedWasm`:
  https://docs.rs/wit-component/0.247.0/wit_component/enum.DecodedWasm.html
- `wit_component::WitPrinter`:
  https://docs.rs/wit-component/0.247.0/wit_component/struct.WitPrinter.html
- `wit_parser::Resolve`:
  https://docs.rs/wit-parser/0.247.0/wit_parser/struct.Resolve.html
- `wit_parser::{Type, TypeDef, TypeDefKind}`:
  https://docs.rs/wit-parser/0.247.0/wit_parser/enum.Type.html
  https://docs.rs/wit-parser/0.247.0/wit_parser/struct.TypeDef.html
  https://docs.rs/wit-parser/0.247.0/wit_parser/enum.TypeDefKind.html

## Files you own

- `pg_wasm/src/wit.rs` — currently a flat file. Convert to
  `pg_wasm/src/wit/mod.rs` plus:
  - `pg_wasm/src/wit/world.rs`
  - `pg_wasm/src/wit/typing.rs`
  (Leave `pg_wasm/src/wit/udt.rs` for the later Wave-2 `udt-registration`
  task; do not create that file.)

## Files you must not touch

- `pg_wasm/src/lib.rs`.
- `pg_wasm/src/mapping.rs` — that's touched in Wave 2 (`component-marshal-dynamic`).
- `pg_wasm/src/abi.rs` — owned by the `abi-detect` agent; if you need
  classification info, assume `abi::Abi::Component` bytes as input.
- Every other `pg_wasm/src/*.rs` file.
- `Cargo.toml`, `pg_wasm/Cargo.toml`, `pg_wasm/pg_wasm.control`.
- Any line of the plan file other than your own todo's `status:` line.

## Implementation notes

- **`wit::world::decode(bytes: &[u8]) -> Result<DecodedWorld, PgWasmError>`**:
  - Call `wit_component::decode(bytes)`.
  - Destructure `DecodedWasm::Component(resolve, world_id)`. If it's a
    `WitPackage` variant or anything else, return
    `PgWasmError::InvalidModule("component did not embed a world")`.
  - Bundle `Resolve` + `WorldId` in a `DecodedWorld { resolve, world_id
    }`.
  - Print the world to normalized WIT text via `WitPrinter::default()`
    then `.print_world(&resolve, world_id, ...)`; store as a `String`
    field `wit_text` on `DecodedWorld` — the later `load-orchestration`
    task will persist this into `pg_wasm.modules.wit_world`.
- **`wit::typing`** — the central WIT → PostgreSQL type planner. Public
  API at minimum:
  ```rust
  pub(crate) struct TypePlan { /* stable, hashable */ }

  pub(crate) fn plan_types(
      module_prefix: &str,
      decoded: &DecodedWorld,
  ) -> Result<TypePlan, PgWasmError>;
  ```
  - `module_prefix` is used to derive stable schema-qualified names for
    UDTs: `<prefix>_<wit_type_name_snake_case>`. This keeps later
    `ALTER TYPE` reloads safe (same plan → same names → same OIDs).
  - `TypePlan` contains an ordered list of WIT type definitions with
    their mapped PG representation (enum vs composite vs domain vs
    scalar) plus their dependency order (so UDT registration can issue
    CREATE TYPE / CREATE DOMAIN in the right sequence).
  - Cover every `TypeDefKind` listed in the plan: record, variant, enum,
    flags, option, result, tuple, list, resource, handle. Primitive
    mappings (`bool`, `sN`, `uN`, `f32`, `f64`, `char`, `string`,
    `list<u8>`) go to built-in PG types (`boolean`, `int2/int4/int8`
    with domain wrappers for unsigned, `real`, `double precision`,
    `"char"`, `text`, `bytea`).
- **Stable keys**: derive a deterministic `type_key: String` for each
  WIT type (e.g. `"<package>:<interface>/<type_name>"`) that survives
  reload. Stash it in `TypePlan` so `wit_types` catalog rows can be
  matched across loads.
- **Determinism**: sort every collection (record fields, variant cases,
  flags) by their WIT-declared order — do not sort alphabetically. Two
  equivalent `TypePlan`s must hash identically.

## Validation expectations

- `cargo check -p pg_wasm` passes.
- Host-only `#[test]`s in `wit::typing` / `wit::world`:
  - Decode a minimal component fixture that embeds a simple world with
    one record and one enum; assert the returned plan matches an
    expected snapshot (inline, or via `insta` if already a dev-dep — do
    **not** add new deps; if `insta` is not present, use `assert_eq!`
    on a `Debug` string).
  - Round-trip `WitPrinter` output: decode → print → parse again →
    print → assert stable.
  - Error paths: core-module bytes return `InvalidModule`; corrupted
    component bytes return `InvalidModule`.
  - A known-empty world returns an empty `TypePlan`.
- Fixture source: you can either hand-encode a trivial component or call
  `wit_component::ComponentEncoder` at test time to build one from an
  inline WIT string.

## Final commit

Flip the `status:` line for `wit-type-resolver` in
`.cursor/plans/pg_wasm_extension_implementation_v2.plan.md` from `pending`
to `completed`. No other edits to that file.
