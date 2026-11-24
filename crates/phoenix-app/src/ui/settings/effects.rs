//! Settings read deep inside renderers (text runs, clocks, media) that have
//! no access to `PhoenixApp`: mirrored here once per app render by [`sync`].

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering::Relaxed};

use super::Prefs;

static SPOILERS_ALWAYS: AtomicBool = AtomicBool::new(false);
/// `timestamp_hour_cycle`: 0 auto (locale), 1 12h, 2 24h.
static HOUR_CYCLE: AtomicU8 = AtomicU8::new(0);
static GIF_AUTOPLAY: AtomicBool = AtomicBool::new(true);
static UNDERLINE_LINKS: AtomicBool = AtomicBool::new(false);
static REDUCED_MOTION: AtomicBool = AtomicBool::new(false);

pub fn sync(p: &Prefs) {
    // Spoilers, clocks and link underlines are baked into cached message renders.
    let a = SPOILERS_ALWAYS.swap(p.synced.pick("spoilers") == 1, Relaxed);
    let h = HOUR_CYCLE.swap(p.synced.pick("time-format") as u8, Relaxed);
    let u = UNDERLINE_LINKS.swap(p.local.flag("underline", false), Relaxed);
    if a != spoilers_always() || h != HOUR_CYCLE.load(Relaxed) || u != underline_links() {
        crate::theme::bump_style_generation();
    }
    GIF_AUTOPLAY.store(p.synced.flag("gifs"), Relaxed);
    REDUCED_MOTION.store(p.local.reduced_motion, Relaxed);
}

/// "Show spoiler content: Always".
pub fn spoilers_always() -> bool {
    SPOILERS_ALWAYS.load(Relaxed)
}

/// `Some(true)` = 24h, `Some(false)` = 12h, `None` = locale default.
pub fn hour24() -> Option<bool> {
    match HOUR_CYCLE.load(Relaxed) {
        1 => Some(false),
        2 => Some(true),
        _ => None,
    }
}

/// "Play GIFs when Discord is focused".
pub fn gif_autoplay() -> bool {
    GIF_AUTOPLAY.load(Relaxed)
}

/// Duration of a movement / zoom animation: near-instant with Accessibility
/// "Enable Reduced Motion" (the client's `reduce-motion` class drops them).
pub fn motion(ms: u64) -> std::time::Duration {
    std::time::Duration::from_millis(if REDUCED_MOTION.load(Relaxed) { 1 } else { ms })
}

/// Accessibility "Always underline links".
pub fn underline_links() -> bool {
    UNDERLINE_LINKS.load(Relaxed)
}
