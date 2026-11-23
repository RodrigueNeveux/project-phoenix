//! Forwarded message frame (`container_4dy_KE`): a 4px quote bar, a "Forwarded" header and an
//! origin footer around the snapshot's content (vanilla guild text, CDP 2026-11-23).

use gpui::{AnyElement, FontWeight, IntoElement, ParentElement, Styled, div, prelude::*};

use crate::{
    app::PhoenixApp,
    model::Forward,
    theme::{c, generated::idx, tok, z},
};

fn tk(i: usize) -> gpui::Hsla {
    c(tok(i))
}

pub fn wrap(app: &PhoenixApp, f: &Forward, body: gpui::Div) -> AnyElement {
    let channel = app.channels.get(&f.channel).map(|c| c.name.clone());
    let guild = f.guild.and_then(|g| app.guilds.iter().find(|x| x.id == g));
    let when = chrono::DateTime::from_timestamp(f.timestamp, 0).map(|t| crate::i18n::clock(t.with_timezone(&chrono::Local)));
    let footer_text = match (channel, when) {
        (Some(ch), Some(t)) => format!("{ch} \u{2022} {t}"),
        (Some(ch), None) => ch.to_string(),
        (None, Some(t)) => t,
        (None, None) => String::new(),
    };
    div()
        .flex() // container_ display flex, width 100%
        .w_full()
        .child(
            // quote_: 4px, radius 2px, --border-subtle, margin-inline-end 12px
            div().flex_none().w(z(4.)).rounded(z(2.)).mr(z(12.)).bg(tk(idx::BORDER_SUBTLE)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .flex_grow(1.)
                .min_w_0()
                .child(
                    // headerContainer_: gap 4px, margin 2px 0 4px; headerText_ body-sm/semibold italic, --text-muted
                    div()
                        .flex()
                        .items_center()
                        .gap(z(4.))
                        .mt(z(2.))
                        .mb(z(4.))
                        .h(z(18.))
                        .text_size(z(14.))
                        .line_height(z(18.))
                        .text_color(tk(idx::TEXT_MUTED))
                        .child(div().w(z(16.)).flex().justify_center().child("\u{21B1}"))
                        .child(div().font_weight(FontWeight::SEMIBOLD).italic().child(crate::t!("chat.forwarded"))),
                )
                .child(body)
                .when(!footer_text.is_empty(), |d| {
                    // footerContainer_: gap 4px, margin-top 4px, padding 1px 0, radius 4px; footerText_ body-sm/medium --text-muted
                    d.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(z(4.))
                            .mt(z(4.))
                            .py(z(1.))
                            .rounded(z(4.))
                            .text_size(z(14.))
                            .line_height(z(18.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(tk(idx::TEXT_MUTED))
                            .when_some(guild, |d, g| d.child(crate::ui::invite::guild_icon(Some(&g.icon), &g.name, 16. * crate::theme::zoom())))
                            .child(div().min_w_0().text_ellipsis().child(format!("{footer_text} \u{203A}"))),
                    )
                }),
        )
        .into_any_element()
}
