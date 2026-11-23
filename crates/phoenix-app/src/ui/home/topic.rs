//! Channel topic popout (click the topic in the chat header).

use crate::theme::z;
use gpui::{AnyElement, Context, FontWeight, IntoElement, ParentElement, Styled, div, prelude::*};

use crate::{
    app::PhoenixApp,
    model::Id,
    theme::{Palette, c, ca},
};

pub fn render(app: &PhoenixApp, ch: Id, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let Some(channel) = app.channels.get(&ch) else { return div().into_any_element() };
    let topic = channel.topic.clone().unwrap_or_default();
    div()
        .id("topic-backdrop")
        .absolute()
        .inset_0()
        .bg(ca(0x000000B3))
        .flex()
        .items_center()
        .justify_center()
        .on_click(cx.listener(|a, _, _, cx| a.close_overlay(cx)))
        .child(
            div()
                .id("topic-popout")
                .w(z(490.))
                .max_h(z(560.))
                .rounded(z(8.))
                .bg(c(Palette::BG_PRIMARY))
                .shadow_lg()
                .p(z(16.))
                .flex()
                .flex_col()
                .gap(z(12.))
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .text_size(z(20.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(Palette::TEXT_HEADER))
                        .child(channel.name.clone()),
                )
                .child(
                    div()
                        .id("topic-text")
                        .overflow_y_scroll()
                        .text_size(z(16.))
                        .line_height(z(22.))
                        .text_color(c(Palette::TEXT_NORMAL))
                        .child(topic),
                ),
        )
        .into_any_element()
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
