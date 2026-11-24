//! Token overrides inside a Nitro-themed profile (`.custom-user-profile-theme`).
//!
//! The client re-declares a handful of variables on the themed card
//! (`.custom-user-profile-theme.theme-dark{…}` and the control rule next to it).
//! Elements resolve their colours while they are built, so the card body is
//! built inside [`scoped`] and [`crate::theme::resolve`] consults [`lookup`].

use std::cell::Cell;

use crate::theme::generated::idx;

thread_local! {
    /// Colours of the themed card being built (None outside one).
    static ACTIVE: Cell<Option<(u32, u32)>> = const { Cell::new(None) };
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
    let t = ACTIVE.with(Cell::get)?;
    let s = scheme(t);
    // `:root .custom-user-profile-theme.theme-*`: --control-primary-background-*
    // = button colour, hover/active mixed 90/80% (srgb) with black (dark) or white (light).
    if matches!(i, idx::CONTROL_PRIMARY_BACKGROUND_DEFAULT | idx::CONTROL_PRIMARY_BACKGROUND_HOVER | idx::CONTROL_PRIMARY_BACKGROUND_ACTIVE) {
        let k = match i {
            idx::CONTROL_PRIMARY_BACKGROUND_HOVER => 0.9,
            idx::CONTROL_PRIMARY_BACKGROUND_ACTIVE => 0.8,
            _ => 1.,
        };
        let to = if s == Scheme::Light { 255. } else { 0. };
        let c = split(button_color(t.0)).map(|v| v * k + to * (1. - k));
        return Some((join(c) << 8) | 0xFF);
    }
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
    let prev = ACTIVE.with(|s| s.replace(theme));
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

/// CIE Lab (D65), as the client's colour library computes it.
mod lab {
    const XN: f64 = 0.950_470;
    const YN: f64 = 1.;
    const ZN: f64 = 1.088_830;
    const T0: f64 = 0.137_931_034;
    const T1: f64 = 0.206_896_552;
    const T2: f64 = 0.128_418_55;
    const T3: f64 = 0.008_856_452;

    fn lin(c: f64) -> f64 {
        let c = c / 255.;
        if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    }
    fn delin(c: f64) -> f64 {
        255. * if c <= 0.003_04 { 12.92 * c } else { 1.055 * c.powf(1. / 2.4) - 0.055 }
    }
    fn f(t: f64) -> f64 {
        if t > T3 { t.cbrt() } else { t / T2 + T0 }
    }
    fn finv(t: f64) -> f64 {
        if t > T1 { t * t * t } else { T2 * (t - T0) }
    }
    pub fn from_rgb(rgb: [f64; 3]) -> [f64; 3] {
        let [r, g, b] = rgb.map(lin);
        let x = f((0.412_456_4 * r + 0.357_576_1 * g + 0.180_437_5 * b) / XN);
        let y = f((0.212_672_9 * r + 0.715_152_2 * g + 0.072_175 * b) / YN);
        let z = f((0.019_333_9 * r + 0.119_192 * g + 0.950_304_1 * b) / ZN);
        let l = 116. * y - 16.;
        [if l < 0. { 0. } else { l }, 500. * (x - y), 200. * (y - z)]
    }
    /// Lab → sRGB, clipped to 0..255 (unrounded).
    pub fn to_rgb([l, a, b]: [f64; 3]) -> [f64; 3] {
        let y = (l + 16.) / 116.;
        let x = if a.is_nan() { y } else { y + a / 500. };
        let z = if b.is_nan() { y } else { y - b / 200. };
        let (x, y, z) = (XN * finv(x), YN * finv(y), ZN * finv(z));
        let r = delin(3.240_454_2 * x - 1.537_138_5 * y - 0.498_531_4 * z);
        let g = delin(-0.969_266 * x + 1.876_010_8 * y + 0.041_556 * z);
        let bl = delin(0.055_643_4 * x - 0.204_025_9 * y + 1.057_225_2 * z);
        [r, g, bl].map(|v| v.clamp(0., 255.))
    }
    /// WCAG relative luminance.
    pub fn luminance(rgb: [f64; 3]) -> f64 {
        let [r, g, b] = rgb.map(lin);
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }
    pub fn contrast(a: [f64; 3], b: [f64; 3]) -> f64 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }
}

fn split(rgb: u32) -> [f64; 3] {
    [16, 8, 0].map(|sh| ((rgb >> sh) & 0xFF) as f64)
}

fn join(c: [f64; 3]) -> u32 {
    let [r, g, b] = c.map(|v| v.round().clamp(0., 255.) as u32);
    (r << 16) | (g << 8) | b
}

fn hsl(rgb: u32) -> (f32, f32, f32) {
    let ch = |sh: u32| ((rgb >> sh) & 0xFF) as f32 / 255.;
    let h: gpui::Hsla = gpui::Rgba { r: ch(16), g: ch(8), b: ch(0), a: 1. }.into();
    (h.h, h.s, h.l)
}

/// --profile-gradient-modal-background-color, from the client bundle: mix
/// primary and secondary 50% in Lab, round its HSL lightness to 2 decimals,
/// clamp it to 0..0.1 (dark card) or 0.8..1 (light card), keep hue/saturation.
pub fn modal_background(theme: (u32, u32)) -> gpui::Hsla {
    let (a, b) = (lab::from_rgb(split(theme.0)), lab::from_rgb(split(theme.1)));
    let mid = join(lab::to_rgb([0, 1, 2].map(|i| (a[i] + b[i]) / 2.)));
    let (h, s, l) = hsl(mid);
    let l = (l * 100.).round() / 100.;
    let l = if scheme(theme) == Scheme::Light { l.clamp(0.8, 1.) } else { l.clamp(0., 0.1) };
    gpui::hsla(h, s, l, 1.)
}

/// --profile-gradient-button-color (themed --control-primary-background-*),
/// from the client bundle: starting at the primary colour, step Lab lightness
/// by 18 (darken while the contrast against white is under 7, brighten while
/// it is over 7 + 3), at most 100 steps.
pub fn button_color(primary: u32) -> u32 {
    const WHITE: [f64; 3] = [255.; 3];
    let mut c = split(primary);
    let mut k = lab::contrast(WHITE, c);
    for _ in 0..100 {
        let (low, high) = (k < 7., k > 10.);
        if !low && !high {
            break;
        }
        let mut l = lab::from_rgb(c);
        // never brighter than white, so: too low → darken, too high → brighten
        l[0] += if low { -18. } else { 18. };
        c = lab::to_rgb(l);
        k = lab::contrast(WHITE, c);
    }
    join(c)
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
        assert!(near(rgb((0, 0xFF_00_00)), [46, 10, 5]));
        assert!(near(rgb((0, 0xFF_FF_FF)), [25, 26, 25]));
        assert_eq!(rgb((0, 0)), [0, 0, 0]);
    }

    #[test]
    fn button_color_matches_live_samples() {
        let near = |a: u32, b: u32| [16, 8, 0].iter().all(|sh| (((a >> sh) & 0xFF) as i32 - ((b >> sh) & 0xFF) as i32).abs() <= 3);
        // hsla(120,100%,18.8%) / (0,100%,27.1%) / (0,0%,33.3%)
        assert!(near(button_color(0x38_F5_38), 0x00_60_00), "{:06x}", button_color(0x38_F5_38));
        assert!(near(button_color(0xFF_09_00), 0x8A_00_00), "{:06x}", button_color(0xFF_09_00));
        assert!(near(button_color(0), 0x55_55_55), "{:06x}", button_color(0));
    }
}
