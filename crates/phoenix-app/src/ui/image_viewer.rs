//! Full-window image viewer opened by clicking an attachment or embed image.
//! Click the backdrop (or press Escape via `media::close_viewer`) to close.

use crate::theme::z;
use gpui::{
    AnyElement, App, FontWeight, ParentElement, Styled, Window, div, img, prelude::*, px,
};

use crate::{
    media::{self, MediaState},
    theme::{Palette, c, ca},
};

/// Overlay element, or `None` when no image is open.
pub fn render(window: &mut Window, cx: &mut App) -> Option<AnyElement> {
    let v = cx.try_global::<MediaState>()?.viewer.clone()?;
    let vp = window.viewport_size();
    let (w, h) = media::fit(v.width, v.height, f32::from(vp.width) * 0.9, f32::from(vp.height) * 0.8);
    let original = v.original.clone();
    Some(
        div()
            .id("image-viewer")
            .absolute()
            .inset_0()
            .bg(ca(0x000000D9))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(z(8.))
            .occlude()
            .on_click(|_, _, cx| {
                media::close_viewer(cx);
            })
            .child(
                img(v.url.clone())
                    .id("image-viewer-img")
                    .w(px(w))
                    .h(px(h))
                    .on_click(|_, _, cx| cx.stop_propagation()),
            )
            .child(
                div()
                    .id("image-viewer-open")
                    .text_size(z(14.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c(Palette::TEXT_NORMAL))
                    .opacity(0.8)
                    .hover(|s| s.opacity(1.).underline())
                    .cursor_pointer()
                    .child(crate::t!("viewer.open_browser"))
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        cx.open_url(&original);
                    }),
            )
            .into_any_element(),
    )
}
