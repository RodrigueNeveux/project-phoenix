//! Centered modal dialog over a dimmed backdrop (Discord's confirm dialogs:
//! title, body, grey footer with "Cancel" link + coloured primary button).

use crate::theme::z;
use std::rc::Rc;

use gpui::{
    AnyElement, App, FontWeight, IntoElement, MouseButton, ParentElement, SharedString, Styled,
    Window, deferred, div, prelude::*, px,
};

use std::time::Duration;

use gpui::{Animation, AnimationExt, ease_out_quint};

use crate::theme::{Palette, c, ca};

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
            .bg(ca(0x000000B3))
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
                |d, t| d.bg(ca(0x000000B3).opacity(t)),
            ),
    )
    .with_priority(20)
    .into_any_element()
}

pub fn confirm(m: Confirm) -> AnyElement {
    let cancel = m.on_cancel.clone();
    let cancel2 = m.on_cancel.clone();
    let ok = m.on_confirm.clone();
    let card = div()
        .relative()
        .w(z(440.))
        .rounded(z(8.))
        .overflow_hidden()
        .bg(c(Palette::BG_PRIMARY))
        .shadow_lg()
        .flex()
        .flex_col()
        .child(
            div()
                .p(z(16.))
                .flex()
                .flex_col()
                .gap(z(12.))
                .child(
                    div()
                        .text_size(z(20.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(Palette::TEXT_HEADER))
                        .child(m.title),
                )
                .children(m.body),
        )
        .child(
            div()
                .id("modal-close")
                .absolute()
                .top(z(12.))
                .right(z(12.))
                .size(z(24.))
                .rounded(z(4.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .text_color(c(Palette::INTERACTIVE))
                .hover(|s| s.text_color(c(Palette::INTERACTIVE_HOVER)))
                .on_click({
                    let cancel = m.on_cancel.clone();
                    move |_, w, cx| cancel(w, cx)
                })
                .child(gpui::svg().path("icons/close.svg").size(z(24.)).text_color(c(Palette::INTERACTIVE))),
        )
        .child(
            div()
                .p(z(16.))
                .bg(c(Palette::BG_SECONDARY))
                .flex()
                .justify_end()
                .gap(z(8.))
                .child(
                    div()
                        .id("modal-cancel")
                        .h(z(38.))
                        .px(z(16.))
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .text_size(z(14.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(gpui::white())
                        .hover(|s| s.underline())
                        .on_click(move |_, w, cx| cancel2(w, cx))
                        .child("Cancel"),
                )
                .child(
                    div()
                        .id("modal-confirm")
                        .h(z(38.))
                        .min_w(z(96.))
                        .px(z(16.))
                        .rounded(z(4.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .bg(c(if m.danger { 0xDA373C } else { Palette::BLURPLE }))
                        .hover(move |s| s.bg(c(if m.danger { 0xA12828 } else { 0x4752C4 })))
                        .text_size(z(14.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(gpui::white())
                        .on_click(move |_, w, cx| ok(w, cx))
                        .child(m.confirm_label),
                ),
        );
    frame("modal-backdrop", cancel, card)
}
