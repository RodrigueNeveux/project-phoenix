//! User nameplates (collectibles) behind DM list / member list rows.
//!
//! Structure from the vanilla member list dump (`nameplated_J1Adu_` row):
//! `container_AFi54G` absolute, fills the row, `border-radius: inherit`
//! (8px), `overflow: hidden`, background
//! `linear-gradient(90deg, transparent 0%, P/.08 20%, P/.08 50%, P/.2 100%)`
//! where P is the palette's `darkBackground`; then `img_AFi54G`
//! (`height: 100%`, `inset-inline-end: 0`, `object-position: right`,
//! `opacity: .6`, `.selected` .9, `.channel.hover` .4).

use gpui::{div, img, prelude::*, AnyElement, Hsla, Rgba};

use crate::model::User;
use crate::theme::z;

/// Nameplate palette name -> `darkBackground` (client palette table).
pub fn palette_dark(name: &str) -> Option<u32> {
    Some(match name {
        "crimson" => 0x900007,
        "berry" => 0x893A99,
        "sky" => 0x0080B7,
        "teal" => 0x086460,
        "forest" => 0x2D5401,
        "bubble_gum" => 0xDC3E97,
        "violet" => 0x730BC8,
        "cobalt" => 0x0131C2,
        "clover" => 0x047B20,
        "lemon" => 0xF6CD12,
        "white" => 0xFFFFFF,
        "black" => 0x000000,
        _ => return None,
    })
}

fn rgba(c: u32, a: f32) -> Hsla {
    let mut r: Rgba = gpui::rgb(c);
    r.a = a;
    r.into()
}

/// Where the nameplate is drawn: opacity of `img_AFi54G` differs.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// Member list (`.img` .6, `.selected` .9).
    Member,
    /// DM list (`.channel`: .6, hover .4, selected .9).
    Channel,
}

/// Absolute layer to put first inside a `relative` row with rounded corners.
/// `h` is the row height in px (image width follows the asset's ~5.33:1 ratio,
/// 213x40 in the dump).
pub fn layer(user: &User, h: f32, place: Place, hovered: bool, selected: bool) -> Option<AnyElement> {
    layer_with_fade(user, h, place, hovered, selected, 220.)
}

/// [`layer`] with an explicit horizontal mask fade start (`fade_start_px` from
/// the left edge of `img_AFi54G`).
///
/// Live client (2026-11-24, `img_AFi54G`): `opacity: 0.6` (`.selected` 0.9,
/// `.channel.hover` 0.4) combined with
/// `mask-image: linear-gradient(to right, rgba(0, 0, 0, 0.3) <fade>px, rgb(0, 0, 0) <fade+50>px)`.
/// Long rows put `<fade>` at 220px (uniform 0.3× attenuation); short rows (e.g.
/// "Clem" + tag, measured 151.5px -> 201.5px) reveal the right-side art at full
/// opacity.
pub fn layer_with_fade(
    user: &User,
    h: f32,
    place: Place,
    hovered: bool,
    selected: bool,
    fade_start_px: f32,
) -> Option<AnyElement> {
    let (url, pal) = user.nameplate.clone()?;
    let base_opacity = match (place, selected, hovered) {
        (_, true, _) => 0.9,
        (Place::Channel, false, true) => 0.4,
        _ => 0.6,
    };
    let seg = |l: f32, r: f32, a0: f32, a1: f32, c: u32| {
        div()
            .absolute()
            .top_0()
            .bottom_0()
            .left(gpui::relative(l))
            .w(gpui::relative(r - l))
            .bg(gpui::linear_gradient(90., gpui::linear_color_stop(rgba(c, a0), 0.), gpui::linear_color_stop(rgba(c, a1), 1.)))
    };
    let mut d = div().absolute().top_0().left_0().size_full().rounded(z(8.)).overflow_hidden();
    if let Some(c) = pal {
        // GPUI clips children to a plain rectangle, so the pieces touching the
        // rounded corners carry the 8px radius themselves.
        d = d
            .child(seg(0., 0.2, 0., 0.08, c).rounded_l(z(8.)))
            .child(seg(0.2, 0.5, 0.08, 0.08, c))
            .child(seg(0.5, 1., 0.08, 0.2, c).rounded_r(z(8.)));
    }
    let img_w = h * 213. / 40.;
    let make_img = |op: f32| {
        // Absolute paths load from disk (offscreen mock renders); URLs as usual.
        img(if url.starts_with('/') {
            gpui::ImageSource::from(std::path::PathBuf::from(url.as_ref()))
        } else {
            gpui::ImageSource::from(url.clone())
        })
        .h_full()
        .w(z(img_w))
        .object_fit(gpui::ObjectFit::Cover)
        .opacity(op)
    };
    if fade_start_px >= img_w {
        return Some(
            d.child(make_img(base_opacity * 0.3).absolute().top_0().right_0().rounded_r(z(8.)))
                .into_any_element(),
        );
    }
    // Approximate the 50px linear-gradient mask ramp with clipped horizontal slices
    // inside a right-aligned img_w container.
    let mut art = div().absolute().top_0().right_0().h_full().w(z(img_w)).rounded_r(z(8.)).overflow_hidden();
    let fade_end = (fade_start_px + 50.).min(img_w);
    let mut slices: Vec<(f32, f32, f32)> = Vec::with_capacity(6);
    if fade_start_px > 0. {
        slices.push((0., fade_start_px.min(img_w), 0.3));
    }
    let steps = 4usize;
    let ramp_w = (fade_end - fade_start_px).max(0.);
    if ramp_w > 0. {
        for i in 0..steps {
            let x0 = fade_start_px + ramp_w * (i as f32 / steps as f32);
            let x1 = fade_start_px + ramp_w * ((i + 1) as f32 / steps as f32);
            let t = (i as f32 + 0.5) / steps as f32;
            slices.push((x0, x1, 0.3 + 0.7 * t));
        }
    }
    if fade_end < img_w {
        slices.push((fade_end, img_w, 1.0));
    }
    for (x0, x1, mask_a) in slices {
        let mut slot = div()
            .absolute()
            .top_0()
            .bottom_0()
            .left(z(x0))
            .w(z((x1 - x0).max(0.5)))
            .overflow_hidden();
        if (x1 - img_w).abs() < 0.5 {
            slot = slot.rounded_r(z(8.));
        }
        art = art.child(slot.child(make_img(base_opacity * mask_a).absolute().top_0().left(z(-x0))));
    }
    Some(d.child(art).into_any_element())
}
