//! Discord theme tokens (Dark, Light, Ash, Onyx), UI zoom and font scale, and layout metrics.

use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};

use gpui::{App, Hsla, Pixels, Rgba, Window, px, rgb, rgba};
use gpui_component::theme::{Theme, ThemeMode};

use crate::ui::settings::AppTheme;

pub mod generated;

use generated::idx;

/// A palette value (token or plain `0xRRGGBB`) as a colour for the active theme.
pub fn c(hex: u32) -> Hsla {
    if is_token(hex) {
        rgba(resolve(hex)).into()
    } else {
        rgb(hex).into()
    }
}

/// A palette value (token or plain `0xRRGGBBAA`) as a colour for the active theme.
pub fn ca(hex_rgba: u32) -> Hsla {
    rgba(resolve(hex_rgba)).into()
}

/// [`c`] for a given theme instead of the active one (theme preview cards).
pub fn c_for(hex: u32, theme: AppTheme) -> Hsla {
    if is_token(hex) {
        rgba(resolve_for(hex, theme)).into()
    } else {
        rgb(hex).into()
    }
}

/// `--font-primary` ("gg sans" from the local Discord cache, else Noto Sans).
#[allow(non_snake_case)]
pub fn FONT_UI() -> &'static str {
    crate::fonts::ui_family()
}
/// `--font-code` ("gg mono" from the local Discord cache, else DejaVu Sans Mono).
#[allow(non_snake_case)]
pub fn FONT_MONO() -> &'static str {
    crate::fonts::mono_family()
}

/// Tag a generated token index (see [`generated::idx`]) as a palette value.
pub const fn tok(i: usize) -> u32 {
    TOKEN | i as u32
}

/// Theme tokens, each mapped by name to a CSS variable from [`generated`].
/// Resolved against the active theme by [`c`]/[`ca`] at render time, so every
/// `c(Palette::X)` restyles live when the theme changes in Settings.
/// Any `--css-variable` can also be used directly via `tok(idx::NAME)`.
pub struct Palette;
#[allow(dead_code)]
impl Palette {
    /// `--background-base-lowest`
    pub const BG_TERTIARY: u32 = tok(idx::BACKGROUND_BASE_LOWEST);
    /// `--background-base-lower`
    pub const BG_SECONDARY: u32 = tok(idx::BACKGROUND_BASE_LOWER);
    /// `--background-base-low`
    pub const BG_SECONDARY_ALT: u32 = tok(idx::BACKGROUND_BASE_LOW);
    /// `--background-base-lower`
    pub const BG_PRIMARY: u32 = tok(idx::BACKGROUND_BASE_LOWER);
    /// `--background-surface-high`
    pub const BG_FLOATING: u32 = tok(idx::BACKGROUND_SURFACE_HIGH);
    /// `--background-mod-subtle`
    pub const BG_MOD_HOVER: u32 = tok(idx::BACKGROUND_MOD_SUBTLE);
    /// `--background-mod-strong`
    pub const BG_MOD_SELECTED: u32 = tok(idx::BACKGROUND_MOD_STRONG);
    /// `--background-mod-muted`
    pub const BG_MSG_HOVER: u32 = tok(idx::BACKGROUND_MOD_MUTED);
    /// `--chat-background-default`
    pub const INPUT_BG: u32 = tok(idx::CHAT_BACKGROUND_DEFAULT);
    /// `--background-surface-high`
    pub const EMBED_BG: u32 = tok(idx::BACKGROUND_SURFACE_HIGH);
    /// `--background-base-lowest`
    pub const CODE_BG: u32 = tok(idx::BACKGROUND_BASE_LOWEST);
    /// `--border-subtle`
    pub const CODE_BORDER: u32 = tok(idx::BORDER_SUBTLE);
    /// `--border-subtle`
    pub const DIVIDER: u32 = tok(idx::BORDER_SUBTLE);
    /// `--border-subtle`
    pub const HEADER_SHADOW: u32 = tok(idx::BORDER_SUBTLE);
    /// `--text-default`
    pub const TEXT_NORMAL: u32 = tok(idx::TEXT_DEFAULT);
    /// `--text-muted`
    pub const TEXT_MUTED: u32 = tok(idx::TEXT_MUTED);
    /// `--text-strong`
    pub const TEXT_HEADER: u32 = tok(idx::TEXT_STRONG);
    /// `--text-link`
    pub const TEXT_LINK: u32 = tok(idx::TEXT_LINK);
    /// `--interactive-text-default`
    pub const INTERACTIVE: u32 = tok(idx::INTERACTIVE_TEXT_DEFAULT);
    /// `--interactive-text-hover`
    pub const INTERACTIVE_HOVER: u32 = tok(idx::INTERACTIVE_TEXT_HOVER);
    /// `--interactive-muted`
    pub const INTERACTIVE_MUTED: u32 = tok(idx::INTERACTIVE_MUTED);
    /// `--channel-icon`
    pub const CHANNEL_ICON: u32 = tok(idx::CHANNEL_ICON);
    /// `--background-brand`
    pub const BLURPLE: u32 = tok(idx::BACKGROUND_BRAND);
    /// `--status-positive`
    pub const GREEN: u32 = tok(idx::STATUS_POSITIVE);
    /// `--status-warning`
    pub const YELLOW: u32 = tok(idx::STATUS_WARNING);
    /// `--status-danger`
    pub const RED: u32 = tok(idx::STATUS_DANGER);
    /// `--mention-foreground`
    pub const MENTION_FG: u32 = tok(idx::MENTION_FOREGROUND);
    /// `--spoiler-hidden-background`
    pub const SPOILER_BG: u32 = tok(idx::SPOILER_HIDDEN_BACKGROUND);
    /// `--app-frame-background`
    pub const APP_FRAME: u32 = tok(idx::APP_FRAME_BACKGROUND);
    /// `--background-surface-higher`
    pub const SURFACE_HIGHER: u32 = tok(idx::BACKGROUND_SURFACE_HIGHER);
    /// `--background-surface-highest`
    pub const SURFACE_HIGHEST: u32 = tok(idx::BACKGROUND_SURFACE_HIGHEST);
    /// `--background-feedback-notification`
    pub const NOTIFICATION: u32 = tok(idx::BACKGROUND_FEEDBACK_NOTIFICATION);
    /// `--text-brand`
    pub const TEXT_BRAND: u32 = tok(idx::TEXT_BRAND);
    /// `--app-frame-border`
    pub const GUILD_SEPARATOR: u32 = tok(idx::APP_FRAME_BORDER);
    /// `--border-subtle`
    pub const PANEL_BORDER: u32 = tok(idx::BORDER_SUBTLE);
    /// `--background-mod-subtle`
    pub const GUILD_ICON_BG: u32 = tok(idx::BACKGROUND_MOD_SUBTLE);
    /// `--text-feedback-critical`
    pub const TEXT_CRITICAL: u32 = tok(idx::TEXT_FEEDBACK_CRITICAL);
    /// `--border-muted`
    pub const BORDER_MUTED: u32 = tok(idx::BORDER_MUTED);
    /// `--app-frame-border`
    pub const APP_FRAME_BORDER: u32 = tok(idx::APP_FRAME_BORDER);
    /// `--background-base-low`
    pub const BG_BASE_LOW: u32 = tok(idx::BACKGROUND_BASE_LOW);
    /// `--icon-status-offline`
    pub const GREY: u32 = tok(idx::ICON_STATUS_OFFLINE);
    /// `--mention-background`
    pub const MENTION_BG: u32 = tok(idx::MENTION_BACKGROUND);
    /// `--message-mentioned-background-default`
    pub const MENTIONED_MSG_BG: u32 = tok(idx::MESSAGE_MENTIONED_BACKGROUND_DEFAULT);
    /// `--icon-feedback-warning`
    pub const MENTIONED_MSG_BAR: u32 = tok(idx::ICON_FEEDBACK_WARNING);
    /// `--spoiler-revealed-background`
    pub const SPOILER_REVEALED: u32 = tok(idx::SPOILER_REVEALED_BACKGROUND);
}

/// Tag bit marking a palette value as a theme token index rather than a hex colour.
pub const TOKEN: u32 = 0x0100_0000;
/// Number of theme tokens.
pub const TOKEN_COUNT: usize = generated::COLOR_COUNT;

fn is_token(hex: u32) -> bool {
    hex & 0xFF00_0000 == TOKEN && ((hex & 0xFFFF) as usize) < TOKEN_COUNT
}

static THEME: AtomicU8 = AtomicU8::new(0);
/// Accessibility "Enable High Contrast mode" (`.high-contrast-mode` on <html>).
static HIGH_CONTRAST: AtomicBool = AtomicBool::new(false);

pub fn high_contrast() -> bool {
    HIGH_CONTRAST.load(Ordering::Relaxed)
}
static ZOOM: AtomicU32 = AtomicU32::new(100);
static FONT_PX: AtomicU32 = AtomicU32::new(16);
static DENSITY: AtomicU8 = AtomicU8::new(Density::Cozy as u8);
/// Accessibility "Saturation" percent (the client's `--saturation-factor`).
static SATURATION: AtomicU32 = AtomicU32::new(100);

/// Scale the HSL saturation of `0xRRGGBBAA` by the saturation preference.
pub fn saturate(v: u32) -> u32 {
    let f = SATURATION.load(Ordering::Relaxed);
    if f >= 100 {
        return v;
    }
    let mut h: Hsla = rgba(v).into();
    h.s *= f as f32 / 100.;
    let r: Rgba = h.into();
    let ch = |x: f32| (x.clamp(0., 1.) * 255.).round() as u32;
    (ch(r.r) << 24) | (ch(r.g) << 16) | (ch(r.b) << 8) | ch(r.a)
}

/// UI density (`html.density-compact|default|cozy`). Each one rebinds the
/// semantic `--space-*` tokens to a different step of the raw scale.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Density {
    Compact = 0,
    Default = 1,
    /// What the user's client uses (`htmlClass` in the live dumps).
    #[default]
    Cozy = 2,
}

impl Density {
    pub const ALL: [Density; 3] = [Density::Compact, Density::Default, Density::Cozy];
    pub fn key(self) -> &'static str {
        match self {
            Density::Compact => "compact",
            Density::Default => "default",
            Density::Cozy => "cozy",
        }
    }
    pub fn from_key(k: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|d| d.key() == k)
    }
    /// `[--space-xxs, xs, sm, md, lg, xl, xxl]` in px for this density
    /// (`.density-compact` / `.density-default` / `.density-cozy` rules).
    pub const fn scale(self) -> [f32; 7] {
        match self {
            Density::Compact => [4., 6., 10., 12., 16., 20., 26.],
            Density::Default => [4., 8., 12., 16., 20., 24., 32.],
            Density::Cozy => [4., 10., 16., 20., 24., 30., 40.],
        }
    }
}

/// The active density, as last synced from `app.prefs`.
pub fn density() -> Density {
    match DENSITY.load(Ordering::Relaxed) {
        0 => Density::Compact,
        1 => Density::Default,
        _ => Density::Cozy,
    }
}

/// Density-aware `--space-*` tokens in px (unzoomed; wrap in [`z`]).
pub mod space {
    use super::density;
    pub fn xxs() -> f32 { density().scale()[0] }
    pub fn xs() -> f32 { density().scale()[1] }
    pub fn sm() -> f32 { density().scale()[2] }
    pub fn md() -> f32 { density().scale()[3] }
    pub fn lg() -> f32 { density().scale()[4] }
    pub fn xl() -> f32 { density().scale()[5] }
    pub fn xxl() -> f32 { density().scale()[6] }
}
/// Bumped whenever theme, zoom or font size change, so views that cache
/// resolved colours or measured heights know to rebuild them.
static STYLE_GEN: AtomicU32 = AtomicU32::new(0);

/// Invalidate cached renders (a setting that changes rendered text changed).
pub fn bump_style_generation() {
    STYLE_GEN.fetch_add(1, Ordering::Relaxed);
}

/// Current style generation (see [`STYLE_GEN`]).
pub fn style_generation() -> u32 {
    STYLE_GEN.load(Ordering::Relaxed)
}

fn theme_index(t: AppTheme) -> u8 {
    match t {
        AppTheme::Dark => 0,
        AppTheme::Light => 1,
        AppTheme::Ash => 2,
        AppTheme::Onyx => 3,
    }
}

/// Resolve a palette token to `0xRRGGBBAA` for the active theme; other values
/// are returned unchanged.
pub fn resolve(hex: u32) -> u32 {
    if is_token(hex) {
        if let Some(v) = crate::ui::profile_theme::lookup((hex & 0xFFFF) as usize) {
            return v;
        }
        let tables = if high_contrast() { &generated::THEMES_HC } else { &generated::THEMES };
        saturate(tables[THEME.load(Ordering::Relaxed) as usize][(hex & 0xFFFF) as usize])
    } else {
        hex
    }
}

/// [`resolve`] for a given theme instead of the active one.
pub fn resolve_for(hex: u32, theme: AppTheme) -> u32 {
    if is_token(hex) {
        generated::THEMES[theme_index(theme) as usize][(hex & 0xFFFF) as usize]
    } else {
        hex
    }
}

/// The active theme, as last synced from `app.prefs`.
pub fn active() -> AppTheme {
    match THEME.load(Ordering::Relaxed) {
        1 => AppTheme::Light,
        2 => AppTheme::Ash,
        3 => AppTheme::Onyx,
        _ => AppTheme::Dark,
    }
}

/// The frame borders that `.theme-midnight` and `.high-contrast-mode` both add
/// (`.page_pog62O` border-inline-start, `.callContainer_hbKzX7` border-top, ...).
pub fn frame_borders() -> bool {
    active() == AppTheme::Onyx || high_contrast()
}

pub fn is_light() -> bool {
    active() == AppTheme::Light
}

/// Sync theme, zoom and font size from the user's prefs. Called once per frame
/// from the root view; this is the single place other views read these from.
pub fn sync(prefs: &crate::ui::settings::Prefs, window: &mut Window, cx: &mut App) {
    let t = theme_index(prefs.synced.theme);
    let prev = THEME.swap(t, Ordering::Relaxed);
    let pz = ZOOM.swap(prefs.local.zoom.clamp(50, 200), Ordering::Relaxed);
    let pf = FONT_PX.swap(prefs.local.font_size.round().clamp(12., 24.) as u32, Ordering::Relaxed);
    let pd = DENSITY.swap(prefs.synced.density as u8, Ordering::Relaxed);
    let ps = SATURATION.swap(prefs.local.saturation.min(100), Ordering::Relaxed);
    let hc = prefs.local.flag("contrast", false);
    if HIGH_CONTRAST.swap(hc, Ordering::Relaxed) != hc {
        STYLE_GEN.fetch_add(1, Ordering::Relaxed);
    }
    if ps != prefs.local.saturation.min(100) {
        STYLE_GEN.fetch_add(1, Ordering::Relaxed);
    }
    // The locale too: messages rendered before the synced settings arrived kept English
    // times and day dividers ("November 13, 2026") in the French UI.
    static LOCALE: std::sync::Mutex<&str> = std::sync::Mutex::new("");
    let locale = crate::i18n::locale();
    let locale_changed = LOCALE.lock().map(|mut l| std::mem::replace(&mut *l, locale) != locale).unwrap_or(false);
    if locale_changed || prev != t || pz != zoom_raw() || pf != FONT_PX.load(Ordering::Relaxed) || pd != prefs.synced.density as u8 {
        STYLE_GEN.fetch_add(1, Ordering::Relaxed);
    }
    // Zoom is Chromium page zoom (Electron `webFrame.setZoomFactor`): the whole
    // window is scaled by GPUI (vendor patch "page zoom"), so layout stays in
    // CSS px and `rem` stays 16 px like the client's root font size.
    window.set_page_zoom(page_zoom());
    if window.rem_size() != px(16.) {
        window.set_rem_size(px(16.));
    }
    if prev != t {
        let mode = if t == 1 { ThemeMode::Light } else { ThemeMode::Dark };
        Theme::change(mode, Some(window), cx);
    }
}

fn zoom_raw() -> u32 {
    ZOOM.load(Ordering::Relaxed)
}

/// Page zoom factor (Settings > Appearance > Zoom level, Ctrl+=/-/0), 1.0 = 100 %.
/// Applied to the whole window by [`sync`]; views never multiply by it.
pub fn page_zoom() -> f32 {
    ZOOM.load(Ordering::Relaxed) as f32 / 100.
}

/// Layout zoom multiplier for authored lengths. Always 1.0 now that zoom is a
/// window-level page zoom (like Chromium): CSS px stay CSS px at every zoom
/// level, so fixed and "scaled" lengths can no longer disagree. Kept so the
/// existing `z()` / `* zoom()` call sites stay valid.
pub fn zoom() -> f32 {
    1.0
}

/// Chat font scale relative to Discord's 16 px default.
pub fn font_scale() -> f32 {
    FONT_PX.load(Ordering::Relaxed) as f32 / 16.
}

/// A layout length scaled by the UI zoom. Use instead of `px()` for sizes.
pub fn z(v: f32) -> Pixels {
    px(v * zoom())
}

/// A chat text size scaled by zoom and the font-size preference.
pub fn fz(v: f32) -> Pixels {
    px(v * zoom() * font_scale())
}

/// Background colours of Discord's five default avatar images (asset colours,
/// not CSS tokens; blurple and gold match `--background-brand`/`--premium-perk-gold`).
pub const DEFAULT_AVATAR: [u32; 5] = [0x5865F2, 0x757E8A, 0x3BA55C, 0xFAA61A, 0xED4245];

pub fn presence_color(p: crate::model::Presence) -> Hsla {
    use crate::model::Presence::*;
    c(match p {
        Online => Palette::GREEN,
        Idle => Palette::YELLOW,
        Dnd => Palette::RED,
        Offline => Palette::GREY,
    })
}

pub fn role_color(v: Option<u32>) -> Hsla {
    v.map(c).unwrap_or_else(|| c(Palette::TEXT_NORMAL))
}

#[allow(dead_code)]
pub fn to_rgba(h: Hsla) -> Rgba {
    h.into()
}

/// Layout metrics in px.
pub mod m {
    use super::generated::global as g;
    /// `--custom-guild-list-width` = avatar 40 + 2 × `--custom-guild-list-padding`
    /// (`--space-md`, 20px under `.density-cozy`). Dump -channels-N-N+ #50 `nav.guilds_` 80px.
    pub const SERVER_BAR_W: f32 = 80.;
    /// `--guildbar-avatar-size` (default density).
    pub const GUILD_ICON: f32 = g::GUILDBAR_AVATAR_SIZE;
    /// `--custom-guild-sidebar-width`.
    pub const SIDEBAR_W: f32 = g::CUSTOM_GUILD_SIDEBAR_WIDTH;
    /// `--custom-member-list-width` under `.density-cozy` (the default density
    /// class on <html>, overriding the 264px base value).
    pub const MEMBER_LIST_W: f32 = 268.;
    /// `--custom-channel-header-height` (includes the 1px bottom border).
    pub const HEADER_H: f32 = g::CUSTOM_CHANNEL_HEADER_HEIGHT;
    /// `--custom-app-top-bar-height`.
    pub const TITLE_BAR_H: f32 = g::CUSTOM_APP_TOP_BAR_HEIGHT;
    /// `--custom-app-top-bar-item-radius`.
    pub const TITLE_BAR_ITEM_RADIUS: f32 = g::CUSTOM_APP_TOP_BAR_ITEM_RADIUS;
    /// `--chat-input-icon-size`: channel header toolbar glyphs.
    pub const HEADER_ICON: f32 = g::CHAT_INPUT_ICON_SIZE;
    /// `--radius-md`: rounded top-left corner of the channel list.
    pub const FRAME_RADIUS: f32 = g::RADIUS_MD;
    pub const USER_PANEL_H: f32 = 56.;
    pub const AVATAR: f32 = 40.;
    pub const MSG_GUTTER: f32 = 72.;
    pub const MEMBER_ROW_H: f32 = 42.;
}

#[cfg(test)]
mod tests {
    use super::generated::{self, global, idx, shadow};
    use super::*;

    #[test]
    fn generated_tokens_parse_known_values() {
        // .theme-darker (Dark) / .theme-dark (Ash) --background-base-low.
        assert_eq!(generated::DARK[idx::BACKGROUND_BASE_LOW], 0x202024FF);
        assert_eq!(generated::ASH[idx::BACKGROUND_BASE_LOW], 0x36373EFF);
        assert_eq!(generated::DARK[idx::TEXT_DEFAULT], 0xEFEFF1FF);
        // Translucent token from hsl()/alpha.
        assert_eq!(generated::DARK[idx::MENTION_BACKGROUND], 0x5865F23D);
        assert_eq!(generated::NAMES[idx::BACKGROUND_BRAND], "--background-brand");
        assert_eq!(global::SPACE_8, 8.0);
        assert_eq!(global::RADIUS_MD, 12.0);
        assert_eq!(global::GUILDBAR_AVATAR_SIZE, 40.0);
        assert_eq!(shadow::SHADOW_HIGH[0][0].blur, 24.0);
        assert_eq!(resolve_for(Palette::BG_TERTIARY, AppTheme::Dark), 0x121214FF);
        assert_eq!(resolve_for(Palette::BLURPLE, AppTheme::Light), 0x5865F2FF);
    }
}

#[cfg(test)]
mod density_tests {
    use super::Density;

    #[test]
    fn density_scales_match_css() {
        // .density-cozy: --space-xs 10, --space-md 20, --space-xl 30.
        let c = Density::Cozy.scale();
        assert_eq!((c[1], c[3], c[5]), (10., 20., 30.));
        assert_eq!(Density::Default.scale()[3], 16.);
        assert_eq!(Density::Compact.scale()[6], 26.);
        assert_eq!(Density::default(), Density::Cozy);
        for d in Density::ALL {
            assert_eq!(Density::from_key(d.key()), Some(d));
        }
    }
}
