//! Discord form controls: marker slider, switch and radio cards.
//!
//! Observed values (dark theme): slider track 4px `#595A63`, fill brand
//! `#5865F2`, round white 16px grabber with a subtle border, marker labels
//! 12px/500 above the bar (default marker brighter), 48px tall with marks.

use crate::theme::z;
use std::rc::Rc;

use gpui::{
    App, Div, ElementId, FontWeight, InteractiveElement, MouseButton, ParentElement, SharedString,
    Styled, Window, div, prelude::*, relative,
};

use crate::theme::{c, generated::idx, tok};


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
    // markValue_OnqacX sits at the value's proportional position (vanilla
    // accessibility dump: 12/14/15/16/18/20/24px are unevenly spaced). Labels
    // that aren't numbers fall back to even spacing.
    let nums: Vec<f32> = labels
        .iter()
        .filter_map(|l| l.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect::<String>().parse().ok())
        .collect();
    let pos_v: Vec<f32> = match (nums.len() == labels.len() && n == labels.len(), nums.first(), nums.last()) {
        (true, Some(&a), Some(&b)) if b > a => nums.iter().map(|v| (v - a) / (b - a)).collect(),
        _ => (0..n).map(|i| i as f32 / span).collect(),
    };
    let pos = |i: usize| pos_v.get(i).copied().unwrap_or(1.);
    let frac = pos(current.min(n - 1));

    let mut marks = div().absolute().top_0().left_0().size_full();
    for (i, l) in labels.iter().enumerate() {
        let is_def = default == Some(i);
        marks = marks.child(
            div()
                .absolute()
                .top_0()
                .left(relative(pos(i)))
                .w(z(0.))
                .flex()
                .flex_col()
                .items_center()
                .child({
                    // markValue_OnqacX (vanilla-settings-09 #2777-#2795): 12/16 500,
                    // --text-subtle, the default one --text-default; the end labels
                    // align to the bar edges instead of centring on their marker.
                    let label = div()
                        .whitespace_nowrap()
                        .text_size(z(12.))
                        .line_height(z(16.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(c(tok(if is_def { idx::TEXT_DEFAULT } else { idx::TEXT_SUBTLE })))
                        .child(l.clone());
                    if i == 0 {
                        div().w(z(0.)).flex().justify_start().child(label.ml(z(-1.)))
                    } else if i + 1 == labels.len() {
                        div().w(z(0.)).flex().justify_end().child(label)
                    } else {
                        div().child(label)
                    }
                })
                // marker dash crossing the bar
                .child(div().mt(z(8.)).w(z(2.)).h(z(24.)).rounded(z(12.)).bg(c(tok(idx::SLIDER_TRACK_BACKGROUND)))),
        );
    }

    // Hit zones: one per marker, centred on it (ends are half width).
    let mut hits = div().absolute().top_0().left_0().size_full().flex();
    for i in 0..n {
        // zone = between the midpoints to the neighbouring markers
        let lo = if i == 0 { 0. } else { (pos(i - 1) + pos(i)) / 2. };
        let hi = if i == n - 1 { 1. } else { (pos(i) + pos(i + 1)) / 2. };
        let w = hi - lo;
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
        .h(z(48.))
        .mx(z(8.))
        .mb(z(20.))
        .child(marks)
        // bar
        .child(
            div()
                .absolute()
                .top(z(32.))
                .left_0()
                .w_full()
                .h(z(4.))
                .rounded(z(2.))
                .bg(c(tok(idx::SLIDER_TRACK_BACKGROUND)))
                .child(div().h_full().w(relative(frac)).rounded(z(2.)).bg(c(tok(idx::SLIDER_INDICATOR_BACKGROUND)))),
        )
        .child(hits)
        // grabber, above the hit zones so it gets its own hover (ref-cache
        // settings-grab0-hover: 16 -> 20px, centre kept by -2 margins); its
        // hitbox doesn't block the zones underneath.
        .child(
            div().absolute().top(z(26.)).left(relative(frac)).w(z(0.)).flex().justify_center().child(
                div()
                    .id(ElementId::Name(format!("{id}-grab").into()))
                    .flex_none()
                    .size(z(16.))
                    .rounded_full()
                    .bg(gpui::white())
                    .border_1()
                    .border_color(c(tok(idx::BORDER_NORMAL))) // slider_ .grabber_ border --border-normal
                    .shadow_sm()
                    .cursor_pointer()
                    .hover(|d| d.size(z(20.)).m(z(-2.))),
            ),
        )
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

/// Group name a clickable row sets so pressing it shows the switch :active.
pub const SWITCH_ROW: &str = "phx-switch-row";

/// Mana switch (switchIndicator_9AraPt, live settings-accessibility dump):
/// 48x24, 1px border, --radius-lg (16); thumb is a 24px box (margin -1) holding
/// a 16px white disc, at left 1 off / 24 on.
///
/// `id` is a per-caller element id so the switch tracks its own pressed state.
/// Pressed (`--switch-background-active`, set from JS in the client while the
/// pointer is down) applies both on the switch itself and when the enclosing
/// row carries `.group(SWITCH_ROW)`.
pub fn switch(id: impl Into<ElementId>, on: bool) -> gpui::Stateful<Div> {
    // Hover (ref-cache settings-switch-states): on #5865F2 → #4452BB, off border .2 → .4 alpha.
    let (bg, border, hbg, hborder) = if on {
        (
            idx::SWITCH_BACKGROUND_SELECTED_DEFAULT,
            idx::SWITCH_BORDER_SELECTED_DEFAULT,
            idx::SWITCH_BACKGROUND_SELECTED_HOVER,
            idx::SWITCH_BORDER_SELECTED_HOVER,
        )
    } else {
        (idx::SWITCH_BACKGROUND_DEFAULT, idx::SWITCH_BORDER_DEFAULT, idx::SWITCH_BACKGROUND_HOVER, idx::SWITCH_BORDER_HOVER)
    };
    div()
        .id(id.into())
        .relative()
        .flex_none()
        .w(z(48.))
        .h(z(24.))
        .rounded(z(16.)) // --radius-lg
        .border_1()
        .border_color(c(tok(border)))
        .bg(c(tok(bg)))
        .cursor_pointer()
        .hover(move |d| d.bg(c(tok(hbg))).border_color(c(tok(hborder))))
        .when(on, |d| {
            d.active(|d| d.bg(c(tok(idx::SWITCH_BACKGROUND_ACTIVE))))
                .group_active(SWITCH_ROW, |d| d.bg(c(tok(idx::SWITCH_BACKGROUND_ACTIVE))))
        })
        .child(
            div()
                .absolute()
                .top(z(-1.))
                .left(z(if on { 23. } else { 0. })) // left 24 / 1, margin-left -1
                .size(z(24.))
                .flex()
                .items_center()
                .justify_center()
                .child(div().size(z(16.)).rounded_full().bg(c(tok(if on {
                    idx::SWITCH_THUMB_BACKGROUND_SELECTED_DEFAULT
                } else {
                    idx::SWITCH_THUMB_BACKGROUND_DEFAULT
                })))),
        )
}

/// radioIndicator_oJuRbG (20px, viewBox 40): outer disc filled
/// --radio-background-[selected-]default with a 2/40 (1px) --radio-border-*
/// stroke; selected adds the r=8/40 (8px) --radio-thumb-background-active dot.
pub fn radio_indicator(selected: bool) -> Div {
    let (bg, border) = if selected {
        (idx::RADIO_BACKGROUND_SELECTED_DEFAULT, idx::RADIO_BORDER_SELECTED_DEFAULT)
    } else {
        (idx::RADIO_BACKGROUND_DEFAULT, idx::RADIO_BORDER_DEFAULT)
    };
    div()
        .size(z(20.))
        .flex_none()
        .rounded_full()
        .border_1()
        .border_color(c(tok(border)))
        .bg(c(tok(bg)))
        .flex()
        .items_center()
        .justify_center()
        .when(selected, |d| d.child(div().size(z(8.)).rounded_full().bg(c(tok(idx::RADIO_THUMB_BACKGROUND_ACTIVE)))))
}

/// checkboxOption_vTUVyc: 20px indicator (1px border, --radius-xs 4px), label at
/// 20 + --space-12, body-md 16/20. Selected: --checkbox-background/border-selected-default
/// with the --checkbox-icon-active check; hover recolours the indicator.
pub fn checkbox_option(id: impl Into<ElementId>, label: impl Into<SharedString>, selected: bool) -> gpui::Stateful<Div> {
    let (bg, border, hbg, hborder) = if selected {
        (
            idx::CHECKBOX_BACKGROUND_SELECTED_DEFAULT,
            idx::CHECKBOX_BORDER_SELECTED_DEFAULT,
            idx::CHECKBOX_BACKGROUND_SELECTED_HOVER,
            idx::CHECKBOX_BORDER_SELECTED_HOVER,
        )
    } else {
        (idx::CHECKBOX_BACKGROUND_DEFAULT, idx::CHECKBOX_BORDER_DEFAULT, idx::CHECKBOX_BACKGROUND_HOVER, idx::CHECKBOX_BORDER_HOVER)
    };
    div()
        .id(id.into())
        .group("checkbox-option")
        .flex()
        .items_start()
        .gap(z(12.)) // --custom-checkbox-gap: --space-12
        .rounded(z(8.)) // --radius-sm
        .cursor_pointer()
        .child(
            div()
                .size(z(20.)) // --custom-checkbox-size
                .flex_none()
                .rounded(z(4.)) // --radius-xs
                .border_1()
                .border_color(c(tok(border)))
                .bg(c(tok(bg)))
                .group_hover("checkbox-option", |s| s.bg(c(tok(hbg))).border_color(c(tok(hborder))))
                .flex()
                .items_center()
                .justify_center()
                .when(selected, |d| {
                    d.child(gpui::svg().path(crate::icons::path("check")).size(z(16.)).text_color(c(tok(idx::CHECKBOX_ICON_ACTIVE))))
                }),
        )
        .child(
            div()
                .text_size(z(16.))
                .line_height(z(20.))
                .text_color(c(tok(idx::TEXT_DEFAULT)))
                .child(label.into()),
        )
}

/// radioGroupOption_oJuRbG: 20px indicator, label at 20 + --space-12,
/// body-md 16/20, radius 8. Selected/hover recolour the indicator only.
pub fn radio_option(id: impl Into<ElementId>, label: impl Into<SharedString>, selected: bool) -> gpui::Stateful<Div> {
    div()
        .id(id.into())
        .flex()
        .items_center()
        .gap(z(12.)) // --custom-radio-text-start - --custom-radio-size
        .rounded(z(8.))
        .cursor_pointer()
        .child(radio_indicator(selected))
        .child(
            div()
                .text_size(z(16.))
                .line_height(z(20.))
                .text_color(c(tok(idx::TEXT_DEFAULT)))
                .child(label.into()),
        )
}

/// Mana button (button_F7qvhY), md size: 40px (7/15 padding + 1px border),
/// radius 8, label 16/500/20. `kind`: secondary / primary / critical.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BtnKind {
    Secondary,
    Primary,
    Critical,
}

pub fn button(id: impl Into<ElementId>, label: impl Into<SharedString>, kind: BtnKind) -> gpui::Stateful<Div> {
    // [default, hover, active] × (background, text, border) per variant:
    // `.secondary_F7qvhY` / `.primary_F7qvhY` / `.critical-primary_F7qvhY` (:hover, :active).
    use idx::*;
    let [bg, fg, border, hbg, hfg, hborder, abg, afg, aborder] = match kind {
        BtnKind::Secondary => [
            CONTROL_SECONDARY_BACKGROUND_DEFAULT,
            CONTROL_SECONDARY_TEXT_DEFAULT,
            CONTROL_SECONDARY_BORDER_DEFAULT,
            CONTROL_SECONDARY_BACKGROUND_HOVER,
            CONTROL_SECONDARY_TEXT_HOVER,
            CONTROL_SECONDARY_BORDER_HOVER,
            CONTROL_SECONDARY_BACKGROUND_ACTIVE,
            CONTROL_SECONDARY_TEXT_ACTIVE,
            CONTROL_SECONDARY_BORDER_ACTIVE,
        ],
        BtnKind::Primary => [
            CONTROL_PRIMARY_BACKGROUND_DEFAULT,
            CONTROL_PRIMARY_TEXT_DEFAULT,
            CONTROL_PRIMARY_BORDER_DEFAULT,
            CONTROL_PRIMARY_BACKGROUND_HOVER,
            CONTROL_PRIMARY_TEXT_HOVER,
            CONTROL_PRIMARY_BORDER_HOVER,
            CONTROL_PRIMARY_BACKGROUND_ACTIVE,
            CONTROL_PRIMARY_TEXT_ACTIVE,
            CONTROL_PRIMARY_BORDER_ACTIVE,
        ],
        BtnKind::Critical => [
            CONTROL_CRITICAL_PRIMARY_BACKGROUND_DEFAULT,
            CONTROL_CRITICAL_PRIMARY_TEXT_DEFAULT,
            CONTROL_CRITICAL_PRIMARY_BORDER_DEFAULT,
            CONTROL_CRITICAL_PRIMARY_BACKGROUND_HOVER,
            CONTROL_CRITICAL_PRIMARY_TEXT_HOVER,
            CONTROL_CRITICAL_PRIMARY_BORDER_HOVER,
            CONTROL_CRITICAL_PRIMARY_BACKGROUND_ACTIVE,
            CONTROL_CRITICAL_PRIMARY_TEXT_ACTIVE,
            CONTROL_CRITICAL_PRIMARY_BORDER_ACTIVE,
        ],
    };
    div()
        .id(id.into())
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .h(z(40.))
        .min_w(z(100.)) // --__button-min-width (md)
        .px(z(16.))
        .rounded(z(8.))
        .border_1()
        .border_color(c(tok(border)))
        .bg(c(tok(bg)))
        // button_F7qvhY transition is 50ms ease-in: close enough to instant.
        .hover(move |s| s.bg(c(tok(hbg))).text_color(c(tok(hfg))).border_color(c(tok(hborder))))
        .active(move |s| s.bg(c(tok(abg))).text_color(c(tok(afg))).border_color(c(tok(aborder))))
        .cursor_pointer()
        .text_size(z(16.))
        .line_height(z(20.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(c(tok(fg)))
        .whitespace_nowrap()
        .child(label.into())
}

/// `button_F7qvhY` with a leading 20px `icon_F7qvhY` (buttonChildren_ gap 4),
/// e.g. vanilla-settings-10 #2743 "+ Add a keybind".
pub fn button_icon(id: impl Into<ElementId>, icon: &'static str, label: impl Into<SharedString>, kind: BtnKind) -> gpui::Stateful<Div> {
    let fg = match kind {
        BtnKind::Secondary => idx::CONTROL_SECONDARY_TEXT_DEFAULT,
        BtnKind::Primary => idx::CONTROL_PRIMARY_TEXT_DEFAULT,
        BtnKind::Critical => idx::CONTROL_CRITICAL_PRIMARY_TEXT_DEFAULT,
    };
    let label: SharedString = label.into();
    let b = button(id, SharedString::default(), kind);
    b.gap(z(4.)).child(gpui::svg().path(crate::icons::path(icon)).flex_none().size(z(20.)).text_color(c(tok(fg)))).child(label)
}

/// `sm_F7qvhY` button (vanilla-settings-10 #2772): 32px, padding 3 11 inside
/// the 1px border, body-sm/medium 14/18.
pub fn button_sm(id: impl Into<ElementId>, label: impl Into<SharedString>, kind: BtnKind) -> gpui::Stateful<Div> {
    button(id, label, kind).h(z(32.)).min_w(z(60.)).px(z(12.)).text_size(z(14.)).line_height(z(18.))
}

/// Select field (selectField_vWC4bk): 40px, 1px --input-border-default,
/// radius 8, --input-background-default; value body-md 16/20 at 4+8 inset;
/// 20px chevron in a 36px button on the right.
pub fn select_field(id: impl Into<ElementId>, value: impl Into<SharedString>) -> gpui::Stateful<Div> {
    div()
        .id(id.into())
        .h(z(40.))
        .pl(z(12.))
        .flex()
        .items_center()
        .rounded(z(8.))
        .border_1()
        .border_color(c(tok(idx::INPUT_BORDER_DEFAULT)))
        .hover(|s| s.border_color(c(tok(idx::INPUT_BORDER_HOVER))))
        .bg(c(tok(idx::INPUT_BACKGROUND_DEFAULT)))
        .cursor_pointer()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_size(z(16.))
                .line_height(z(20.))
                .text_color(c(tok(idx::TEXT_STRONG)))
                .child(value.into()),
        )
        .child(
            div().w(z(36.)).h_full().flex().items_center().justify_center().pr(z(4.)).child(
                gpui::svg().path("icons/chevron.svg").size(z(20.)).text_color(c(tok(idx::TEXT_STRONG))),
            ),
        )
}

/// baseControlItem_FiiQ6I: row with title (heading-sm/medium 16/20
/// --text-strong), optional description (body-sm 14/18 --text-subtle, gap 4)
/// and a trailing control; padding 12, margin-inline -12, radius --radius-lg,
/// gap 12, min-height 48.
pub fn control_item(title: impl Into<SharedString>, desc: Option<SharedString>, trailing: Option<gpui::AnyElement>) -> Div {
    div()
        .flex()
        .gap(z(12.))
        .mx(z(-12.))
        .p(z(12.))
        .min_h(z(48.))
        .rounded(z(16.))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .justify_center()
                .gap(z(4.))
                .child(
                    div()
                        .text_size(z(16.))
                        .line_height(z(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(c(tok(idx::TEXT_STRONG)))
                        .child(title.into()),
                )
                .children(desc.filter(|d| !d.is_empty()).map(|d| {
                    div().text_size(z(14.)).line_height(z(18.)).text_color(c(tok(idx::TEXT_SUBTLE))).child(d)
                })),
        )
        .children(trailing.map(|t| div().flex_none().flex().items_center().justify_center().child(t)))
}

/// Settings category heading (heading-xl/medium 24/500/32 --text-strong) with
/// its 24px bottom padding.
pub fn section_title(t: impl Into<SharedString>) -> Div {
    div()
        .pb(z(24.))
        .text_size(z(24.))
        .line_height(z(32.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(c(tok(idx::TEXT_STRONG)))
        .child(t.into())
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
