use std::time::{Duration, Instant};

use gpui::{
    App, Bounds, Element, ElementId, GlobalElementId, GradientScroll, Hsla, InspectorElementId, IntoElement, LayoutId,
    Pixels, SharedString, Style, TextAlign, TextGradient, Window, point, px, relative, size,
};

pub const ROLE_SCROLL: Duration = Duration::from_millis(1500);
pub const PRISM_SCROLL: Duration = Duration::from_millis(2000);
pub const GLOW_BLUR: f32 = 4.;
pub const GLOW_OPACITY: f32 = 0.7;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Motion {
    #[default]
    Still,
    Once,
    Loop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MotionEnv {
    pub reduced: bool,
    pub focused: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NameFill {
    Solid(u32),
    Role { stops: Vec<(u32, f32)>, period: Pixels },
    Diagonal(Vec<(u32, f32)>),
    Prism { stops: Vec<(u32, f32)>, chars: usize, min: Pixels },
}

impl NameFill {
    pub fn first(&self) -> u32 {
        match self {
            NameFill::Solid(c) => *c,
            NameFill::Role { stops, .. } | NameFill::Diagonal(stops) | NameFill::Prism { stops, .. } => {
                stops.first().map_or(0, |s| s.0)
            }
        }
    }

    pub fn gradient(&self, origin: gpui::Point<Pixels>, width: Pixels, line_height: Pixels, ch: Pixels) -> Option<TextGradient> {
        let tile = |w: Pixels| Bounds::new(origin, size(w, line_height));
        let (base, stops) = match self {
            NameFill::Solid(_) => return None,
            NameFill::Role { stops, period } => (TextGradient::new(90., tile(*period)).repeat_x(true), stops),
            NameFill::Diagonal(stops) => (TextGradient::to_corner(true, true, tile(width)), stops),
            NameFill::Prism { stops, chars, min } => {
                let cycle = (ch * *chars as f32).max(*min);
                (TextGradient::new(90., tile(cycle)).repeat_x(true), stops)
            }
        };
        if stops.len() < 2 {
            return None;
        }
        Some(stops.iter().fold(base, |g, &(c, p)| g.stop(gpui::rgb(c), p)))
    }

    pub fn scroll(&self, motion: Motion, env: MotionEnv, start: Instant, tile: Pixels) -> Option<GradientScroll> {
        if motion == Motion::Still || env.reduced || tile <= px(0.) {
            return None;
        }
        let tile = f32::from(tile);
        match self {
            NameFill::Role { .. } if env.focused => Some(GradientScroll::new(tile / ROLE_SCROLL.as_secs_f32(), start, None)),
            NameFill::Prism { .. } => {
                let duration = (motion == Motion::Once).then_some(PRISM_SCROLL);
                Some(GradientScroll::new(-tile / PRISM_SCROLL.as_secs_f32(), start, duration))
            }
            _ => None,
        }
    }

    pub fn glows(&self, motion: Motion) -> bool {
        motion != Motion::Still && matches!(self, NameFill::Role { .. })
    }
}

pub struct NameText {
    text: SharedString,
    spacing: Pixels,
    fill: Option<NameFill>,
    motion: Motion,
}

impl NameText {
    pub fn new(text: SharedString, spacing: Pixels, fill: Option<NameFill>) -> Self {
        Self { text, spacing, fill, motion: Motion::Still }
    }

    pub fn motion(mut self, motion: Motion) -> Self {
        self.motion = motion;
        self
    }
}

pub struct Layout {
    chars: Vec<(String, Pixels)>,
    ellipsis: Pixels,
    line_height: Pixels,
    ch: Pixels,
    underline: Option<(Pixels, Pixels, Pixels, gpui::UnderlineStyle)>,
}

impl NameText {
    fn shape(&self, s: &str, window: &mut Window, color: Hsla) -> gpui::ShapedLine {
        let style = window.text_style();
        let mut run = style.to_run(s.len());
        run.color = color;
        run.underline = None;
        let size = style.font_size.to_pixels(window.rem_size());
        window.text_system().shape_line(SharedString::from(s.to_owned()), size, &[run], None)
    }
}

impl Element for NameText {
    type RequestLayoutState = Layout;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        (self.motion != Motion::Still && self.fill.is_some()).then(|| ElementId::Name("name-motion".into()))
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Layout) {
        let color = window.text_style().color;
        let line_height = window.text_style().line_height_in_pixels(window.rem_size());
        let full = self.shape(&self.text, window, color);
        let text = self.text.clone();
        let starts: Vec<(usize, &str)> = unicode_segmentation::UnicodeSegmentation::grapheme_indices(text.as_ref(), true).collect();
        let per: Vec<(String, Pixels)> = starts
            .iter()
            .enumerate()
            .map(|(i, (b, g))| {
                let x0 = full.x_for_index(*b);
                let x1 = starts.get(i + 1).map_or(full.width, |(nb, _)| full.x_for_index(*nb));
                ((*g).to_owned(), (x1 - x0).max(px(0.)))
            })
            .collect();
        let em = window.text_style().font_size.to_pixels(window.rem_size()) * 1.25;
        let per = per.into_iter().map(|(g, a)| if is_pictograph(&g) { (g, if a > em { em } else { a }) } else { (g, a) }).collect();
        let chars = merge_ligatures(per);
        let ellipsis = self.shape("\u{2026}", window, color).width;
        let ch = if matches!(self.fill, Some(NameFill::Prism { .. })) { self.shape("0", window, color).width } else { px(0.) };
        let underline = window.text_style().underline.map(|u| {
            let style = window.text_style();
            let size = style.font_size.to_pixels(window.rem_size());
            let ts = window.text_system();
            let font = ts.resolve_font(&style.font());
            (size, ts.ascent(font, size), ts.descent(font, size), u)
        });
        let natural = chars.iter().fold(px(0.), |w, (_, a)| w + *a + self.spacing);
        let mut style = Style::default();
        style.size.width = natural.into();
        style.size.height = line_height.into();
        style.min_size.width = px(0.).into();
        style.max_size.width = relative(1.).into();
        style.flex_shrink = 1.;
        let id = window.request_layout(style, [], cx);
        (id, Layout { chars, ellipsis, line_height, ch, underline })
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _layout: &mut Layout,
        _window: &mut Window,
        _cx: &mut App,
    ) {
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Layout,
        _prepaint: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let base = window.text_style().color;
        let natural = layout.chars.iter().fold(px(0.), |w, (_, a)| w + *a + self.spacing);
        let fits = natural <= bounds.size.width + px(0.5);
        let limit = if fits { natural } else { bounds.size.width - layout.ellipsis };
        let total = if fits { natural } else { bounds.size.width };
        let first: Option<Hsla> = self.fill.as_ref().map(|f| gpui::rgb(f.first()).into());
        let color = first.unwrap_or(base);
        let mut gradient = self.fill.as_ref().and_then(|f| f.gradient(bounds.origin, total, layout.line_height, layout.ch));
        let start = id.map(|id| window.with_element_state::<Instant, _>(id, |s, _| {
            let s = s.unwrap_or_else(Instant::now);
            (s, s)
        }));
        let mut glow = None;
        if let (Some(fill), Some(g), Some(start)) = (self.fill.as_ref(), gradient.as_mut(), start) {
            let env = MotionEnv { reduced: crate::ui::settings::effects::reduced_motion(), focused: window.is_window_active() };
            g.scroll = fill.scroll(self.motion, env, start, g.tile.size.width);
            if fill.glows(self.motion) {
                glow = Some((crate::theme::z(GLOW_BLUR), GLOW_OPACITY));
            }
        }
        let paint = |window: &mut Window, cx: &mut App, glyphs_only: bool| {
            let mut x = px(0.);
            for (s, adv) in &layout.chars {
                if !fits && x + *adv > limit {
                    break;
                }
                if !(glyphs_only && is_pictograph(s)) {
                    let line = self.shape(s, window, color);
                    let _ = line.paint(point(bounds.origin.x + x, bounds.origin.y), layout.line_height, TextAlign::Left, None, window, cx);
                }
                x += *adv + self.spacing;
            }
            if !fits {
                let line = self.shape("\u{2026}", window, color);
                let _ = line.paint(point(bounds.origin.x + x, bounds.origin.y), layout.line_height, TextAlign::Left, None, window, cx);
                x += layout.ellipsis + self.spacing;
            }
            x
        };
        let x = window.with_text_gradient(gradient.as_ref(), |window| {
            if glow.is_some() {
                window.with_text_glow(glow, |window| paint(window, cx, true));
            }
            paint(window, cx, false)
        });
        if let Some((size, ascent, descent, mut u)) = layout.underline {
            let (y, thickness) =
                gpui::underline_geometry(bounds.origin.y, layout.line_height, size, ascent, descent, window.scale_factor());
            u.color = u.color.or(first).or(Some(base));
            if !u.wavy {
                u.thickness = thickness;
            }
            let width = (x - self.spacing).max(px(0.));
            window.paint_underline(point(bounds.origin.x, y), width, &u);
        }
    }
}

fn is_pictograph(g: &str) -> bool {
    g.contains(['\u{FE0F}', '\u{200D}']) || g.chars().next().is_some_and(|c| c as u32 >= 0x1F000)
}

fn merge_ligatures(per: Vec<(String, Pixels)>) -> Vec<(String, Pixels)> {
    let mut out: Vec<(String, Pixels)> = Vec::with_capacity(per.len());
    for (g, adv) in per {
        match out.last_mut() {
            Some(last) if adv <= px(0.) || last.1 <= px(0.) => {
                last.0.push_str(&g);
                last.1 += adv;
            }
            _ => out.push((g, adv)),
        }
    }
    out
}

impl IntoElement for NameText {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ligatures_paint_as_one_cluster() {
        let v = |s: &str, a: f32| (s.to_owned(), px(a));
        assert_eq!(merge_ligatures(vec![v("f", 9.), v("l", 0.), v("a", 7.)]), vec![v("fl", 9.), v("a", 7.)]);
        assert_eq!(merge_ligatures(vec![v("f", 0.), v("l", 9.), v("a", 7.)]), vec![v("fl", 9.), v("a", 7.)]);
        assert_eq!(merge_ligatures(vec![v("a", 7.), v("b", 7.)]), vec![v("a", 7.), v("b", 7.)]);
    }

    #[test]
    fn pictographs() {
        assert!(is_pictograph("🌸"));
        assert!(is_pictograph("❤\u{FE0F}"));
        assert!(!is_pictograph("a"));
        assert!(!is_pictograph("☲"));
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn role_gradient_is_horizontal_and_repeats_every_period() {
        let fill = NameFill::Role { stops: vec![(0x9e6bff, 0.), (0x9fc1ff, 0.5), (0x9e6bff, 1.)], period: px(100.) };
        let g = fill.gradient(point(px(10.), px(4.)), px(250.), px(20.), px(0.)).unwrap();
        assert_eq!(g.angle, 90.);
        assert!(g.repeat_x);
        assert_eq!(g.tile, Bounds::new(point(px(10.), px(4.)), size(px(100.), px(20.))));
        assert!(close(g.position_at(point(px(60.), px(10.))), 0.5));
        assert!(close(g.position_at(point(px(160.), px(10.))), 0.5));
        let mid = g.color_at(point(px(60.), px(10.)));
        assert!(close(mid.r, 0x9f as f32 / 255.) && close(mid.g, 0xc1 as f32 / 255.));
    }

    #[test]
    fn display_name_gradient_runs_to_the_bottom_right_corner_of_the_box() {
        let fill = NameFill::Diagonal(vec![(0x666666, 0.1), (0xfffffe, 0.9)]);
        let g = fill.gradient(point(px(0.), px(0.)), px(35.), px(20.), px(0.)).unwrap();
        assert!(!g.repeat_x);
        assert!(close(g.angle, 150.2551));
        assert!(close(g.position_at(point(px(35.), px(0.))), 0.5));
        assert!(close(g.position_at(point(px(0.), px(20.))), 0.5));
        assert!(close(g.color_at(point(px(0.), px(0.))).r, 0x66 as f32 / 255.));
        assert!(close(g.color_at(point(px(35.), px(20.))).r, 1.));
    }

    #[test]
    fn prism_cycle_is_the_name_length_in_ch_with_a_38px_floor() {
        let fill = NameFill::Prism { stops: vec![(0xff0000, 0.), (0x0000ff, 0.5), (0xff0000, 1.)], chars: 4, min: px(38.) };
        let short = fill.gradient(point(px(0.), px(0.)), px(30.), px(20.), px(8.)).unwrap();
        assert_eq!(short.tile.size.width, px(38.));
        let long = NameFill::Prism { stops: vec![(0xff0000, 0.), (0x0000ff, 0.5), (0xff0000, 1.)], chars: 12, min: px(38.) };
        let g = long.gradient(point(px(0.), px(0.)), px(90.), px(20.), px(8.)).unwrap();
        assert_eq!(g.tile.size.width, px(96.));
        assert_eq!(g.angle, 90.);
        assert!(g.repeat_x);
    }

    const MOVING: MotionEnv = MotionEnv { reduced: false, focused: true };

    fn role() -> NameFill {
        NameFill::Role { stops: vec![(0x9e6bff, 0.), (0x9fc1ff, 0.5), (0x9e6bff, 1.)], period: px(100.) }
    }

    fn prism() -> NameFill {
        NameFill::Prism { stops: vec![(0xff0000, 0.), (0x0000ff, 0.5), (0xff0000, 1.)], chars: 12, min: px(38.) }
    }

    #[test]
    fn role_gradient_scrolls_100px_to_the_right_every_1_5s_like_the_client_keyframes() {
        let start = Instant::now();
        let scroll = role().scroll(Motion::Loop, MOVING, start, px(100.)).unwrap();
        assert_eq!(scroll.duration, None);
        let s = scroll.to_glyph_scroll(1.5);
        let t0 = gpui::gradient_clock(start);
        let period = 150.;
        assert!(close(s.displacement(t0, period), 0.));
        assert!(close(s.displacement(t0 + 0.375, period), 37.5));
        assert!(close(s.displacement(t0 + 0.75, period), 75.));
        assert!(close(s.displacement(t0 + 1.5, period), 0.));
        assert!(close(s.displacement(t0 + 2.25, period), 75.));
        assert!(s.running(t0 + 60.));
        let g = role().gradient(point(px(0.), px(0.)), px(250.), px(20.), px(0.)).unwrap().scroll(Some(scroll));
        let d = g.to_glyph_gradient(1.5, 1.);
        let moved = s.apply(d, t0 + 0.375);
        assert!(close(moved.position_at(37.5, 0.), 0.));
        assert!(close(moved.position_at(0., 0.), 0.75));
    }

    #[test]
    fn prism_scrolls_its_cycle_to_the_left_in_2s_once_or_forever() {
        let start = Instant::now();
        let t0 = gpui::gradient_clock(start);
        let once = prism().scroll(Motion::Once, MOVING, start, px(96.)).unwrap().to_glyph_scroll(1.);
        assert!(close(once.displacement(t0 + 0.5, 96.), 72.));
        assert!(close(once.displacement(t0 + 1., 96.), 48.));
        assert!(once.running(t0 + 1.9));
        assert!(!once.running(t0 + 2.01));
        assert!(close(once.displacement(t0 + 30., 96.), 0.));
        let looped = prism().scroll(Motion::Loop, MOVING, start, px(96.)).unwrap().to_glyph_scroll(1.);
        assert!(looped.running(t0 + 30.));
        assert!(close(looped.displacement(t0 + 2.5, 96.), 72.));
    }

    #[test]
    fn reduced_motion_window_focus_and_still_names_do_not_scroll() {
        let start = Instant::now();
        let reduced = MotionEnv { reduced: true, focused: true };
        let blurred = MotionEnv { reduced: false, focused: false };
        assert!(role().scroll(Motion::Still, MOVING, start, px(100.)).is_none());
        assert!(role().scroll(Motion::Loop, reduced, start, px(100.)).is_none());
        assert!(prism().scroll(Motion::Loop, reduced, start, px(96.)).is_none());
        assert!(role().scroll(Motion::Loop, blurred, start, px(100.)).is_none());
        assert!(prism().scroll(Motion::Loop, blurred, start, px(96.)).is_some());
        assert!(NameFill::Diagonal(vec![(0, 0.1), (0xffffff, 0.9)]).scroll(Motion::Loop, MOVING, start, px(40.)).is_none());
    }

    #[test]
    fn only_moving_role_gradients_glow() {
        assert!(role().glows(Motion::Loop));
        assert!(!role().glows(Motion::Still));
        assert!(!prism().glows(Motion::Loop));
        assert_eq!((GLOW_BLUR, GLOW_OPACITY), (4., 0.7));
    }

    #[test]
    fn solid_and_single_stop_fills_paint_without_a_gradient() {
        assert!(NameFill::Solid(0x123456).gradient(point(px(0.), px(0.)), px(30.), px(20.), px(8.)).is_none());
        assert!(NameFill::Diagonal(vec![(0x123456, 0.1)]).gradient(point(px(0.), px(0.)), px(30.), px(20.), px(8.)).is_none());
        assert_eq!(NameFill::Diagonal(vec![(0x123456, 0.1), (0, 0.9)]).first(), 0x123456);
    }
}
