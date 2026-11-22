//! Discord dark theme palette and metrics (observed values).

use gpui::{Hsla, Rgba, rgb, rgba};

pub fn c(hex: u32) -> Hsla {
    rgb(hex).into()
}
pub fn ca(hex_rgba: u32) -> Hsla {
    rgba(hex_rgba).into()
}

pub const FONT_UI: &str = "Noto Sans";
pub const FONT_MONO: &str = "DejaVu Sans Mono";

pub struct Palette;
#[allow(dead_code)]
impl Palette {
    pub const BG_TERTIARY: u32 = 0x1E1F22; // server bar, inputs on dark
    pub const BG_SECONDARY: u32 = 0x2B2D31; // channel sidebar, member list
    pub const BG_SECONDARY_ALT: u32 = 0x232428; // user panel
    pub const BG_PRIMARY: u32 = 0x313338; // chat
    pub const BG_FLOATING: u32 = 0x111214;
    pub const BG_MOD_HOVER: u32 = 0x35373C; // channel row hover
    pub const BG_MOD_SELECTED: u32 = 0x404249; // channel row selected
    pub const BG_MSG_HOVER: u32 = 0x2E3035;
    pub const INPUT_BG: u32 = 0x383A40;
    pub const EMBED_BG: u32 = 0x2B2D31;
    pub const CODE_BG: u32 = 0x2B2D31;
    pub const CODE_BORDER: u32 = 0x1E1F22;
    pub const DIVIDER: u32 = 0x3F4147;
    pub const HEADER_SHADOW: u32 = 0x1F2023;

    pub const TEXT_NORMAL: u32 = 0xDBDEE1;
    pub const TEXT_MUTED: u32 = 0x949BA4;
    pub const TEXT_HEADER: u32 = 0xF2F3F5;
    pub const TEXT_LINK: u32 = 0x00A8FC;
    pub const INTERACTIVE: u32 = 0xB5BAC1;
    pub const INTERACTIVE_HOVER: u32 = 0xDBDEE1;
    pub const INTERACTIVE_MUTED: u32 = 0x4E5058;
    pub const CHANNEL_ICON: u32 = 0x80848E;

    pub const BLURPLE: u32 = 0x5865F2;
    pub const GREEN: u32 = 0x23A55A;
    pub const YELLOW: u32 = 0xF0B232;
    pub const RED: u32 = 0xF23F43;
    pub const GREY: u32 = 0x80848E;
    pub const MENTION_BG: u32 = 0x5865F24D; // rgba
    pub const MENTION_FG: u32 = 0xC9CDFB;
    pub const MENTIONED_MSG_BG: u32 = 0xF0B2320F;
    pub const MENTIONED_MSG_BAR: u32 = 0xF0B232;
    pub const SPOILER_BG: u32 = 0x1E1F22;
    pub const SPOILER_REVEALED: u32 = 0xFFFFFF1A;
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
    pub const GUILD_ICON: f32 = 48.;
    pub const SIDEBAR_W: f32 = 240.;
    pub const MEMBER_LIST_W: f32 = 240.;
    pub const HEADER_H: f32 = 48.;
    pub const USER_PANEL_H: f32 = 52.;
    pub const AVATAR: f32 = 40.;
    pub const MSG_GUTTER: f32 = 72.;
    pub const MEMBER_ROW_H: f32 = 42.;
}
