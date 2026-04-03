//! WASM execution backends. At most one primary backend is used per module; see [`selection`] and
//! [`dispatch`].

mod stub;

pub mod dispatch;
#[cfg(feature = "runtime-extism")]
pub mod extism_backend;
pub mod selection;
#[cfg(feature = "runtime-extism")]
pub mod wasm_bytes_exports;
#[cfg(feature = "runtime-wasmer")]
pub mod wasmer_backend;
#[cfg(feature = "runtime-wasmtime")]
pub mod wasmtime_backend;

pub use stub::StubWasmBackend;
pub use selection::ModuleExecutionBackend;

/// Which concrete runtime executes a module.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RuntimeKind {
    Wasmtime,
    Wasmer,
    Extism,
}

/// Common surface for runtime-specific engines (filled in as invocation is implemented).
pub trait WasmRuntimeBackend: Send + Sync {
    fn kind(&self) -> RuntimeKind;

    fn label(&self) -> &'static str;
}
