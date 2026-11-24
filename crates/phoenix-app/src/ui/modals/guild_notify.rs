//! Guild "Notification Settings" modal of the server header menu, laid out
//! like the 2026 client (dump vanilla-guild-notifications-modal, 480px
//! `container_dvN-MP`): mute switch, server level radios, community activity
//! alerts, suppress switches and the channel override table.
//! Mute and level are saved (`NavRequest::Mute` / `NotifyLevel`); the other
//! switches keep their state for the session.

use gpui::{AnyElement, Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, prelude::*};

use super::{Modal, card, close_action};
use crate::{
    actions::with_modal,
    app::PhoenixApp,
    model::Id,
    theme::{c, generated::idx, tok, z},
    ui::{
        controls::{BtnKind, button, radio_option, switch},
        home::data::NavRequest,
    },
};

/// Switches after the level radios, in the client's order.
pub const FLAGS: [(&str, &str); 7] = [
    ("gn.in_app", "gn.in_app_desc"),
    ("gn.push", "gn.push_desc"),
    ("gn.suppress_everyone", ""),
    ("gn.suppress_roles", ""),
    ("gn.highlights", "gn.highlights_desc"),
    ("gn.mute_events", ""),
    ("gn.mobile_push", ""),
];

pub struct State {
    pub guild: Id,
    pub muted: bool,
    /// 0 all, 1 mentions, 2 nothing.
    pub level: u8,
    pub flags: [bool; 7],
}

impl State {
    pub fn new(app: &PhoenixApp, guild: Id) -> Self {
        let p = app.backend.notify_prefs().and_then(|p| p.guilds.get(&guild.0).cloned()).unwrap_or_default();
        use crate::notifications::Level::*;
        let level = match p.level {
            All => 0,
            Mentions | Inherit => 1,
            Nothing => 2,
        };
        // in-app alerts, push, suppress @everyone, suppress roles, highlights,
        // mute new events, mobile push (client defaults for the unstored ones).
        let flags = [true, true, p.suppress_everyone, p.suppress_roles, false, false, true];
        Self { guild, muted: p.muted, level, flags }
    }
}

fn with_state(f: impl FnOnce(&mut State)) {
    with_modal(|m| {
        if let Some(Modal::GuildNotify(s)) = m {
            f(s)
        }
    });
}

/// `container_VhFsFg` switch row: label body-md/medium --text-strong,
/// description body-sm --text-muted (gap 4), switch top-aligned, gap 24.
fn row(id: impl Into<gpui::ElementId>, title: SharedString, desc: Option<SharedString>, on: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id.into())
        .flex()
        .items_start()
        .gap(z(24.))
        .cursor_pointer()
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap(z(4.))
                .child(div().text_size(z(16.)).line_height(z(20.)).font_weight(FontWeight::MEDIUM).text_color(c(tok(idx::TEXT_STRONG))).child(title))
                .children(desc.map(|d| div().text_size(z(14.)).line_height(z(18.)).text_color(c(tok(idx::TEXT_MUTED))).child(d))),
        )
        .group(crate::ui::controls::SWITCH_ROW)
        .child(switch("notify-switch", on))
}

fn label(s: SharedString) -> gpui::Div {
    div().text_size(z(16.)).line_height(z(20.)).font_weight(FontWeight::MEDIUM).text_color(c(tok(idx::TEXT_STRONG))).child(s)
}

/// `divider_` between groups: 1px --border-subtle, 24px around.
fn divider() -> gpui::Div {
    div().my(z(24.)).h(z(1.)).bg(c(tok(idx::BORDER_SUBTLE)))
}

pub fn render(app: &PhoenixApp, s: &State, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let name = app.guilds.iter().find(|g| g.id == s.guild).map(|g| g.name.to_string()).unwrap_or_default();
    let guild = s.guild;
    let mute = row("gn-mute", crate::t!("gn.mute", name = name), Some(crate::t!("gn.mute_desc")), s.muted).on_click(cx.listener(move |a, _, _, cx| {
        let mut now = false;
        with_state(|s| {
            s.muted = !s.muted;
            now = s.muted;
        });
        a.nav_request(NavRequest::Mute { guild: Some(guild), channel: None, muted: now, secs: None }, cx, |_, _, _| {});
        cx.notify();
    }));
    // Server level: radioGroup_ options 36 apart.
    let radios = div().flex().flex_col().gap(z(16.)).child(label(crate::t!("gn.server_level")));
    let mut opts = div().flex().flex_col().gap(z(16.));
    for (n, k) in ["menu.notify.all", "gn.mentions", "gn.nothing"].into_iter().enumerate() {
        let lv = n as u8;
        opts = opts.child(radio_option(("gn-level", n), crate::t!(k), s.level == lv).on_click(cx.listener(move |a, _, _, cx| {
            with_state(|s| s.level = lv);
            a.nav_request(NavRequest::NotifyLevel { guild: Some(guild), channel: None, level: lv }, cx, |_, _, _| {});
            cx.notify();
        })));
    }
    let flag = |i: usize, cx: &mut Context<PhoenixApp>| {
        let (t, d) = FLAGS[i];
        row(("gn-flag", i), crate::t!(t), (!d.is_empty()).then(|| crate::t!(d)), s.flags[i]).on_click(cx.listener(move |_, _, _, cx| {
            with_state(|s| s.flags[i] = !s.flags[i]);
            cx.notify();
        }))
    };
    let community = div()
        .flex()
        .flex_col()
        .gap(z(8.))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(z(4.))
                .child(label(crate::t!("gn.community")))
                .child(div().text_size(z(14.)).line_height(z(18.)).text_color(c(tok(idx::TEXT_MUTED))).child(crate::t!("gn.community_desc"))),
        )
        .child(flag(0, cx))
        .child(div().h(z(8.)))
        .child(flag(1, cx));
    let suppress = div().flex().flex_col().gap(z(32.)).child(flag(2, cx)).child(flag(3, cx)).child(flag(4, cx)).child(flag(5, cx)).child(flag(6, cx));
    let overrides = div()
        .flex()
        .flex_col()
        .gap(z(4.))
        .child(label(crate::t!("gn.override")))
        .child(div().text_size(z(14.)).line_height(z(18.)).text_color(c(tok(idx::TEXT_MUTED))).child(crate::t!("gn.override_desc")));
    let body = div()
        .id("gn-body")
        .max_h(z(560.))
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .child(mute)
        .child(divider())
        .child(radios.child(opts))
        .child(divider())
        .child(community)
        .child(divider())
        .child(suppress)
        .child(divider())
        .child(overrides);
    let footer = button("gn-done", crate::t!("common.done"), BtnKind::Primary).flex_1().on_click({
        let close = close_action(cx);
        move |_, w, cx| close(w, cx)
    });
    card(480., crate::t!("menu.notify_settings"), None, body.into_any_element(), Some(footer.into_any_element()), cx)
}
