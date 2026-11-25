use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::{Arc, OnceLock};

use parking_lot::RwLock;
use serde_json::Value;

use super::cache;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lottie {
    Mute,
    Deafen,
    Settings,
    Camera,
    Stream,
    Krisp,
    Disconnect,
    Soundboard,
    Activities,
    Gift,
    Gif,
    Sticker,
}

impl Lottie {
    pub const ALL: [Lottie; 12] = [
        Lottie::Mute,
        Lottie::Deafen,
        Lottie::Settings,
        Lottie::Camera,
        Lottie::Stream,
        Lottie::Krisp,
        Lottie::Disconnect,
        Lottie::Soundboard,
        Lottie::Activities,
        Lottie::Gift,
        Lottie::Gif,
        Lottie::Sticker,
    ];

    pub fn file(self) -> &'static str {
        match self {
            Lottie::Mute => "mute",
            Lottie::Deafen => "deafen",
            Lottie::Settings => "settings",
            Lottie::Camera => "camera",
            Lottie::Stream => "stream",
            Lottie::Krisp => "krisp",
            Lottie::Disconnect => "disconnect",
            Lottie::Soundboard => "soundboard",
            Lottie::Activities => "activities",
            Lottie::Gift => "gift",
            Lottie::Gif => "gif",
            Lottie::Sticker => "sticker",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Lottie::Mute => "Lottie_HoverOnOff_Microphone",
            Lottie::Deafen => "Lottie_HoverOnOff_Deafen",
            Lottie::Settings => "L",
            Lottie::Camera => "Lottie_HoverOnOff_Camera",
            Lottie::Stream => "Lottie_Hover_Screenshare",
            Lottie::Krisp => "Lottie_HoverOnOff_NoiseSuppression",
            Lottie::Disconnect => "Lottie_Hover_EndCall",
            Lottie::Soundboard => "Lottie_HoverOn_Soundboard",
            Lottie::Activities => "Shapes-ButtonDark 2",
            Lottie::Gift => "Lottie_Web_Gift",
            Lottie::Gif => "Lottie_HoverOn_GIF",
            Lottie::Sticker => "Lottie_HoverOn_Sticker",
        }
    }

    pub fn identify(v: &Value) -> Option<Lottie> {
        let nm = v.get("nm")?.as_str()?;
        let k = Lottie::ALL.into_iter().find(|k| k.name() == nm)?;
        let ok = match k {
            Lottie::Settings => {
                v.get("w").and_then(Value::as_f64) == Some(24.)
                    && v.get("op").and_then(Value::as_f64) == Some(66.)
                    && {
                        let m = markers(v);
                        m.len() == 1 && m.contains_key("all")
                    }
            }
            _ => !markers(v).is_empty(),
        };
        ok.then_some(k)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Marker {
    pub start: f64,
    pub duration: f64,
}

impl Marker {
    pub fn end(self) -> f64 {
        self.start + self.duration
    }
}

pub fn markers(v: &Value) -> HashMap<&str, Marker> {
    let mut out = HashMap::new();
    for m in v.get("markers").and_then(Value::as_array).into_iter().flatten() {
        let (Some(name), Some(tm), Some(dr)) =
            (m.get("cm").and_then(Value::as_str), m.get("tm").and_then(Value::as_f64), m.get("dr").and_then(Value::as_f64))
        else {
            continue;
        };
        out.insert(name, Marker { start: tm, duration: dr });
    }
    out
}

pub struct Anim {
    pub root: Value,
    pub fps: f64,
    pub markers: HashMap<String, Marker>,
}

impl Anim {
    pub fn new(root: Value) -> Self {
        let fps = root.get("fr").and_then(Value::as_f64).filter(|f| *f > 0.).unwrap_or(60.);
        let markers = markers(&root).into_iter().map(|(k, m)| (k.to_owned(), m)).collect();
        Anim { root, fps, markers }
    }

    pub fn marker(&self, name: &str) -> Option<Marker> {
        self.markers.get(name).copied()
    }
}

fn store() -> &'static RwLock<HashMap<Lottie, Arc<Anim>>> {
    static S: OnceLock<RwLock<HashMap<Lottie, Arc<Anim>>>> = OnceLock::new();
    S.get_or_init(Default::default)
}

pub fn get(k: Lottie) -> Option<Arc<Anim>> {
    store().read().get(&k).cloned()
}

pub fn install(k: Lottie, root: Value) {
    store().write().insert(k, Arc::new(Anim::new(root)));
}

pub fn extract(js: &str) -> Vec<(Lottie, Value)> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = js[from..].find("JSON.parse('{\"v\":") {
        let start = from + i + "JSON.parse('".len();
        let bytes = js.as_bytes();
        let mut j = start;
        while j < bytes.len() && !(bytes[j] == b'\'' && bytes[j - 1] != b'\\') {
            j += 1;
        }
        from = j.min(js.len());
        let raw = &js[start..from];
        if !raw.contains("\"markers\":[") {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(&crate::ui::voice_effects::unescape_js(raw))
            && let Some(k) = Lottie::identify(&v)
        {
            out.push((k, v));
        }
    }
    out
}

fn load_dir(dir: &Path) {
    for k in Lottie::ALL {
        if let Some(v) = std::fs::read(dir.join(format!("{}.json", k.file()))).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            && Lottie::identify(&v) == Some(k)
        {
            install(k, v);
        }
    }
}

const MAX_CHUNK: u64 = 16 * 1024;

pub(super) fn init(icons: &Path) {
    let dir = icons.join("lottie");
    load_dir(&dir);
    let caches = super::discord_caches();
    let entries: Vec<_> = caches
        .iter()
        .flat_map(|c| cache::js_entries(c))
        .filter(|(_, p)| std::fs::metadata(p).is_ok_and(|m| m.len() <= MAX_CHUNK))
        .collect();
    if entries.is_empty() {
        return;
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    entries.iter().for_each(|(k, _)| k.hash(&mut h));
    1u8.hash(&mut h);
    let stamp = format!("{:016x}", h.finish());
    let stamp_file = dir.join(".build");
    let have_all = Lottie::ALL.iter().all(|k| get(*k).is_some());
    if have_all && std::fs::read_to_string(&stamp_file).is_ok_and(|s| s.trim() == stamp) {
        return;
    }
    let mut found: HashMap<Lottie, Value> = HashMap::new();
    for (_, p) in &entries {
        if found.len() == Lottie::ALL.len() {
            break;
        }
        let Some(js) = cache::read_body(p) else { continue };
        if !js.contains("\"markers\":[") {
            continue;
        }
        for (k, v) in extract(&js) {
            found.entry(k).or_insert(v);
        }
    }
    if found.is_empty() {
        return;
    }
    let _ = std::fs::create_dir_all(&dir);
    for (k, v) in &found {
        if let Ok(b) = serde_json::to_vec(v) {
            let _ = std::fs::write(dir.join(format!("{}.json", k.file())), b);
        }
    }
    let _ = std::fs::write(stamp_file, stamp);
    for (k, v) in found {
        install(k, v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_icons_in_a_chunk() {
        let js = r#"(this.webpackChunkdiscord_app=this.webpackChunkdiscord_app||[]).push([[29597],{29597(e){e.exports=JSON.parse('{"v":"5.9.0","fr":60,"ip":0,"op":66,"w":24,"h":24,"nm":"L","layers":[],"markers":[{"cm":"all","tm":0,"dr":66}]}')},1(e){e.exports=JSON.parse('{"v":"5.9.0","fr":60,"ip":0,"op":300,"w":24,"h":24,"nm":"Lottie_HoverOnOff_Microphone","layers":[],"markers":[{"tm":0,"cm":"mute","dr":70},{"tm":180,"cm":"hover_unmuted","dr":40}]}')},2(e){e.exports=JSON.parse('{"v":"5.9.0","nm":"Other","markers":[{"tm":0,"cm":"all","dr":1}]}')}}]);"#;
        let got = extract(js);
        assert_eq!(got.iter().map(|(k, _)| *k).collect::<Vec<_>>(), vec![Lottie::Settings, Lottie::Mute]);
        let a = Anim::new(got[1].1.clone());
        assert_eq!(a.fps, 60.);
        assert_eq!(a.marker("hover_unmuted"), Some(Marker { start: 180., duration: 40. }));
        assert_eq!(a.marker("hover_unmuted").map(Marker::end), Some(220.));
    }

    #[test]
    fn settings_needs_its_shape() {
        let v = serde_json::json!({"nm": "L", "w": 512, "op": 66, "markers": [{"cm": "all", "tm": 0, "dr": 66}]});
        assert_eq!(Lottie::identify(&v), None);
    }

    #[test]
    fn no_lottie_json_in_repo() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut bad = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                let p = e.path();
                if p.file_name().is_some_and(|n| n == "target" || n == ".git") {
                    continue;
                }
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "json")
                    && std::fs::read_to_string(&p).is_ok_and(|s| Lottie::ALL.iter().any(|k| s.contains(&format!("\"nm\":\"{}\"", k.name())) && k.name().len() > 1))
                {
                    bad.push(p);
                }
            }
        }
        assert!(bad.is_empty(), "client Lottie icons must not be committed: {bad:?}");
    }
}
