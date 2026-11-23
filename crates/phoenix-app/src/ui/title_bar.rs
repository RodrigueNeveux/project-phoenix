//! App top bar of the refreshed client (`--custom-app-top-bar-height`, 32px):
//! spans the whole window above the guild list, channel list and page, shows
//! the current place centered and the inbox / help buttons on the right.

use gpui::{Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, prelude::*};

use crate::{
    app::PhoenixApp,
    assets::Icon,
    theme::{Palette, c, m, z, zoom},
    ui::{home::data::HomeView, icon},
};

fn title(app: &PhoenixApp) -> (Icon, SharedString) {
    if let Some(g) = app.selected_guild.and_then(|g| app.guilds.iter().find(|x| x.id == g)) {
        return (Icon::Hash, g.name.clone());
    }
    if app.nav.home == HomeView::Dm {
        let name = app.selected_channel.and_then(|c| app.channels.get(&c)).map(|c| c.name.clone());
        return (Icon::Members, name.unwrap_or_else(|| "Direct Messages".into()));
    }
    (Icon::Members, "Friends".into())
}

fn bar_button(id: &'static str, i: Icon, active: bool, tip: &'static str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .size(z(24.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(z(m::TITLE_BAR_ITEM_RADIUS))
        .cursor_pointer()
        .when(active, |d| d.bg(c(Palette::BG_MOD_SELECTED)))
        .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
        .child(icon(i, 18. * zoom(), c(if active { Palette::INTERACTIVE_HOVER } else { Palette::INTERACTIVE })))
        .tooltip(crate::ui::tooltip::tip(tip))
}

pub fn render(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let (glyph, text) = title(app);
    let in_guild = app.selected_guild.is_some();
    div()
        .id("app-title-bar")
        .h(z(m::TITLE_BAR_H))
        .flex_none()
        .w_full()
        .relative()
        .flex()
        .items_center()
        .justify_end()
        .pr(z(12.))
        .bg(c(Palette::BG_TERTIARY))
        .child(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .gap(z(6.))
                .text_size(z(14.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(c(Palette::TEXT_HEADER))
                .when(!in_guild, |d| d.child(icon(glyph, 16. * zoom(), c(Palette::INTERACTIVE))))
                .child(div().max_w(z(480.)).text_ellipsis().child(text)),
        )
        .child(
            div()
                .relative()
                .flex()
                .items_center()
                .gap(z(12.))
                .child(
                    bar_button("title-inbox", Icon::Inbox, app.parity.inbox.open, "Inbox")
                        .on_click(cx.listener(|a, _, _, cx| a.toggle_inbox(cx))),
                )
                .child(bar_button("title-help", Icon::Help, false, "Help")),
        )
}
