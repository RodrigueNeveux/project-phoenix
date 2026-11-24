//! Frame times per interaction, offscreen on mock data (no window on screen).
//! `cargo run --release -p phoenix-app --example frames [-- N]`
//!
//! For each scenario: p50 / p95 / max frame time, frames over 16 ms, and
//! allocations per frame. A "frame" is refresh + layout + paint
//! (`run_until_parked`), without the GPU readback of screenshots.
//! Prints one `scenario p50_us p95_us max_us over16 allocs bytes` line each,
//! then RSS. See docs/perf.md.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicU64, Ordering},
};

#[path = "common/frames_impl.rs"]
mod frames_impl;

struct Counting;
static ALLOCS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(l.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(n as u64, Ordering::Relaxed);
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static A: Counting = Counting;

fn main() -> anyhow::Result<()> {
    let _ = frames_impl::COUNTER.set(|| (ALLOCS.load(Ordering::Relaxed), BYTES.load(Ordering::Relaxed)));
    frames_impl::run()
}
