//! Discord theme tokens (Dark, Light, Ash, Onyx), UI zoom and font scale, and layout metrics.

use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};

use gpui::{App, Hsla, Pixels, Rgba, Window, px, rgb, rgba};
use gpui_component::theme::{Theme, ThemeMode};

use crate::ui::settings::AppTheme;

pub fn c(hex: u32) -> Hsla {
    rgb(resolve(hex)).into()
}
pub fn ca(hex_rgba: u32) -> Hsla {
    rgba(hex_rgba).into()
}

pub const FONT_UI: &str = "Noto Sans";
pub const FONT_MONO: &str = "DejaVu Sans Mono";

/// Theme tokens. Values tagged with [`TOKEN`] are resolved against the active
/// theme by [`c`] at render time, so every `c(Palette::X)` restyles live when
/// the theme changes in Settings. Untagged entries are fixed colours.
pub struct Palette;
#[allow(dead_code)]
impl Palette {
    pub const BG_TERTIARY: u32 = TOKEN;
    pub const BG_SECONDARY: u32 = TOKEN | 1;
    pub const BG_SECONDARY_ALT: u32 = TOKEN | 2;
    pub const BG_PRIMARY: u32 = TOKEN | 3;
    pub const BG_FLOATING: u32 = TOKEN | 4;
    pub const BG_MOD_HOVER: u32 = TOKEN | 5;
    pub const BG_MOD_SELECTED: u32 = TOKEN | 6;
    pub const BG_MSG_HOVER: u32 = TOKEN | 7;
    pub const INPUT_BG: u32 = TOKEN | 8;
    pub const EMBED_BG: u32 = TOKEN | 9;
    pub const CODE_BG: u32 = TOKEN | 10;
    pub const CODE_BORDER: u32 = TOKEN | 11;
    pub const DIVIDER: u32 = TOKEN | 12;
    pub const HEADER_SHADOW: u32 = TOKEN | 13;
    pub const TEXT_NORMAL: u32 = TOKEN | 14;
    pub const TEXT_MUTED: u32 = TOKEN | 15;
    pub const TEXT_HEADER: u32 = TOKEN | 16;
    pub const TEXT_LINK: u32 = TOKEN | 17;
    pub const INTERACTIVE: u32 = TOKEN | 18;
    pub const INTERACTIVE_HOVER: u32 = TOKEN | 19;
    pub const INTERACTIVE_MUTED: u32 = TOKEN | 20;
    pub const CHANNEL_ICON: u32 = TOKEN | 21;
    pub const BLURPLE: u32 = TOKEN | 22;
    pub const GREEN: u32 = TOKEN | 23;
    pub const YELLOW: u32 = TOKEN | 24;
    pub const RED: u32 = TOKEN | 25;
    pub const MENTION_FG: u32 = TOKEN | 26;
    pub const SPOILER_BG: u32 = TOKEN | 27;
    pub const APP_FRAME: u32 = TOKEN | 28;
    pub const SURFACE_HIGHER: u32 = TOKEN | 29;
    pub const SURFACE_HIGHEST: u32 = TOKEN | 30;
    pub const NOTIFICATION: u32 = TOKEN | 31;
    pub const TEXT_BRAND: u32 = TOKEN | 32;
    pub const GUILD_SEPARATOR: u32 = TOKEN | 33;
    pub const PANEL_BORDER: u32 = TOKEN | 34;
    pub const GUILD_ICON_BG: u32 = TOKEN | 35;
    pub const TEXT_CRITICAL: u32 = TOKEN | 36;
    pub const BORDER_MUTED: u32 = TOKEN | 37;
    /// `--app-frame-border` (same row as the guild separator).
    pub const APP_FRAME_BORDER: u32 = TOKEN | 33;
    /// `--background-base-low`.
    pub const BG_BASE_LOW: u32 = TOKEN | 2;
    pub const GREY: u32 = 0x80848E;
    pub const MENTION_BG: u32 = 0x5865F23D; // rgba, --mention-background
    pub const MENTIONED_MSG_BG: u32 = 0xF0B2320F;
    pub const MENTIONED_MSG_BAR: u32 = 0xF0B232;
    pub const SPOILER_REVEALED: u32 = 0xFFFFFF1A;
}

/// Tag bit marking a palette value as a theme token index rather than a hex colour.
pub const TOKEN: u32 = 0x0100_0000;

// Token values per theme, observed from the official client's CSS variables
// (theme-darker = Dark, theme-light = Light, theme-dark = Ash, theme-midnight = Onyx).
// Translucent modifiers are pre-composited over the surface they sit on.
// Columns: [Dark, Light, Ash, Onyx]
pub const TOKEN_COUNT: usize = 38;
static TABLE: [[u32; 4]; TOKEN_COUNT] = [
    [0x121214, 0xF3F3F4, 0x2C2D32, 0x000000], // BG_TERTIARY --background-base-lowest
    [0x1A1A1E, 0xFBFBFB, 0x323339, 0x000000], // BG_SECONDARY --background-base-lower
    [0x202024, 0xFBFBFB, 0x36373E, 0x000000], // BG_SECONDARY_ALT --background-base-low
    [0x1A1A1E, 0xFBFBFB, 0x323339, 0x000000], // BG_PRIMARY --background-base-lower
    [0x242429, 0xFFFFFF, 0x393A41, 0x0A0A0C], // BG_FLOATING --background-surface-high
    [0x29292E, 0xEFEFF0, 0x3E3F45, 0x121213], // BG_MOD_HOVER --background-mod-subtle
    [0x333338, 0xE3E3E5, 0x46474D, 0x242426], // BG_MOD_SELECTED --background-mod-strong
    [0x242428, 0xF3F3F4, 0x3A3B41, 0x0C0C0C], // BG_MSG_HOVER --background-mod-muted
    [0x222327, 0xFFFFFF, 0x393A41, 0x101013], // INPUT_BG --chat-background-default
    [0x242429, 0xFFFFFF, 0x393A41, 0x0A0A0C], // EMBED_BG --background-surface-high
    [0x121214, 0xF3F3F4, 0x2C2D32, 0x000000], // CODE_BG --background-base-lowest
    [0x29292E, 0xDFDFE1, 0x3E3F45, 0x1E1E20], // CODE_BORDER --border-subtle
    [0x29292E, 0xDFDFE1, 0x3E3F45, 0x1E1E20], // DIVIDER --border-subtle
    [0x29292E, 0xDFDFE1, 0x3E3F45, 0x1E1E20], // HEADER_SHADOW --border-subtle
    [0xEFEFF1, 0x2E2E34, 0xF3F3F4, 0xD4D5D8], // TEXT_NORMAL --text-default
    [0x96979E, 0x6C6D76, 0xABACB2, 0x81828A], // TEXT_MUTED --text-muted
    [0xFBFBFB, 0x28282D, 0xFFFFFF, 0xDCDCDF], // TEXT_HEADER --text-strong
    [0x4D96EE, 0x006DD4, 0x76AFF6, 0x2781E7], // TEXT_LINK --text-link
    [0xABACB2, 0x595A63, 0xC5C6CA, 0x96979E], // INTERACTIVE --interactive-text-default
    [0xFBFBFB, 0x28282D, 0xFFFFFF, 0xDCDCDF], // INTERACTIVE_HOVER --interactive-text-hover
    [0x4D505B, 0xC4C9CE, 0x4E5058, 0x4D505B], // INTERACTIVE_MUTED --interactive-muted
    [0x81828A, 0x666770, 0x999AA1, 0x7A7B83], // CHANNEL_ICON --channel-icon
    [0x5865F2, 0x5865F2, 0x5865F2, 0x5865F2], // BLURPLE --background-brand
    [0x23A55A, 0x24934E, 0x23A55A, 0x23A55A], // GREEN --status-positive
    [0xF0B232, 0xAF7615, 0xF0B232, 0xF0B232], // YELLOW --status-warning
    [0xF23F43, 0xDA373C, 0xF23F43, 0xF23F43], // RED --status-danger
    [0xA9BBFF, 0x2E3C88, 0xCDD7FF, 0x8CA0FD], // MENTION_FG --mention-foreground
    [0x666770, 0x8F9097, 0x7D7E87, 0x595A63], // SPOILER_BG --spoiler-hidden-background
    [0x070709, 0xE8E8EA, 0x25262A, 0x000000], // APP_FRAME --app-frame-background
    [0x28282D, 0xFFFFFF, 0x3C3D45, 0x121214], // SURFACE_HIGHER --background-surface-higher
    [0x2C2D32, 0xFFFFFF, 0x3F4048, 0x17181B], // SURFACE_HIGHEST --background-surface-highest
    [0xD22D39, 0xD22D39, 0xD22D39, 0xD22D39], // NOTIFICATION --background-feedback-notification
    [0x798DF9, 0x525FE0, 0x94A8FF, 0x6374F4], // TEXT_BRAND --text-brand
    [0x222225, 0xD9D9DC, 0x393A3F, 0x1E1E20], // GUILD_SEPARATOR --app-frame-border
    [0x2E2E33, 0xDFDFE1, 0x42434A, 0x1E1E20], // PANEL_BORDER --border-subtle
    [0x222225, 0xE8E8EA, 0x393A3F, 0x121213], // GUILD_ICON_BG --background-mod-subtle
    [0xF87E7A, 0xB92733, 0xFFA09B, 0xEB5F5E], // TEXT_CRITICAL --text-feedback-critical
    [0x2A2A2E, 0xEBEBEC, 0x3D3E44, 0x0E0E0F], // BORDER_MUTED --border-muted (over base-low)
];

static THEME: AtomicU8 = AtomicU8::new(0);
static ZOOM: AtomicU32 = AtomicU32::new(100);
static FONT_PX: AtomicU32 = AtomicU32::new(16);
/// Bumped whenever theme, zoom or font size change, so views that cache
/// resolved colours or measured heights know to rebuild them.
static STYLE_GEN: AtomicU32 = AtomicU32::new(0);

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

/// Resolve a palette value (token or plain 0xRRGGBB) to a hex colour for the active theme.
pub fn resolve(hex: u32) -> u32 {
    if hex & 0xFF00_0000 == TOKEN && ((hex & 0xFFFF) as usize) < TOKEN_COUNT {
        TABLE[(hex & 0xFFFF) as usize][THEME.load(Ordering::Relaxed) as usize]
    } else {
        hex
    }
}

/// Resolve a palette value for a given theme instead of the active one (theme
/// preview cards).
pub fn resolve_for(hex: u32, theme: AppTheme) -> u32 {
    if hex & 0xFF00_0000 == TOKEN && ((hex & 0xFFFF) as usize) < TOKEN_COUNT {
        TABLE[(hex & 0xFFFF) as usize][theme_index(theme) as usize]
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
    if prev != t || pz != zoom_raw() || pf != FONT_PX.load(Ordering::Relaxed) {
        STYLE_GEN.fetch_add(1, Ordering::Relaxed);
    }
    let rem = px(16. * zoom());
    if window.rem_size() != rem {
        window.set_rem_size(rem);
    }
    if prev != t {
        let mode = if t == 1 { ThemeMode::Light } else { ThemeMode::Dark };
        Theme::change(mode, Some(window), cx);
    }
}

fn zoom_raw() -> u32 {
    ZOOM.load(Ordering::Relaxed)
}

/// UI zoom factor (Settings > Appearance > Zoom level), 1.0 = 100 %.
pub fn zoom() -> f32 {
    ZOOM.load(Ordering::Relaxed) as f32 / 100.
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

/// Discord's five default avatar background colours.
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
    pub const SERVER_BAR_W: f32 = 72.;
    /// `--guildbar-avatar-size` (default density).
    pub const GUILD_ICON: f32 = 40.;
    /// `--custom-guild-sidebar-width`.
    pub const SIDEBAR_W: f32 = 268.;
    /// `--custom-member-list-width` (default density).
    pub const MEMBER_LIST_W: f32 = 264.;
    /// `--custom-channel-header-height` (includes the 1px bottom border).
    pub const HEADER_H: f32 = 49.;
    /// `--custom-app-top-bar-height`.
    pub const TITLE_BAR_H: f32 = 32.;
    /// `--custom-app-top-bar-item-radius`.
    pub const TITLE_BAR_ITEM_RADIUS: f32 = 6.;
    /// `--chat-input-icon-size`: channel header toolbar glyphs.
    pub const HEADER_ICON: f32 = 20.;
    /// `--radius-md`: rounded top-left corner of the channel list.
    pub const FRAME_RADIUS: f32 = 12.;
    pub const USER_PANEL_H: f32 = 56.;
    pub const AVATAR: f32 = 40.;
    pub const MSG_GUTTER: f32 = 72.;
    pub const MEMBER_ROW_H: f32 = 42.;
}
