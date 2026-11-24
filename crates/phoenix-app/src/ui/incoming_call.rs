//! Incoming call popup (client `IncomingCallModal`, `root_dcNks8`): shown
//! while a CALL_CREATE / CALL_UPDATE lists us in `ringing`. Ctrl+Enter joins
//! the call, Esc (or the X button) stops our ringing, like the client's
//! keydown handler (`ESCAPE` → `stopRinging`).
//!
//! CSS (`*_dcNks8`): root min-width 232, padding 16, radius 8,
//! --background-surface-high, --shadow-border + --shadow-high, column,
//! centred; mainChannelInfo icon margin 16 0 (80px avatar), title
//! body-lg/semibold --text-strong margin-bottom 2, subtitle body-md/normal
//! --text-default margin-bottom 24; actions_dcNks8 flex gap 8.

use gpui::{AnyElement, Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, prelude::*};

use crate::{
    app::PhoenixApp,
    model::Id,
    theme::{c, generated::idx, tok, z},
};

/// Ringing channel, if any.
pub fn ringing(app: &PhoenixApp) -> Option<Id> {
    app.parity.incoming_call
}

/// Ctrl+Enter: answer (join the call's voice channel).
pub fn answer(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) {
    let Some(ch) = app.parity.incoming_call.take() else { return };
    app.join_voice(Some(ch), cx);
    cx.notify();
}

/// Esc / X: stop ringing for us only (POST call/stop-ringing).
pub fn decline(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) {
    let Some(ch) = app.parity.incoming_call.take() else { return };
    crate::ui::pickers::detach(app.backend.stop_ringing(ch), cx);
    cx.notify();
}

/// CALL_CREATE / CALL_UPDATE / CALL_DELETE: track whether we're rung.
pub fn on_call(app: &mut PhoenixApp, channel: Id, ringing: &[Id]) {
    if ringing.contains(&app.me.id) {
        app.parity.incoming_call = Some(channel);
    } else if app.parity.incoming_call == Some(channel) {
        app.parity.incoming_call = None;
    }
}

/// Key handling for [`crate::shortcuts`]: true when consumed.
pub fn on_key(app: &mut PhoenixApp, ks: &gpui::Keystroke, cx: &mut Context<PhoenixApp>) -> bool {
    if app.parity.incoming_call.is_none() {
        return false;
    }
    let m = ks.modifiers;
    match ks.key.as_str() {
        "enter" if m.control && !m.alt && !m.shift => answer(app, cx),
        "escape" if !m.control && !m.alt && !m.shift => decline(app, cx),
        _ => return false,
    }
    true
}

/// 56px round call action (centerButton): red X to decline, green phone to join.
fn action(id: &'static str, icon: &'static str, bg: usize, hover: usize, tip: SharedString) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .line_height(z(0.)) // actionButton_dcNks8
        .size(z(56.))
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(c(tok(bg)))
        .hover(move |s| s.bg(c(tok(hover))))
        .cursor_pointer()
        .tooltip(crate::ui::tooltip::tip(tip))
        .child(gpui::svg().path(crate::icons::path(icon)).size(z(24.)).text_color(c(tok(idx::WHITE))))
}

pub fn render(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    let ch = ringing(app)?;
    let channel = app.channels.get(&ch);
    let user = channel.and_then(|c| c.voice_users.first()).and_then(|u| app.users.get(u));
    let name: SharedString = channel.map(|c| c.name.clone()).or_else(|| user.map(|u| u.display_name.clone())).unwrap_or_default();
    let card = div()
        .id("incoming-call")
        .occlude()
        .min_w(z(232.))
        .p(z(16.))
        .rounded(z(8.))
        .bg(c(tok(idx::BACKGROUND_SURFACE_HIGH)))
        .border_1()
        .border_color(c(tok(idx::BORDER_SUBTLE))) // --shadow-border
        .shadow_lg() // --shadow-high
        .flex()
        .flex_col()
        .items_center()
        .child(div().my(z(16.)).child(crate::ui::avatar(user, 80. * crate::theme::zoom())))
        .child(
            div()
                .max_w_full()
                .mb(z(2.))
                .text_center()
                .text_size(z(18.))
                .line_height(z(22.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c(tok(idx::TEXT_STRONG)))
                .child(name),
        )
        .child(
            div()
                .mb(z(24.))
                .text_center()
                .text_size(z(16.))
                .line_height(z(20.))
                .text_color(c(tok(idx::TEXT_DEFAULT)))
                .child(crate::t!("call.incoming")),
        )
        .child(
            div()
                .flex()
                .gap(z(8.))
                .child(
                    action(
                        "call-decline",
                        "close",
                        idx::CONTROL_CRITICAL_PRIMARY_BACKGROUND_DEFAULT,
                        idx::CONTROL_CRITICAL_PRIMARY_BACKGROUND_HOVER,
                        crate::t!("call.decline"),
                    )
                    .on_click(cx.listener(|a, _, _, cx| decline(a, cx))),
                )
                .child(
                    action(
                        "call-accept",
                        "phone-call",
                        idx::CONTROL_CONNECTED_BACKGROUND_DEFAULT,
                        idx::CONTROL_CONNECTED_BACKGROUND_HOVER,
                        crate::t!("call.join_voice"),
                    )
                    .on_click(cx.listener(|a, _, _, cx| answer(a, cx))),
                ),
        );
    // wrapper_dcNks8: draggable popup; initial position centred near the top.
    Some(div().absolute().top_0().left_0().size_full().flex().items_start().justify_center().pt(z(96.)).child(card).into_any_element())
}
