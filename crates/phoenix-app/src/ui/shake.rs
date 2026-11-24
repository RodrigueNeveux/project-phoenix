//! SHAKE_APP: the client's Shakeable around the app layers (web bundle class `iw`).
//! Every other animation frame it sets `translate3d(dir * intensity, random(-intensity,
//! intensity), 0)` with `dir` flipping each time, for `duration` ms; the decay factor
//! `min(1, cbrt(duration - progress / 1000))` stays 1 for the short shakes it is used for.
//! Skipped when the window is not focused or with reduced motion.

use std::{cell::Cell, time::Instant};

thread_local! {
    static SHAKE: Cell<Option<(Instant, u32, f32)>> = const { Cell::new(None) };
}

/// Starts a shake of `duration_ms` and `intensity` px (SLOWMODE_COOLDOWN: 200, 2).
pub fn start(duration_ms: u32, intensity: f32) {
    SHAKE.with(|s| s.set(Some((Instant::now(), duration_ms, intensity))));
}

/// Offset of frame `frame` (60 fps): moves on odd frames only, x alternates, y is pseudo-random.
fn frame_offset(frame: u32, intensity: f32) -> (f32, f32) {
    let step = frame / 2;
    let dir = if step % 2 == 0 { -1. } else { 1. };
    // xorshift of the step: deterministic "random" in [-1, 1].
    let mut h = step.wrapping_mul(2_654_435_761) ^ 0x9e37_79b9;
    h ^= h << 13;
    h ^= h >> 17;
    h ^= h << 5;
    let r = (h % 2001) as f32 / 1000. - 1.;
    (dir * intensity, r * intensity)
}

/// Current offset (px), or `None` when not shaking. Asks for the next frame while it runs.
pub fn offset(window: &mut gpui::Window, reduced_motion: bool) -> Option<(f32, f32)> {
    let (t, dur, i) = SHAKE.with(Cell::get)?;
    let ms = t.elapsed().as_millis() as u32;
    if ms > dur || reduced_motion || !window.is_window_active() {
        SHAKE.with(|s| s.set(None));
        return None;
    }
    window.request_animation_frame();
    Some(frame_offset(ms * 60 / 1000, i))
}

#[cfg(test)]
mod tests {
    use super::frame_offset;

    #[test]
    fn x_alternates_and_y_stays_in_range() {
        let a = frame_offset(0, 2.);
        let b = frame_offset(2, 2.);
        assert_eq!(a.0, -2.);
        assert_eq!(b.0, 2.);
        for f in 0..40 {
            let (_, y) = frame_offset(f, 2.);
            assert!((-2. ..=2.).contains(&y));
        }
    }
}
