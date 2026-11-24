//! Blurred preview of a spoilered upload. The client applies
//! `filter: blur(var(--custom-channel-attachment-upload-spoiler-blur-radius))`
//! (44px) to `spoiler_sBdlOA`; GPUI has no blur filter, so the decoded image
//! is downscaled, blurred and re-encoded once, off the UI thread.

use std::sync::Arc;

use gpui::{Image, ImageFormat};

/// --custom-channel-attachment-upload-spoiler-blur-radius.
pub const BLUR_RADIUS: f32 = 44.;
/// The preview box is ~184px wide; work at 1/4 scale (the blur hides the loss).
const WORK_WIDTH: u32 = 64;

/// PNG bytes of a blurred copy of `bytes` (any format the `image` crate decodes),
/// `None` when it can't be decoded.
pub fn blurred_png(bytes: &[u8]) -> Option<Vec<u8>> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return None;
    }
    let scale = WORK_WIDTH as f32 / w.max(h) as f32;
    let (sw, sh) = (((w as f32 * scale).round() as u32).max(1), ((h as f32 * scale).round() as u32).max(1));
    let small = image::imageops::resize(&img, sw, sh, image::imageops::FilterType::Triangle);
    // 44px on a ~200px preview == 44 * WORK_WIDTH / 200 at work scale; sigma ~ radius / 2.
    let sigma = BLUR_RADIUS * WORK_WIDTH as f32 / 200. / 2.;
    let blurred = image::imageops::fast_blur(&small, sigma);
    let mut out = std::io::Cursor::new(Vec::new());
    blurred.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

pub fn blurred_image(bytes: &[u8]) -> Option<Arc<Image>> {
    blurred_png(bytes).map(|png| Arc::new(Image::from_bytes(ImageFormat::Png, png)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blurs_a_png() {
        // 32x16 checkerboard: the blurred copy is smooth (no pure black/white neighbours).
        let mut img = image::RgbaImage::new(32, 16);
        for (x, y, p) in img.enumerate_pixels_mut() {
            *p = if (x + y) % 2 == 0 { image::Rgba([255, 255, 255, 255]) } else { image::Rgba([0, 0, 0, 255]) };
        }
        let mut png = std::io::Cursor::new(Vec::new());
        img.write_to(&mut png, image::ImageFormat::Png).unwrap();
        let out = blurred_png(png.get_ref()).expect("blurred");
        let b = image::load_from_memory(&out).unwrap().to_rgba8();
        assert_eq!(b.dimensions(), (64, 32));
        let p = b.get_pixel(32, 16).0[0];
        assert!((60..200).contains(&p), "{p}");
        assert!(blurred_png(b"not an image").is_none());
    }
}
