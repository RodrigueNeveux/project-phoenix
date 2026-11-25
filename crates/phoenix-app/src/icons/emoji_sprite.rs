use std::path::Path;
use std::sync::OnceLock;

use super::cache;

pub const COLS: u32 = 20;
pub const ROWS: u32 = 4;
pub const FACES: u32 = 77;
pub const REST_FACE: u32 = 50;

pub struct Sprite {
    pub colored: image::RgbaImage,
    pub mask: image::RgbaImage,
}

static SPRITE: OnceLock<Option<Sprite>> = OnceLock::new();

pub fn get() -> Option<&'static Sprite> {
    SPRITE.get().and_then(Option::as_ref)
}

pub fn install(s: Sprite) {
    let _ = SPRITE.set(Some(s));
}

pub fn css_urls(css: &str) -> Option<(String, String)> {
    let url_after = |marker: &str, prop: &str| -> Option<String> {
        css.match_indices(marker).find_map(|(at, _)| {
            let rule_end = at + css[at..].find('}')?;
            let rule = &css[at..rule_end];
            let p = rule.find(prop)? + prop.len();
            let rest = &rule[p..];
            let name = rest.strip_prefix("url(\"/assets/").or_else(|| rest.strip_prefix("url(/assets/"))?;
            let end = name.find(['"', ')'])?;
            Some(name[..end].to_owned())
        })
    };
    Some((url_after(".spriteColored_", "background-image:")?, url_after(".spriteGreyscale_", "mask-image:")?))
}

pub fn cell(img: &image::RgbaImage, face: u32) -> image::RgbaImage {
    let (w, h) = (img.width() / COLS, img.height() / ROWS);
    let f = face % (COLS * ROWS);
    image::imageops::crop_imm(img, (f % COLS) * w, (f / COLS) * h, w, h).to_image()
}

fn decode(p: &Path) -> Option<image::RgbaImage> {
    Some(image::load_from_memory(&std::fs::read(p).ok()?).ok()?.to_rgba8())
}

pub(super) fn init(icons: &Path) {
    let dir = icons.join("art");
    let (cp, mp) = (dir.join("emoji-sprite-colored.png"), dir.join("emoji-sprite-mask.png"));
    if let (Some(colored), Some(mask)) = (decode(&cp), decode(&mp)) {
        install(Sprite { colored, mask });
        return;
    }
    let caches = super::discord_caches();
    let Some((c, m)) = caches.iter().flat_map(|c| cache::asset_entries(c, ".css")).find_map(|(_, p)| {
        let body = cache::read_body(&p)?;
        if !body.contains(".spriteColored_") {
            return None;
        }
        css_urls(&body)
    }) else {
        return;
    };
    let pngs: Vec<_> = caches.iter().flat_map(|c| cache::asset_entries(c, ".png")).collect();
    let bytes = |name: &str| pngs.iter().find(|(k, _)| k == name).and_then(|(_, p)| cache::read_bytes(p));
    let (Some(cb), Some(mb)) = (bytes(&c), bytes(&m)) else { return };
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(&cp, &cb);
    let _ = std::fs::write(&mp, &mb);
    if let (Ok(colored), Ok(mask)) = (image::load_from_memory(&cb), image::load_from_memory(&mb)) {
        install(Sprite { colored: colored.to_rgba8(), mask: mask.to_rgba8() });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_sprite_urls_from_the_client_css() {
        let css = r#".spriteColored_SjG8Ug{background-image:url("/assets/1b05b930a77fbff9.png");background-position:var(--custom-emoji-sprite-bg-position)}.spriteGreyscale_SjG8Ug{background-color:var(--interactive-text-default)}.spriteGreyscale_SjG8Ug,.spritePremiumColored_SjG8Ug{-webkit-mask-image:url("/assets/decf85f79e02d276.png");mask-image:url("/assets/decf85f79e02d276.png")}"#;
        assert_eq!(css_urls(css), Some(("1b05b930a77fbff9.png".into(), "decf85f79e02d276.png".into())));
    }

    #[test]
    fn crops_the_grid_cell() {
        let img = image::RgbaImage::from_fn(960, 192, |x, y| image::Rgba([(x / 48) as u8, (y / 48) as u8, 0, 255]));
        let c = cell(&img, 50);
        assert_eq!((c.width(), c.height()), (48, 48));
        assert_eq!(c.get_pixel(0, 0).0[..2], [10, 2]);
        assert_eq!(cell(&img, 76).get_pixel(5, 5).0[..2], [16, 3]);
    }
}
