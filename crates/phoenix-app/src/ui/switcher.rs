//! Ctrl+K quick switcher (stub: channel name search).

use gpui::{
    Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, prelude::*, px,
};
use gpui_component::input::Input;

use crate::{
    app::{PhoenixApp, Switcher},
    theme::{Palette, c, ca},
    ui::{icon, sidebar::channel_icon},
};

pub fn render(app: &PhoenixApp, sw: &Switcher, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let mut results = div().mt(px(8.)).flex().flex_col().max_h(px(320.)).overflow_hidden();
    for (i, id) in sw.results.iter().enumerate() {
        let Some(ch) = app.channels.get(id) else { continue };
        let guild: SharedString = ch
            .guild
            .and_then(|g| app.guilds.iter().find(|x| x.id == g))
            .map(|g| g.name.to_uppercase().into())
            .unwrap_or_else(|| "DIRECT MESSAGE".into());
        let id = *id;
        results = results.child(
            div()
                .id(("sw", id.0))
                .h(px(34.))
                .px(px(8.))
                .rounded(px(4.))
                .flex()
                .items_center()
                .gap(px(6.))
                .cursor_pointer()
                .when(i == 0, |d| d.bg(c(Palette::BG_MOD_SELECTED)))
                .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
                .on_click(cx.listener(move |a, _, w, cx| {
                    a.close_switcher(w, cx);
                    a.select_channel(id, cx);
                }))
                .child(icon(channel_icon(ch.kind), 18., c(Palette::CHANNEL_ICON)))
                .child(
                    div()
                        .text_size(px(16.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(c(Palette::TEXT_HEADER))
                        .child(ch.name.clone()),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(Palette::TEXT_MUTED))
                        .child(guild),
                ),
        );
    }
    div()
        .id("switcher-backdrop")
        .absolute()
        .inset_0()
        .bg(ca(0x000000B3))
        .flex()
        .justify_center()
        .pt(px(120.))
        .on_click(cx.listener(|a, _, w, cx| a.close_switcher(w, cx)))
        .child(
            div()
                .id("switcher")
                .w(px(570.))
                .h_auto()
                .p(px(16.))
                .rounded(px(8.))
                .bg(c(Palette::BG_SECONDARY))
                .shadow_lg()
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .h(px(70.))
                        .px(px(16.))
                        .rounded(px(4.))
                        .bg(c(Palette::BG_TERTIARY))
                        .flex()
                        .items_center()
                        .text_size(px(20.))
                        .text_color(c(Palette::TEXT_NORMAL))
                        .child(Input::new(&sw.input).appearance(false).bordered(false)),
                )
                .child(results)
                .child(
                    div()
                        .mt(px(12.))
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(Palette::TEXT_MUTED))
                        .child("PROTIP: Start searches with @, #, !, * to narrow down results."),
                ),
        )
}
