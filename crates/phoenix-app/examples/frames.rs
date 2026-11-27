use std::{
    alloc::{GlobalAlloc, Layout, System},
    backtrace::Backtrace,
    cell::Cell,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

#[path = "common/frames_impl.rs"]
mod frames_impl;

struct Counting;
static ALLOCS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);
static SAMPLE_EVERY: AtomicU64 = AtomicU64::new(0);
static SITES: Mutex<Vec<Backtrace>> = Mutex::new(Vec::new());

thread_local! {
    static SAMPLING: Cell<bool> = const { Cell::new(false) };
}

fn sample(n: u64) {
    let every = SAMPLE_EVERY.load(Ordering::Relaxed);
    if every == 0 || !n.is_multiple_of(every) {
        return;
    }
    let _ = SAMPLING.try_with(|s| {
        if s.replace(true) {
            return;
        }
        let bt = Backtrace::force_capture();
        if let Ok(mut v) = SITES.lock() {
            v.push(bt);
        }
        s.set(false);
    });
}

fn start_sampling(every: u64) {
    SAMPLE_EVERY.store(every, Ordering::Relaxed);
}

fn stop_sampling() -> Vec<Backtrace> {
    SAMPLE_EVERY.store(0, Ordering::Relaxed);
    SITES.lock().map(|mut v| std::mem::take(&mut *v)).unwrap_or_default()
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        sample(ALLOCS.fetch_add(1, Ordering::Relaxed));
        BYTES.fetch_add(l.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        sample(ALLOCS.fetch_add(1, Ordering::Relaxed));
        BYTES.fetch_add(n as u64, Ordering::Relaxed);
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static A: Counting = Counting;

fn main() -> anyhow::Result<()> {
    let _ = frames_impl::COUNTER.set(|| (ALLOCS.load(Ordering::Relaxed), BYTES.load(Ordering::Relaxed)));
    let _ = frames_impl::SAMPLER.set((start_sampling, stop_sampling));
    frames_impl::run()
}
