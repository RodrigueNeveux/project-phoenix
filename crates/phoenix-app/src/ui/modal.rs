//! Centered modal dialog over a dimmed backdrop (Discord's confirm dialogs:
//! 776412 `container_dvN-MP` layout: header, body, action bar of Mana buttons).

use crate::theme::z;
use std::rc::Rc;

use gpui::{
    AnyElement, App, FontWeight, IntoElement, MouseButton, ParentElement, SharedString, Styled,
    Window, deferred, div, prelude::*, px,
};

use std::time::Duration;

use gpui::{Animation, AnimationExt, ease_out_quint};

use crate::theme::{c, generated::idx, tok};

pub type Action = Rc<dyn Fn(&mut Window, &mut App)>;

pub struct Confirm {
    pub title: SharedString,
    pub body: Vec<AnyElement>,
    pub confirm_label: SharedString,
    pub danger: bool,
    pub on_confirm: Action,
    pub on_cancel: Action,
}

/// Generic frame: backdrop + 440px card. Clicking the backdrop cancels.
pub fn frame(id: &'static str, on_cancel: Action, card: impl IntoElement) -> AnyElement {
    deferred(
        div()
            .id(id)
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .bg(c(tok(idx::BACKGROUND_SCRIM))) // --background-scrim
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, move |_, w, cx| on_cancel(w, cx))
            .child(
                div()
                    .id("modal-card")
                    .occlude()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(card)
                    // Discord: card fades in and grows from ~0.9 over ~200ms.
                    .with_animation(
                        id,
                        Animation::new(Duration::from_millis(200)).with_easing(ease_out_quint()),
                        |d, t| d.opacity(t).mt(px(24. * (1. - t))),
                    ),
            )
            .with_animation(
                "modal-backdrop-fade",
                Animation::new(Duration::from_millis(150)),
                |d, t| d.bg(c(tok(idx::BACKGROUND_SCRIM)).opacity(t)),
            ),
    )
    .with_priority(20)
    .into_any_element()
}

pub fn confirm(m: Confirm) -> AnyElement {
    let cancel = m.on_cancel.clone();
    let cancel2 = m.on_cancel.clone();
    let ok = m.on_confirm.clone();
    let danger = m.danger;
    // 776412 `container_dvN-MP`: --background-surface-high, 1px --border-subtle,
    // --radius-md, --shadow-high; `size-md` max-width 480px; `padding-size-sm`:
    // padding-top --space-8, padding-bottom --space-16, --custom-modal-padding 24px.
    let card = div()
        .relative()
        .w(z(480.))
        .rounded(z(12.))
        .bg(c(tok(idx::BACKGROUND_SURFACE_HIGH)))
        .border_1()
        .border_color(c(tok(idx::BORDER_SUBTLE)))
        .shadow_lg()
        .pt(z(8.))
        .pb(z(16.))
        .flex()
        .flex_col()
        // section_dvN-MP header: padding 16px 24px 8px
        .child(
            div()
                .pt(z(16.))
                .px(z(24.))
                .pb(z(8.))
                .pr(z(24. + 32.))
                .text_size(z(20.))
                .line_height(z(24.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c(tok(idx::TEXT_STRONG)))
                .child(m.title),
        )
        // bodyInner_dvN-MP: padding-inline --custom-modal-padding
        .child(
            div()
                .px(z(24.))
                .flex()
                .flex_col()
                .gap(z(12.))
                .text_size(z(16.))
                .line_height(z(20.))
                .text_color(c(tok(idx::TEXT_DEFAULT)))
                .children(m.body),
        )
        // headerTrailingAbsolute: top --custom-modal-padding-md (16) pulled up by
        // headerTrailing margin-top -16 / margin-right -(24-8): 8px from the card edge,
        // in the padding-top 8 -> top 16, right 8. Icon-only `sm` button 32px.
        .child(
            div()
                .id("modal-close")
                .absolute()
                .top(z(16.))
                .right(z(8.))
                .size(z(32.))
                .rounded(z(8.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(|s| s.bg(c(tok(idx::CONTROL_ICON_ONLY_BACKGROUND_HOVER))))
                .on_click({
                    let cancel = m.on_cancel.clone();
                    move |_, w, cx| cancel(w, cx)
                })
                .child(gpui::svg().path("icons/close.svg").size(z(20.)).text_color(c(tok(idx::CONTROL_ICON_ONLY_ICON_DEFAULT)))),
        )
        // footer section padding 16px 24px 8px; actionBarTrailing gap --space-8; md buttons.
        .child(
            div()
                .pt(z(16.))
                .px(z(24.))
                .pb(z(8.))
                .flex()
                .justify_end()
                .gap(z(8.))
                .child(
                    crate::ui::profile::mana_button("modal-cancel", crate::t!("common.cancel"), false, true)
                        .on_click(move |_, w, cx| cancel2(w, cx)),
                )
                .child({
                    let b = crate::ui::profile::mana_button("modal-confirm", m.confirm_label, true, true);
                    // critical-primary_F7qvhY
                    if danger {
                        b.bg(c(tok(idx::CONTROL_CRITICAL_PRIMARY_BACKGROUND_DEFAULT)))
                            .border_color(c(tok(idx::CONTROL_CRITICAL_PRIMARY_BORDER_DEFAULT)))
                            .text_color(c(tok(idx::CONTROL_CRITICAL_PRIMARY_TEXT_DEFAULT)))
                            .hover(|s| s.bg(c(tok(idx::CONTROL_CRITICAL_PRIMARY_BACKGROUND_HOVER))))
                    } else {
                        b
                    }
                    .on_click(move |_, w, cx| ok(w, cx))
                }),
        );
    frame("modal-backdrop", cancel, card)
}
