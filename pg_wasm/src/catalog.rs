//! Catalog schema and catalog access utilities.

/// Called once from `_PG_init`. The `catalog-schema` task fills this in to
/// validate catalog shape (via `catalog::migrations`) against the versioned
/// SQL ship in `pg_wasm/sql/`. Landing the entry point here so `_PG_init`
/// does not need to edit for Wave 1 subtasks.
#[allow(dead_code)]
pub(crate) fn init() {}
