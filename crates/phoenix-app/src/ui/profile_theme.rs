//! Token overrides inside a Nitro-themed profile (`.custom-user-profile-theme`).
//!
//! The client re-declares a handful of variables on the themed card
//! (`.custom-user-profile-theme.theme-dark{…}` and the control rule next to it).
//! Elements resolve their colours while they are built, so the card body is
//! built inside [`scoped`] and [`crate::theme::resolve`] consults [`lookup`].

use std::cell::Cell;

use crate::theme::generated::idx;

thread_local! {
    /// Scheme of the themed card being built (None outside one).
    static ACTIVE: Cell<Option<Scheme>> = const { Cell::new(None) };
}

/// `theme-dark` / `theme-light` class the client puts on a themed card.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scheme {
    Dark,
    Light,
}

/// The client picks the card scheme from the primary colour: light when its
/// perceived brightness (0.299 R + 0.587 G + 0.114 B) is above half
/// (live: rgb(56,245,56) → theme-light, rgb(255,9,0) / black → theme-dark).
pub fn scheme((primary, _): (u32, u32)) -> Scheme {
    let ch = |sh: u32| ((primary >> sh) & 0xFF) as f32;
    let y = (0.299 * ch(16) + 0.587 * ch(8) + 0.114 * ch(0)) / 255.;
    if y > 0.5 { Scheme::Light } else { Scheme::Dark }
}

/// `hsl(var(--opacity-*-hsl))` base: 240 4% 60.784% → #97979f.
const OPACITY: u32 = 0x9797_9F00;
const WHITE: u32 = 0xFFFF_FF00;

/// Alpha byte of a CSS `/ a` fraction.
const fn a(f: f32) -> u32 {
    (f * 255. + 0.5) as u32
}

fn app_is_light() -> bool {
    crate::theme::active() == crate::ui::settings::AppTheme::Light
}

/// Override for a token index while a themed profile is being built.
pub fn lookup(i: usize) -> Option<u32> {
    let s = ACTIVE.with(Cell::get)?;
    let v = match s {
        // `.custom-user-profile-theme.theme-dark{…}`
        Scheme::Dark => match i {
            idx::TEXT_DEFAULT => Some(WHITE | a(0.960_784_3)), // --opacity-white-96
            idx::TEXT_MUTED => Some(WHITE | a(0.760_784_3)),   // --opacity-white-76
            idx::TEXT_SUBTLE => Some(WHITE | a(0.878_431_4)),  // --opacity-white-88
            idx::USER_PROFILE_ACTIVITY_TOOLBAR_BACKGROUND => Some(OPACITY | a(0.239_215_7)),
            idx::USER_PROFILE_BACKGROUND_HOVER => Some(WHITE | a(0.039_215_7)),
            idx::USER_PROFILE_BORDER => Some(WHITE | a(0.078_431_4)),
            idx::USER_PROFILE_NOTE_BACKGROUND_FOCUS => Some(a(0.3)),
            idx::USER_PROFILE_OVERLAY_BACKGROUND => Some(OPACITY | a(0.160_784_3)),
            idx::USER_PROFILE_OVERLAY_BACKGROUND_HOVER => Some(OPACITY | a(0.2)),
            idx::USER_PROFILE_TOOLBAR_BACKGROUND => Some(a(0.478_431_4)),
            idx::USER_PROFILE_TOOLBAR_BORDER => Some(OPACITY | a(0.039_215_7)),
            idx::CONTROL_SECONDARY_BACKGROUND_DEFAULT => Some(WHITE | a(0.08)),
            idx::CONTROL_SECONDARY_BACKGROUND_HOVER => Some(WHITE | a(0.16)),
            idx::CONTROL_SECONDARY_BACKGROUND_ACTIVE => Some(WHITE | a(0.12)),
            _ => None,
        },
        // `.custom-user-profile-theme.theme-light{…}` (live modal, ref-cache
        // profile-theme-light-tokens.json)
        Scheme::Light => match i {
            idx::TEXT_DEFAULT => Some(a(0.960_784_3)), // black / .96
            idx::TEXT_MUTED => Some(a(0.760_784_3)),
            idx::TEXT_SUBTLE => Some(a(0.878_431_4)),
            idx::USER_PROFILE_ACTIVITY_TOOLBAR_BACKGROUND => Some(WHITE | a(0.478_431_4)),
            idx::USER_PROFILE_BACKGROUND_HOVER => Some(a(0.078_431_4)),
            idx::USER_PROFILE_BORDER => Some(a(0.078_431_4)),
            idx::USER_PROFILE_NOTE_BACKGROUND_FOCUS => Some(WHITE | a(0.3)),
            idx::USER_PROFILE_OVERLAY_BACKGROUND => Some(WHITE | a(0.239_215_7)),
            idx::USER_PROFILE_OVERLAY_BACKGROUND_HOVER => Some(WHITE | a(0.360_784_3)),
            idx::USER_PROFILE_TOOLBAR_BACKGROUND => Some(WHITE | a(0.6)),
            idx::USER_PROFILE_TOOLBAR_BORDER => Some(OPACITY | a(0.2)),
            idx::CONTROL_SECONDARY_BACKGROUND_DEFAULT => Some(WHITE | a(0.721_568_6)),
            idx::CONTROL_SECONDARY_BACKGROUND_HOVER => Some(WHITE | a(0.921_568_6)),
            idx::CONTROL_SECONDARY_BACKGROUND_ACTIVE => Some(WHITE | a(0.839_215_7)),
            _ => None,
        },
    };
    // Every other token follows the card's own theme class.
    v.or_else(|| match (s, app_is_light()) {
        (Scheme::Light, false) => Some(crate::theme::generated::THEMES[1][i]),
        (Scheme::Dark, true) => Some(crate::theme::generated::THEMES[0][i]),
        _ => None,
    })
}

/// Runs `f` with the overrides of a card themed `theme` (none when `None`).
pub fn scoped<R>(theme: Option<(u32, u32)>, f: impl FnOnce() -> R) -> R {
    let prev = ACTIVE.with(|s| s.replace(theme.map(scheme)));
    let r = f();
    ACTIVE.with(|s| s.set(prev));
    r
}

/// --profile-gradient-overlay-color: black / .6 on a dark card, white / .6 on a light one.
fn overlay_rgb(s: Scheme) -> f32 {
    if s == Scheme::Light { 255. } else { 0. }
}

/// The overlay composited on `rgb` (opaque result, e.g. the avatar ring).
pub fn composite(theme: (u32, u32), rgb: u32) -> u32 {
    let o = overlay_rgb(scheme(theme));
    let ch = |sh: u32| ((((rgb >> sh) & 0xFF) as f32) * 0.4 + o * 0.6).round() as u32;
    (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

/// --profile-gradient-overlay-color as an Hsla.
pub fn overlay(theme: (u32, u32)) -> gpui::Hsla {
    gpui::hsla(0., 0., overlay_rgb(scheme(theme)) / 255., 0.6)
}

fn hsl(rgb: u32) -> (f32, f32, f32) {
    let ch = |sh: u32| ((rgb >> sh) & 0xFF) as f32 / 255.;
    let h: gpui::Hsla = gpui::Rgba { r: ch(16), g: ch(8), b: ch(0), a: 1. }.into();
    (h.h, h.s, h.l)
}

/// --profile-gradient-modal-background-color (set by the client's JS). Live
/// samples fit hsl(hue/sat of the primary+secondary average, sat ≤ 80%,
/// lightness 10% dark / 80% light): (56,245,56)+(25,83,25) → rgb(174,234,174),
/// red+black → rgb(46,10,5), black+black → black.
pub fn modal_background(theme: (u32, u32)) -> gpui::Hsla {
    let avg = |sh: u32| ((((theme.0 >> sh) & 0xFF) + ((theme.1 >> sh) & 0xFF)) / 2) << sh;
    let (h, s, l) = hsl(avg(16) | avg(8) | avg(0));
    if l == 0. {
        return gpui::hsla(0., 0., 0., 1.);
    }
    let light = if scheme(theme) == Scheme::Light { 0.8 } else { 0.1 };
    gpui::hsla(h, s.min(0.8), light, 1.)
}

/// `color-mix(in oklab, overlay p%, modal background)` for the modal-v2 inner
/// gradient (80% at the top, 20% at 300px). Approximated in premultiplied sRGB.
pub fn modal_inner(theme: (u32, u32), p: f32) -> gpui::Hsla {
    let m: gpui::Rgba = modal_background(theme).into();
    let o = overlay_rgb(scheme(theme)) / 255.;
    let (wa, wb) = (0.6 * p, 1. - p); // premultiplied weights
    let al = wa + wb;
    let mix = |x: f32| (o * wa + x * wb) / al;
    gpui::Rgba { r: mix(m.r), g: mix(m.g), b: mix(m.b), a: al }.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overrides_only_inside_scope() {
        assert_eq!(lookup(idx::TEXT_DEFAULT), None);
        let v = scoped(Some((0xFF_09_00, 0)), || lookup(idx::TEXT_DEFAULT));
        assert_eq!(v, Some(0xFFFF_FFF5));
        let v = scoped(Some((0x38_F5_38, 0x19_53_19)), || lookup(idx::TEXT_DEFAULT));
        assert_eq!(v, Some(0xF5));
        assert_eq!(lookup(idx::TEXT_DEFAULT), None);
        assert_eq!(composite((0, 0), 0xFF_80_00), 0x66_33_00);
    }

    #[test]
    fn modal_background_matches_live_samples() {
        let rgb = |t| {
            let c: gpui::Rgba = modal_background(t).into();
            [c.r, c.g, c.b].map(|v| (v * 255.).round() as i32)
        };
        let near = |a: [i32; 3], b: [i32; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() <= 6);
        assert!(near(rgb((0x38_F5_38, 0x19_53_19)), [174, 234, 174]));
        assert!(near(rgb((0xFF_09_00, 0)), [46, 10, 5]));
        assert_eq!(rgb((0, 0)), [0, 0, 0]);
    }
}
