//! App top bar of the refreshed client (`--custom-app-top-bar-height`, 32px):
//! spans the whole window above the guild list, channel list and page, shows
//! the current place centered and the inbox / help buttons on the right.

use gpui::{Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, prelude::*};

use crate::{
    app::PhoenixApp,
    assets::Icon,
    theme::{c, generated::idx, m, tok, z, zoom},
    ui::{home::data::HomeView, icon},
};

fn title(app: &PhoenixApp) -> (Icon, SharedString) {
    if let Some(g) = app.selected_guild.and_then(|g| app.guilds.iter().find(|x| x.id == g)) {
        return (Icon::Hash, g.name.clone());
    }
    // vanilla-dm #29: the bar reads "Direct Messages" in every DM, not the DM name.
    if app.nav.home == HomeView::Dm {
        return (Icon::Members, crate::t!("dm.title"));
    }
    (Icon::Members, crate::t!("friends.title"))
}

fn bar_button(id: &'static str, i: Icon, active: bool, tip: SharedString) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .size(z(24.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(z(m::TITLE_BAR_ITEM_RADIUS))
        .cursor_pointer()
        // `clickable_fErCLM`: `--icon-muted`; hover `--background-mod-normal` +
        // `--interactive-icon-hover`; selected `--interactive-icon-active`.
        .hover(|s| s.bg(c(tok(idx::BACKGROUND_MOD_NORMAL))))
        .child(icon(i, 18. * zoom(), c(tok(if active { idx::INTERACTIVE_ICON_ACTIVE } else { idx::ICON_MUTED }))))
        .tooltip(crate::ui::tooltip::tip(tip))
}

/// `backForwardButtons_nS9Yqn` half (dump -channels-N-N+ #20/#22): 26×24, 18px
/// arrow, `padding 0 2 0 6` / `0 6 0 2`, outer corners `6px`, `--interactive-icon-default`.
fn nav_arrow(id: &'static str, name: &'static str, back: bool, enabled: bool) -> gpui::Stateful<gpui::Div> {
    let d = div()
        .id(id)
        .w(z(26.))
        .h(z(24.))
        .flex_none()
        .flex()
        .items_center()
        .when(back, |d| d.pl(z(6.)).pr(z(2.)).rounded_l(z(6.)))
        .when(!back, |d| d.pl(z(2.)).pr(z(6.)).rounded_r(z(6.)))
        .child(
            gpui::svg()
                .path(crate::icons::path(name))
                .size(z(18.))
                .flex_none()
                .text_color(c(tok(idx::INTERACTIVE_ICON_DEFAULT))),
        );
    if enabled {
        d.cursor_pointer().hover(|s| s.bg(c(tok(idx::BACKGROUND_MOD_NORMAL))))
    } else {
        // `disabled_nS9Yqn`
        d.opacity(0.5)
    }
}

pub fn render(app: &PhoenixApp, window: &gpui::Window, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let (glyph, text) = title(app);
    let guild = app.selected_guild.and_then(|g| app.guilds.iter().find(|x| x.id == g));
    // `bar_3zSH5d` (dump #13): 32px, no fill (the app background shows through).
    div()
        .id("app-title-bar")
        .h(z(m::TITLE_BAR_H))
        .flex_none()
        .w_full()
        .relative()
        .flex()
        .items_center()
        .justify_between()
        .child(
            // `title_LnjVAX` (dump #15): centered, gap 8; 16px guild icon (radius 4) +
            // `body-sm/medium` name in `--text-strong`.
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .gap(z(8.))
                .text_size(z(14.))
                .line_height(z(18.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(c(tok(idx::TEXT_STRONG)))
                .map(|d| match guild {
                    Some(g) => d.child(
                        div().rounded(z(4.)).overflow_hidden().child(crate::ui::invite::guild_icon(Some(&g.icon), &g.name, 16. * zoom())),
                    ),
                    None => d.child(icon(glyph, 16. * zoom(), c(tok(idx::INTERACTIVE_ICON_DEFAULT)))),
                })
                .child(div().max_w(z(480.)).text_ellipsis().child(text)),
        )
        .child(
            // `leading_3zSH5d` > `backForwardButtons_nS9Yqn` (dump #18/#19): 80px wide, gap 2.
            div()
                .relative()
                .w(z(80.))
                .h(z(24.))
                .flex()
                .justify_center()
                .gap(z(2.))
                .child(nav_arrow("title-back", "nav/back", true, true))
                .child(nav_arrow("title-forward", "nav/forward", false, false)),
        )
        .child(
            // `trailing_3zSH5d` (dump #24): gap 12, 24px `clickable_fErCLM` buttons.
            // With client-side decorations the window buttons end flush right (#46: 1163..1280).
            div()
                .relative()
                .when(!matches!(window.window_decorations(), gpui::Decorations::Client { .. }), |d| d.pr(z(12.)))
                .flex()
                .items_center()
                .gap(z(12.))
                .child(
                    bar_button("title-inbox", Icon::Inbox, app.parity.inbox.open, crate::t!("inbox.title"))
                        .on_click(cx.listener(|a, _, _, cx| a.toggle_inbox(cx))),
                )
                .child(bar_button("title-help", Icon::Help, false, crate::t!("title.help")))
                .when(matches!(window.window_decorations(), gpui::Decorations::Client { .. }), |d| d.child(win_buttons(cx))),
        )
}

/// `winButtons_3zSH5d winButtonsWithDivider_3zSH5d` (vanilla-guild-text #46..#52): gap
/// `--space-4`, `:before` divider 1px `--border-subtle`, height 32 - `--space-12`,
/// margin-inline `--space-4`; 32×32 `winButton_3zSH5d` with a 12px glyph in
/// `--interactive-text-default`. Only drawn when the window has no server-side frame.
fn win_buttons(cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let btn = |id: &'static str, glyph: &'static str, close: bool| {
        div()
            .id(id)
            .size(z(m::TITLE_BAR_H))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .text_color(c(tok(idx::INTERACTIVE_TEXT_DEFAULT)))
            .map(|d| {
                if close {
                    // `winButtonClose_3zSH5d:hover`
                    d.hover(|s| {
                        s.bg(c(tok(idx::CONTROL_CRITICAL_PRIMARY_BACKGROUND_DEFAULT)))
                            .text_color(c(tok(idx::CONTROL_CRITICAL_PRIMARY_ICON_DEFAULT)))
                    })
                } else {
                    // `winButtonMinMax_3zSH5d:hover`
                    d.hover(|s| s.bg(c(tok(idx::INTERACTIVE_BACKGROUND_HOVER))).text_color(c(tok(idx::INTERACTIVE_TEXT_HOVER))))
                }
            })
            .child(div().text_size(z(12.)).line_height(z(12.)).child(glyph))
    };
    div()
        .flex()
        .items_center()
        .gap(z(4.))
        .child(div().mx(z(4.)).w(z(1.)).h(z(m::TITLE_BAR_H - 12.)).bg(c(tok(idx::BORDER_SUBTLE))))
        .child(btn("win-min", "\u{2013}", false).on_click(cx.listener(|_, _, w, _| w.minimize_window())))
        .child(btn("win-max", "\u{25a1}", false).on_click(cx.listener(|_, _, w, _| w.zoom_window())))
        .child(btn("win-close", "\u{2715}", true).on_click(cx.listener(|_, _, w, _| w.remove_window())))
}
