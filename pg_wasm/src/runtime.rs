//! Wasmtime engine, linker, and instance management primitives.

/// Called once from `_PG_init`. The `engine-and-epoch-ticker` task fills this
/// in to lazily build the shared `wasmtime::Engine`, spawn the epoch-ticker
/// thread, and register an atexit hook. Landing the entry point here so
/// `_PG_init` does not need to edit for Wave 1 subtasks.
#[allow(dead_code)]
pub(crate) fn init() {}
