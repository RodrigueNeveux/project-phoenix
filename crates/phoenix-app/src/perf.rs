//! Lightweight timing logs, enabled with `PHX_TIMINGS=1`.
//!
//! Every line goes to stderr as `PHX_TIMING t=<ms since start> <what> [<ms>]`
//! so scripts can grep them (see docs/perf.md). When the variable is unset each
//! call is one relaxed atomic load.

use std::{
    collections::HashMap,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

static START: OnceLock<Instant> = OnceLock::new();
static ENABLED: OnceLock<bool> = OnceLock::new();
/// Channel switches waiting for their first painted frame with messages.
static PENDING: Mutex<Option<HashMap<u64, Instant>>> = Mutex::new(None);
static HAS_PENDING: AtomicBool = AtomicBool::new(false);

fn lock() -> std::sync::MutexGuard<'static, Option<HashMap<u64, Instant>>> {
    PENDING.lock().unwrap_or_else(|e| e.into_inner())
}

/// Record the process start (first call wins). Call it first thing in `main`.
pub fn start() {
    START.get_or_init(Instant::now);
}

pub fn enabled() -> bool {
    *ENABLED.get_or_init(|| std::env::var_os("PHX_TIMINGS").is_some_and(|v| !v.is_empty() && v != "0"))
}

fn since_start_ms() -> f64 {
    START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
}

/// A point in time relative to process start.
pub fn mark(what: &str) {
    if enabled() {
        eprintln!("PHX_TIMING t={:.1} {what}", since_start_ms());
    }
}

/// A duration measured from `from`.
pub fn span(what: &str, from: Instant) {
    if enabled() {
        eprintln!("PHX_TIMING t={:.1} {what} {:.1}ms", since_start_ms(), from.elapsed().as_secs_f64() * 1000.0);
    }
}

/// A channel was opened: the next frame painted after its messages arrived ends the span.
pub fn channel_opened(channel: u64) {
    if enabled() {
        lock().get_or_insert_with(HashMap::new).insert(channel, Instant::now());
    }
}

/// The channel's messages are in the app state (from network or cache).
pub fn channel_messages_ready(channel: u64, source: &str) {
    if !enabled() {
        return;
    }
    if let Some(t) = lock().as_ref().and_then(|m| m.get(&channel)) {
        span(&format!("channel_messages_ready[{source}]"), *t);
        HAS_PENDING.store(true, Ordering::Relaxed);
    }
}

/// Called at the end of the main view's render: reports switches whose
/// messages were ready before this frame.
pub fn on_render(selected: Option<u64>, has_messages: bool) {
    if !HAS_PENDING.load(Ordering::Relaxed) || !has_messages {
        return;
    }
    let Some(ch) = selected else { return };
    let mut g = lock();
    if let Some(t) = g.as_mut().and_then(|m| m.remove(&ch)) {
        span("channel_switch_frame", t);
    }
    if g.as_ref().is_none_or(|m| m.is_empty()) {
        HAS_PENDING.store(false, Ordering::Relaxed);
    }
}
