# Wave-2 Cloud Agent: `component-marshal-dynamic`

**Branch**: `wave-2/component-marshal-dynamic` (base: `main`)
**PR title**: `[wave-2] component-marshal-dynamic: dynamic Val marshaling for WIT composites`

Read `@.cursor/cloud-agent-prompts/wave-2/README.md` and
`@.cursor/cloud-agent-prompts/wave-1/README.md` first.

## Task (copied verbatim)

> Implement `mapping::composite` and `mapping::list` on the dynamic
> `wasmtime::component::Val` path. For each WIT type produce a
> marshaler that consumes a PG `Datum` and returns a `Val`, and vice
> versa. Cover records (named + anonymous tuples), variants, enums,
> flags, options, results, and typed lists. Call exports via
> `wasmtime::component::Func::call(&mut store, &[Val], &mut [Val])`
> (v43 takes a caller-provided result slice rather than returning a
> `Vec`) and call `Func::post_return` after each invocation before
> reusing the instance.

Design ref: `docs/architecture.md` §§ "Dynamic marshaling",
"Invocation lifecycle".

Pinned API:
- `wasmtime::component::Val`:
  https://docs.rs/wasmtime/43.0.0/wasmtime/component/enum.Val.html
- `wasmtime::component::Func::call` / `post_return`:
  https://docs.rs/wasmtime/43.0.0/wasmtime/component/struct.Func.html

## Files you own

- `pg_wasm/src/mapping/composite.rs` (new)
- `pg_wasm/src/mapping/list.rs` (new)

Declare both in `pg_wasm/src/mapping/mod.rs` (add
`pub mod composite;` and `pub mod list;`). If the `mapping.rs` → `mapping/`
conversion was not already done by `core-module-scalar-path`, do it
here: move current `mapping.rs` content into `mapping/mod.rs`.

## Files you must not touch

- All files listed in `@.cursor/cloud-agent-prompts/wave-2/README.md`.
- `pg_wasm/src/mapping/scalars.rs` (sibling — `core-module-scalar-path`).
- `pg_wasm/src/wit/*` (Wave-1 outputs; read-only).
- `pg_wasm/src/runtime/*` (sibling Wave-2 territory).

## Implementation notes

- **Marshal plan**: `mapping::composite` exposes
  `pub(crate) fn plan_marshaler(plan: &wit::typing::TypePlan,
  export: &wit::typing::Export) -> MarshalPlan`. The plan is built
  **once at load time** (called by future `load-orchestration`) and
  cached. Per-call marshaling walks the plan, not the raw
  `TypeDefKind`.
- **`MarshalPlan`** enum mirroring `TypeDefKind`: `Scalar(ScalarKind)`,
  `Record(Vec<FieldPlan>)`, `Variant(Vec<CasePlan>)`, `Enum(&[name])`,
  `Flags(&[name])`, `Option(Box<MarshalPlan>)`,
  `Result(Box<MarshalPlan>, Box<MarshalPlan>)`,
  `Tuple(Vec<MarshalPlan>)`, `List(Box<MarshalPlan>)`.
- **Direction**: one function per direction
  - `pub(crate) fn datum_to_val(plan: &MarshalPlan, datum: pg_sys::Datum,
    is_null: bool, typmod: i32) -> Result<Val, PgWasmError>`
  - `pub(crate) fn val_to_datum(plan: &MarshalPlan, val: &Val) ->
    Result<(pg_sys::Datum, /*is_null*/ bool), PgWasmError>`
- **Records**: PG composite rows in and out. Build `HeapTuple` via
  pgrx's `PgHeapTuple` (or equivalent pgrx 0.18 helper) for out
  direction; destructure via `pgrx::composite_type!` for in direction.
  Preserve field order from the `TypePlan`.
- **Variants**: mirror whatever shape `wit::typing` emitted (usually
  composite `(discriminant text, payload <subtype>)`). Discriminant
  name lookup is via the plan.
- **Enums / flags**: PG `enum` text value ↔ Wasm `u32` index / bitset.
- **Lists**: `mapping::list` handles `list<T>`:
  - Special-case `list<u8>` to PG `bytea` for zero-copy.
  - General lists to PG arrays via `pgrx::Array<T>` where T has a PG
    mapping; unsupported element types → `PgWasmError::Unsupported`.
- **`list<u8>` zero-copy**: accept PG `bytea::varlena_ptr`; produce
  `Val::List` of `Val::U8` only if the component actually requires a
  list (don't unpack bytea if a direct slice view will work through
  `Val::Bytes`-like — check v43 API; fall back to `Vec<Val>` if
  needed).
- **Call helper** `invoke_component(func: &Func, store: &mut Store,
  args: &[Val]) -> Result<Vec<Val>, PgWasmError>`:
  - Allocate `results = vec![Val::Bool(false); expected_result_count]`
    sized from the pre-computed plan.
  - `func.call(&mut *store, args, &mut results)?`
  - `func.post_return(&mut *store)?` — **always** call before returning
    a non-error result so the instance is reusable.
  - Map `wasmtime::Error` to `PgWasmError` via the trap-downcast
    rules in the `invocation-path` todo (for now, map all errors to
    `PgWasmError::Internal`; `invocation-path` in Wave 3 will refine
    to trap-specific codes).

## Validation expectations

- `cargo check -p pg_wasm` passes.
- Host-only `#[test]` per direction for each kind: record, variant,
  enum, flags, option::some / option::none, result::ok / result::err,
  tuple, list<T> (at least for `u8`, `u32`, and nested
  `list<record>`).

## Final commit

Flip `component-marshal-dynamic`'s `status:` line to `completed`.
