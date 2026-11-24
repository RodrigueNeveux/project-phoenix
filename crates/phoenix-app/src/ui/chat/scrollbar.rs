//! The client's two scrollbar styles, drawn over a `list`'s stable gutter.
//!
//! Chat: the always-visible "auto" scrollbar in the 16px gutter.
//!
//! `auto_tqapBM::-webkit-scrollbar` 16px wide; thumb `--scrollbar-auto-thumb`, 4px
//! transparent border (so 8px of paint), radius 8px, min-height 40px; track
//! `--scrollbar-auto-track` with a 1px border. Live client: the message scroller
//! is 630.6px with a 615px client width.

use gpui::{
    AnyElement, InteractiveElement, IntoElement, ListState, ParentElement, ScrollHandle, Styled, div, px,
};

use crate::theme::{c, generated::idx, tok, z};

pub fn auto_scrollbar(state: &ListState) -> Option<AnyElement> {
    let view = state.viewport_bounds();
    let vh = f32::from(view.size.height);
    let max = f32::from(state.max_offset_for_scrollbar().y);
    if vh <= 0. || max <= 0. {
        return None;
    }
    let off = (-f32::from(state.scroll_px_offset_for_scrollbar().y)).clamp(0., max);
    let track = vh - 2.; // track border 1px top and bottom
    let thumb = (track * vh / (vh + max)).max(f32::from(z(40.))).min(track);
    let top = 1. + (track - thumb) * off / max;
    Some(
        div()
            .absolute()
            .top_0()
            .right_0()
            .bottom_0()
            .w(z(16.))
            .child(
                div()
                    .absolute()
                    .top(px(1.))
                    .bottom(px(1.))
                    .left(px(1.))
                    .right(px(1.))
                    .bg(c(tok(idx::SCROLLBAR_AUTO_TRACK))),
            )
            .child(
                div()
                    .absolute()
                    // painted thumb = thumb box minus its 4px transparent border
                    .top(px(top) + z(4.))
                    .h(px(thumb) - z(8.))
                    .left(z(4.))
                    .w(z(8.))
                    .rounded(z(8.))
                    .bg(c(tok(idx::SCROLLBAR_AUTO_THUMB))),
            )
            .into_any_element(),
    )
}

/// Group name to put on the element that holds a [`thin_fade_scrollbar`]: the bar shows
/// while the pointer is over it (`.fade_tqapBM:hover`).
pub const THIN_FADE_GROUP: &str = "thin-fade-scroller";

/// The "thin fade" scrollbar of the member list, the channel list and the DM list
/// (`scroller_ scrollbarGutterStable_ thin_tqapBM fade_tqapBM`, CDP 2026-11-24):
/// - `.thin_tqapBM::-webkit-scrollbar { width: 8px }` in the scroller's stable 8px gutter;
/// - track `--scrollbar-thin-track` (transparent in every theme), so nothing is painted
///   besides the thumb;
/// - thumb `--scrollbar-thin-thumb`, `border: 2px solid transparent`,
///   `background-clip: padding-box`, `border-radius: 4px`, `min-height: 40px`: a 4px wide
///   bar inset 2px on every side, with a 2px inner radius;
/// - `.fade_tqapBM::-webkit-scrollbar-thumb { visibility: hidden }`, visible on
///   `.fade_tqapBM:hover`. Live, 1280×720 member list: thumb #595a63 at x 1274..1278.
///
/// The parent must be `relative()` and carry `.group(THIN_FADE_GROUP)`; returns `None`
/// when the list does not overflow (no scrollbar in the client either).
pub fn thin_fade_scrollbar(state: &ListState) -> Option<AnyElement> {
    thin_fade(
        f32::from(state.viewport_bounds().size.height),
        f32::from(state.max_offset_for_scrollbar().y),
        -f32::from(state.scroll_px_offset_for_scrollbar().y),
    )
}

/// [`thin_fade_scrollbar`] for a scrolling div (`overflow_y_scroll` + `track_scroll`).
pub fn thin_fade_scrollbar_div(scroll: &ScrollHandle) -> Option<AnyElement> {
    thin_fade(
        f32::from(scroll.bounds().size.height),
        f32::from(scroll.max_offset().y),
        -f32::from(scroll.offset().y),
    )
}

fn thin_fade(vh: f32, max: f32, off: f32) -> Option<AnyElement> {
    if vh <= 0. || max <= 0. {
        return None;
    }
    let off = off.clamp(0., max);
    // No scrollbar buttons: the track is the whole scroller height.
    let thumb = (vh * vh / (vh + max)).max(f32::from(z(40.))).min(vh);
    let top = (vh - thumb) * off / max;
    Some(
        div()
            .absolute()
            .top_0()
            .right_0()
            .bottom_0()
            .w(z(8.))
            .opacity(0.)
            .group_hover(THIN_FADE_GROUP, |s| s.opacity(1.))
            .child(
                div()
                    .absolute()
                    .top(px(top) + z(2.))
                    .h(px(thumb) - z(4.))
                    .left(z(2.))
                    .w(z(4.))
                    .rounded(z(2.))
                    .bg(c(tok(idx::SCROLLBAR_THIN_THUMB))),
            )
            .into_any_element(),
    )
}
