//! Token overrides inside a Nitro-themed profile (`.custom-user-profile-theme`).
//!
//! The client re-declares a handful of variables on the themed card
//! (`.custom-user-profile-theme.theme-dark{…}` and the control rule next to it).
//! Elements resolve their colours while they are built, so the card body is
//! built inside [`scoped`] and [`crate::theme::resolve`] consults [`lookup`].

use std::cell::Cell;

use crate::theme::generated::idx;

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
}

/// `hsl(var(--opacity-*-hsl))` base: 240 4% 60.784% → #97979f.
const OPACITY: u32 = 0x9797_9F00;
const WHITE: u32 = 0xFFFF_FF00;

/// Alpha byte of a CSS `/ a` fraction.
const fn a(f: f32) -> u32 {
    (f * 255. + 0.5) as u32
}

/// Override for a token index while a themed profile is being built (dark themes).
pub fn lookup(i: usize) -> Option<u32> {
    if !ACTIVE.with(Cell::get) || crate::theme::active() == crate::ui::settings::AppTheme::Light {
        return None;
    }
    Some(match i {
        idx::TEXT_DEFAULT => WHITE | a(0.960_784_3),                         // --opacity-white-96
        idx::TEXT_MUTED => WHITE | a(0.760_784_3),                           // --opacity-white-76
        idx::TEXT_SUBTLE => WHITE | a(0.878_431_4),                          // --opacity-white-88
        idx::USER_PROFILE_ACTIVITY_TOOLBAR_BACKGROUND => OPACITY | a(0.239_215_7), // --opacity-24
        idx::USER_PROFILE_BACKGROUND_HOVER => WHITE | a(0.039_215_7),        // --opacity-white-4
        idx::USER_PROFILE_BORDER => WHITE | a(0.078_431_4),                  // --opacity-white-8
        idx::USER_PROFILE_NOTE_BACKGROUND_FOCUS => a(0.3),                   // --black / .3
        idx::USER_PROFILE_OVERLAY_BACKGROUND => OPACITY | a(0.160_784_3),    // --opacity-16
        idx::USER_PROFILE_OVERLAY_BACKGROUND_HOVER => OPACITY | a(0.2),      // --opacity-20
        idx::USER_PROFILE_TOOLBAR_BACKGROUND => a(0.478_431_4),              // --opacity-black-48
        idx::USER_PROFILE_TOOLBAR_BORDER => OPACITY | a(0.039_215_7),        // --opacity-4
        idx::CONTROL_SECONDARY_BACKGROUND_DEFAULT => WHITE | a(0.08),        // --opacity-white-8
        idx::CONTROL_SECONDARY_BACKGROUND_HOVER => WHITE | a(0.16),          // --opacity-white-16
        idx::CONTROL_SECONDARY_BACKGROUND_ACTIVE => WHITE | a(0.12),         // --opacity-white-12
        _ => return None,
    })
}

/// Runs `f` with the themed-profile overrides on when `on`.
pub fn scoped<R>(on: bool, f: impl FnOnce() -> R) -> R {
    let prev = ACTIVE.with(|s| s.replace(on));
    let r = f();
    ACTIVE.with(|s| s.set(prev));
    r
}

/// The `inner_P1YEjQ` colour over the outer gradient: --profile-gradient-overlay
/// (black / .6 in dark themes, white / .6 in light) composited on `rgb`.
pub fn composite(rgb: u32) -> u32 {
    let light = crate::theme::active() == crate::ui::settings::AppTheme::Light;
    let ch = |sh: u32| {
        let v = ((rgb >> sh) & 0xFF) as f32;
        (if light { v * 0.4 + 255. * 0.6 } else { v * 0.4 }).round() as u32
    };
    (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

/// --profile-gradient-overlay as an Hsla.
pub fn overlay() -> gpui::Hsla {
    let light = crate::theme::active() == crate::ui::settings::AppTheme::Light;
    gpui::hsla(0., 0., if light { 1. } else { 0. }, 0.6)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overrides_only_inside_scope() {
        assert_eq!(lookup(idx::TEXT_DEFAULT), None);
        let v = scoped(true, || lookup(idx::TEXT_DEFAULT));
        assert_eq!(v, Some(0xFFFF_FFF5));
        assert_eq!(lookup(idx::TEXT_DEFAULT), None);
        assert_eq!(composite(0xFF_80_00), 0x66_33_00);
    }
}
