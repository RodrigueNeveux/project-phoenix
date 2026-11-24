//! DM call header: the call area at the top of a DM / group DM while it rings us
//! (CALL_CREATE lists us in `ringing`, see [`crate::ui::incoming_call`]). We never
//! place or answer calls ourselves from here; the buttons reuse the incoming-call
//! answer/decline paths.
//!
//! CSS (current client):
//! - `callContainer_hbKzX7`: background --black, flex 1 1 auto, overflow hidden;
//!   Onyx and High contrast add a 1px --app-frame-border top border.
//! - `wrapper_NdapwK.ringingIncoming_NdapwK:after`: 3px --interactive-text-active
//!   circle around the avatar, `incoming-call-pulse` over
//!   --custom-call-avatar-incoming-duration (5.407s = 4 pulses: opacity 1 → 0 while
//!   scaling 1 → 1.1).
//! - `voiceAvatar_NdapwK.ringingOutgoing_NdapwK` opacity .3 (outgoing ring; we never
//!   call, so only the incoming state is drawn).

use std::time::Duration;

use gpui::prelude::FluentBuilder as _;
use gpui::{AnimationExt, AnyElement, Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled, div};

use crate::{
    app::PhoenixApp,
    theme::{c, generated::idx, tok, z},
};

/// Avatar size in the call area (same 80px as the incoming call card).
const AVATAR: f32 = 80.;
/// One pulse of `incoming-call-pulse` (5.407s / 4).
const PULSE_MS: u64 = 1352;

fn ringing_avatar(app: &PhoenixApp, user: crate::model::Id) -> AnyElement {
    let size = AVATAR;
    div()
        .relative()
        .size(z(size))
        .flex_none()
        .child(crate::ui::avatar(app.users.get(&user), size * crate::theme::zoom()))
        .child(
            div()
                .id(("dm-call-pulse", user.0))
                .absolute()
                .top(z(-3.))
                .left(z(-3.))
                .size(z(size + 6.))
                .rounded_full()
                .border(z(3.))
                .border_color(c(tok(idx::INTERACTIVE_TEXT_ACTIVE)))
                .with_animation(
                    ("dm-call-pulse-anim", user.0),
                    gpui::Animation::new(Duration::from_millis(PULSE_MS)).repeat(),
                    move |d, t| {
                        // scale 1 → 1.1 around the centre, opacity 1 → 0.
                        let grow = (size + 6.) * 0.1 * t;
                        d.opacity(1. - t)
                            .top(z(-3. - grow / 2.))
                            .left(z(-3. - grow / 2.))
                            .size(z(size + 6. + grow))
                    },
                ),
        )
        .into_any_element()
}

/// Call area for the selected DM while it rings us, else `None`.
pub fn render(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    let ch = app.selected_channel?;
    if app.parity.incoming_call != Some(ch) {
        return None;
    }
    let channel = app.channels.get(&ch)?;
    if channel.guild.is_some() {
        return None;
    }
    let users: Vec<crate::model::Id> = channel.voice_users.iter().copied().take(6).collect();
    let button = |id: &'static str, icon: &'static str, bg: usize, hover: usize, tip: gpui::SharedString| {
        // Same 56px round centerButton as the incoming call card.
        div()
            .id(id)
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
    };
    Some(
        div()
            .id("dm-call-header")
            .w_full()
            .flex_none()
            // The client's call area is resizable; this is its default open height.
            .h(z(240.))
            .bg(c(tok(idx::BLACK)))
            .when(crate::theme::frame_borders(), |d| d.border_t_1().border_color(c(tok(idx::APP_FRAME_BORDER))))
            .overflow_hidden()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(z(24.))
            .child(div().flex().gap(z(16.)).children(users.into_iter().map(|u| ringing_avatar(app, u))))
            .child(
                div()
                    .flex()
                    .gap(z(16.))
                    .child(
                        button(
                            "dm-call-join",
                            "phone-call",
                            idx::CONTROL_CONNECTED_BACKGROUND_DEFAULT,
                            idx::CONTROL_CONNECTED_BACKGROUND_HOVER,
                            crate::t!("call.join_voice"),
                        )
                        .on_click(cx.listener(|a, _, _, cx| crate::ui::incoming_call::answer(a, cx))),
                    )
                    .child(
                        button(
                            "dm-call-decline",
                            "close",
                            idx::CONTROL_CRITICAL_PRIMARY_BACKGROUND_DEFAULT,
                            idx::CONTROL_CRITICAL_PRIMARY_BACKGROUND_HOVER,
                            crate::t!("call.decline"),
                        )
                        .on_click(cx.listener(|a, _, _, cx| crate::ui::incoming_call::decline(a, cx))),
                    ),
            )
            .into_any_element(),
    )
}
