//! Last READY / READY_SUPPLEMENTAL on disk, so the next launch can draw the
//! guilds, channels and DMs before the gateway answers.
//!
//! Files live in `~/.cache/discord-phoenix/session/` (0700 dir, 0600 files).
//! They never contain the token (READY doesn't carry it). Logout deletes them.

use std::{
    fs,
    io::Write,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::PathBuf,
    sync::Arc,
};

use phoenix_api::{Cache, GatewayEvent, gateway::RawDispatchHook, gateway::parse_dispatch};

use crate::backend::{AppEvent, ReadyData};

use super::{convert, nav};

const NAMES: [&str; 2] = ["READY", "READY_SUPPLEMENTAL"];

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

/// Gateway hook: persist READY and READY_SUPPLEMENTAL off the network task.
pub fn hook() -> RawDispatchHook {
    Arc::new(|name: &str, raw: &str| {
        if !NAMES.contains(&name) {
            return;
        }
        let (name, raw) = (name.to_owned(), raw.to_owned());
        let _ = std::thread::Builder::new().name("phoenix-snapshot".into()).spawn(move || write(&name, &raw));
    })
}

fn write(name: &str, raw: &str) {
    let (Some(d), Some(p)) = (dir(), file(name)) else { return };
    if fs::DirBuilder::new().recursive(true).mode(0o700).create(&d).is_err() {
        return;
    }
    let tmp = p.with_extension("part");
    let ok = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&tmp)
        .and_then(|mut f| f.write_all(raw.as_bytes()))
        .is_ok();
    if !(ok && fs::rename(&tmp, &p).is_ok()) {
        let _ = fs::remove_file(&tmp);
    }
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
}

/// Parse the stored READY into the same [`ReadyData`] a live login builds,
/// plus the nav events (folders, relationships, threads) sent before it.
/// Blocking: call it off the UI thread.
pub fn load() -> Option<(ReadyData, Vec<AppEvent>)> {
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
    if let Some(sup) = file("READY_SUPPLEMENTAL").and_then(|p| fs::read_to_string(p).ok()) {
        let ev = parse_dispatch("READY_SUPPLEMENTAL", &sup);
        if matches!(ev, GatewayEvent::ReadySupplemental(_)) {
            cache.apply(&ev);
        }
    }
    let prefs = convert::Prefs::from_ready(&ready);
    let data = convert::build_ready(&cache, &ready, &prefs);
    crate::perf::span("snapshot_built", t0);
    let mut events: Vec<AppEvent> = convert::self_activities(&ready).into_iter().collect();
    events.extend(nav::snapshot(&cache, &ready));
    Some((data, events))
}

type Loaded = Option<(ReadyData, Vec<AppEvent>)>;
static PRELOAD: std::sync::Mutex<Option<std::sync::mpsc::Receiver<Loaded>>> = std::sync::Mutex::new(None);

/// Start [`load`] on its own thread at backend creation, before the window exists.
pub fn preload() {
    if !exists() {
        return;
    }
    let (tx, rx) = std::sync::mpsc::channel();
    if std::thread::Builder::new().name("phoenix-snapshot-load".into()).spawn(move || {
        let _ = tx.send(load());
    }).is_ok() {
        *PRELOAD.lock().unwrap_or_else(|e| e.into_inner()) = Some(rx);
    }
}

/// The preloaded snapshot (waits for it), or a fresh [`load`]. Blocking.
pub fn take() -> Loaded {
    let rx = PRELOAD.lock().unwrap_or_else(|e| e.into_inner()).take();
    match rx {
        Some(rx) => rx.recv().ok().flatten(),
        None => load(),
    }
}
