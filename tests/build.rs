//! Build integration-test WebAssembly components (`wasm-tools` CLI).

use std::path::PathBuf;
use std::process::Command;

fn run(cmd: &mut Command) {
    let status = cmd.status().unwrap_or_else(|e| {
        panic!(
            "failed to spawn {:?}: {e} (is the binary on PATH?)",
            cmd.get_program()
        );
    });
    assert!(
        status.success(),
        "command failed with {status:?}: {:?}",
        cmd
    );
}

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let fixtures = manifest_dir.join("fixtures");

    let core_wat = fixtures.join("core.wat");
    let itest_wit = fixtures.join("itest.wit");

    println!("cargo:rerun-if-changed={}", core_wat.display());
    println!("cargo:rerun-if-changed={}", itest_wit.display());

    let core_wasm = out_dir.join("itest_core.wasm");
    run(Command::new("wasm-tools")
        .arg("parse")
        .arg(&core_wat)
        .arg("-o")
        .arg(&core_wasm));

    let embedded = out_dir.join("itest_embedded.wasm");
    run(Command::new("wasm-tools")
        .arg("component")
        .arg("embed")
        .arg(&itest_wit)
        .arg(&core_wasm)
        .arg("-o")
        .arg(&embedded));

    let itest_component = out_dir.join("itest.component.wasm");
    run(Command::new("wasm-tools")
        .arg("component")
        .arg("new")
        .arg(&embedded)
        .arg("-o")
        .arg(&itest_component));
}
