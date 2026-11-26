use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use base64::Engine as _;
use gpui::{AnyElement, Context, RenderImage, Rgba, Styled, div, img, prelude::*};
use serde_json::Value;

use crate::app::PhoenixApp;
use crate::model::Id;
use crate::theme::z;

pub const EFFECT_CHUNKS: [u32; 18] = [
    277339, 517087, 4289, 366414, 331165, 158541, 910169, 276601, 623997, 91770, 109302, 196515,
    674149, 484163, 252029, 667133, 684765, 235583,
];
pub const MAX_PER_KIND: usize = 5;
const RENDER_PX: u32 = 240;
const DARK_BG: (f64, f64, f64) = (54., 57., 62.);
const WHITE: (f64, f64, f64) = (255., 255., 255.);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub accent: (u8, u8, u8),
    pub background: (u8, u8, u8),
    pub opacity: f32,
}

impl Palette {
    pub fn accent_rgba(&self, a: f32) -> Rgba {
        rgba(self.accent, a)
    }
    pub fn background_rgba(&self, a: f32) -> Rgba {
        rgba(self.background, a)
    }
}

fn rgba((r, g, b): (u8, u8, u8), a: f32) -> Rgba {
    Rgba {
        r: r as f32 / 255.,
        g: g as f32 / 255.,
        b: b as f32 / 255.,
        a: a.clamp(0., 1.),
    }
}

pub fn parse_hex(s: &str) -> Option<(u8, u8, u8)> {
    let h = s.trim().trim_start_matches('#');
    let h: String = if h.len() == 3 {
        h.chars().flat_map(|c| [c, c]).collect()
    } else {
        h.get(..6)?.to_owned()
    };
    let v = u32::from_str_radix(&h, 16).ok()?;
    Some(((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

fn hsv_rank((r, g, b): (u8, u8, u8)) -> f64 {
    let (r, g, b) = (r as f64 / 255., g as f64 / 255., b as f64 / 255.);
    let max = r.max(g).max(b);
    let d = max - r.min(g).min(b);
    let s = if d == 0. { 0. } else { d / max };
    let round = |v: f64| (v * 100.).round() / 100.;
    round(100. * s) + round(100. * max)
}

fn rgb_to_hsl((r, g, b): (f64, f64, f64)) -> (f64, f64, f64) {
    let (r, g, b) = (r / 255., g / 255., b / 255.);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.;
    if max == min {
        return (0., 0., l);
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2. - max - min)
    } else {
        d / (max + min)
    };
    let h = if max == r {
        (g - b) / d + if g < b { 6. } else { 0. }
    } else if max == g {
        (b - r) / d + 2.
    } else {
        (r - g) / d + 4.
    };
    (h * 60., s, l)
}

fn hsl_to_rgb((h, s, l): (f64, f64, f64)) -> (f64, f64, f64) {
    if s == 0. {
        return (l * 255., l * 255., l * 255.);
    }
    let h = h / 360.;
    let q = if l < 0.5 { l * (1. + s) } else { l + s - l * s };
    let p = 2. * l - q;
    let f = |mut t: f64| {
        if t < 0. {
            t += 1.
        }
        if t > 1. {
            t -= 1.
        }
        if t < 1. / 6. {
            p + (q - p) * 6. * t
        } else if t < 0.5 {
            q
        } else if t < 2. / 3. {
            p + (q - p) * (2. / 3. - t) * 6.
        } else {
            p
        }
    };
    (f(h + 1. / 3.) * 255., f(h) * 255., f(h - 1. / 3.) * 255.)
}

fn round3(c: (f64, f64, f64)) -> (f64, f64, f64) {
    (
        c.0.round().clamp(0., 255.),
        c.1.round().clamp(0., 255.),
        c.2.round().clamp(0., 255.),
    )
}

fn shift_lightness(c: (f64, f64, f64), by: f64, lighten: bool) -> (f64, f64, f64) {
    let (h, s, l) = rgb_to_hsl(c);
    let l = if lighten {
        if l + by > 1. { 0.9 } else { l + by }
    } else if l - by < 0. {
        0.1
    } else {
        l - by
    };
    round3(hsl_to_rgb((h, s, l)))
}

fn luminance((r, g, b): (f64, f64, f64)) -> f64 {
    let ch = |c: f64| {
        let c = c / 255.;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b)
}

fn contrast(a: (f64, f64, f64), b: (f64, f64, f64)) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

fn ensure_contrast(mut fg: (f64, f64, f64), bg: (f64, f64, f64), ratio: f64) -> (f64, f64, f64) {
    let dark = luminance(bg) <= 0.5;
    let mut c = contrast(fg, bg);
    for _ in 0..10 {
        if c >= ratio {
            break;
        }
        let (h, s, l) = rgb_to_hsl(fg);
        if dark {
            if l < 0.95 {
                fg = hsl_to_rgb((h, s, l + 0.05));
            } else {
                break;
            }
        } else if l > 0.05 {
            fg = hsl_to_rgb((h, s, l - 0.05));
        } else {
            break;
        }
        c = contrast(fg, bg);
    }
    fg
}

fn to_u8(c: (f64, f64, f64)) -> (u8, u8, u8) {
    let r = round3(c);
    (r.0 as u8, r.1 as u8, r.2 as u8)
}

pub fn palette(colors: &[impl AsRef<str>], dark: bool) -> Option<Palette> {
    let mut best: Option<((u8, u8, u8), f64)> = None;
    for c in colors.iter().take(3).filter_map(|c| parse_hex(c.as_ref())) {
        let rank = hsv_rank(c);
        if best.is_none_or(|(_, r)| rank > r) {
            best = Some((c, rank));
        }
    }
    let (base, _) = best?;
    let base_f = (base.0 as f64, base.1 as f64, base.2 as f64);
    let (_, sat, _) = rgb_to_hsl(base_f);
    let (bg, ratio_bg, ratio_accent, lighten) = if dark {
        (DARK_BG, 5., 7., false)
    } else {
        (WHITE, 3., 7., true)
    };
    let background = ensure_contrast(shift_lightness(base_f, 0.6, lighten), bg, ratio_bg);
    let background = round3(hsl_to_rgb(rgb_to_hsl(round3(background))));
    let accent = ensure_contrast(base_f, bg, ratio_accent);
    let opacity = match (dark, sat < 0.1) {
        (true, true) => 0.5,
        (true, false) => 0.2,
        (false, true) => 0.35,
        (false, false) => 0.1,
    };
    Some(Palette {
        accent: to_u8(accent),
        background: to_u8(background),
        opacity,
    })
}

pub fn effect_tints(accent: (u8, u8, u8)) -> [(u8, u8, u8); 2] {
    let c = (accent.0 as f64, accent.1 as f64, accent.2 as f64);
    let (h, s, l) = rgb_to_hsl(c);
    let mut hue = (h).round();
    if hue < 0. {
        hue += 360.;
    }
    let mut s = (100. * s * 10.).round() / 10.;
    let mut l = (100. * l * 10.).round() / 10.;
    if s < 30. {
        s += 30.;
    }
    if l > 80. {
        l -= 40.;
    }
    if l < 20. {
        l += 15.;
    }
    let mut out = [(0, 0, 0); 2];
    for o in out.iter_mut() {
        hue -= 120.;
        if hue < 0. {
            hue += 360.;
        }
        let n = l / 100.;
        let a = s * n.min(1. - n) / 100.;
        let ch = |t: f64| {
            let k = (t + hue / 30.) % 12.;
            (255. * (n - a * (k - 3.).min(9. - k).min(1.).max(-1.)))
                .round()
                .clamp(0., 255.) as u8
        };
        *o = (ch(0.), ch(8.), ch(4.));
    }
    out
}

fn js_num(v: f64) -> String {
    let s = format!("{v}");
    s.strip_suffix(".0").map(str::to_owned).unwrap_or(s)
}

pub fn tint_json(json: &str, accent: (u8, u8, u8)) -> String {
    let [a, b] = effect_tints(accent);
    let tri = |c: (u8, u8, u8)| {
        format!(
            "[{},{},{},",
            js_num(c.0 as f64 / 255.),
            js_num(c.1 as f64 / 255.),
            js_num(c.2 as f64 / 255.)
        )
    };
    json.replace("[1,0,0,", &tri(a)).replace("[0,0,1,", &tri(b))
}

pub fn hash(s: &str) -> u64 {
    let mut t: f64 = 0.;
    for u in s.encode_utf16() {
        let i = (t as i64) as u32 as i32;
        t = (i.wrapping_shl(5)) as f64 - t + u as f64;
    }
    t.abs() as u64
}

pub fn effect_index(channel: Id, msg: Id, name: &str) -> usize {
    (hash(&format!("{}{}{name}", channel.0, msg.0)) % EFFECT_CHUNKS.len() as u64) as usize
}

pub fn chunk_hash(web_js: &str, chunk: u32) -> Option<String> {
    let needle = format!("{chunk}:\"");
    let mut from = 0;
    while let Some(i) = web_js[from..].find(&needle) {
        let at = from + i;
        let ok_before = at == 0 || !web_js.as_bytes()[at - 1].is_ascii_digit();
        let rest = &web_js[at + needle.len()..];
        if ok_before && let Some(end) = rest.find('"') {
            let h = &rest[..end];
            if h.len() >= 8 && h.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Some(h.to_owned());
            }
        }
        from = at + needle.len();
    }
    None
}

pub fn extract_effect(js: &str) -> Option<String> {
    let start = js.find("JSON.parse('")? + "JSON.parse('".len();
    let b = js.as_bytes();
    let mut j = start;
    while j < b.len() && !(b[j] == b'\'' && b[j - 1] != b'\\') {
        j += 1;
    }
    let raw = crate::ui::voice_effects::unescape_js(js.get(start..j)?);
    raw.contains("\"nm\":\"BurstReaction_").then_some(raw)
}

fn http_get(url: &str) -> Option<Vec<u8>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;
    rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .ok()?;
        let r = client.get(url).send().await.ok()?;
        if !r.status().is_success() {
            return None;
        }
        Some(r.bytes().await.ok()?.to_vec())
    })
}

fn web_map() -> Option<&'static str> {
    static WEB: OnceLock<Option<String>> = OnceLock::new();
    WEB.get_or_init(|| {
        for cache in crate::icons::discord_caches() {
            for (_, entry) in crate::icons::cache::js_entries(&cache)
                .into_iter()
                .filter(|(n, _)| n.starts_with("web."))
                .take(4)
            {
                if let Some(js) = crate::icons::cache::read_body(&entry)
                    && chunk_hash(&js, EFFECT_CHUNKS[0]).is_some()
                {
                    return Some(js);
                }
            }
        }
        None
    })
    .as_deref()
}

fn effect_source(index: usize) -> Option<Arc<String>> {
    static LOADED: OnceLock<Mutex<HashMap<usize, Option<Arc<String>>>>> = OnceLock::new();
    let store = LOADED.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(v) = store.lock().ok()?.get(&index) {
        return v.clone();
    }
    let hash = chunk_hash(web_map()?, *EFFECT_CHUNKS.get(index)?)?;
    let name = format!("{hash}.js");
    let cached = crate::icons::discord_caches()
        .into_iter()
        .find_map(|cache| {
            crate::icons::cache::js_entries(&cache)
                .into_iter()
                .find(|(n, _)| *n == name)
                .and_then(|(_, p)| crate::icons::cache::read_body(&p))
        });
    let js = cached.or_else(|| {
        http_get(&format!("https://discord.com/assets/{name}"))
            .and_then(|b| String::from_utf8(b).ok())
    });
    let v = js.as_deref().and_then(extract_effect).map(Arc::new);
    if let Ok(mut s) = store.lock() {
        s.insert(index, v.clone());
    }
    v
}

fn emoji_image(token: &str, emoji_id: Option<Id>) -> Option<Vec<u8>> {
    static IMAGES: OnceLock<Mutex<HashMap<String, Vec<u8>>>> = OnceLock::new();
    let url = match emoji_id {
        Some(id) => format!("https://cdn.discordapp.com/emojis/{}.png?size=64", id.0),
        None => crate::media::emoji::twemoji_url(token)?.to_string(),
    };
    let store = IMAGES.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(b) = store.lock().ok()?.get(&url) {
        return Some(b.clone());
    }
    let b = http_get(&url)?;
    if let Ok(mut s) = store.lock() {
        if s.len() >= 64 {
            s.clear();
        }
        s.insert(url, b.clone());
    }
    Some(b)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Hover,
    Random,
    External,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    pub channel: Id,
    pub msg: Id,
    pub emoji: String,
}

impl Key {
    pub fn new(channel: Id, msg: Id, token: &str, emoji_id: Option<Id>) -> Self {
        let emoji = emoji_id.map_or_else(|| token.to_owned(), |i| i.0.to_string());
        Self {
            channel,
            msg,
            emoji,
        }
    }
}

struct Active {
    kind: Kind,
    serial: u64,
    started: Instant,
    until: Option<Instant>,
    img: Option<Arc<RenderImage>>,
}

thread_local! {
    static ACTIVE: RefCell<HashMap<Key, Active>> = RefCell::new(HashMap::new());
    static SERIAL: RefCell<u64> = const { RefCell::new(0) };
    static RANDOM_DONE: RefCell<std::collections::HashSet<Key>> = RefCell::new(std::collections::HashSet::new());
}

pub fn admit(map: &HashMap<Key, Kind>, key: &Key, kind: Kind) -> bool {
    let same = map
        .iter()
        .filter(|(k, v)| k.channel == key.channel && **v == kind)
        .count();
    if same >= MAX_PER_KIND {
        return false;
    }
    match map.get(key) {
        None => true,
        Some(Kind::Hover) => kind == Kind::External,
        Some(_) => false,
    }
}

pub fn allowed(cx: &gpui::App) -> bool {
    !cx.reduce_motion() && crate::media::emoji::animate_emoji()
}

pub struct Play<'a> {
    pub channel: Id,
    pub msg: Id,
    pub token: &'a str,
    pub emoji_id: Option<Id>,
    pub accent: (u8, u8, u8),
    pub kind: Kind,
}

pub fn play(p: Play<'_>, cx: &mut Context<PhoenixApp>) {
    if !allowed(cx) {
        return;
    }
    let key = Key::new(p.channel, p.msg, p.token, p.emoji_id);
    let kinds: HashMap<Key, Kind> = ACTIVE.with(|a| {
        a.borrow()
            .iter()
            .map(|(k, v)| (k.clone(), v.kind))
            .collect()
    });
    if !admit(&kinds, &key, p.kind) {
        return;
    }
    let serial = SERIAL.with(|s| {
        let mut s = s.borrow_mut();
        *s += 1;
        *s
    });
    ACTIVE.with(|a| {
        a.borrow_mut().insert(
            key.clone(),
            Active {
                kind: p.kind,
                serial,
                started: Instant::now(),
                until: None,
                img: None,
            },
        )
    });
    let name = crate::ui::chat::state::reaction_name(p.token);
    let index = effect_index(p.channel, p.msg, &name);
    let (token, emoji_id, accent) = (p.token.to_owned(), p.emoji_id, p.accent);
    let renderer = cx.svg_renderer();
    cx.spawn(async move |this, cx| {
        let made = cx
            .background_executor()
            .spawn(async move {
                let src = effect_source(index)?;
                let bytes = emoji_image(&token, emoji_id)?;
                let mime = if bytes.starts_with(b"\x89PNG") {
                    "image/png"
                } else {
                    "image/webp"
                };
                let uri = format!(
                    "data:{mime};base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(&bytes)
                );
                let v: Value = serde_json::from_str(&tint_json(&src, accent)).ok()?;
                let v = crate::ui::voice_effects::with_image(&v, &uri);
                let dur = crate::media::lottie::duration(&v);
                let img = crate::media::lottie::rasterize_speed(&v, &renderer, RENDER_PX, 1.)?;
                Some((Arc::new(img), dur))
            })
            .await;
        let Some((img, dur)) = made else {
            ACTIVE.with(|a| {
                let mut a = a.borrow_mut();
                if a.get(&key).is_some_and(|x| x.serial == serial) {
                    a.remove(&key);
                }
            });
            let _ = this.update(cx, |_, cx| cx.notify());
            return;
        };
        let now = Instant::now();
        ACTIVE.with(|a| {
            if let Some(x) = a.borrow_mut().get_mut(&key).filter(|x| x.serial == serial) {
                x.img = Some(img);
                x.started = now;
                x.until = Some(now + dur);
            }
        });
        let _ = this.update(cx, |_, cx| cx.notify());
        let (tx, rx) = futures::channel::oneshot::channel::<()>();
        std::thread::spawn(move || {
            std::thread::sleep(dur);
            let _ = tx.send(());
        });
        let _ = rx.await;
        ACTIVE.with(|a| {
            let mut a = a.borrow_mut();
            if a.get(&key).is_some_and(|x| x.serial == serial) {
                a.remove(&key);
            }
        });
        let _ = this.update(cx, |_, cx| cx.notify());
    })
    .detach();
}

pub fn playing(key: &Key) -> Option<(Arc<RenderImage>, Instant)> {
    let now = Instant::now();
    ACTIVE.with(|a| {
        a.borrow()
            .get(key)
            .filter(|x| x.until.is_some_and(|u| u > now))
            .and_then(|x| Some((x.img.clone()?, x.started)))
    })
}

pub fn pending(key: &Key) -> bool {
    ACTIVE.with(|a| a.borrow().contains_key(key))
}

pub fn random_roll(now_ms: u128, channel: Id, msg: Id, name: &str, count: u32) -> bool {
    let mut e = hash(&format!("{now_ms}{}{}{name}", channel.0, msg.0)) % 10;
    e += if count > 4 {
        4
    } else {
        u64::from(count.saturating_sub(1))
    };
    e > 7
}

pub fn ambient(key: &Key, p: Play<'_>, count: u32, cx: &mut Context<PhoenixApp>) {
    if RANDOM_DONE.with(|d| d.borrow().contains(key)) {
        return;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let name = crate::ui::chat::state::reaction_name(p.token);
    if random_roll(now, p.channel, p.msg, &name, count) {
        RANDOM_DONE.with(|d| d.borrow_mut().insert(key.clone()));
        play(p, cx);
    }
}

pub fn overlay(img: Arc<RenderImage>, emoji: f32) -> AnyElement {
    let g = 7.5 * emoji;
    let shift = (g + emoji) / 2.;
    div()
        .relative()
        .h_0()
        .w(z(emoji))
        .child(
            div()
                .absolute()
                .left(z((emoji - g) / 2.))
                .bottom(z(-2. - shift))
                .w(z(g))
                .h(z(g))
                .overflow_hidden()
                .child(img_el(img, g)),
        )
        .into_any_element()
}

fn img_el(img_: Arc<RenderImage>, g: f32) -> impl IntoElement {
    img(img_).w(z(g)).h(z(g))
}

pub const SHAKE: [(f32, f32); 11] = [
    (1., 1.),
    (-1., -1.),
    (-2., 0.),
    (2., 1.),
    (1., -1.),
    (-1., 1.),
    (-2., 1.),
    (2., 1.),
    (-1., -1.),
    (1., 1.),
    (1., -1.),
];

pub fn shake_offset(t: f32) -> (f32, f32) {
    let t = t.clamp(0., 1.) * 10.;
    let i = (t.floor() as usize).min(9);
    let f = t - i as f32;
    let (a, b) = (SHAKE[i], SHAKE[i + 1]);
    (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f)
}

pub fn clear() {
    ACTIVE.with(|a| a.borrow_mut().clear());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_matches_the_client() {
        assert_eq!(hash(""), 0);
        assert_eq!(hash("a"), 97);
        assert_eq!(hash("ab"), 3105);
        assert_eq!(hash("hello"), 99162322);
        assert_eq!(hash("1403447573389381652"), hash("1403447573389381652"));
    }

    #[test]
    fn effect_index_is_stable() {
        let i = effect_index(Id(1), Id(2), "🔥");
        assert!(i < 18);
        assert_eq!(i, effect_index(Id(1), Id(2), "🔥"));
    }

    #[test]
    fn palette_picks_the_most_vivid_colour() {
        let p = palette(&["#202020", "#f8b229", "#e0e0e0"], true).unwrap();
        assert_eq!(p.opacity, 0.2);
        let c = |x: (u8, u8, u8)| (x.0 as f64, x.1 as f64, x.2 as f64);
        assert!(contrast(c(p.accent), DARK_BG) >= 7. || p.accent.0 > 240);
        let grey = palette(&["#808080"], true).unwrap();
        assert_eq!(grey.opacity, 0.5);
        assert_eq!(palette(&["#808080"], false).unwrap().opacity, 0.35);
        assert!(palette(&Vec::<String>::new(), true).is_none());
    }

    #[test]
    fn tints_replace_the_marker_colours() {
        let out = tint_json(r#"{"c":{"k":[1,0,0,1]},"d":{"k":[0,0,1,1]}}"#, (255, 0, 0));
        assert!(!out.contains("[1,0,0,") && !out.contains("[0,0,1,"));
        let [a, b] = effect_tints((255, 0, 0));
        assert_ne!(a, b);
    }

    #[test]
    fn chunk_hashes_come_from_the_runtime_map() {
        let js =
            r#"x={1277339:"aaaaaaaaaaaaaaaa",277339:"81123bf1e2b44856",4289:"0123456789abcdef"}"#;
        assert_eq!(chunk_hash(js, 277339).as_deref(), Some("81123bf1e2b44856"));
        assert_eq!(chunk_hash(js, 4289).as_deref(), Some("0123456789abcdef"));
        assert_eq!(chunk_hash(js, 91770), None);
    }

    #[test]
    fn effects_are_capped_and_hover_does_not_restart() {
        let k = |m: u64| Key {
            channel: Id(1),
            msg: Id(m),
            emoji: "x".into(),
        };
        let mut map = HashMap::new();
        assert!(admit(&map, &k(1), Kind::Hover));
        map.insert(k(1), Kind::Hover);
        assert!(!admit(&map, &k(1), Kind::Hover));
        assert!(admit(&map, &k(1), Kind::External));
        for m in 2..7 {
            map.insert(k(m), Kind::External);
        }
        assert!(!admit(&map, &k(9), Kind::External));
        assert!(admit(&map, &k(9), Kind::Hover));
    }

    #[test]
    fn random_roll_needs_more_reactions_to_fire_often() {
        let fired = |count| {
            (0..1000u128)
                .filter(|t| random_roll(*t, Id(1), Id(2), "x", count))
                .count()
        };
        assert!(fired(1) < fired(5));
    }

    #[test]
    fn shake_follows_the_keyframes() {
        assert_eq!(shake_offset(0.), (1., 1.));
        assert_eq!(shake_offset(1.), (1., -1.));
        assert_eq!(shake_offset(0.1), (-1., -1.));
    }
}
