use gpui::{AnyElement, Context, IntoElement, ParentElement, SharedString, Styled, deferred, div, prelude::*};

use super::{Modal, card, close_action, settings_form};
use crate::{
    actions::{open_modal, with_modal},
    app::PhoenixApp,
    theme::{c, generated::idx, tok, z},
    ui::controls::{BtnKind, button},
};

pub const PROBLEMS: [&str; 9] = ["black-screen", "blurry", "lagging", "out-of-sync", "audio-missing", "audio-poor", "stream-stopped", "vibes-off", "other"];
pub const MAX_VISIBLE: usize = 4;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub problem: Option<usize>,
    pub open: bool,
    pub submitted: bool,
}

pub fn open() {
    open_modal(Modal::StreamFeedback(State::default()), None);
}

pub fn label(id: &str) -> SharedString {
    match id {
        "black-screen" => crate::t!("stream_fb.black_screen"),
        "blurry" => crate::t!("stream_fb.blurry"),
        "lagging" => crate::t!("stream_fb.lagging"),
        "out-of-sync" => crate::t!("stream_fb.out_of_sync"),
        "audio-missing" => crate::t!("stream_fb.audio_missing"),
        "audio-poor" => crate::t!("stream_fb.audio_poor"),
        "stream-stopped" => crate::t!("stream_fb.stream_stopped"),
        "vibes-off" => crate::t!("stream_fb.vibes_off"),
        _ => crate::t!("stream_fb.other"),
    }
}

pub fn can_submit(s: &State) -> bool {
    s.problem.is_some()
}

fn update(f: impl FnOnce(&mut State)) {
    with_modal(|m| {
        if let Some(Modal::StreamFeedback(s)) = m {
            f(s);
        }
    });
}

fn select(s: &State, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let (value, color) = match s.problem {
        Some(i) => (label(PROBLEMS[i]), idx::TEXT_STRONG),
        None => (crate::t!("stream_fb.placeholder"), idx::TEXT_MUTED),
    };
    let open = s.open;
    let boxed = settings_form::select_box("stream-fb-select", "", open)
        .child(div().absolute().left(z(12.)).right(z(36.)).top_0().bottom_0().flex().items_center().child(div().truncate().text_size(z(16.)).line_height(z(20.)).text_color(c(tok(color))).child(value)))
        .relative()
        .on_click(cx.listener(|_, _, _, cx| {
            update(|s| s.open = !s.open);
            cx.notify();
        }));
    let mut wrap = div().relative().w_full().child(boxed);
    if open {
        let mut list = div()
            .id("stream-fb-list")
            .occlude()
            .max_h(z(40. * MAX_VISIBLE as f32 + 8.))
            .overflow_y_scroll()
            .p(z(4.))
            .rounded(z(8.))
            .border_1()
            .border_color(c(tok(idx::BORDER_SUBTLE)))
            .bg(c(tok(idx::BACKGROUND_SURFACE_HIGHER)))
            .shadow(vec![gpui::BoxShadow { color: gpui::hsla(0., 0., 0., 0.16), offset: gpui::point(z(0.), z(2.)), blur_radius: z(8.), spread_radius: z(0.), inset: false }])
            .flex()
            .flex_col();
        for (k, id) in PROBLEMS.iter().enumerate() {
            let on = s.problem == Some(k);
            list = list.child(
                div()
                    .id(("stream-fb-opt", k))
                    .flex_none()
                    .h(z(40.))
                    .px(z(8.))
                    .rounded(z(4.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(z(16.))
                    .line_height(z(20.))
                    .text_color(c(tok(idx::TEXT_STRONG)))
                    .when(on, |d| d.bg(c(tok(idx::BACKGROUND_MOD_NORMAL))))
                    .hover(|s| s.bg(c(tok(idx::BACKGROUND_MOD_SUBTLE))))
                    .cursor_pointer()
                    .child(div().truncate().child(label(id)))
                    .when(on, |d| d.child(gpui::svg().path(crate::icons::path("check")).size(z(20.)).text_color(c(tok(idx::TEXT_BRAND)))))
                    .on_click(cx.listener(move |_, _, _, cx| {
                        update(|s| {
                            s.problem = Some(k);
                            s.open = false;
                        });
                        cx.notify();
                    })),
            );
        }
        wrap = wrap.child(deferred(div().absolute().top(z(44.)).left_0().w_full().child(list)).with_priority(30));
    }
    settings_form::field(crate::t!("stream_fb.label"), None, wrap, None).pb(z(16.)).into_any_element()
}

pub fn render(s: &State, cx: &mut Context<PhoenixApp>) -> AnyElement {
    if s.submitted {
        let footer = div().flex_1().flex().child(button("stream-fb-done", crate::t!("stream_fb.done"), BtnKind::Primary).flex_1().on_click({
            let close = close_action(cx);
            move |_, w, cx| close(w, cx)
        }));
        let body = settings_form::text(crate::t!("stream_fb.thanks"), 16., 20., idx::TEXT_DEFAULT).pb(z(16.));
        return card(480., crate::t!("stream_fb.submitted"), None, body.into_any_element(), Some(footer.into_any_element()), cx);
    }
    let ready = can_submit(s);
    let footer = div()
        .flex_1()
        .flex()
        .gap(z(8.))
        .child(button("stream-fb-cancel", crate::t!("common.cancel"), BtnKind::Secondary).flex_1().on_click({
            let close = close_action(cx);
            move |_, w, cx| close(w, cx)
        }))
        .child(
            button("stream-fb-submit", crate::t!("stream_fb.submit"), BtnKind::Primary)
                .flex_1()
                .when(!ready, |d| d.opacity(0.5).cursor(gpui::CursorStyle::OperationNotAllowed))
                .when(ready, |d| {
                    d.on_click(cx.listener(|_, _, _, cx| {
                        update(|s| {
                            s.open = false;
                            s.submitted = true;
                        });
                        cx.notify();
                    }))
                }),
        );
    card(480., crate::t!("stream_fb.title"), Some(crate::t!("stream_fb.subtitle")), select(s, cx), Some(footer.into_any_element()), cx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn problems_follow_the_client_order() {
        assert_eq!(PROBLEMS.len(), 9);
        assert_eq!(PROBLEMS[0], "black-screen");
        assert_eq!(PROBLEMS[8], "other");
        assert!(!can_submit(&State::default()));
        assert!(can_submit(&State { problem: Some(2), ..Default::default() }));
    }
}
