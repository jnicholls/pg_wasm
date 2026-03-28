//! PostgreSQL ↔ WASM value representation (marshal/unmarshal filled in with the trampoline).

use pgrx::pg_sys::Oid;

/// Classifies how a SQL argument maps to the WASM ABI for core modules.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PgWasmTypeKind {
    I32,
    I64,
    F32,
    F64,
    /// Length-prefixed UTF-8 or pointer/length pair (runtime-specific).
    String,
    /// Opaque bytes (e.g. JSONB serialized).
    Bytes,
}

/// Describes one SQL argument position for dynamic dispatch.
#[derive(Clone, Debug)]
pub struct PgWasmArgDesc {
    pub pg_oid: Oid,
    pub kind: PgWasmTypeKind,
}

/// Describes the return mapping for a WASM export registered as a UDF.
#[derive(Clone, Debug)]
pub struct PgWasmReturnDesc {
    pub pg_oid: Oid,
    pub kind: PgWasmTypeKind,
}

impl Default for PgWasmReturnDesc {
    fn default() -> Self {
        Self {
            pg_oid: pgrx::pg_sys::INT4OID,
            kind: PgWasmTypeKind::I32,
        }
    }
}

/// Placeholder for the per-export signature table used by the trampoline.
#[derive(Clone, Debug, Default)]
pub struct ExportSignature {
    pub args: Vec<PgWasmArgDesc>,
    pub ret: PgWasmReturnDesc,
}
