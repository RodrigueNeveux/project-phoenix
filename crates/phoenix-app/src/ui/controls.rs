//! Discord form controls: marker slider, switch and radio cards.
//!
//! Observed values (dark theme): slider track 4px `#595A63`, fill brand
//! `#5865F2`, round white 16px grabber with a subtle border, marker labels
//! 12px/500 above the bar (default marker brighter), 48px tall with marks.

use std::rc::Rc;

use gpui::{
    App, Div, ElementId, FontWeight, InteractiveElement, MouseButton, ParentElement, SharedString,
    Styled, Window, div, prelude::*, px, relative,
};

use crate::theme::{Palette, c, ca};

const TRACK: u32 = 0x595A63;
const GRABBER_BORDER: u32 = 0x97979F33;
const MARK_LABEL: u32 = 0x949BA4;

pub type OnSet = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// Slider that sticks to evenly spaced markers. `labels[i]` is shown above
/// marker `i`; `current` and `default` are marker indices. Click or drag
/// anywhere on the track to pick the nearest marker.
pub fn marker_slider(
    id: impl Into<SharedString>,
    labels: Vec<SharedString>,
    current: usize,
    default: Option<usize>,
    on_set: OnSet,
) -> Div {
    let id: SharedString = id.into();
    let n = labels.len().max(2);
    let span = (n - 1) as f32;
    let frac = (current.min(n - 1) as f32) / span;
    let pos = |i: usize| i as f32 / span;

    let mut marks = div().absolute().top_0().left_0().size_full();
    for (i, l) in labels.iter().enumerate() {
        let is_def = default == Some(i);
        marks = marks.child(
            div()
                .absolute()
                .top_0()
                .left(relative(pos(i)))
                .w(px(0.))
                .flex()
                .flex_col()
                .items_center()
                .child(
                    div()
                        .whitespace_nowrap()
                        .text_size(px(12.))
                        .line_height(px(16.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(c(if is_def { Palette::TEXT_NORMAL } else { MARK_LABEL }))
                        .child(l.clone()),
                )
                // marker dash crossing the bar
                .child(div().mt(px(8.)).w(px(2.)).h(px(24.)).rounded(px(12.)).bg(c(TRACK))),
        );
    }

    // Hit zones: one per marker, centred on it (ends are half width).
    let mut hits = div().absolute().top_0().left_0().size_full().flex();
    for i in 0..n {
        let w = if i == 0 || i == n - 1 { 0.5 / span } else { 1. / span };
        let (s1, s2) = (on_set.clone(), on_set.clone());
        hits = hits.child(
            div()
                .id(ElementId::NamedInteger(format!("{id}-hit").into(), i as u64))
                .h_full()
                .w(relative(w))
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, move |_, w, cx| s1(i, w, cx))
                .on_mouse_move(move |ev, w, cx| {
                    if ev.dragging() {
                        s2(i, w, cx)
                    }
                }),
        );
    }

    div()
        .relative()
        .w_full()
        .h(px(48.))
        .mx(px(8.))
        .mb(px(20.))
        .child(marks)
        // bar
        .child(
            div()
                .absolute()
                .top(px(32.))
                .left_0()
                .w_full()
                .h(px(4.))
                .rounded(px(2.))
                .bg(c(TRACK))
                .child(div().h_full().w(relative(frac)).rounded(px(2.)).bg(c(Palette::BLURPLE))),
        )
        // grabber
        .child(
            div().absolute().top(px(26.)).left(relative(frac)).w(px(0.)).flex().justify_center().child(
                div()
                    .flex_none()
                    .size(px(16.))
                    .rounded_full()
                    .bg(gpui::white())
                    .border_1()
                    .border_color(ca(GRABBER_BORDER))
                    .shadow_sm(),
            ),
        )
        .child(hits)
}

/// Pick the marker index nearest to `value`.
pub fn nearest(values: &[f32], value: f32) -> usize {
    values
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1 - value).abs().total_cmp(&(b.1 - value).abs()))
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// Discord switch: 40x24 pill, green when on, white 18px knob.
pub fn switch(on: bool) -> Div {
    div()
        .w(px(40.))
        .h(px(24.))
        .flex_none()
        .rounded(px(12.))
        .bg(c(if on { Palette::GREEN } else { 0x80848E }))
        .p(px(3.))
        .flex()
        .when(on, |d| d.justify_end())
        .child(div().size(px(18.)).rounded_full().bg(gpui::white()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn nearest_marker() {
        let v = [12., 14., 15., 16., 18., 20., 24.];
        assert_eq!(super::nearest(&v, 16.), 3);
        assert_eq!(super::nearest(&v, 23.), 6);
        assert_eq!(super::nearest(&v, 1.), 0);
    }
}
