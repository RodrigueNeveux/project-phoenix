use std::cell::RefCell;

use gpui::{AnyElement, Bounds, Context, DispatchPhase, IntoElement, MouseButton, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, SharedString, Styled, canvas, div, prelude::*};
use phoenix_api::rest::guild_server::{ProfilePatch, ServerCall};

use super::{sf, t, text, write};
use crate::{
    app::PhoenixApp,
    model::Id,
    theme::{generated::idx, z},
};

pub const PICKER_W: f32 = 150.;
pub const SATURATION_H: f32 = 150.;
pub const HUE_H: f32 = 8.;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsv {
    pub h: f32,
    pub s: f32,
    pub v: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Saturation,
    Hue,
}

#[derive(Default)]
struct Popout {
    guild: Option<Id>,
    open: bool,
    colors: [Option<Hsv>; 2],
    dirty: bool,
    drag: Option<(usize, Part)>,
    closed_at: Option<std::time::Instant>,
    bounds: [[Option<Bounds<Pixels>>; 2]; 2],
}

thread_local! {
    static POP: RefCell<Popout> = RefCell::new(Popout::default());
}

pub fn hex_to_rgb(s: &str) -> Option<[u8; 3]> {
    let h = s.trim().trim_start_matches('#');
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

pub fn rgb_to_hex(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

pub fn rgb_to_hsv(c: [u8; 3]) -> Hsv {
    let [r, g, b] = c.map(|x| x as f32 / 255.);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0. {
        0.
    } else if max == r {
        60. * (((g - b) / d).rem_euclid(6.))
    } else if max == g {
        60. * ((b - r) / d + 2.)
    } else {
        60. * ((r - g) / d + 4.)
    };
    Hsv { h, s: if max == 0. { 0. } else { d / max }, v: max }
}

pub fn hsv_to_rgb(c: Hsv) -> [u8; 3] {
    let h = c.h.rem_euclid(360.) / 60.;
    let ch = c.v * c.s;
    let x = ch * (1. - (h % 2. - 1.).abs());
    let (r, g, b) = match h as u32 {
        0 => (ch, x, 0.),
        1 => (x, ch, 0.),
        2 => (0., ch, x),
        3 => (0., x, ch),
        4 => (x, 0., ch),
        _ => (ch, 0., x),
    };
    let m = c.v - ch;
    [r, g, b].map(|v| ((v + m) * 255.).round().clamp(0., 255.) as u8)
}

fn rgba(c: [u8; 3]) -> gpui::Hsla {
    gpui::rgb(((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32).into()
}

pub fn is_open(guild: Id) -> bool {
    POP.with_borrow(|p| p.open && p.guild == Some(guild))
}

pub fn toggle(guild: Id, primary: &str, secondary: &str) {
    POP.with_borrow_mut(|p| {
        if p.open && p.guild == Some(guild) {
            p.open = false;
            return;
        }
        if p.closed_at.is_some_and(|t| t.elapsed() < std::time::Duration::from_millis(250)) {
            p.closed_at = None;
            return;
        }
        *p = Popout { guild: Some(guild), open: true, colors: [hex_to_rgb(primary).map(rgb_to_hsv), hex_to_rgb(secondary).map(rgb_to_hsv)], ..Default::default() };
    });
}

fn close(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) {
    let (guild, dirty, colors) = POP.with_borrow_mut(|p| {
        let out = (p.guild, p.dirty, p.colors);
        p.open = false;
        p.dirty = false;
        p.drag = None;
        p.closed_at = Some(std::time::Instant::now());
        out
    });
    if let (Some(g), true) = (guild, dirty) {
        commit(app, g, colors, cx);
    }
}

fn commit(app: &mut PhoenixApp, guild: Id, colors: [Option<Hsv>; 2], cx: &mut Context<PhoenixApp>) {
    let hex = |c: Option<Hsv>| c.map(|c| rgb_to_hex(hsv_to_rgb(c)));
    let patch = ProfilePatch { badge_color_primary: hex(colors[0]).map(Some), badge_color_secondary: hex(colors[1]).map(Some), ..Default::default() };
    write(app, ServerCall::SetProfile { guild: sf(guild), patch }, None, cx);
}

fn pick(part_frac: (f32, f32), part: Part, cur: Hsv) -> Hsv {
    let (fx, fy) = (part_frac.0.clamp(0., 1.), part_frac.1.clamp(0., 1.));
    match part {
        Part::Saturation => Hsv { s: fx, v: 1. - fy, ..cur },
        Part::Hue => Hsv { h: fx * 360., ..cur },
    }
}

fn apply(i: usize, part: Part, pos: gpui::Point<Pixels>) -> bool {
    POP.with_borrow_mut(|p| {
        let Some(b) = p.bounds[i][part as usize] else { return false };
        let w = f32::from(b.size.width).max(1.);
        let h = f32::from(b.size.height).max(1.);
        let frac = ((f32::from(pos.x) - f32::from(b.origin.x)) / w, (f32::from(pos.y) - f32::from(b.origin.y)) / h);
        let cur = p.colors[i].unwrap_or(Hsv { h: 0., s: 0., v: 0. });
        p.colors[i] = Some(pick(frac, part, cur));
        p.dirty = true;
        true
    })
}

fn probe(i: usize, part: Part) -> impl IntoElement {
    canvas(move |b: Bounds<Pixels>, _, _| POP.with_borrow_mut(|p| p.bounds[i][part as usize] = Some(b)), |_, _, _, _| {}).absolute().top_0().left_0().size_full()
}

fn area(id: (&'static str, usize), i: usize, part: Part) -> gpui::Stateful<gpui::Div> {
    div().id(id).relative().cursor_crosshair().child(probe(i, part)).on_mouse_down(MouseButton::Left, move |ev, window, cx| {
        POP.with_borrow_mut(|p| p.drag = Some((i, part)));
        apply(i, part, ev.position);
        window.refresh();
        cx.stop_propagation();
    })
}

fn saturation(i: usize, c: Hsv) -> AnyElement {
    let base = rgba(hsv_to_rgb(Hsv { h: c.h, s: 1., v: 1. }));
    let white = gpui::linear_gradient(90., gpui::linear_color_stop(gpui::white(), 0.), gpui::linear_color_stop(gpui::white().opacity(0.), 1.));
    let black = gpui::linear_gradient(180., gpui::linear_color_stop(gpui::black().opacity(0.), 0.), gpui::linear_color_stop(gpui::black(), 1.));
    area(("tag-color-sat", i), i, Part::Saturation)
        .w_full()
        .h(z(SATURATION_H))
        .rounded(z(3.))
        .overflow_hidden()
        .bg(base)
        .child(div().absolute().inset_0().bg(white))
        .child(div().absolute().inset_0().bg(black))
        .child(
            div()
                .absolute()
                .left(gpui::relative(c.s))
                .top(gpui::relative(1. - c.v))
                .ml(z(-2.))
                .mt(z(-2.))
                .size(z(4.))
                .rounded_full()
                .border(z(1.5))
                .border_color(gpui::white())
                .shadow(vec![gpui::BoxShadow { color: gpui::black().opacity(0.3), offset: gpui::point(z(0.), z(0.)), blur_radius: z(1.), spread_radius: z(1.5), inset: false }]),
        )
        .into_any_element()
}

fn hue(i: usize, c: Hsv) -> AnyElement {
    let stops = [0., 60., 120., 180., 240., 300., 360.];
    let mut bar = div().absolute().inset_0().flex().rounded(z(3.)).overflow_hidden();
    for w in stops.windows(2) {
        let a = rgba(hsv_to_rgb(Hsv { h: w[0], s: 1., v: 1. }));
        let b = rgba(hsv_to_rgb(Hsv { h: w[1] % 360., s: 1., v: 1. }));
        bar = bar.child(div().flex_1().h_full().bg(gpui::linear_gradient(90., gpui::linear_color_stop(a, 0.), gpui::linear_color_stop(b, 1.))));
    }
    area(("tag-color-hue", i), i, Part::Hue)
        .w_full()
        .h(z(HUE_H))
        .my(z(8.))
        .child(bar)
        .child(
            div()
                .absolute()
                .left(gpui::relative(c.h / 360.))
                .ml(z(-4.))
                .top(z(-3.))
                .w(z(8.))
                .h(z(16.))
                .rounded(z(3.))
                .bg(gpui::white())
                .cursor_ew_resize()
                .shadow(vec![gpui::BoxShadow { color: gpui::black().opacity(0.6), offset: gpui::point(z(0.), z(0.)), blur_radius: z(2.), spread_radius: z(0.), inset: false }]),
        )
        .into_any_element()
}

fn hex_field(c: Hsv) -> AnyElement {
    let hex = rgb_to_hex(hsv_to_rgb(c));
    div()
        .h(z(40.))
        .px(z(12.))
        .rounded(z(8.))
        .border_1()
        .border_color(t(idx::INPUT_BORDER_DEFAULT))
        .bg(t(idx::INPUT_BACKGROUND_DEFAULT))
        .flex()
        .items_center()
        .gap(z(4.))
        .text_size(z(16.))
        .child(div().text_color(t(idx::TEXT_MUTED)).child("#"))
        .child(div().flex_1().min_w_0().text_color(t(idx::TEXT_DEFAULT)).child(SharedString::from(hex[1..].to_owned())))
        .child(super::icon("server/dropper", 20., idx::INTERACTIVE_ICON_DEFAULT))
        .into_any_element()
}

fn column(i: usize, c: Hsv, label: SharedString) -> AnyElement {
    div()
        .w(z(PICKER_W))
        .flex()
        .flex_col()
        .items_center()
        .gap(z(8.))
        .child(div().w_full().h(z(50.)).rounded(z(3.)).bg(rgba(hsv_to_rgb(c))))
        .child(text(12., 16., gpui::FontWeight::MEDIUM, idx::TEXT_DEFAULT, label))
        .child(div().w_full().rounded(z(4.)).bg(t(idx::BACKGROUND_BASE_LOW)).flex().flex_col().gap(z(16.)).child(div().flex().flex_col().child(saturation(i, c)).child(hue(i, c))).child(hex_field(c)))
        .into_any_element()
}

pub fn render(guild: Id, badge: u32, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    if !is_open(guild) {
        return None;
    }
    let colors = POP.with_borrow(|p| p.colors);
    let two = crate::icons::clan_badges::colors(badge) >= 2;
    let fallback = |i: usize| colors[i].unwrap_or_else(|| rgb_to_hsv(hex_to_rgb(if i == 0 { crate::icons::clan_badges::PALETTES[0].0 } else { crate::icons::clan_badges::PALETTES[0].1 }).unwrap_or([0; 3])));
    let weak = cx.entity().downgrade();
    let tracker = canvas(
        |_, _, _| {},
        move |_, _, window, _| {
            if POP.with_borrow(|p| p.drag.is_none()) {
                return;
            }
            let w1 = weak.clone();
            window.on_mouse_event(move |ev: &MouseMoveEvent, phase, _, cx| {
                let Some((i, part)) = POP.with_borrow(|p| p.drag) else { return };
                if phase == DispatchPhase::Bubble && apply(i, part, ev.position) {
                    let _ = w1.update(cx, |_, cx| cx.notify());
                }
            });
            window.on_mouse_event(move |_: &MouseUpEvent, phase, _, _| {
                if phase == DispatchPhase::Bubble {
                    POP.with_borrow_mut(|p| p.drag = None);
                }
            });
        },
    )
    .absolute()
    .size_full();
    let pickers = div()
        .flex()
        .flex_row()
        .items_center()
        .justify_center()
        .gap(z(16.))
        .child(column(0, fallback(0), crate::t!("gs2.tag_color_primary")))
        .when(two, |d| d.child(column(1, fallback(1), crate::t!("gs2.tag_color_secondary"))));
    let pop = div()
        .id("tag-color-popout")
        .occlude()
        .relative()
        .p(z(12.))
        .rounded(z(8.))
        .border_1()
        .border_color(t(idx::BORDER_STRONG))
        .bg(t(idx::BACKGROUND_SURFACE_HIGH))
        .flex()
        .flex_col()
        .gap(z(8.))
        .child(tracker)
        .child(text(14., 18., gpui::FontWeight::MEDIUM, idx::TEXT_DEFAULT, crate::t!("gs2.tag_color_custom")))
        .child(pickers)
        .on_mouse_down_out(cx.listener(|a, _, _, cx| {
            close(a, cx);
            cx.notify();
        }));
    let width = 24. + 2. + if two { 2. * PICKER_W + 16. } else { PICKER_W };
    Some(
        gpui::deferred(
            gpui::anchored()
                .position_mode(gpui::AnchoredPositionMode::Local)
                .anchor(gpui::Corner::BottomLeft)
                .position(gpui::point(z(32. - width / 2.), z(-8.)))
                .child(pop),
        )
        .with_priority(2)
        .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_and_hsv_round_trip() {
        for hex in ["#ff1c90", "#ff7fc0", "#222222", "#cccccc", "#00ff00", "#123456"] {
            let rgb = hex_to_rgb(hex).unwrap();
            assert_eq!(rgb_to_hex(hsv_to_rgb(rgb_to_hsv(rgb))), hex);
        }
        assert_eq!(hex_to_rgb("12345"), None);
        assert_eq!(hex_to_rgb("#zzzzzz"), None);
    }

    #[test]
    fn saturation_and_hue_follow_the_pointer() {
        let c = Hsv { h: 200., s: 0.5, v: 0.5 };
        assert_eq!(pick((1., 0.), Part::Saturation, c), Hsv { h: 200., s: 1., v: 1. });
        assert_eq!(pick((-1., 2.), Part::Saturation, c), Hsv { h: 200., s: 0., v: 0. });
        assert_eq!(pick((0.5, 0.3), Part::Hue, c), Hsv { h: 180., s: 0.5, v: 0.5 });
    }

    #[test]
    fn opening_twice_closes() {
        toggle(Id(9), "#ff1c90", "#ff7fc0");
        assert!(is_open(Id(9)));
        assert_eq!(POP.with_borrow(|p| p.colors[0].map(|c| rgb_to_hex(hsv_to_rgb(c)))), Some("#ff1c90".into()));
        toggle(Id(9), "#ff1c90", "#ff7fc0");
        assert!(!is_open(Id(9)));
    }
}
