use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::cache;

const UPLOAD: [&str; 3] = ["document", "image", "code"];

pub const UPLOAD_CANVAS_W: f32 = 120.;
pub const UPLOAD_CANVAS_H: f32 = 130.;
pub const UPLOAD_GLOW_PAD: f32 = 60.;
pub const UPLOAD_MAX_TILT: usize = 36;

pub struct UploadArt {
    pub one: Vec<PathBuf>,
    pub two: PathBuf,
    pub three: Vec<PathBuf>,
}

static UPLOAD_ICONS: OnceLock<Option<UploadArt>> = OnceLock::new();

pub fn upload_icons() -> Option<&'static UploadArt> {
    UPLOAD_ICONS.get().and_then(|o| o.as_ref())
}

pub(super) fn init(dir: &Path) {
    let _ = UPLOAD_ICONS.set(upload(&dir.join("art")));
    let _ = GUILD_UPLOAD.set(load_guild_upload(&dir.join("art")));
}

pub struct GuildUpload {
    pub ring: PathBuf,
    pub plus: PathBuf,
}

static GUILD_UPLOAD: OnceLock<Option<GuildUpload>> = OnceLock::new();

pub fn guild_upload() -> Option<&'static GuildUpload> {
    GUILD_UPLOAD.get().and_then(|o| o.as_ref())
}

const GUILD_UPLOAD_MARK: &str = "viewBox:\"0 0 80 80\",fill:\"none\",\"aria-hidden\":\"true\",children:[(0,";

fn load_guild_upload(out: &Path) -> Option<GuildUpload> {
    let art = GuildUpload { ring: out.join("guild-upload-ring.svg"), plus: out.join("guild-upload-plus.svg") };
    if art.ring.is_file() && art.plus.is_file() {
        return Some(art);
    }
    let caches = super::discord_caches();
    let js = caches.iter().flat_map(|c| cache::asset_entries(c, ".js")).find_map(|(_, p)| {
        let body = cache::read_body(&p)?;
        (body.contains(GUILD_UPLOAD_MARK) && body.contains("M54.8694")).then_some(body)
    })?;
    let (ring, plus) = upload_svgs(&js)?;
    std::fs::create_dir_all(out).ok()?;
    std::fs::write(&art.ring, ring).ok()?;
    std::fs::write(&art.plus, plus).ok()?;
    Some(art)
}

fn jsx_attrs(src: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(colon) = rest.find(":\"") {
        let key = rest[..colon].trim_start_matches(',').trim_matches('"').to_owned();
        let after = &rest[colon + 2..];
        let Some(end) = after.find('"') else { break };
        let name = match key.as_str() {
            "fillRule" => "fill-rule".to_owned(),
            "clipRule" => "clip-rule".to_owned(),
            k => k.to_owned(),
        };
        out.push((name, after[..end].to_owned()));
        rest = &after[end + 1..];
    }
    out
}

fn upload_svgs(js: &str) -> Option<(String, String)> {
    let start = js.find(GUILD_UPLOAD_MARK)? + GUILD_UPLOAD_MARK.len();
    let body = &js[start..];
    let body = &body[..body.find("]})")?];
    let (mut ring, mut plus) = (String::new(), String::new());
    for el in body.split("(0,").skip(1).chain(std::iter::once(body.split("(0,").next().unwrap_or_default())) {
        let Some(tag_start) = el.find("(\"") else { continue };
        let tail = &el[tag_start + 2..];
        let Some(tag_end) = tail.find('"') else { continue };
        let tag = &tail[..tag_end];
        if tag != "path" && tag != "circle" {
            continue;
        }
        let Some(open) = tail.find('{') else { continue };
        let Some(close) = tail.rfind('}') else { continue };
        let attrs = jsx_attrs(&tail[open + 1..close]);
        let fill = attrs.iter().find(|(k, _)| k == "fill").map(|(_, v)| v.as_str()).unwrap_or("currentColor");
        let el: String = attrs.iter().filter(|(k, _)| k != "fill").map(|(k, v)| format!(" {k}=\"{v}\"")).collect();
        match (tag, fill) {
            ("path", "currentColor") => ring.push_str(&format!("<path{el} fill=\"#fff\"/>")),
            ("path", "white") => plus.push_str(&format!("<path{el} fill=\"#fff\"/>")),
            _ => {}
        }
    }
    if ring.is_empty() || plus.is_empty() {
        return None;
    }
    let wrap = |inner: &str| format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"80\" height=\"80\" viewBox=\"0 0 80 80\" fill=\"none\">{inner}</svg>");
    Some((wrap(&ring), wrap(&plus)))
}

fn upload(out: &Path) -> Option<UploadArt> {
    let side = |n: &str| (0..=UPLOAD_MAX_TILT).map(|d| out.join(format!("upload-{n}-r{d}.svg"))).collect::<Vec<_>>();
    let art = UploadArt { one: side("one"), two: out.join("upload-two-glow.svg"), three: side("three") };
    let all = || art.one.iter().chain(&art.three).chain(std::iter::once(&art.two));
    if all().all(|p| p.is_file()) {
        return Some(art);
    }
    let caches = super::discord_caches();
    let css = caches.iter().flat_map(|c| cache::asset_entries(c, ".css")).find_map(|(_, p)| {
        let body = cache::read_body(&p)?;
        body.contains(".uploadDropModal_").then_some(body)
    })?;
    let hash = module_hash(&css, ".uploadDropModal_")?;
    let names: Vec<String> = UPLOAD.iter().map(|class| art_url(&css, &format!(".{class}_{hash}"))).collect::<Option<_>>()?;
    let svgs = caches.iter().flat_map(|c| cache::asset_entries(c, ".svg")).collect::<Vec<_>>();
    let body = |name: &String| svgs.iter().find(|(k, _)| k == name).and_then(|e| cache::read_body(&e.1));
    let [doc, image, code] = [body(&names[0])?, body(&names[1])?, body(&names[2])?];
    std::fs::create_dir_all(out).ok()?;
    for d in 0..=UPLOAD_MAX_TILT {
        std::fs::write(&art.one[d], pose(&doc, -(d as f32))?).ok()?;
        std::fs::write(&art.three[d], pose(&code, d as f32)?).ok()?;
    }
    std::fs::write(&art.two, glow(&image)?).ok()?;
    Some(art)
}

fn module_hash<'a>(css: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = &css[css.find(prefix)? + prefix.len()..];
    let end = rest.find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'))?;
    (end > 0).then_some(&rest[..end])
}

fn art_url(css: &str, selector: &str) -> Option<String> {
    let pat = format!("{selector}{{background-image:url(\"/assets/");
    let rest = &css[css.find(&pat)? + pat.len()..];
    let name = &rest[..rest.find('"')?];
    (name.ends_with(".svg") && !name.contains('/')).then(|| name.to_owned())
}

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

fn glow(svg: &str) -> Option<String> {
    let (w, h) = root_size(svg)?;
    let doc = &svg[svg.find("<svg")?..];
    let (cw, ch) = (100. + 2. * UPLOAD_GLOW_PAD, 130. + 2. * UPLOAD_GLOW_PAD);
    Some(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{cw}\" height=\"{ch}\" viewBox=\"0 0 {cw} {ch}\"><defs><filter id=\"g\" filterUnits=\"userSpaceOnUse\" x=\"0\" y=\"0\" width=\"{cw}\" height=\"{ch}\"><feDropShadow dx=\"0\" dy=\"0\" stdDeviation=\"24\" flood-color=\"rgb(88,101,242)\" flood-opacity=\"0.5\"/></filter></defs><g filter=\"url(#g)\"><g transform=\"translate({} {})\">{doc}</g></g></svg>",
        (cw - w) / 2.,
        (ch - h) / 2.,
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
    fn splits_the_guild_upload_art() {
        let js = format!(
            "x({{{GUILD_UPLOAD_MARK}C.jsx)(\"path\",{{fillRule:\"evenodd\",clipRule:\"evenodd\",d:\"M54.8694 2Z\",fill:\"currentColor\"}}),(0,C.jsx)(\"circle\",{{cx:\"68\",cy:\"12\",r:\"12\",fill:\"#5865f2\"}}),(0,C.jsx)(\"path\",{{d:\"M68 6Z\",fill:\"white\"}})]}})"
        );
        let (ring, plus) = upload_svgs(&js).unwrap();
        assert!(ring.contains("<path fill-rule=\"evenodd\" clip-rule=\"evenodd\" d=\"M54.8694 2Z\" fill=\"#fff\"/>"), "{ring}");
        assert!(!ring.contains("circle") && !ring.contains("M68 6Z"), "{ring}");
        assert!(plus.contains("d=\"M68 6Z\""), "{plus}");
        assert!(plus.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"80\" height=\"80\""));
    }

    #[test]
    fn poses_svg() {
        let s = pose("<svg fill=\"none\" height=\"96\" viewBox=\"0 0 72 96\" width=\"72\"><path d=\"m0 0\"/></svg>", -30.).unwrap();
        assert!(s.contains("rotate(-30 60 65) translate(24 17)"), "{s}");
        assert!(s.ends_with("</svg></g></svg>"));
    }

    #[test]
    fn glow_svg_centres_art_on_padded_canvas() {
        let s = glow("<svg fill=\"none\" height=\"96\" viewBox=\"0 0 72 96\" width=\"72\"><path d=\"m0 0\"/></svg>").unwrap();
        assert!(s.contains("width=\"220\" height=\"250\""), "{s}");
        assert!(s.contains("translate(74 77)"), "{s}");
        assert!(s.contains("stdDeviation=\"24\""), "{s}");
    }
}
