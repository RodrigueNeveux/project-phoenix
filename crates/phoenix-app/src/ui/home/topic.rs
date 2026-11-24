//! Channel topic modal (click the topic in the chat header).
//!
//! Live client (vanilla-topic-modal dump, 2026-11-24): a `container_dvN-MP size-md`
//! card 480px wide, radius 12, 1px `--border-subtle`, padding 8 0 16; header
//! padding 16 24 8 with the "Channel Topic" `heading-lg/semibold` (20/24 600
//! `--text-strong`), a 40px icon-only close button (`headerTrailing_` margin -16 -16 0 12),
//! and a subtitle row: 18px channel glyph + 4px gap + channel name (`body-md/normal`
//! 16/20 `--text-subtle`). `bodySpacerTop_` 16, `bodyInner_` padding 0 8 0 24 (+ the
//! scroller gutter), markdown topic 16/22 `--text-default`, `bodySpacerBottom_` 8.

use crate::theme::z;
use gpui::{AnyElement, Context, FontWeight, IntoElement, MouseButton, ParentElement, Styled, div, prelude::*};

use crate::{
    app::PhoenixApp,
    model::Id,
    theme::{c, generated::idx, tok},
};

pub fn render(app: &PhoenixApp, ch: Id, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let Some(channel) = app.channels.get(&ch) else { return div().into_any_element() };
    let topic = channel.topic.clone().unwrap_or_default();
    let e = cx.entity();
    let close: crate::ui::modal::Action = std::rc::Rc::new(move |_, cx| e.update(cx, |a, cx| a.close_overlay(cx)));
    let close2 = close.clone();
    let card = div()
        .id("topic-popout")
        .relative()
        .w(z(480.))
        .max_h(z(720.))
        .rounded(z(12.))
        .bg(c(tok(idx::BACKGROUND_SURFACE_HIGH)))
        .border_1()
        .border_color(c(tok(idx::BORDER_SUBTLE)))
        .pt(z(8.))
        .pb(z(16.))
        .flex()
        .flex_col()
        .child(
            div()
                .pt(z(16.))
                .px(z(24.))
                .pb(z(8.))
                .flex()
                .flex_col()
                .gap(z(8.))
                .child(
                    div()
                        .pr(z(40.))
                        .text_size(z(20.))
                        .line_height(z(24.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(tok(idx::TEXT_STRONG)))
                        .child(crate::t!("topic.title")),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(z(4.))
                        .text_size(z(16.))
                        .line_height(z(20.))
                        .text_color(c(tok(idx::TEXT_SUBTLE)))
                        .child(crate::ui::icon(crate::ui::sidebar::channel_icon(channel.kind), 18. * crate::theme::zoom(), c(tok(idx::TEXT_SUBTLE))))
                        .child(div().min_w_0().text_ellipsis().child(crate::ui::emoji_text::label(channel.name.clone(), 16.))),
                ),
        )
        .child(
            div()
                .id("topic-text")
                .mt(z(16.))
                .mb(z(8.))
                .pl(z(24.))
                .pr(z(16.))
                .overflow_y_scroll()
                .text_size(z(16.))
                .line_height(z(22.))
                .text_color(c(tok(idx::TEXT_DEFAULT)))
                .child(crate::ui::components::markdown(app, &topic)),
        )
        .child(
            div()
                .id("topic-close")
                .absolute()
                .top(z(8.))
                .right(z(8.))
                .size(z(40.))
                .rounded(z(8.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(|s| s.bg(c(tok(idx::CONTROL_ICON_ONLY_BACKGROUND_HOVER))))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(move |_, w, cx| close(w, cx))
                .child(gpui::svg().path(crate::icons::path("close")).size(z(20.)).text_color(c(tok(idx::CONTROL_ICON_ONLY_ICON_DEFAULT)))),
        );
    crate::ui::modal::frame("topic-backdrop", close2, card)
}

/// Click handler for the topic text in the chat header.
pub fn on_click(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static {
    let ch = app.selected_channel;
    cx.listener(move |a, _, _, cx| {
        if let Some(ch) = ch {
            a.show_overlay(crate::ui::home::data::Overlay::Topic(ch), cx);
        }
    })
}
