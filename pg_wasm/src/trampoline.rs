//! Single exported C symbol for all dynamically registered WASM UDFs.
//!
//! PostgreSQL `prosrc` must be [`TRAMPOLINE_PG_SYMBOL`] (not the `…_wrapper` suffix used by
//! `#[pg_extern]`), with a matching `pg_finfo_*` entry for the v1 call convention.

use pgrx::{pg_sys, prelude::*};

/// `CREATE FUNCTION … AS '$libdir/pg_wasm', '…'` link name for the trampoline body.
pub const TRAMPOLINE_PG_SYMBOL: &str = "pg_wasm_udf_trampoline";

#[unsafe(no_mangle)]
#[doc(hidden)]
pub extern "C" fn pg_finfo_pg_wasm_udf_trampoline() -> &'static pg_sys::Pg_finfo_record {
    const V1: pg_sys::Pg_finfo_record = pg_sys::Pg_finfo_record { api_version: 1 };
    &V1
}

/// Entry point for every WASM-backed SQL function; dispatch uses `flinfo->fn_oid` and the registry.
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn pg_wasm_udf_trampoline(
    fcinfo: pg_sys::FunctionCallInfo,
) -> pg_sys::Datum {
    unsafe { pgrx::pg_sys::ffi::pg_guard_ffi_boundary(|| dispatch_from_trampoline(fcinfo)) }
}

fn dispatch_from_trampoline(fcinfo: pg_sys::FunctionCallInfo) -> pg_sys::Datum {
    if fcinfo.is_null() {
        error!("pg_wasm: null fcinfo in trampoline");
    }
    let flinfo = unsafe { (*fcinfo).flinfo };
    if flinfo.is_null() {
        error!("pg_wasm: null flinfo in trampoline");
    }
    let oid = unsafe { (*flinfo).fn_oid };
    match crate::registry::lookup_by_fn_oid(oid) {
        Some(reg) => trampoline_return_placeholder(reg),
        None => error!("pg_wasm: no wasm dispatch entry for function OID {}", oid),
    }
}

fn trampoline_return_placeholder(_reg: crate::registry::RegisteredFunction) -> pg_sys::Datum {
    // Replaced with real WASM invocation in a later todo.
    42i32
        .into_datum()
        .expect("pg_wasm: trampoline int4 into_datum failed")
}
