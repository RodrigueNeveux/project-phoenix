//! Shared Discord-style tooltip, tooltip_ZDdyhE: --background-surface-high,
//! --radius-sm (8px), inset 1px --border-subtle + --shadow-high, padding
//! --space-8 --space-12, max-width 200px, --text-default, 14px/500 text, with a 5px pointer toward the
//! hovered element. gpui shows it after its hover delay (Discord: ~500ms) and
//! places it under the cursor, so the pointer is on top.
//!
//! Use: `.tooltip(tooltip::tip("Mute"))`.

use crate::theme::z;
use gpui::{AnyView, App, AppContext, FontWeight, IntoElement, ParentElement, Render, SharedString, Styled, Window, div, px};

use crate::theme::{c, generated::idx, tok};

struct Tip(SharedString);

impl Render for Tip {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        let mut pointer = div().flex().flex_col().items_center();
        for i in 0..5 {
            pointer = pointer.child(div().w(px(1. + 2. * i as f32)).h(z(1.)).bg(c(tok(idx::BACKGROUND_SURFACE_HIGH))));
        }
        div()
            .flex()
            .flex_col()
            .items_center()
            .pt(z(4.))
            .child(pointer)
            .child(
                div()
                    .max_w(z(200.)) // tooltip_ZDdyhE max-width
                    .px(z(12.))
                    .py(z(8.))
                    .rounded(z(8.))
                    .bg(c(tok(idx::BACKGROUND_SURFACE_HIGH)))
                    // live tooltip_ZDdyhE: inset 0 0 0 1px --border-subtle, then --shadow-high
                    .border_1()
                    .border_color(c(tok(idx::BORDER_SUBTLE)))
                    .shadow(vec![gpui::BoxShadow {
                        color: gpui::hsla(0., 0., 0., 0.24),
                        offset: gpui::point(z(0.), z(12.)),
                        blur_radius: z(24.),
                        spread_radius: z(0.),
                        inset: false,
                    }])
                    .text_size(z(14.))
                    .line_height(z(18.)) // body-sm/medium
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c(tok(idx::TEXT_DEFAULT)))
                    .child(self.0.clone()),
            )
    }
}

/// Tooltip builder for `.tooltip(...)`.
pub fn tip(text: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text = text.into();
    move |_, cx| cx.new(|_| Tip(text.clone())).into()
}
