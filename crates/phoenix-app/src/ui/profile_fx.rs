//! Profile effects and profile frames drawn over / around a profile card
//! (editor preview, popout, full profile).
//!
//! Structure from the vanilla profile popout dump: `profileEffects_ZjcCyW`
//! is `position: absolute`, at the card origin, the card's width,
//! `z-index: 2` (above the card content), `pointer-events: none`; its
//! `inner_ZjcCyW` clips, and each layer is an `img.effect_ZjcCyW` at
//! `top: 0`, `width: 100%` (height from the APNG's aspect ratio, 300x587 for
//! a 300 px card). The client plays each `effects[]` layer from `start` for
//! `duration` ms, looping layers again after `loopDelay`; when reduced motion
//! is on it shows `reducedMotionSrc` instead.
//!
//! GPUI only animates GIF / WebP, so the APNG layers are decoded here into a
//! multi-frame `RenderImage`. Playback is driven by paint: nothing is
//! scheduled while the card is not on screen, and a card that reappears
//! restarts the effect (the client re-mounts it).

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{
    canvas, div, img, prelude::*, AnyElement, App, Context, Global, ImageSource, RenderImage, SharedString,
};

use crate::{app::PhoenixApp, ui::settings::collectibles::{Anchor, Collectible, FrameSpec}};

/// Decoded APNG width: the popout card width (`user-profile-popout` 300 px)
/// at 2x would be 600; the layers are 450 wide, so keep them as is but cap
/// memory by scaling anything wider down to this.
const MAX_W: u32 = 450;
/// Decoded layers kept in memory (one effect has 1-3 layers).
const MAX_ANIMS: usize = 4;
/// Intro delay before the first layer (`delayIntro` default: clock at -500 ms).
const INTRO_MS: u64 = 500;

#[derive(Default)]
struct Store {
    /// SKU -> product (None while fetching or when it failed).
    products: HashMap<u64, Option<Collectible>>,
    /// Layer URL -> decoded frames (None while loading / failed).
    anims: HashMap<SharedString, (Option<Arc<RenderImage>>, Instant)>,
    /// Playback clock per effect SKU: (start, last paint).
    clocks: HashMap<u64, (Instant, Instant)>,
}

impl Global for Store {}

/// Resolves a collectible SKU (`GET /collectibles-products/{sku}`), fetching
/// it once; re-renders the app when it arrives.
pub fn product(app: &PhoenixApp, sku: u64, cx: &mut Context<PhoenixApp>) -> Option<Collectible> {
    let s = cx.default_global::<Store>();
    if let Some(p) = s.products.get(&sku) {
        return p.clone();
    }
    s.products.insert(sku, None);
    let fut = app.backend.collectible_product(sku);
    cx.spawn(async move |this, cx| {
        let p = fut.await.ok().flatten();
        let _ = this.update(cx, |_, cx| {
            cx.default_global::<Store>().products.insert(sku, p);
            cx.notify();
        });
    })
    .detach();
    None
}

/// Decodes a PNG / APNG into BGRA frames (the `image` APNG decoder composes
/// each frame with its dispose / blend ops onto the canvas, so every frame is
/// a full image). `still`: one frame only. An APNG used as a still (the
/// `reducedMotionSrc`) keeps its most opaque frame: the default image of an
/// effect APNG is often the empty intro frame, while the browser shows the
/// composed animation.
fn decode(bytes: &[u8], still: bool) -> anyhow::Result<RenderImage> {
    use image::{AnimationDecoder, codecs::png::PngDecoder};
    let dec = PngDecoder::new(std::io::Cursor::new(bytes))?;
    let mut frames: Vec<image::Frame> = if dec.is_apng()? {
        dec.apng()?.into_frames().collect_frames()?
    } else {
        let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)?.into_rgba8();
        vec![image::Frame::new(img)]
    };
    if still && frames.len() > 1 {
        let ink = |f: &image::Frame| f.buffer().pixels().map(|p| p.0[3] as u64).sum::<u64>();
        let best = (0..frames.len()).max_by_key(|&i| ink(&frames[i])).unwrap_or(0);
        frames = vec![frames.swap_remove(best)];
    }
    let frames = frames
        .into_iter()
        .map(|f| {
            let delay = f.delay();
            let mut buf = f.into_buffer();
            if buf.width() > MAX_W {
                let h = (buf.height() as u64 * MAX_W as u64 / buf.width() as u64) as u32;
                buf = image::imageops::resize(&buf, MAX_W, h.max(1), image::imageops::FilterType::Triangle);
            }
            // GPUI textures are BGRA.
            for p in buf.chunks_exact_mut(4) {
                p.swap(0, 2);
            }
            image::Frame::from_parts(buf, 0, 0, delay)
        })
        .collect::<Vec<_>>();
    Ok(RenderImage::new(frames))
}

fn anim(src: &SharedString, still: bool, cx: &mut Context<PhoenixApp>) -> Option<Arc<RenderImage>> {
    let url = &SharedString::from(if still { format!("still:{src}") } else { src.to_string() });
    let s = cx.default_global::<Store>();
    if let Some((a, used)) = s.anims.get_mut(url) {
        *used = Instant::now();
        return a.clone();
    }
    if s.anims.len() >= MAX_ANIMS
        && let Some(old) = s.anims.iter().min_by_key(|(_, (_, t))| *t).map(|(k, _)| k.clone())
    {
        s.anims.remove(&old);
    }
    s.anims.insert(url.clone(), (None, Instant::now()));
    let http = cx.http_client();
    let u = url.clone();
    let fetch_url = src.clone();
    cx.spawn(async move |this, cx| {
        use futures::AsyncReadExt;
        let mut bytes = Vec::new();
        let ok = match http.get(&fetch_url, Default::default(), true).await {
            Ok(mut r) => r.body_mut().read_to_end(&mut bytes).await.is_ok(),
            Err(_) => false,
        };
        let img = if ok { cx.background_spawn(async move { decode(&bytes, still).ok() }).await.map(Arc::new) } else { None };
        let _ = this.update(cx, |_, cx| {
            if let Some(e) = cx.default_global::<Store>().anims.get_mut(&u) {
                e.0 = img;
            }
            cx.notify();
        });
    })
    .detach();
    None
}

/// Elapsed playback time of an effect. A gap of more than a second without a
/// paint means the card was hidden: restart, like the client re-mounting.
fn clock(sku: u64, cx: &mut App) -> Duration {
    let now = Instant::now();
    let s = cx.default_global::<Store>();
    let e = s.clocks.entry(sku).or_insert((now, now));
    if now.duration_since(e.1) > Duration::from_secs(1) {
        e.0 = now;
    }
    e.1 = now;
    now.duration_since(e.0)
}

/// The effect overlay: put last inside a `relative` card of width `w`.
pub fn effect(c: &Collectible, w: f32, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    let sku = c.sku;
    // `img.effect_` is `width: 100%` with the height from the image's aspect
    // ratio; GPUI's `img` with only a width keeps the natural height and
    // letterboxes (object-fit contain), pushing the art down. Size it from
    // the decoded frame instead.
    let layer = |a: Arc<RenderImage>, id: Option<SharedString>| {
        let sz = a.size(0);
        let h = if sz.width.0 > 0 { w * sz.height.0 as f32 / sz.width.0 as f32 } else { w };
        let el = img(ImageSource::Render(a)).absolute().top_0().left_0().w(crate::theme::z(w)).h(crate::theme::z(h));
        match id {
            Some(id) => el.id(id).into_any_element(),
            None => el.into_any_element(),
        }
    };
    let mut layers: Vec<AnyElement> = Vec::new();
    if c.effects.is_empty() || cx.reduce_motion() {
        // Reduced motion: one `img` with `reducedMotionSrc` at `top: 0`.
        if let Some(src) = c.full.clone().or(c.preview.clone())
            && let Some(a) = anim(&src, true, cx)
        {
            layers.push(layer(a, None));
        }
    } else {
        // The client starts its clock at -500 ms (`delayIntro`).
        let t = (clock(sku, cx).as_millis() as u64).saturating_sub(INTRO_MS);
        for (i, l) in c.effects.iter().enumerate() {
            let Some(a) = anim(&l.src, false, cx) else { continue };
            if t < l.start || l.duration == 0 {
                continue;
            }
            let since = t - l.start;
            let (cycle, active) = if l.looping {
                let period = l.duration + l.loop_delay;
                (since / period, since % period < l.duration)
            } else {
                (0, since < l.duration)
            };
            if !active {
                continue;
            }
            // A fresh id per cycle restarts the APNG from its first frame.
            layers.push(layer(a, Some(SharedString::from(format!("pfx-{sku}-{i}-{cycle}")))));
        }
    }
    Some(
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .overflow_hidden()
            .children(layers)
            // Keep painting while visible so the layer schedule advances
            // (idle frames between loops); stops as soon as the card is gone.
            .child(canvas(|_, _, _| {}, |_, _, window, _| window.request_animation_frame()).absolute().size_0())
            .into_any_element(),
    )
}

/// The frame art around a card of width `w`: an absolute box overflowing the
/// card by the frame's `overflow_*` (scaled by `w / inner_width`), each layer
/// at full width, anchored top / centre / bottom. Put inside a `relative`
/// wrapper that does NOT clip.
pub fn frame(f: &FrameSpec, w: f32, front: bool) -> Option<AnyElement> {
    let k = w / f.inner_width;
    let z = crate::theme::z;
    let layers: Vec<_> = f.layers.iter().filter(|l| l.2 == front).collect();
    if layers.is_empty() {
        return None;
    }
    let mut d = div()
        .absolute()
        .top(z(-f.overflow_top * k))
        .bottom(z(-f.overflow_bottom * k))
        .left(z(-f.overflow_horizontal * k))
        .w(z(w + 2. * f.overflow_horizontal * k));
    for (url, anchor, _) in layers {
        let col = div().absolute().top_0().left_0().size_full().flex().flex_col();
        let col = match anchor {
            Anchor::Top => col.justify_start(),
            Anchor::Center => col.justify_center(),
            Anchor::Bottom => col.justify_end(),
        };
        d = d.child(col.child(img(url.clone()).w_full().flex_none()));
    }
    Some(d.into_any_element())
}

#[cfg(test)]
mod tests {
    /// 4x4 APNG: an empty default (intro) frame, then a 2x2 red patch blended
    /// over it, then a blue 1x1 patch blended over that.
    fn apng() -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut e = png::Encoder::new(&mut out, 4, 4);
            e.set_color(png::ColorType::Rgba);
            e.set_depth(png::BitDepth::Eight);
            e.set_animated(3, 0).unwrap();
            let mut w = e.write_header().unwrap();
            w.set_dispose_op(png::DisposeOp::None).unwrap();
            w.set_blend_op(png::BlendOp::Over).unwrap();
            w.write_image_data(&[0; 64]).unwrap();
            w.set_frame_dimension(2, 2).unwrap();
            w.write_image_data(&[255, 0, 0, 255].repeat(4)).unwrap();
            w.set_frame_dimension(1, 1).unwrap();
            w.set_frame_position(3, 3).unwrap();
            w.write_image_data(&[0, 0, 255, 255]).unwrap();
            w.finish().unwrap();
        }
        out
    }

    #[test]
    fn decodes_composed_frames_and_skips_empty_intro_for_still() {
        let b = apng();
        let anim = super::decode(&b, false).unwrap();
        assert_eq!(anim.frame_count(), 3);
        // Frame 3 keeps frame 2's red patch (dispose none) plus the blue pixel (BGRA).
        let f = anim.as_bytes(2).unwrap();
        assert_eq!(&f[0..4], &[0, 0, 255, 255]);
        assert_eq!(&f[60..64], &[255, 0, 0, 255]);
        let still = super::decode(&b, true).unwrap();
        assert_eq!(still.frame_count(), 1);
        assert_eq!(still.as_bytes(0).unwrap(), f);
    }
}
