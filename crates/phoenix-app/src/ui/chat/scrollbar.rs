//! The chat's always-visible "auto" scrollbar in the 16px gutter.
//!
//! `auto_tqapBM::-webkit-scrollbar` 16px wide; thumb `--scrollbar-auto-thumb`, 4px
//! transparent border (so 8px of paint), radius 8px, min-height 40px; track
//! `--scrollbar-auto-track` with a 1px border. Live client: the message scroller
//! is 630.6px with a 615px client width.

use gpui::{AnyElement, IntoElement, ListState, ParentElement, Styled, div, px};

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
