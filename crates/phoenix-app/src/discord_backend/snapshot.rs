//! Last session on disk, so the next launch draws the UI before the gateway
//! answers and can RESUME the gateway session instead of a new READY.
//!
//! `~/.cache/discord-phoenix/session/` (0700 dir, 0600 files), never the token:
//! - `READY.json`, `READY_SUPPLEMENTAL.json`: raw payloads of the session start;
//! - `journal.log`: every later dispatch (`seq \t name \t raw json`), so the
//!   state at exit is READY + journal replayed through the same cache code;
//! - `owner.pid`: the process writing them. A second Phoenix running at the
//!   same time writes nothing and never resumes (it would steal the session).
//!
//! Logout deletes everything.

use std::{
    fs,
    io::{BufRead, BufWriter, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock, mpsc},
    time::{Duration, SystemTime},
};

use phoenix_api::{
    Cache, GatewayEvent,
    gateway::{RawDispatchHook, ResumeState, events::Ready, parse_dispatch},
};

use crate::backend::{AppEvent, ReadyData};

use super::{
    convert::{self, Adapter},
    nav,
};

const NAMES: [&str; 2] = ["READY", "READY_SUPPLEMENTAL"];
/// Discord keeps a disconnected session resumable for a short while only.
const RESUME_WINDOW: Duration = Duration::from_secs(150);
/// Past this the journal is dropped (replay would cost more than a READY).
const JOURNAL_CAP: u64 = 48 * 1024 * 1024;

pub fn dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    Some(base.join("discord-phoenix").join("session"))
}

fn file(name: &str) -> Option<PathBuf> {
    Some(dir()?.join(format!("{name}.json")))
}

fn journal_path() -> Option<PathBuf> {
    Some(dir()?.join("journal.log"))
}

fn owner_path() -> Option<PathBuf> {
    Some(dir()?.join("owner.pid"))
}

/// Pid of another live Phoenix that owns the session files, if any.
fn other_owner() -> Option<u32> {
    let pid: u32 = fs::read_to_string(owner_path()?).ok()?.trim().parse().ok()?;
    (pid != std::process::id() && std::path::Path::new(&format!("/proc/{pid}")).exists()).then_some(pid)
}

fn open_private(p: &std::path::Path, append: bool) -> std::io::Result<fs::File> {
    let mut o = fs::OpenOptions::new();
    o.create(true).mode(0o600);
    if append {
        o.append(true);
    } else {
        o.write(true).truncate(true);
    }
    o.open(p)
}

fn write_atomic(p: &std::path::Path, bytes: &[u8]) {
    let tmp = p.with_extension("part");
    let ok = open_private(&tmp, false).and_then(|mut f| f.write_all(bytes)).is_ok();
    if !(ok && fs::rename(&tmp, p).is_ok()) {
        let _ = fs::remove_file(&tmp);
    }
}

struct Rec {
    name: String,
    raw: String,
    seq: Option<u64>,
}

struct Files {
    jp: PathBuf,
    journal: Option<BufWriter<fs::File>>,
    size: u64,
}

impl Files {
    fn handle(&mut self, r: Rec) {
        match r.name.as_str() {
            "READY" => {
                // New session: the old journal no longer applies.
                self.journal = open_private(&self.jp, false).ok().map(BufWriter::new);
                self.size = 0;
                if let Some(p) = file("READY") {
                    write_atomic(&p, r.raw.as_bytes());
                }
            }
            "READY_SUPPLEMENTAL" => {
                if let Some(p) = file("READY_SUPPLEMENTAL") {
                    write_atomic(&p, r.raw.as_bytes());
                }
            }
            // Pure UI noise, never changes the cache.
            "TYPING_START" => {}
            _ => {
                if self.journal.is_none() && self.size == 0 && exists() {
                    // Resumed session: keep appending to the previous run's journal.
                    self.journal = open_private(&self.jp, true).ok().map(BufWriter::new);
                    self.size = fs::metadata(&self.jp).map_or(0, |m| m.len());
                }
                if let Some(w) = self.journal.as_mut() {
                    let line = format!("{}\t{}\t{}\n", r.seq.unwrap_or(0), r.name, r.raw);
                    self.size += line.len() as u64;
                    if w.write_all(line.as_bytes()).is_err() || self.size > JOURNAL_CAP {
                        self.journal = None;
                        self.size = u64::MAX;
                        let _ = fs::remove_file(&self.jp);
                    }
                }
            }
        }
    }
}

/// Single writer thread: keeps the files in dispatch order.
fn writer(rx: mpsc::Receiver<Rec>) {
    let Some(d) = dir() else { return };
    if fs::DirBuilder::new().recursive(true).mode(0o700).create(&d).is_err() || other_owner().is_some() {
        return;
    }
    let (Some(jp), Some(op)) = (journal_path(), owner_path()) else { return };
    write_atomic(&op, std::process::id().to_string().as_bytes());
    let mut files = Files { jp, journal: None, size: 0 };
    while let Ok(first) = rx.recv() {
        let mut next = Some(first);
        while let Some(r) = next {
            files.handle(r);
            next = rx.try_recv().ok();
        }
        // Flush when the burst is over: the file is current within one event.
        if let Some(w) = files.journal.as_mut() {
            let _ = w.flush();
        }
    }
}

/// Gateway hook: hand every dispatch to the writer thread.
pub fn hook() -> RawDispatchHook {
    static TX: OnceLock<Mutex<mpsc::Sender<Rec>>> = OnceLock::new();
    let tx = TX.get_or_init(|| {
        let (tx, rx) = mpsc::channel();
        let _ = std::thread::Builder::new().name("phoenix-snapshot".into()).spawn(move || writer(rx));
        Mutex::new(tx)
    });
    Arc::new(move |name: &str, raw: &str, seq: Option<u64>| {
        let rec = Rec { name: name.to_owned(), raw: raw.to_owned(), seq };
        let _ = tx.lock().unwrap_or_else(|e| e.into_inner()).send(rec);
    })
}

pub fn exists() -> bool {
    file("READY").is_some_and(|p| p.is_file())
}

pub fn clear() {
    for n in NAMES {
        if let Some(p) = file(n) {
            let _ = fs::remove_file(p);
        }
    }
    for p in [journal_path(), owner_path()].into_iter().flatten() {
        let _ = fs::remove_file(p);
    }
}

/// Backend state rebuilt from disk, used by `connect` to RESUME.
pub struct Core {
    pub cache: Cache,
    pub ready: Box<Ready>,
    pub adapter: Adapter,
    pub resume: Option<ResumeState>,
}

type Ui = (ReadyData, Vec<AppEvent>);

/// READY + supplemental + journal replayed, converted for the UI. Blocking.
fn load() -> Option<(Ui, Core)> {
    let t0 = std::time::Instant::now();
    let raw = fs::read_to_string(file("READY")?).ok()?;
    crate::perf::span("snapshot_read", t0);
    let GatewayEvent::Ready(ready) = parse_dispatch("READY", &raw) else { return None };
    crate::perf::span("snapshot_parsed", t0);
    drop(raw);
    let cache = Cache::default();
    let ev = GatewayEvent::Ready(ready);
    cache.apply(&ev);
    let GatewayEvent::Ready(ready) = ev else { return None };
    let mut adapter = Adapter::new(ready.user.id, convert::Prefs::from_ready(&ready));
    if let Some(sup) = file("READY_SUPPLEMENTAL").and_then(|p| fs::read_to_string(p).ok()) {
        let ev = parse_dispatch("READY_SUPPLEMENTAL", &sup);
        if matches!(ev, GatewayEvent::ReadySupplemental(_)) {
            cache.apply(&ev);
            let _ = adapter.map(&cache, &ev);
        }
    }
    let (last_seq, fresh) = replay(&cache, &mut adapter);
    crate::perf::span("snapshot_replayed", t0);
    let data = convert::build_ready(&cache, &ready, &adapter.prefs);
    crate::perf::span("snapshot_built", t0);
    let mut events: Vec<AppEvent> = convert::self_activities(&ready).into_iter().collect();
    events.extend(convert::self_status(&ready));
    events.extend(nav::snapshot(&cache, &ready));
    let resume = (fresh && other_owner().is_none())
        .then_some(last_seq)
        .flatten()
        .map(|seq| ResumeState { session_id: ready.session_id.clone(), resume_url: ready.resume_gateway_url.clone(), seq });
    Some(((data, events), Core { cache, ready, adapter, resume }))
}

/// Apply the journal; returns (last seq, written within the resume window).
fn replay(cache: &Cache, adapter: &mut Adapter) -> (Option<u64>, bool) {
    let Some(jp) = journal_path() else { return (None, false) };
    let Ok(f) = fs::File::open(&jp) else { return (None, false) };
    let fresh = f
        .metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age < RESUME_WINDOW);
    let mut last = None;
    for line in std::io::BufReader::new(f).lines() {
        let Ok(line) = line else { break };
        let mut parts = line.splitn(3, '\t');
        let (Some(seq), Some(name), Some(raw)) = (parts.next(), parts.next(), parts.next()) else { break };
        let ev = parse_dispatch(name, raw);
        cache.apply(&ev);
        let _ = adapter.map(cache, &ev);
        if let Ok(s) = seq.parse::<u64>()
            && s > 0
        {
            last = Some(s);
        }
    }
    (last, fresh)
}

#[derive(Default)]
struct Shared {
    rx: Option<mpsc::Receiver<Option<(Ui, Core)>>>,
    ui: Option<Ui>,
    core: Option<Core>,
}

static SHARED: Mutex<Option<Shared>> = Mutex::new(None);

/// Start [`load`] on its own thread at backend creation, before the window exists.
pub fn preload() {
    if !exists() {
        return;
    }
    let (tx, rx) = mpsc::channel();
    if std::thread::Builder::new()
        .name("phoenix-snapshot-load".into())
        .spawn(move || {
            let _ = tx.send(load());
        })
        .is_ok()
    {
        *SHARED.lock().unwrap_or_else(|e| e.into_inner()) = Some(Shared { rx: Some(rx), ..Default::default() });
    }
}

fn with_loaded<T>(f: impl FnOnce(&mut Shared) -> Option<T>) -> Option<T> {
    let mut g = SHARED.lock().unwrap_or_else(|e| e.into_inner());
    let sh = g.as_mut()?;
    if let Some(rx) = sh.rx.take()
        && let Ok(Some((ui, core))) = rx.recv()
    {
        sh.ui = Some(ui);
        sh.core = Some(core);
    }
    f(sh)
}

/// The UI part of the preloaded snapshot (waits for it). Blocking.
pub fn take() -> Option<Ui> {
    with_loaded(|s| s.ui.take())
}

/// The backend part, only when the gateway session can be resumed. Blocking.
pub fn take_core() -> Option<Core> {
    with_loaded(|s| s.core.take()).filter(|c| c.resume.is_some())
}
