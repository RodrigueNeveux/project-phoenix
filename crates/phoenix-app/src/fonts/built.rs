use std::borrow::Cow;
use std::path::Path;

use serde_json::{Value, json};

pub const FORMAT: u64 = 1;
const CURRENT: &str = "current.json";
const BUILD_PREFIX: &str = "b-";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stamp {
    pub path: String,
    pub len: u64,
    pub mtime: u64,
}

impl Stamp {
    pub fn of(path: &str) -> Self {
        match std::fs::metadata(path) {
            Ok(m) => Self {
                path: path.to_owned(),
                len: m.len(),
                mtime: m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_nanos() as u64),
            },
            Err(_) => Self { path: path.to_owned(), len: u64::MAX, mtime: 0 },
        }
    }

    fn to_json(&self) -> Value {
        json!([self.path, self.len, self.mtime])
    }

    fn from_json(v: &Value) -> Option<Self> {
        let a = v.as_array()?;
        Some(Self { path: a.first()?.as_str()?.to_owned(), len: a.get(1)?.as_u64()?, mtime: a.get(2)?.as_u64()? })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Key {
    pub format: u64,
    pub version: String,
    pub fontconfig: u64,
    pub sources: Vec<Stamp>,
}

impl Key {
    pub fn current(fontconfig: u64, paths: impl IntoIterator<Item = String>) -> Self {
        let mut paths: Vec<String> = paths.into_iter().collect();
        paths.sort();
        paths.dedup();
        Self {
            format: FORMAT,
            version: crate::diagnostics::VERSION.to_owned(),
            fontconfig,
            sources: paths.iter().map(|p| Stamp::of(p)).collect(),
        }
    }

    pub fn valid(&self, fontconfig: u64) -> bool {
        self.format == FORMAT
            && self.version == crate::diagnostics::VERSION
            && self.fontconfig == fontconfig
            && self.sources.iter().all(|s| Stamp::of(&s.path) == *s)
    }

    fn to_json(&self) -> Value {
        json!({
            "format": self.format,
            "version": self.version,
            "fontconfig": self.fontconfig,
            "sources": self.sources.iter().map(Stamp::to_json).collect::<Vec<_>>(),
        })
    }

    fn from_json(v: &Value) -> Option<Self> {
        Some(Self {
            format: v.get("format")?.as_u64()?,
            version: v.get("version")?.as_str()?.to_owned(),
            fontconfig: v.get("fontconfig")?.as_u64()?,
            sources: v.get("sources")?.as_array()?.iter().map(Stamp::from_json).collect::<Option<_>>()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origin {
    Built { file: String, len: u64 },
    Source(String),
}

impl Origin {
    fn to_json(&self) -> Value {
        match self {
            Self::Built { file, len } => json!({ "built": file, "len": len }),
            Self::Source(p) => json!({ "source": p }),
        }
    }

    fn from_json(v: &Value) -> Option<Self> {
        if let Some(p) = v.get("source").and_then(Value::as_str) {
            return Some(Self::Source(p.to_owned()));
        }
        Some(Self::Built { file: v.get("built")?.as_str()?.to_owned(), len: v.get("len")?.as_u64()? })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub key: Key,
    pub build: String,
    pub chain: Vec<String>,
    pub web_noto: bool,
    pub blobs: Vec<Origin>,
}

impl Manifest {
    fn to_json(&self) -> Value {
        json!({
            "key": self.key.to_json(),
            "build": self.build,
            "chain": self.chain,
            "web_noto": self.web_noto,
            "blobs": self.blobs.iter().map(Origin::to_json).collect::<Vec<_>>(),
        })
    }

    fn from_json(v: &Value) -> Option<Self> {
        Some(Self {
            key: Key::from_json(v.get("key")?)?,
            build: v.get("build")?.as_str()?.to_owned(),
            chain: v.get("chain")?.as_array()?.iter().map(|s| s.as_str().map(str::to_owned)).collect::<Option<_>>()?,
            web_noto: v.get("web_noto")?.as_bool()?,
            blobs: v.get("blobs")?.as_array()?.iter().map(Origin::from_json).collect::<Option<_>>()?,
        })
    }
}

pub struct Loaded {
    pub chain: Vec<String>,
    pub web_noto: bool,
    pub blobs: Vec<Cow<'static, [u8]>>,
}

pub fn load(dir: &Path, fontconfig: u64) -> Option<Loaded> {
    let bytes = std::fs::read(dir.join(CURRENT)).ok()?;
    let m = Manifest::from_json(&serde_json::from_slice(&bytes).ok()?)?;
    if !m.key.valid(fontconfig) || m.build.contains(['/', '\\']) {
        return None;
    }
    let build = dir.join(&m.build);
    let mut blobs = Vec::with_capacity(m.blobs.len());
    for o in &m.blobs {
        let path = match o {
            Origin::Built { file, .. } => build.join(file),
            Origin::Source(p) => Path::new(p).to_path_buf(),
        };
        if let Origin::Built { len, .. } = o
            && std::fs::metadata(&path).ok()?.len() != *len
        {
            return None;
        }
        let b: Cow<'static, [u8]> = match crate::memory::map_file(&path) {
            Some(m) => Cow::Borrowed(m),
            None => Cow::Owned(std::fs::read(&path).ok()?),
        };
        blobs.push(b);
    }
    Some(Loaded { chain: m.chain, web_noto: m.web_noto, blobs })
}

pub fn save(dir: &Path, key: Key, chain: &[String], web_noto: bool, blobs: &[(Option<String>, &[u8])]) -> std::io::Result<()> {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let build = format!("{BUILD_PREFIX}{nanos}-{}", std::process::id());
    let bdir = dir.join(&build);
    std::fs::create_dir_all(&bdir)?;
    let mut origins = Vec::with_capacity(blobs.len());
    for (i, (src, bytes)) in blobs.iter().enumerate() {
        match src {
            Some(p) => origins.push(Origin::Source(p.clone())),
            None => {
                let file = format!("{i:02}.ttf");
                std::fs::write(bdir.join(&file), bytes)?;
                origins.push(Origin::Built { file, len: bytes.len() as u64 });
            }
        }
    }
    let m = Manifest { key, build: build.clone(), chain: chain.to_vec(), web_noto, blobs: origins };
    let tmp = dir.join(format!("{CURRENT}.{}.tmp", std::process::id()));
    std::fs::write(&tmp, serde_json::to_vec(&m.to_json()).map_err(std::io::Error::other)?)?;
    std::fs::rename(&tmp, dir.join(CURRENT))?;
    for e in std::fs::read_dir(dir)?.flatten() {
        let name = e.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(BUILD_PREFIX) && name != build && e.path().is_dir() {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("phx-built-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn built_faces_load_back_without_a_rebuild() {
        let d = temp("roundtrip");
        let src = d.join("emoji.ttf");
        std::fs::write(&src, b"source-bytes").unwrap();
        let src = src.to_string_lossy().into_owned();
        let key = Key::current(7, [src.clone(), src.clone()]);
        assert_eq!(key.sources.len(), 1);
        let chain = vec!["Phx Noto Sans".to_string(), "Phx Noto Color Emoji".to_string()];
        let built: &[u8] = b"built-bytes";
        let items: Vec<(Option<String>, &[u8])> = vec![(None, built), (Some(src.clone()), b"ignored".as_slice())];
        save(&d, key, &chain, true, &items).unwrap();
        let l = load(&d, 7).expect("cache hit");
        assert_eq!(l.chain, chain);
        assert!(l.web_noto);
        assert_eq!(l.blobs.iter().map(|b| &b[..]).collect::<Vec<_>>(), vec![built, b"source-bytes".as_slice()]);
        assert!(load(&d, 8).is_none());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_changed_source_or_build_invalidates_the_cache() {
        let d = temp("invalidate");
        let src = d.join("face.ttf");
        std::fs::write(&src, b"one").unwrap();
        let src = src.to_string_lossy().into_owned();
        save(&d, Key::current(1, [src.clone()]), &[], false, &[(None, b"x".as_slice())]).unwrap();
        assert!(load(&d, 1).is_some());
        std::fs::write(&src, b"three").unwrap();
        assert!(load(&d, 1).is_none());
        save(&d, Key::current(1, [src.clone()]), &[], false, &[(None, b"y".as_slice())]).unwrap();
        let builds = std::fs::read_dir(&d).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().starts_with(BUILD_PREFIX)).count();
        assert_eq!(builds, 1);
        let m = Manifest::from_json(&serde_json::from_slice(&std::fs::read(d.join(CURRENT)).unwrap()).unwrap()).unwrap();
        std::fs::write(d.join(&m.build).join("00.ttf"), b"truncated!").unwrap();
        assert!(load(&d, 1).is_none());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn missing_sources_stay_part_of_the_key() {
        let d = temp("missing");
        let gone = d.join("later").to_string_lossy().into_owned();
        let key = Key::current(0, [gone.clone()]);
        assert_eq!(key.sources[0].len, u64::MAX);
        assert!(key.valid(0));
        std::fs::create_dir_all(&gone).unwrap();
        assert!(!key.valid(0));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn manifest_round_trips_through_json() {
        let m = Manifest {
            key: Key { format: FORMAT, version: "1".into(), fontconfig: 3, sources: vec![Stamp { path: "/a".into(), len: 2, mtime: 9 }] },
            build: "b-1".into(),
            chain: vec!["A".into()],
            web_noto: false,
            blobs: vec![Origin::Built { file: "00.ttf".into(), len: 4 }, Origin::Source("/a".into())],
        };
        assert_eq!(Manifest::from_json(&m.to_json()), Some(m));
    }
}
