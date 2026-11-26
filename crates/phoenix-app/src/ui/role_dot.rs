use std::time::{Duration, Instant};

use gpui::{
    App, Bounds, Element, ElementId, GlobalElementId, GradientScroll, Hsla, InspectorElementId, IntoElement, LayoutId, Pixels,
    Rgba, Style, TextGradient, TransformationMatrix, Window, point, size,
};

use crate::theme::generated::idx;

pub const DOT_SCROLL: Duration = Duration::from_secs(3);
pub const RING_OPACITY: f32 = 0.4;

const BACKGROUND: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20"><path d="M0 4C0 1.79086 1.79086 0 4 0H16C18.2091 0 20 1.79086 20 4V16C20 18.2091 18.2091 20 16 20H4C1.79086 20 0 18.2091 0 16V4Z"/></svg>"#;
const CIRCLE_6: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20"><circle cx="10" cy="10" r="6"/></svg>"#;
const CIRCLE_5: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20"><circle cx="10" cy="10" r="5"/></svg>"#;

pub fn dot_colors(color: u32, gradient: Option<crate::ui::role_gradient::RoleGradient>) -> Vec<u32> {
    match gradient {
        Some([a, b, c]) if c == a => vec![a, b],
        Some([a, b, c]) => vec![a, b, c],
        None => vec![color],
    }
}

pub fn reflect_stops(colors: &[u32]) -> Vec<(u32, f32)> {
    let n = colors.len();
    if n < 2 {
        return colors.iter().map(|&c| (c, 0.)).collect();
    }
    let step = 0.5 / (n - 1) as f32;
    let forward = colors.iter().enumerate().map(|(i, &c)| (c, i as f32 * step));
    let back = colors.iter().rev().skip(1).enumerate().map(|(i, &c)| (c, 0.5 + (i + 1) as f32 * step));
    forward.chain(back).collect()
}

pub fn dot_gradient(colors: &[u32], bbox: Bounds<Pixels>, alpha: f32, scroll: Option<Instant>) -> Option<TextGradient> {
    if colors.len() < 2 {
        return None;
    }
    let w = bbox.size.width;
    let tile = Bounds::new(bbox.origin, size(w * 2., bbox.size.height));
    let g = reflect_stops(colors).into_iter().fold(TextGradient::new(90., tile).repeat_x(true), |g, (c, at)| {
        g.stop(Rgba { a: alpha, ..gpui::rgb(c) }, at)
    });
    Some(g.scroll(scroll.map(|start| GradientScroll::new(f32::from(w) * 2. / DOT_SCROLL.as_secs_f32(), start, None))))
}

pub struct RoleDot {
    id: ElementId,
    colors: Vec<u32>,
    hovered: bool,
    size: Pixels,
}

pub fn role_dot(id: impl Into<ElementId>, color: u32, gradient: Option<crate::ui::role_gradient::RoleGradient>, hovered: bool, size: Pixels) -> RoleDot {
    RoleDot { id: id.into(), colors: dot_colors(color, gradient), hovered, size }
}

impl IntoElement for RoleDot {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for RoleDot {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
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
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = self.size.into();
        style.size.height = self.size.into();
        style.flex_shrink = 0.;
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _layout: &mut (),
        _window: &mut Window,
        _cx: &mut App,
    ) {
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _layout: &mut (),
        _prepaint: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(&first) = self.colors.first() else { return };
        let mounted = id.map_or_else(Instant::now, |id| {
            window.with_element_state::<Instant, _>(id, |s, _| {
                let s = s.unwrap_or_else(Instant::now);
                (s, s)
            })
        });
        let unit = bounds.size.width / 20.;
        let circle = |r: f32| Bounds::new(point(bounds.origin.x + unit * (10. - r), bounds.origin.y + unit * (10. - r)), size(unit * 2. * r, unit * 2. * r));
        let svg = |window: &mut Window, key: &'static str, data: &'static str, color: Hsla| {
            let _ = window.paint_svg(bounds, key.into(), Some(data.as_bytes()), TransformationMatrix::unit(), color, cx);
        };
        let solid: Hsla = gpui::rgb(first).into();
        svg(window, "phoenix-role-dot-background", BACKGROUND, crate::ui::chat::tk(idx::BACKGROUND_MOD_NORMAL));
        svg(window, "phoenix-role-dot-6", CIRCLE_6, crate::ui::chat::tk(idx::TEXT_STRONG));
        let ring = dot_gradient(&self.colors, circle(6.), RING_OPACITY, None);
        window.with_text_gradient(ring.as_ref(), |window| svg(window, "phoenix-role-dot-6", CIRCLE_6, solid.opacity(RING_OPACITY)));
        let inner = dot_gradient(&self.colors, circle(5.), 1., self.hovered.then_some(mounted));
        window.with_text_gradient(inner.as_ref(), |window| svg(window, "phoenix-role-dot-5", CIRCLE_5, solid));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::px;

    #[test]
    fn reflected_stops_mirror_the_role_colours_over_two_widths() {
        assert_eq!(reflect_stops(&[1, 2]), vec![(1, 0.), (2, 0.5), (1, 1.)]);
        assert_eq!(reflect_stops(&[1, 2, 3]), vec![(1, 0.), (2, 0.25), (3, 0.5), (2, 0.75), (1, 1.)]);
        assert_eq!(dot_colors(7, Some([1, 2, 1])), vec![1, 2]);
        assert_eq!(dot_colors(7, Some([1, 2, 3])), vec![1, 2, 3]);
        assert_eq!(dot_colors(7, None), vec![7]);
    }

    #[test]
    fn the_inner_dot_gradient_spans_its_circle_and_scrolls_two_widths_in_3s() {
        let bbox = Bounds::new(point(px(5.), px(5.)), size(px(10.), px(10.)));
        let still = dot_gradient(&[0xff0000, 0x0000ff], bbox, 1., None).unwrap();
        assert!(still.repeat_x && still.scroll.is_none());
        assert_eq!(still.tile.size.width, px(20.));
        let left = still.color_at(point(px(5.), px(10.)));
        let right = still.color_at(point(px(15.), px(10.)));
        assert!((left.r - 1.).abs() < 1e-3 && (right.b - 1.).abs() < 1e-3);
        let back = still.color_at(point(px(25.), px(10.)));
        assert!((back.r - 1.).abs() < 1e-3);
        let start = Instant::now();
        let moving = dot_gradient(&[0xff0000, 0x0000ff], bbox, 1., Some(start)).unwrap();
        let s = moving.scroll.unwrap();
        assert!((s.speed - 20. / 3.).abs() < 1e-4);
        assert_eq!((s.start, s.duration), (start, None));
        let ring = dot_gradient(&[0xff0000, 0x0000ff], bbox, RING_OPACITY, None).unwrap();
        assert!((ring.stops[0].0.a - 0.4).abs() < 1e-6);
        assert!(dot_gradient(&[0xff0000], bbox, 1., None).is_none());
    }
}
