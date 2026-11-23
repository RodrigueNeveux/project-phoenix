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

/// Offscreen renders have no compositor, so gpui reports server-side decorations and the
/// window buttons vanish. Examples comparing against the desktop client set this.
pub static FORCE_CLIENT_DECORATIONS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn client_decorated(window: &gpui::Window) -> bool {
    FORCE_CLIENT_DECORATIONS.load(std::sync::atomic::Ordering::Relaxed)
        || matches!(window.window_decorations(), gpui::Decorations::Client { .. })
}

/// Whether the last title bar drew the window buttons (popouts anchored to the
/// inbox button read it; they render without the window).
static DREW_WIN_BUTTONS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Distance from the window's right edge to the inbox button's right edge:
/// help 24 + gap 12 + (window buttons 3×32 + 2×4 gap + 1px divider with 4px
/// margins + 4 gap = 117, + gap 12) or the 12px trailing padding.
/// Vanilla-inbox-1: inbox #36 right edge 1115 of 1280 (165).
pub fn inbox_right_inset() -> f32 {
    let tail = if DREW_WIN_BUTTONS.load(std::sync::atomic::Ordering::Relaxed) { 117. + 12. } else { 12. };
    tail + 24. + 12.
}

/// Window height in CSS px (logical / zoom) at the last title bar render, for
/// popouts whose max-height depends on `@media (max-height: …)`.
static VIEWPORT_H_CSS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

pub fn viewport_h_css() -> f32 {
    f32::from_bits(VIEWPORT_H_CSS.load(std::sync::atomic::Ordering::Relaxed))
}

pub fn render(app: &PhoenixApp, window: &gpui::Window, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let vh = f32::from(window.viewport_size().height) / zoom();
    VIEWPORT_H_CSS.store(vh.to_bits(), std::sync::atomic::Ordering::Relaxed);
    DREW_WIN_BUTTONS.store(client_decorated(window), std::sync::atomic::Ordering::Relaxed);
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
                .when(!client_decorated(window), |d| d.pr(z(12.)))
                .flex()
                .items_center()
                .gap(z(12.))
                .child(
                    bar_button("title-inbox", Icon::Inbox, app.parity.inbox.open, crate::t!("inbox.title"))
                        .on_click(cx.listener(|a, _, _, cx| a.toggle_inbox(cx))),
                )
                .child(bar_button("title-help", Icon::Help, false, crate::t!("title.help")))
                .when(client_decorated(window), |d| d.child(win_buttons(cx))),
        )
}

/// `winButtons_3zSH5d winButtonsWithDivider_3zSH5d` (vanilla-guild-text #46..#52): gap
/// `--space-4`, `:before` divider 1px `--border-subtle`, height 32 - `--space-12`,
/// margin-inline `--space-4`; 32×32 `winButton_3zSH5d` with a 12px glyph in
/// `--interactive-text-default`. Only drawn when the window has no server-side frame.
fn win_buttons(cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let btn = |id: &'static str, glyph: gpui::AnyElement, close: bool| {
        div()
            .id(id)
            .group(id)
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
            .child(glyph)
    };
    div()
        .flex()
        .items_center()
        .gap(z(4.))
        .child(div().mx(z(4.)).w(z(1.)).h(z(m::TITLE_BAR_H - 12.)).bg(c(tok(idx::BORDER_SUBTLE))))
        .child(btn("win-min", div().w(z(12.)).h(z(1.)).bg(gpui::transparent_black()).border_t_1().border_color(c(tok(idx::INTERACTIVE_TEXT_DEFAULT)))
            // `winButtonMinMax_3zSH5d:hover` color `--interactive-text-hover` (glyph stroke)
            .group_hover("win-min", |s| s.border_color(c(tok(idx::INTERACTIVE_TEXT_HOVER)))).into_any_element(), false).on_click(cx.listener(|_, _, w, _| w.minimize_window())))
        .child(btn("win-max", div().size(z(12.)).border_1().border_color(c(tok(idx::INTERACTIVE_TEXT_DEFAULT)))
            .group_hover("win-max", |s| s.border_color(c(tok(idx::INTERACTIVE_TEXT_HOVER)))).into_any_element(), false).on_click(cx.listener(|_, _, w, _| w.zoom_window())))
        .child(btn("win-close", icon(Icon::Close, 14. * zoom(), c(tok(idx::INTERACTIVE_TEXT_DEFAULT))).into_any_element(), true).on_click(cx.listener(|_, _, w, _| w.remove_window())))
}
