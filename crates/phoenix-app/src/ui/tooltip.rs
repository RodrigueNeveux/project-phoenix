//! Shared Discord-style tooltip: floating black card (`--background-floating`),
//! 8px radius, 8px 12px padding, 14px/500 text, with a 5px pointer toward the
//! hovered element. gpui shows it after its hover delay (Discord: ~500ms) and
//! places it under the cursor, so the pointer is on top.
//!
//! Use: `.tooltip(tooltip::tip("Mute"))`.

use crate::theme::z;
use gpui::{AnyView, App, AppContext, FontWeight, IntoElement, ParentElement, Render, SharedString, Styled, Window, div, px};

use crate::theme::{Palette, c};

struct Tip(SharedString);

impl Render for Tip {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        let mut pointer = div().flex().flex_col().items_center();
        for i in 0..5 {
            pointer = pointer.child(div().w(px(1. + 2. * i as f32)).h(z(1.)).bg(c(Palette::BG_FLOATING)));
        }
        div()
            .flex()
            .flex_col()
            .items_center()
            .pt(z(4.))
            .child(pointer)
            .child(
                div()
                    .max_w(z(190.))
                    .px(z(12.))
                    .py(z(8.))
                    .rounded(z(8.))
                    .bg(c(Palette::BG_FLOATING))
                    .border_1()
                    .border_color(c(Palette::PANEL_BORDER))
                    .shadow_lg()
                    .text_size(z(14.))
                    .line_height(z(16.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c(Palette::TEXT_NORMAL))
                    .child(self.0.clone()),
            )
    }
}

/// Tooltip builder for `.tooltip(...)`.
pub fn tip(text: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text = text.into();
    move |_, cx| cx.new(|_| Tip(text.clone())).into()
}
