---
name: pg_wasm Extension Implementation (v2)
overview: |
  Implement a pgrx-based PostgreSQL extension, `pg_wasm`, that binds WebAssembly
  modules and components to SQL-visible functions. v2 centers on the WebAssembly
  Component Model (WIT) with Wasmtime as the single runtime, auto-registers
  user-defined WIT types as PostgreSQL composite types, enums, and domains,
  enforces layered sandbox policy through GUCs and per-module overrides, and
  supports full lifecycle (load, unload, reload, reconfigure) with durable
  catalog tables and on-disk compiled artifacts. Design reference:
  `docs/architecture.md`.
todos:
  - id: bootstrap-layout
    content: Restructure `pg_wasm/src/` into the v2 module layout (guc, errors, catalog, artifacts, shmem, registry, config, policy, abi, wit, runtime, mapping, proc_reg, trampoline, lifecycle, hooks, views). Add workspace deps (wasmtime with component-model, wasmtime-wasi, wasmtime-wasi-http, wit-component, wit-parser, wasmparser, serde_json, thiserror, anyhow, sha2). Keep `hello_pg_wasm` temporarily as a smoke test.
    status: pending
  - id: errors-and-guc
    content: Implement `errors::PgWasmError` with SQLSTATE mapping and define every `pg_wasm.*` GUC in `guc.rs` (enabled, allow_load_from_file, module_path, allowed_path_prefixes, max_module_bytes, max_modules, max_exports, allow_wasi, allow_wasi_{stdio,env,fs,net,http}, wasi_preopens, allowed_hosts, allow_spi, max_memory_pages, max_instances_total, fuel_enabled, fuel_per_invocation, invocation_deadline_ms, epoch_tick_ms, collect_metrics, log_level, follow_symlinks). Register them in `_PG_init`.
    status: pending
  - id: catalog-schema
    content: Add `pg_wasm.modules`, `pg_wasm.exports`, `pg_wasm.wit_types`, `pg_wasm.dependencies` tables in versioned SQL. Implement `catalog::{modules,exports,wit_types}` CRUD via SPI. Set up `pg_wasm_loader` and `pg_wasm_reader` roles with minimal grants. Add `catalog::migrations` that validates shape on `_PG_init`.
    status: pending
  - id: shmem-and-generation
    content: Implement `shmem.rs` with a per-cluster segment sized by `max_modules` / `max_exports`. Provide `bump_generation(module_id)`, `read_generation()`, and atomic per-export counters. Protect mutators with `pg_wasm.CatalogLock` (LWLock). Wire into `shmem_request_hook` and `shmem_startup_hook`.
    status: pending
  - id: artifacts-layout
    content: Implement `artifacts.rs` for `$PGDATA/pg_wasm/<module_id>/` (module.wasm, module.cwasm, world.wit). Include atomic write (temp + rename), directory fsync, checksum verification (sha256), and a `prune_stale` helper for orphaned dirs.
    status: pending
  - id: policy-resolve
    content: Define `config::{LoadOptions, PolicyOverrides, Limits}` and `policy::{EffectivePolicy, resolve}`. Enforce narrowing semantics (overrides can only deny/tighten). Cover with host-only unit tests for every combination.
    status: pending
  - id: abi-detect
    content: Implement `abi::detect` using `wasmparser` to classify bytes as `Component` or `Core`. Honor `options.abi` only to force `core` parsing. Reject unknown encodings with `PgWasmError::ValidationFailed`. Add host-only unit tests with hand-crafted binaries.
    status: pending
  - id: engine-and-epoch-ticker
    content: Implement `runtime::engine::shared_engine()` returning a lazily-initialized `wasmtime::Engine` configured with component model, epoch interruption, optional fuel, async disabled, parallel compilation disabled. Start the epoch ticker thread from `_PG_init` reading `pg_wasm.epoch_tick_ms`.
    status: pending
  - id: trampoline-stub
    content: Add `trampoline::pg_wasm_udf_trampoline` C entry point that resolves `fn_oid` through `registry::FN_OID_MAP`. Initially returns a constant; wire `registry` with a generation-aware cache that refreshes from catalog on miss.
    status: pending
  - id: proc-reg-ddl
    content: Implement `proc_reg::{register, unregister}` wrapping `ProcedureCreate` / `RemoveFunctionById` and `recordDependencyOn(DEPENDENCY_EXTENSION)`. Validate name collision handling per `options.replace_exports`.
    status: pending
  - id: core-module-scalar-path
    content: Implement `runtime::core` for core modules with scalar-only ABI (i32/i64/f32/f64/bool). Implement `mapping::scalars` and end-to-end load -> trampoline -> call on a fixture `add_i32.wat`. Verify via pg_regress golden output.
    status: pending
  - id: wit-type-resolver
    content: Implement `wit::world` (parse components via `wit-component::decode`) and `wit::typing` with the full `wit_to_pg` mapping table (bool, s*/u*, f32/f64, char, string, list<u8>, list<T>, option, result, tuple, record, variant, enum, flags, resource). Produce a stable plan keyed by module prefix.
    status: pending
  - id: udt-registration
    content: Implement `wit::udt::register_type_plan` that issues `CREATE TYPE`, `CREATE DOMAIN`, `CREATE ENUM` DDL via SPI and records rows in `pg_wasm.wit_types` with `recordDependencyOn`. Idempotent for reload-compatible definitions; updates OIDs in-place when definitions match.
    status: pending
  - id: component-compile-and-pool
    content: Implement `runtime::component` to compile a `wasmtime::component::Component`, precompile a `.cwasm` to disk, and stand up a `Linker` with `wasmtime_wasi::preview2::add_to_linker_sync` behind policy toggles. Implement `runtime::pool` with a per-module bounded instance pool sized by `pg_wasm.instances_per_module` (new GUC).
    status: pending
  - id: component-marshal-dynamic
    content: Implement `mapping::composite` and `mapping::list` on the dynamic `component::Val` path. For each WIT type produce a marshaler that consumes a PG `Datum` and returns a `Val`, and vice versa. Cover records (named + anonymous tuples), variants, enums, flags, options, results, and typed lists.
    status: pending
  - id: load-orchestration
    content: Implement `lifecycle::load` running AuthZ -> read -> validate -> classify -> resolve WIT -> plan types -> plan exports -> resolve policy -> compile + persist -> register procs -> on-load hook -> bump generation. All DDL runs via SPI inside one transaction; failure rolls everything back and removes the module dir.
    status: pending
  - id: unload-orchestration
    content: Implement `lifecycle::unload` with `on-unload` hook, `RemoveFunctionById`, UDT drop (respecting `pg_wasm.dependencies` and `options.cascade`), catalog row deletion, artifact dir removal, generation bump.
    status: pending
  - id: reload-orchestration
    content: Implement `lifecycle::reload` that preserves `fn_oid` / `pg_type.oid` when signatures/definitions are unchanged, issues `ALTER TYPE` where possible, and errors on breaking changes unless `options.breaking_changes_allowed`. Atomic module.wasm swap via temp + rename.
    status: pending
  - id: reconfigure-orchestration
    content: Implement `lifecycle::reconfigure` that updates `policy` / `limits` rows, calls `on-reconfigure` hook, and bumps generation. Confirm `StoreLimits` and epoch deadlines pick up the new values on next call via integration test.
    status: pending
  - id: host-interfaces
    content: Implement `pg_wasm:host/log` (maps to `ereport(NOTICE/INFO/WARNING)`) and `pg_wasm:host/query` (SPI read-only by default, gated by `pg_wasm.allow_spi`). Provide WIT text in `pg_wasm/wit/host.wit` and wire into the component `Linker`.
    status: pending
  - id: invocation-path
    content: Flesh out `trampoline::pg_wasm_udf_trampoline` to borrow a pooled instance, set fuel + epoch deadline, marshal args, call typed export, unmarshal, and update shmem counters. Translate `Trap::Interrupt` to `ERRCODE_QUERY_CANCELED`, other traps to `PgWasmError::Trap`. Wrap in `catch_unwind`.
    status: pending
  - id: metrics-and-views
    content: Implement `views::{modules, functions, stats, wit_types, policy_effective}` as SRF table functions backed by catalog rows and shmem atomics. Add grants so `pg_wasm_reader` can read `stats()`. Add regress tests asserting counter shape and monotonicity.
    status: pending
  - id: hooks
    content: Implement `hooks::{on_load, on_unload, on_reconfigure}` invocations with config blob passing. Hooks are optional component exports with stable names; absence is not an error. on-unload failures are logged, not fatal.
    status: pending
  - id: error-mapping
    content: Finalize `errors::PgWasmError` -> `ereport` conversion, including SQLSTATE, MESSAGE, DETAIL (module_id, export_id, wasmtime_version), HINT (policy hints on denials).
    status: pending
  - id: concurrency-safety
    content: Add `pg_wasm.CatalogLock` (LWLock tranche) held during load/unload/reload/reconfigure catalog mutation and shmem generation bumps. Confirm in-flight invocations complete against the old handle under reload. Stress-test with an integration test issuing concurrent loads + calls.
    status: pending
  - id: pg_upgrade-and-extension-upgrade
    content: Verify artifacts survive `pg_upgrade`. Implement `Engine::is_compatible_with_precompiled_component_file` fallback that recompiles on first use. Add `sql/pg_wasm--X.Y--X.Z.sql` scaffolding; `catalog::migrations` validates shape on `_PG_init`.
    status: pending
  - id: test-corpus-and-pg_regress
    content: Build component fixtures (arith, strings, records, enums, variants, hooks, policy_probe, resources) and core fixtures (add_i32, echo_mem). Author pg_regress suites for lifecycle, WIT mappings, policy narrowing, error classes, metrics. Deterministic output with `ORDER BY` and `EXPLAIN (COSTS OFF, TIMING OFF)`.
    status: pending
  - id: integration-tests
    content: Add workspace `tests/` crate using `tokio-postgres`. Cover concurrent-backend load visibility via generation bumps, backend restart recovery, query cancellation via epoch interruption, fuel exhaustion, memory-pages limit, WASI policy denials.
    status: pending
  - id: docs-and-readme
    content: Update `README.md` with component-first usage. Write `docs/guc.md` (every GUC with default, scope, hot/cold reconfig), `docs/wit-mapping.md` (the full WIT -> PG table with examples). Reference them from `docs/architecture.md`.
    status: pending
  - id: build-features
    content: Set `default = ["pg13", "component-model"]`. Feature `core-only` builds without component model by gating `wit/`, `runtime/component`, `mapping/composite`, `mapping/list`. Confirm cargo check passes in both configurations and on `pg13..pg18`.
    status: pending
isProject: false
---

# pg_wasm Extension Implementation Plan (v2)

## Current state

- Workspace with [Cargo.toml](../../Cargo.toml) and [pg_wasm/Cargo.toml](../../pg_wasm/Cargo.toml); pgrx 0.18; PG 13–18 feature flags.
- [pg_wasm/src/lib.rs](../../pg_wasm/src/lib.rs) is the minimal `hello_pg_wasm` stub plus the pgrx test scaffolding.
- Prior v1 experiment (see branch `origin/v1`) explored Wasmtime + Extism with buffer-style ABI and explicit `exports` hints; v2 narrows to **Wasmtime + Component Model (WIT)** with automatic UDT registration.

The authoritative architectural design is in [`docs/architecture.md`](../../docs/architecture.md); this plan is the execution roadmap against that design.

---

## Goals re-stated

1. **Wasmtime-only runtime** with first-class Component Model + WIT support; core modules kept as a degraded scalar path.
2. **Automatic WIT → PostgreSQL type mapping**, including UDT registration of records, variants, enums, flags, and domains for unsigned integers.
3. **Durable state**: catalog tables under the extension schema and on-disk compiled artifacts under `$PGDATA/pg_wasm/`.
4. **Full lifecycle**: `load`, `unload`, `reload`, `reconfigure` with generation-driven cache invalidation across backends.
5. **Strong, narrowable sandbox**: extension-scope GUCs define the ceiling; per-module overrides may only narrow.
6. **Low per-call overhead**: compile at load, amortize instantiate via per-backend instance pools, keep the trampoline thin.
7. **Observability**: SRF views over catalog + shared-memory counters.

---

## Implementation order

The `todos` list above is authoritative. Each entry is designed to land in its own commit and be individually testable. Group boundaries (informational; all still land one-at-a-time):

1. **Foundation**: `bootstrap-layout`, `errors-and-guc`, `catalog-schema`, `shmem-and-generation`, `artifacts-layout`.
2. **Policy and ABI**: `policy-resolve`, `abi-detect`.
3. **Runtime skeleton**: `engine-and-epoch-ticker`, `trampoline-stub`, `proc-reg-ddl`, `core-module-scalar-path`.
4. **Component Model + WIT**: `wit-type-resolver`, `udt-registration`, `component-compile-and-pool`, `component-marshal-dynamic`.
5. **Lifecycle**: `load-orchestration`, `unload-orchestration`, `reload-orchestration`, `reconfigure-orchestration`.
6. **Host surfaces**: `host-interfaces`, `invocation-path`, `hooks`.
7. **Error model and concurrency**: `error-mapping`, `concurrency-safety`.
8. **Operations**: `pg_upgrade-and-extension-upgrade`, `metrics-and-views`.
9. **Testing**: `test-corpus-and-pg_regress`, `integration-tests`.
10. **Polish**: `docs-and-readme`, `build-features`.

---

## Key design decisions captured from the design doc

The full rationale lives in [`docs/architecture.md`](../../docs/architecture.md). Summarized for this plan:

- **One trampoline symbol**, many `pg_proc` rows, resolution via `flinfo->fn_oid` → `(module_id, export)` in a generation-aware process-local cache.
- **Durable catalog** (`pg_wasm.modules`, `pg_wasm.exports`, `pg_wasm.wit_types`, `pg_wasm.dependencies`) plus **on-disk artifacts** (`$PGDATA/pg_wasm/<module_id>/{module.wasm,module.cwasm,world.wit}`).
- **Shared memory** carries the generation counter and per-export atomic counters; sized by bounded GUCs; overflow falls back to non-shared counters with `shared := false`.
- **Policy narrowing** is enforced in `policy::resolve`; per-module overrides can only deny what GUCs permit.
- **WIT resolver** is deterministic and stable so reload can preserve OIDs on unchanged types.
- **Wasmtime configuration**: component model on, epoch interruption on, parallel compilation off, async off, fuel optional.
- **Instance pool** per module per backend, bounded by `pg_wasm.instances_per_module`; fresh `Store` per call with policy-driven `StoreLimits`.
- **Host interfaces** limited to `pg_wasm:host/log` and `pg_wasm:host/query`; everything else is WASI behind feature-scoped allow flags.

---

## Risks and mitigations

- **WIT dynamic marshaling overhead.** Walking the type tree on every call is measurable. Mitigation: cache the marshal plan per export at load time; revisit with bindgen-generated specializations after v2 lands if profiling shows hot spots.
- **Wasmtime vs PG version interactions.** pgrx, PG major versions, and Wasmtime all move. Mitigation: lock Wasmtime in the workspace, run the full `cargo pgrx test` matrix on pg13..pg18 in CI, treat `Engine::is_compatible_with_*` as the upgrade oracle.
- **Reload OID preservation corner cases.** `ALTER TYPE ADD/DROP ATTRIBUTE` on composite types has restrictions (e.g. must not have dependent rows of the type). Mitigation: detect unsupported transitions up front, error with a specific hint, require `breaking_changes_allowed` to continue.
- **Shared-memory sizing.** `max_modules` / `max_exports` are bounded at start-up. Mitigation: document the overflow behavior (`shared := false`), surface in `pg_wasm.stats()`, add a startup log line showing actual sizing.
- **WASI surface growth.** New WASI interfaces arrive regularly. Mitigation: explicit allow-list in `runtime::wasi::build_linker`; unknown interfaces cause instantiation failure with a helpful error.
- **Epoch-ticker thread lifecycle.** A per-process thread must not outlive the backend. Mitigation: start lazily via `OnceLock`, terminate on `atexit` hook registered from `_PG_init`; avoid storing any pgrx handles inside the thread.

---

## Out of scope for v2

- Extism and Wasmer backends.
- Shared-memory-backed guest linear memory (explicit non-goal).
- Hot-patching individual exports (replaced by reload).
- `wasi:keyvalue`, `wasi:blobstore` and other experimental WASI worlds (tracked as open questions in the design doc).

---

## References

- Design doc: [`docs/architecture.md`](../../docs/architecture.md)
- Testing rules: [`AGENTS.md`](../../AGENTS.md), `.cursor/rules/pg-wasm-pgrx-testing.mdc`
- pgrx 0.18 docs: https://docs.rs/pgrx/0.18
- Wasmtime component model: https://docs.rs/wasmtime/latest/wasmtime/component/
- WIT parsing: https://docs.rs/wit-component/latest/wit_component/, https://docs.rs/wit-parser/latest/wit_parser/
