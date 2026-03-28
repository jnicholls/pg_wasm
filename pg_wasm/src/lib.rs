use pgrx::prelude::*;

mod config;
mod mapping;
mod registry;
mod runtime;
mod trampoline;

pub use config::{HostPolicy, LoadOptions};
pub use mapping::{ExportSignature, PgWasmArgDesc, PgWasmReturnDesc, PgWasmTypeKind};
pub use registry::{
    ModuleId, RegisteredFunction, lookup_by_fn_oid, register_fn_oid, unregister_fn_oid,
};
pub use runtime::{RuntimeKind, StubWasmBackend, WasmRuntimeBackend};
pub use trampoline::TRAMPOLINE_PG_SYMBOL;

#[cfg(feature = "runtime_wasmtime")]
pub use runtime::wasmtime_backend::WasmtimeBackend;
#[cfg(feature = "runtime_wasmer")]
pub use runtime::wasmer_backend::WasmerBackend;
#[cfg(feature = "runtime_extism")]
pub use runtime::extism_backend::ExtismBackend;

::pgrx::pg_module_magic!(name, version);

#[pg_extern]
fn hello_pg_wasm() -> &'static str {
    "Hello, pg_wasm"
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;
    use pgrx::spi::Spi;

    use crate::mapping::ExportSignature;
    use crate::registry::{ModuleId, RegisteredFunction, register_fn_oid};
    #[pg_test]
    fn test_hello_pg_wasm() {
        assert_eq!("Hello, pg_wasm", crate::hello_pg_wasm());
    }

    /// `CREATE FUNCTION` pointing at the trampoline, then registry + `SELECT` returns the placeholder.
    #[pg_test]
    fn test_trampoline_dispatch_via_sql_function() {
        let create_sql = concat!(
            "CREATE OR REPLACE FUNCTION public.pg_wasm_trampoline_smoke() ",
            "RETURNS integer LANGUAGE C STRICT VOLATILE PARALLEL UNSAFE ",
            "AS '$libdir/pg_wasm', 'pg_wasm_udf_trampoline'",
        );
        Spi::run(create_sql).expect("create pg_wasm_trampoline_smoke");

        let oid = Spi::get_one::<pg_sys::Oid>(
            "SELECT 'public.pg_wasm_trampoline_smoke()'::regprocedure::oid",
        )
        .expect("spi get oid")
        .expect("missing regprocedure oid");

        register_fn_oid(
            oid,
            RegisteredFunction {
                module_id: ModuleId(1),
                export_name: "smoke".into(),
                signature: ExportSignature::default(),
            },
        );

        let v = Spi::get_one::<i32>("SELECT public.pg_wasm_trampoline_smoke()")
            .expect("spi select")
            .expect("null result");
        assert_eq!(v, 42);

        Spi::run("DROP FUNCTION public.pg_wasm_trampoline_smoke()")
            .expect("drop pg_wasm_trampoline_smoke");
        crate::unregister_fn_oid(oid);
    }

    #[cfg(feature = "runtime_wasmtime")]
    #[pg_test]
    fn test_wasmtime_backend_instantiates() {
        let _ = crate::WasmtimeBackend::new();
    }
}

/// Required by `cargo pgrx test`.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {}

    #[must_use]
    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}

#[cfg(test)]
mod rust_tests {
    use pgrx::pg_sys;

    #[test]
    fn trampoline_link_symbol_is_pg_wasm_udf_trampoline() {
        assert_eq!(crate::TRAMPOLINE_PG_SYMBOL, "pg_wasm_udf_trampoline");
    }

    #[test]
    fn registry_lookup_miss_for_invalid_oid() {
        assert!(crate::lookup_by_fn_oid(pg_sys::InvalidOid).is_none());
    }
}
