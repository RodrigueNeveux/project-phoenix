//! Reads JS bundles out of a Chromium "simple cache" directory (Discord's
//! `Cache/Cache_Data`). Entry layout: 8-byte magic, u32 version, u32 key
//! length, u32 hash, u32 padding, key, then the body (stream 1) up to the
//! stream EOF record. Bodies are stored as transferred (brotli/gzip/plain).

use std::io::Read;
use std::path::{Path, PathBuf};

const MAGIC: [u8; 8] = [0x30, 0x5c, 0x72, 0xa7, 0x1b, 0x6d, 0xfb, 0xfc];
const EOF_MAGIC: [u8; 8] = [0xd8, 0x41, 0x0d, 0x97, 0x45, 0x6f, 0xfa, 0xf4];

fn js_key(head: &[u8]) -> Option<String> {
    if head.len() < 24 || head[..8] != MAGIC {
        return None;
    }
    let kl = u32::from_le_bytes(head[12..16].try_into().ok()?) as usize;
    let key = std::str::from_utf8(head.get(24..24 + kl)?).ok()?;
    let name = key.rsplit_once("/assets/")?.1;
    (name.ends_with(".js") && !name.contains('/')).then(|| name.to_owned())
}

/// `(asset file name, cache entry path)` for every cached JS bundle.
pub fn js_entries(cache: &Path) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(cache) else { return out };
    for e in rd.flatten() {
        let p = e.path();
        if !p.is_file() {
            continue;
        }
        let mut head = vec![0u8; 1024];
        let Ok(mut f) = std::fs::File::open(&p) else { continue };
        let n = f.read(&mut head).unwrap_or(0);
        if let Some(k) = js_key(&head[..n]) {
            out.push((k, p));
        }
    }
    out.sort();
    out
}

fn find(h: &[u8], n: &[u8], from: usize) -> Option<usize> {
    h.get(from..)?.windows(n.len()).position(|w| w == n).map(|p| p + from)
}

/// Decoded text of a cached JS bundle.
pub fn read_body(entry: &Path) -> Option<String> {
    let b = std::fs::read(entry).ok()?;
    let kl = u32::from_le_bytes(b.get(12..16)?.try_into().ok()?) as usize;
    let start = 24 + kl;
    let end = find(&b, &EOF_MAGIC, start).unwrap_or(b.len());
    let body = b.get(start..end)?;
    if let Ok(s) = std::str::from_utf8(body) {
        return Some(s.to_owned());
    }
    let mut out = Vec::new();
    if brotli_decompressor::Decompressor::new(body, 4096).read_to_end(&mut out).is_ok()
        && let Ok(s) = String::from_utf8(out)
    {
        return Some(s);
    }
    let mut out = String::new();
    flate2::read::GzDecoder::new(body).read_to_string(&mut out).ok()?;
    Some(out)
}
