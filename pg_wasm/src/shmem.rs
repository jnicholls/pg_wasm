//! Shared-memory state and generation metadata.

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
