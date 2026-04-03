//! Detect WebAssembly ABI from raw bytes (plan §2): Extism → component model → core module.
//!
//! Order matches the plan: treat [`WasmAbiKind::Extism`] before [`WasmAbiKind::CoreWasm`] for
//! classic modules by scanning imports; component binaries are identified by the wasm header.

use thiserror::Error;
use wasmparser::{Encoding, Parser, Payload};

/// Host import modules used by Extism PDK plugins (see `extism` crate).
const EXTISM_ENV_MODULE: &str = "extism:host/env";
const EXTISM_USER_MODULE: &str = "extism:host/user";

/// Classified ABI for a wasm or component binary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WasmAbiKind {
    /// Extism `extism:host/*` imports (bytes-oriented plugin ABI).
    Extism,
    /// WebAssembly component model (WIT / nested core modules).
    ComponentModel,
    /// Ordinary core wasm module with no Extism host imports.
    CoreWasm,
}

impl WasmAbiKind {
    /// Stable lowercase label for catalogs and SQL helpers.
    pub const fn as_label(self) -> &'static str {
        match self {
            Self::Extism => "extism",
            Self::ComponentModel => "component",
            Self::CoreWasm => "core",
        }
    }
}

#[derive(Debug, Error)]
pub enum AbiDetectError {
    #[error("pg_wasm: could not parse wasm for ABI detection: {0}")]
    Parse(#[from] wasmparser::BinaryReaderError),
    #[error("pg_wasm: wasm binary is empty or truncated")]
    Truncated,
    #[error("pg_wasm: wasm binary has no version header")]
    MissingVersionHeader,
}

/// Inspect `wasm` and classify the ABI without compiling.
pub fn detect_wasm_abi(wasm: &[u8]) -> Result<WasmAbiKind, AbiDetectError> {
    let parser = Parser::new(0);
    let mut iter = parser.parse_all(wasm);

    let first = iter.next().ok_or(AbiDetectError::Truncated)??;

    let encoding = match first {
        Payload::Version { encoding, .. } => encoding,
        _ => return Err(AbiDetectError::MissingVersionHeader),
    };

    if encoding == Encoding::Component {
        return Ok(WasmAbiKind::ComponentModel);
    }

    for payload in iter {
        let payload = payload.map_err(AbiDetectError::from)?;
        if let Payload::ImportSection(reader) = payload {
            for group in reader {
                let imports = group.map_err(AbiDetectError::from)?;
                for imp in imports {
                    let (_, imp) = imp.map_err(AbiDetectError::from)?;
                    if imp.module == EXTISM_ENV_MODULE || imp.module == EXTISM_USER_MODULE {
                        return Ok(WasmAbiKind::Extism);
                    }
                }
            }
        }
    }

    Ok(WasmAbiKind::CoreWasm)
}

/// Returns true if a **core** module imports `wasi_snapshot_preview1` or `wasi_unstable`.
///
/// Component WASI is detected after compilation via import names `wasi:*` (see wasmtime backend).
#[cfg(test)]
pub(crate) fn wasm_imports_wasi_host(wasm: &[u8]) -> Result<bool, AbiDetectError> {
    const P1: &str = "wasi_snapshot_preview1";
    const UNSTABLE: &str = "wasi_unstable";
    let parser = Parser::new(0);
    for payload in parser.parse_all(wasm) {
        let payload = payload?;
        if let Payload::ImportSection(reader) = payload {
            for group in reader {
                let imports = group?;
                for imp in imports {
                    let (_, imp) = imp?;
                    if imp.module == P1 || imp.module == UNSTABLE {
                        return Ok(true);
                    }
                }
            }
        }
    }
    Ok(false)
}

/// Parse `options` JSON key `abi` (`core`, `extism`, `component`).
pub fn parse_abi_override(s: &str) -> Option<WasmAbiKind> {
    match s.trim().to_ascii_lowercase().as_str() {
        "core" | "core_wasm" | "module" => Some(WasmAbiKind::CoreWasm),
        "extism" => Some(WasmAbiKind::Extism),
        "component" | "component_model" | "wit" => Some(WasmAbiKind::ComponentModel),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_fixture_is_core() {
        let wasm = include_bytes!(concat!(env!("OUT_DIR"), "/test_add.wasm"));
        assert_eq!(detect_wasm_abi(wasm).unwrap(), WasmAbiKind::CoreWasm);
    }

    #[test]
    fn test_add_component_fixture_is_component() {
        let wasm = include_bytes!(concat!(env!("OUT_DIR"), "/test_add.component.wasm"));
        assert_eq!(detect_wasm_abi(wasm).unwrap(), WasmAbiKind::ComponentModel);
    }

    #[test]
    fn extism_imports_classify_as_extism() {
        let wat = r#"
            (module
              (import "extism:host/env" "length" (func (param i64) (result i64)))
              (func (export "probe") (result i32)
                (i32.const 0))
            )
        "#;
        let wasm = wat::parse_str(wat).expect("wat");
        assert_eq!(detect_wasm_abi(&wasm).unwrap(), WasmAbiKind::Extism);
    }

    #[test]
    fn wasi_import_detected() {
        let wasm = include_bytes!(concat!(env!("OUT_DIR"), "/test_wasi_fd_write.wasm"));
        assert!(wasm_imports_wasi_host(wasm).unwrap());
    }

    #[test]
    fn user_import_also_extism() {
        let wat = r#"
            (module
              (import "extism:host/user" "foo" (func (param i64) (result i64)))
              (memory (export "mem") 1)
            )
        "#;
        let wasm = wat::parse_str(wat).expect("wat");
        assert_eq!(detect_wasm_abi(&wasm).unwrap(), WasmAbiKind::Extism);
    }
}
