//! Multi-colour illustrations of the official client (not single-colour
//! icons). Like the icons, they are read at runtime from the local Discord
//! cache and written under the icon cache dir as SVG files, which `gpui::img`
//! renders in colour. Nothing extracted is stored in the repo.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::cache;

/// Upload drop modal (865429 `icons_`): the client's default `icons` prop is
/// `[1, 0, 2]` = document, image, code for `one`, `two`, `three`. Each art is
/// the `background-image` of `.document_` / `.image_` / `.code_` (same module
/// hash as `uploadDropModal_`), centred in the 100x130 `icon_` box.
const UPLOAD: [(&str, f32); 3] = [("document", -30.), ("image", 0.), ("code", 30.)];

/// Width of the posed canvas: the 100px `icon_` box plus 10px each side so a
/// 30° rotation of the 72x96 art isn't clipped.
pub const UPLOAD_CANVAS_W: f32 = 120.;
pub const UPLOAD_CANVAS_H: f32 = 130.;

static UPLOAD_ICONS: OnceLock<Option<[PathBuf; 3]>> = OnceLock::new();

/// Posed drop-modal icons `[one, two, three]`, once extracted.
pub fn upload_icons() -> Option<&'static [PathBuf; 3]> {
    UPLOAD_ICONS.get().and_then(|o| o.as_ref())
}

/// Builds the art files in `<dir>/art` (kept across runs). Runs on the
/// extraction thread.
pub(super) fn init(dir: &Path) {
    let _ = UPLOAD_ICONS.set(upload(&dir.join("art")));
}

fn upload(out: &Path) -> Option<[PathBuf; 3]> {
    let paths = ["one", "two", "three"].map(|n| out.join(format!("upload-{n}.svg")));
    if paths.iter().all(|p| p.is_file()) {
        return Some(paths);
    }
    let caches = super::discord_caches();
    let css = caches.iter().flat_map(|c| cache::asset_entries(c, ".css")).find_map(|(_, p)| {
        let body = cache::read_body(&p)?;
        body.contains(".uploadDropModal_").then_some(body)
    })?;
    let hash = module_hash(&css, ".uploadDropModal_")?;
    let names: Vec<String> = UPLOAD.iter().map(|(class, _)| art_url(&css, &format!(".{class}_{hash}"))).collect::<Option<_>>()?;
    let svgs = caches.iter().flat_map(|c| cache::asset_entries(c, ".svg")).collect::<Vec<_>>();
    std::fs::create_dir_all(out).ok()?;
    for ((name, (_, deg)), path) in names.iter().zip(UPLOAD).zip(&paths) {
        let entry = svgs.iter().find(|(k, _)| k == name)?;
        let svg = cache::read_body(&entry.1)?;
        std::fs::write(path, pose(&svg, deg)?).ok()?;
    }
    Some(paths)
}

/// Class hash following `prefix` (e.g. `TLgD1o` in `.uploadDropModal_TLgD1o{`).
fn module_hash<'a>(css: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = &css[css.find(prefix)? + prefix.len()..];
    let end = rest.find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'))?;
    (end > 0).then_some(&rest[..end])
}

/// Asset file of `<selector>{background-image:url("/assets/<file>.svg")`.
fn art_url(css: &str, selector: &str) -> Option<String> {
    let pat = format!("{selector}{{background-image:url(\"/assets/");
    let rest = &css[css.find(&pat)? + pat.len()..];
    let name = &rest[..rest.find('"')?];
    (name.ends_with(".svg") && !name.contains('/')).then(|| name.to_owned())
}

/// Pixel size from the root `<svg width=".." height="..">`.
fn root_size(svg: &str) -> Option<(f32, f32)> {
    let open = &svg[svg.find("<svg")?..];
    let open = &open[..open.find('>')?];
    let attr = |a: &str| -> Option<f32> {
        let pat = format!(" {a}=\"");
        let v = &open[open.find(&pat)? + pat.len()..];
        v[..v.find('"')?].trim_end_matches("px").parse().ok()
    };
    Some((attr("width")?, attr("height")?))
}

/// The art centred in the `icon_` box (on the wider canvas) and rotated by
/// `deg` around the box centre: the final keyframe of
/// `uploadIconAnimateInLeft/Right_` (rotate ∓30deg; the translation is done
/// by the layout).
fn pose(svg: &str, deg: f32) -> Option<String> {
    let (w, h) = root_size(svg)?;
    let doc = &svg[svg.find("<svg")?..];
    let (cx, cy) = (UPLOAD_CANVAS_W / 2., UPLOAD_CANVAS_H / 2.);
    Some(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{UPLOAD_CANVAS_W}\" height=\"{UPLOAD_CANVAS_H}\" viewBox=\"0 0 {UPLOAD_CANVAS_W} {UPLOAD_CANVAS_H}\"><g transform=\"rotate({deg} {cx} {cy}) translate({} {})\">{doc}</g></svg>",
        cx - w / 2.,
        cy - h / 2.,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_art_in_css() {
        let css = ".x_a{}.uploadDropModal_TLg-1o{display:flex}.document_TLg-1o{background-image:url(\"/assets/ab12.svg\")}";
        let hash = module_hash(css, ".uploadDropModal_").unwrap();
        assert_eq!(hash, "TLg-1o");
        assert_eq!(art_url(css, &format!(".document_{hash}")).as_deref(), Some("ab12.svg"));
        assert_eq!(art_url(css, ".code_TLg-1o"), None);
    }

    #[test]
    fn poses_svg() {
        let s = pose("<svg fill=\"none\" height=\"96\" viewBox=\"0 0 72 96\" width=\"72\"><path d=\"m0 0\"/></svg>", -30.).unwrap();
        assert!(s.contains("rotate(-30 60 65) translate(24 17)"), "{s}");
        assert!(s.ends_with("</svg></g></svg>"));
    }
}
