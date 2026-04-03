//! Per-export invocation counters and timings; optional sampled guest linear memory (plan §7–8).
//!
//! Stats are **process-local** (each PostgreSQL backend has its own counters).

use std::{
    collections::HashMap,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

use crate::registry::ModuleId;

static MEMORY_PEAK_BYTES: OnceLock<Mutex<HashMap<ModuleId, u64>>> = OnceLock::new();

fn memory_peaks() -> &'static Mutex<HashMap<ModuleId, u64>> {
    MEMORY_PEAK_BYTES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Counters updated from the trampoline on each WASM call (when collection is enabled).
#[derive(Debug, Default)]
pub struct ExportStats {
    pub(super) invocations: AtomicU64,
    pub(super) errors: AtomicU64,
    pub(super) total_time_ns: AtomicU64,
}

impl ExportStats {
    pub fn invocations(&self) -> u64 {
        self.invocations.load(Ordering::Relaxed)
    }

    pub fn errors(&self) -> u64 {
        self.errors.load(Ordering::Relaxed)
    }

    pub fn total_time_ns(&self) -> u64 {
        self.total_time_ns.load(Ordering::Relaxed)
    }
}

pub fn alloc_export_stats() -> std::sync::Arc<ExportStats> {
    std::sync::Arc::new(ExportStats::default())
}

pub fn collecting() -> bool {
    crate::guc::collect_metrics()
}

#[must_use]
pub fn timer_start() -> Option<Instant> {
    collecting().then(Instant::now)
}

pub fn timer_finish_ok(stats: &ExportStats, start: Option<Instant>) {
    if !collecting() {
        return;
    }
    let Some(t0) = start else { return };
    let elapsed = t0.elapsed();
    stats.invocations.fetch_add(1, Ordering::Relaxed);
    stats
        .total_time_ns
        .fetch_add(elapsed.as_nanos() as u64, Ordering::Relaxed);
}

pub fn timer_finish_err(stats: &ExportStats, _start: Option<Instant>) {
    if !collecting() {
        return;
    }
    stats.errors.fetch_add(1, Ordering::Relaxed);
}

pub fn record_memory_sample(module: ModuleId, byte_size: u64) {
    if !collecting() || byte_size == 0 {
        return;
    }
    let mut g = memory_peaks()
        .lock()
        .expect("pg_wasm metrics memory peak map poisoned");
    let e = g.entry(module).or_insert(0);
    *e = (*e).max(byte_size);
}

pub fn guest_memory_peak_bytes(module: ModuleId) -> Option<u64> {
    let g = memory_peaks()
        .lock()
        .expect("pg_wasm metrics memory peak map poisoned");
    g.get(&module).copied()
}

pub fn remove_module_memory_peak(module: ModuleId) {
    let mut g = memory_peaks()
        .lock()
        .expect("pg_wasm metrics memory peak map poisoned");
    g.remove(&module);
}
