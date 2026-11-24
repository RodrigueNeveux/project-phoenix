//! Interactive states shared by Discord's chat-input buttons (button_hfM7yP):
//! rest `--interactive-text-default`; hover `--interactive-text-hover` on a
//! `--interactive-background-selected` 8px tile; pressed `--interactive-text-active`.
//! Audit: docs/parity/states.md.

use crate::theme::{c, generated::idx, tok, z};
use crate::ui::Icon;
use gpui::{AnimationExt, Div, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled, div, svg};
use gpui::prelude::FluentBuilder as _;

/// 32px composer button (attach, gift, GIF, sticker, emoji, apps), 20px glyph
/// (`--chat-input-icon-size`). The glyph follows the button's hover/active state.
pub fn composer_button(id: &'static str, i: Icon) -> gpui::Stateful<Div> {
    composer_button_ext(id, i, 20., true)
}

/// `glyph` px icon; `hover_bg` false = `noHover_hfM7yP` (gift button: no hover fill).
pub fn composer_button_ext(id: &'static str, i: Icon, glyph: f32, hover_bg: bool) -> gpui::Stateful<Div> {
    let k = key(("composer-btn", id));
    track(div().id(id), k)
        .relative()
        // button_hfM7yP:not(.noHover) transition-duration .2s (ease)
        .when(hover_bg, |d| d.child(fade_layer(k, gpui::transparent_black(), c(tok(idx::INTERACTIVE_BACKGROUND_SELECTED)), 200, 8.)))
        .group(id)
        .size(z(32.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(z(8.)) // button_hfM7yP:not(.noHover) border-radius 8px
        .cursor_pointer()
        .child(
            svg()
                .path(i.path())
                .size(z(glyph))
                .flex_none()
                .text_color(c(tok(idx::INTERACTIVE_TEXT_DEFAULT))) // button_hfM7yP color
                .group_hover(id, |s| s.text_color(c(tok(idx::INTERACTIVE_TEXT_HOVER)))) // :hover color
                .id("glyph")
                .group_active(id, |s| s.text_color(c(tok(idx::INTERACTIVE_TEXT_ACTIVE)))), // :active color
        )
}

thread_local! {
    /// Hover state of the elements that fade between colours, by key, with when it last changed.
    static HOVER: std::cell::RefCell<std::collections::HashMap<u64, (bool, std::time::Instant)>> = Default::default();
}

/// Stable key for [`track`] / [`fade_bg`] from any hashable id.
pub fn key(id: impl std::hash::Hash) -> u64 {
    use std::hash::Hasher;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut h);
    h.finish()
}

/// Hover state tracked under `key` and when it last changed.
pub fn state(key: u64) -> Option<(bool, std::time::Instant)> {
    HOVER.with(|h| h.borrow().get(&key).copied())
}

/// CSS `cubic-bezier(x1, y1, x2, y2)` timing function (Newton then bisection on x).
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> impl Fn(f32) -> f32 + Copy + 'static {
    move |t: f32| {
        let t = t.clamp(0., 1.);
        let bez = |p1: f32, p2: f32, s: f32| 3. * (1. - s) * (1. - s) * s * p1 + 3. * (1. - s) * s * s * p2 + s * s * s;
        let dx = |s: f32| 3. * (1. - s) * (1. - s) * x1 + 6. * (1. - s) * s * (x2 - x1) + 3. * s * s * (1. - x2);
        let mut s = t;
        for _ in 0..8 {
            let d = dx(s);
            if d.abs() < 1e-6 {
                break;
            }
            s = (s - (bez(x1, x2, s) - t) / d).clamp(0., 1.);
        }
        let (mut lo, mut hi) = (0f32, 1f32);
        if (bez(x1, x2, s) - t).abs() > 1e-4 {
            for _ in 0..30 {
                s = (lo + hi) / 2.;
                if bez(x1, x2, s) < t { lo = s } else { hi = s }
            }
        }
        bez(y1, y2, s)
    }
}

/// Whether the element tracked under `key` is hovered.
pub fn hovered(key: u64) -> bool {
    HOVER.with(|h| h.borrow().get(&key).is_some_and(|s| s.0))
}

/// Record hover enter/leave of `el` under `key` and redraw, so [`fade_bg`] can start its transition.
pub fn track(el: gpui::Stateful<Div>, key: u64) -> gpui::Stateful<Div> {
    el.on_hover(move |over, window, _| {
        let changed = HOVER.with(|h| {
            let mut h = h.borrow_mut();
            let prev = h.get(&key).is_some_and(|s| s.0);
            h.insert(key, (*over, std::time::Instant::now()));
            prev != *over
        });
        if changed {
            window.refresh();
        }
    })
}

/// Linear blend of two colours in RGB space (CSS transitions interpolate sRGB).
pub fn mix(a: gpui::Hsla, b: gpui::Hsla, t: f32) -> gpui::Hsla {
    let (a, b): (gpui::Rgba, gpui::Rgba) = (a.into(), b.into());
    let l = |x: f32, y: f32| x + (y - x) * t;
    gpui::Rgba { r: l(a.r, b.r), g: l(a.g, b.g), b: l(a.b, b.b), a: l(a.a, b.a) }.into()
}

/// CSS `background-color <ms> ease-out` between `rest` and `hover`, as an absolutely
/// positioned layer to put FIRST in a `.relative()` parent tracked with [`track`] under `key`
/// (the parent must not set its own hover background). Only a state change younger than
/// `ms` animates, so first paints and remounts don't flash.
pub fn fade_layer(key: u64, rest: gpui::Hsla, hover: gpui::Hsla, ms: u64, radius: f32) -> gpui::AnyElement {
    fade_layer_pressed(key, rest, hover, None, ms, radius)
}

/// [`fade_layer`] that also shows `pressed.1` while the group `pressed.0` is held down.
pub fn fade_layer_pressed(
    key: u64,
    rest: gpui::Hsla,
    hover: gpui::Hsla,
    pressed: Option<(&'static str, gpui::Hsla)>,
    ms: u64,
    radius: f32,
) -> gpui::AnyElement {
    use gpui::{AnimationExt, IntoElement};
    let state = HOVER.with(|h| h.borrow().get(&key).copied());
    let on = state.is_some_and(|s| s.0);
    let (from, to) = if on { (rest, hover) } else { (hover, rest) };
    let mut layer = div().absolute().top_0().left_0().size_full().rounded(z(radius)).id("fade-layer");
    if let Some((group, col)) = pressed {
        layer = layer.group_active(group, move |s| s.bg(col));
    }
    match state {
        Some((_, at)) if at.elapsed().as_millis() < ms as u128 => layer
            .with_animation(
                gpui::ElementId::Name(format!("fade-{key}-{on}-{at:?}").into()),
                gpui::Animation::new(std::time::Duration::from_millis(ms)).with_easing(ease_out),
                move |d, t| d.bg(mix(from, to, t)),
            )
            .into_any_element(),
        _ => layer.bg(to).into_any_element(),
    }
}

/// CSS `ease-out` (cubic-bezier(0, 0, .58, 1)), approximated as 1-(1-t)^2.
pub fn ease_out(t: f32) -> f32 {
    1. - (1. - t) * (1. - t)
}

/// Tooltip card under a button (tooltip_ZDdyhE `data-position="bottom"`): 5px caret
/// pointing up whose tip sits 6px below the button, card 11px below it, centred.
/// Live header tooltip (ref-cache tooltip-header-appear.json): shows ~50ms after
/// hover (no delay), body-sm/medium 14/18, padding 8 12.
pub fn below_tip(text: gpui::SharedString, button_w: f32, button_h: f32) -> gpui::AnyElement {
    use gpui::{AnchoredPositionMode, FontWeight, IntoElement, anchored, deferred, point};
    let bg = c(tok(idx::BACKGROUND_SURFACE_HIGH));
    let mut caret = div().flex().flex_col().items_center();
    for i in 0..5 {
        caret = caret.child(div().w(z(1. + 2. * i as f32)).h(z(1.)).bg(bg));
    }
    const SPAN: f32 = 240.;
    let card = div()
        .max_w(z(200.)) // tooltip_ZDdyhE max-width
        .px(z(12.)) // --space-12
        .py(z(8.)) // --space-8
        .rounded(z(8.)) // --radius-sm
        .bg(bg)
        .border_1()
        .border_color(c(tok(idx::BORDER_SUBTLE))) // inset 0 0 0 1px --border-subtle
        .shadow(vec![gpui::BoxShadow {
            // --shadow-high
            color: gpui::hsla(0., 0., 0., 0.24),
            offset: point(z(0.), z(12.)),
            blur_radius: z(24.),
            spread_radius: z(0.),
            inset: false,
        }])
        .text_size(z(14.))
        .line_height(z(18.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(c(tok(idx::TEXT_DEFAULT)))
        .whitespace_nowrap()
        .child(text)
        .with_animation("tip-zoom", tip_zoom(), zoom_card);
    let tip = deferred(
        anchored().position_mode(AnchoredPositionMode::Local).position(point(z(button_w / 2. - SPAN / 2.), z(button_h + 6.))).child(
            div().w(z(SPAN)).flex().flex_col().items_center().child(caret).child(card),
        ),
    )
    .with_priority(2);
    // Zero-size anchor at the button's top-left, out of the flex flow.
    div().absolute().top_0().left_0().child(tip).into_any_element()
}

/// Channel header toolbar button (iconWrapper_ybUK7Z .clickable_): 32px, no background,
/// 20px glyph `--icon-muted`, `--icon-subtle` on hover, `--icon-strong` when selected,
/// with its label as a tooltip below.
pub fn header_button(id: &'static str, path: gpui::SharedString, selected: bool, label: gpui::SharedString) -> gpui::Stateful<Div> {
    let k = key(("hdr-btn", id));
    let rest = c(tok(if selected { idx::ICON_STRONG } else { idx::ICON_MUTED }));
    let hover = c(tok(if selected { idx::ICON_STRONG } else { idx::ICON_SUBTLE }));
    track(div().id(id), k)
        .group(id)
        .relative()
        .size(z(32.)) // --space-32
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .child(
            svg()
                .path(path)
                .size(z(20.)) // --chat-input-icon-size
                .flex_none()
                .text_color(rest)
                .group_hover(id, move |s| s.text_color(hover)),
        )
        .children(hovered(k).then(|| below_tip(label, 32., 32.)))
}

/// [`fade_layer`] for elements that already know their hover state (e.g. the guild rail's
/// own tracking): a change of `on` seen at render time starts the transition.
pub fn fade_to(key: u64, on: bool, rest: gpui::Hsla, hover: gpui::Hsla, ms: u64, radius: f32) -> gpui::AnyElement {
    HOVER.with(|h| {
        let mut h = h.borrow_mut();
        match h.get(&key) {
            Some(s) if s.0 == on => {}
            // First sight: no transition.
            None => {
                h.insert(key, (on, std::time::Instant::now() - std::time::Duration::from_secs(1)));
            }
            Some(_) => {
                h.insert(key, (on, std::time::Instant::now()));
            }
        }
    });
    fade_layer(key, rest, hover, ms, radius)
}

/// Tooltip enter zoom: the client's spring takes the card from ~0.95 to 1 in ~100ms
/// (ref-cache tooltip-{header,rail}-appear.json: 43→45 / 140→148px wide), opacity 1.
pub fn tip_zoom() -> gpui::Animation {
    gpui::Animation::new(crate::ui::settings::effects::motion(100)).with_easing(ease_out)
}

/// gpui has no CSS transform on divs, so the zoom scales the card's own metrics
/// (padding, radius, 14/18 text) by 0.95 → 1, which is what `scale()` looks like for a text card.
pub fn zoom_card(d: Div, t: f32) -> Div {
    zoom_card_sized(d, t, 14., 18.)
}

/// [`zoom_card`] for cards with another text size / line height (rail tooltip: 16/20).
pub fn zoom_card_sized(d: Div, t: f32, font: f32, line: f32) -> Div {
    let k = 0.95 + 0.05 * t;
    d.px(z(12. * k)).py(z(8. * k)).rounded(z(8. * k)).text_size(z(font * k)).line_height(z(line * k))
}

/// Drop the remembered state of `key`, so the next [`fade_to`] with `on = true` animates
/// from `rest` (used for one-way transitions such as a settings row becoming selected).
pub fn forget(key: u64) {
    HOVER.with(|h| {
        let mut h = h.borrow_mut();
        if h.get(&key).is_none_or(|s| s.0) {
            h.insert(key, (false, std::time::Instant::now() - std::time::Duration::from_secs(1)));
        }
    });
}
