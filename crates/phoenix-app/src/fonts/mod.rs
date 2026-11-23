//! Runtime provider for the fonts used by Discord "display name styles"
//! (Sakura, Pricedown, Journal...). Nothing is bundled: at startup we read the
//! local Discord web cache (Chromium simple cache, like `icons::cache`), parse
//! the cached CSS bundles' `@font-face` rules to map family -> `/assets/*.woff2`,
//! and copy the matching cached font bodies to ~/.cache/discord-phoenix/fonts/.
//! WOFF2 -> TTF decoding and registration with GPUI's text system come next.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Families used by display name styles (from the client's @font-face rules).
pub const NAME_STYLE_FAMILIES: &[&str] = &[
    "Sakura", "Pricedown", "Journal", "Jellybean", "Headbang", "Mainframe", "Medieval",
    "Modern", "Monkey Bars", "Munro", "Tempo", "Vampyre", "8Bit", "Corinthia",
    "Delicious Handrawn", "Fraunces",
];

const MAGIC: [u8; 8] = [0x30, 0x5c, 0x72, 0xa7, 0x1b, 0x6d, 0xfb, 0xfc];
const EOF_MAGIC: [u8; 8] = [0xd8, 0x41, 0x0d, 0x97, 0x45, 0x6f, 0xfa, 0xf4];

pub fn out_dir() -> Option<PathBuf> {
    crate::icons::out_dir().map(|d| d.with_file_name("fonts"))
}

/// `/assets/<name>` of a cache entry header, query dropped.
fn asset_key(head: &[u8]) -> Option<String> {
    if head.len() < 24 || head[..8] != MAGIC {
        return None;
    }
    let kl = u32::from_le_bytes(head[12..16].try_into().ok()?) as usize;
    let key = std::str::from_utf8(head.get(24..24 + kl)?).ok()?;
    let name = key.rsplit_once("/assets/")?.1;
    let name = name.split_once('?').map_or(name, |(n, _)| n);
    (!name.contains('/')).then(|| name.to_owned())
}

/// Decoded body bytes (plain, brotli or gzip) of a cache entry.
fn body(entry: &Path) -> Option<Vec<u8>> {
    let b = std::fs::read(entry).ok()?;
    let kl = u32::from_le_bytes(b.get(12..16)?.try_into().ok()?) as usize;
    let start = 24 + kl;
    let end = memchr::memmem::find(b.get(start..)?, &EOF_MAGIC).map_or(b.len(), |p| p + start);
    let raw = b.get(start..end)?;
    if is_font(raw) || raw.starts_with(b"@") || std::str::from_utf8(raw).is_ok() {
        return Some(raw.to_vec());
    }
    let mut out = Vec::new();
    if brotli_decompressor::Decompressor::new(raw, 4096).read_to_end(&mut out).is_ok() {
        return Some(out);
    }
    out.clear();
    flate2::read::GzDecoder::new(raw).read_to_end(&mut out).ok()?;
    Some(out)
}

fn is_font(b: &[u8]) -> bool {
    b.starts_with(b"wOF2") || b.starts_with(b"wOFF") || b.starts_with(&[0, 1, 0, 0]) || b.starts_with(b"OTTO")
}

/// Parses `@font-face` rules: `(family, weight, asset file name)`.
pub fn parse_font_faces(css: &str) -> Vec<(String, u16, String)> {
    let mut out = Vec::new();
    for rule in css.split("@font-face{").skip(1) {
        let rule = rule.split('}').next().unwrap_or("");
        let prop = |k: &str| {
            rule.split(';').find_map(|d| d.trim().strip_prefix(k)).map(|v| v.trim().trim_matches('"').trim_matches('\''))
        };
        let Some(family) = prop("font-family:") else { continue };
        let weight = prop("font-weight:").and_then(|w| w.parse().ok()).unwrap_or(400);
        let Some(src) = prop("src:") else { continue };
        let Some(url) = src.split("/assets/").nth(1).and_then(|s| s.split(['"', ')', '\'']).next()) else {
            continue;
        };
        out.push((family.to_owned(), weight, url.to_owned()));
    }
    out
}

/// Extracts the name-style fonts from `cache` into `dir`. Returns the files written.
pub fn extract(cache: &Path, dir: &Path) -> Vec<PathBuf> {
    let mut entries: HashMap<String, PathBuf> = HashMap::new();
    for e in std::fs::read_dir(cache).into_iter().flatten().flatten() {
        let p = e.path();
        let mut head = [0u8; 1024];
        let Ok(mut f) = std::fs::File::open(&p) else { continue };
        let n = f.read(&mut head).unwrap_or(0);
        if let Some(k) = asset_key(&head[..n]) {
            entries.insert(k, p);
        }
    }
    let mut faces = Vec::new();
    for (k, p) in &entries {
        if k.ends_with(".css")
            && let Some(b) = body(p)
            && let Ok(css) = String::from_utf8(b)
        {
            faces.extend(parse_font_faces(&css));
        }
    }
    let mut written = Vec::new();
    let _ = std::fs::create_dir_all(dir);
    for (family, weight, asset) in faces {
        if !NAME_STYLE_FAMILIES.contains(&family.as_str()) {
            continue;
        }
        let Some(b) = entries.get(&asset).and_then(|p| body(p)).filter(|b| is_font(b)) else { continue };
        let ext = asset.rsplit('.').next().unwrap_or("woff2");
        let out = dir.join(format!("{}-{weight}.{ext}", family.replace(' ', "_")));
        if std::fs::write(&out, &b).is_ok() {
            written.push(out);
        }
    }
    written
}

/// Extracts fonts from every local Discord cache (best effort, call off the UI thread).
pub fn init() -> Vec<PathBuf> {
    let Some(dir) = out_dir() else { return Vec::new() };
    crate::icons::discord_caches().iter().flat_map(|c| extract(c, &dir)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_font_face() {
        let css = r#"@font-face{font-display:swap;font-family:Sakura;font-weight:400;src:url("/assets/ab12.woff2") format("woff2")}"#;
        assert_eq!(parse_font_faces(css), vec![("Sakura".into(), 400, "ab12.woff2".into())]);
    }

    /// Manual: `cargo test -p phoenix-app --lib fonts::tests::extract_local -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn extract_local() {
        println!("{:#?}", init());
    }

    /// Extracted Discord fonts must never end up in the repository.
    #[test]
    fn no_font_files_in_repo() {
        fn walk(d: &Path, bad: &mut Vec<PathBuf>) {
            for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
                let p = e.path();
                if p.file_name().is_some_and(|n| n == "target" || n == ".git") {
                    continue;
                }
                if p.is_dir() {
                    walk(&p, bad);
                } else if p.extension().is_some_and(|x| ["woff", "woff2", "ttf", "otf"].iter().any(|e| x == *e)) {
                    bad.push(p);
                }
            }
        }
        let mut bad = Vec::new();
        walk(Path::new(env!("CARGO_MANIFEST_DIR")), &mut bad);
        assert!(bad.is_empty(), "font files must not be committed (extracted at runtime): {bad:?}");
    }
}
