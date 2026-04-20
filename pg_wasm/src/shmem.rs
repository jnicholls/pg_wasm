//! Shared-memory state and generation metadata.

/// Called once from `_PG_init`. The `shmem-and-generation` task fills this in
/// to install `shmem_request_hook` / `shmem_startup_hook` and size the
/// per-cluster segment. Keeping the entry point on `shmem` so `_PG_init` does
/// not need to edit for Wave 1 subtasks.
#[allow(dead_code)]
pub(crate) fn init() {}

/// Fixed shared-memory module metrics slot count.
///
/// If loaded modules exceed this bound, overflowed modules use process-local
/// dynamic counters and are reported as non-shared (degraded mode).
#[allow(dead_code)]
pub(crate) const SHMEM_MODULE_SLOTS: usize = 256;

/// Fixed shared-memory export metrics slot count.
///
/// If loaded exports exceed this bound, overflowed exports use process-local
/// dynamic counters and are reported as non-shared (degraded mode).
#[allow(dead_code)]
pub(crate) const SHMEM_EXPORT_SLOTS: usize = 4_096;
