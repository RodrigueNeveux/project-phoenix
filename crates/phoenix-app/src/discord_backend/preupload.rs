use std::{
    collections::HashMap,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use parking_lot::Mutex;
use phoenix_api::{
    RestClient,
    models::ChannelId,
    rest::{
        FileUpload,
        uploads::{CloudUpload, UPLOAD_CANCELLED},
    },
};
use tokio::sync::watch;

use crate::backend::UploadProgress;

pub struct Pre {
    pub sent: Arc<AtomicU64>,
    pub rx: watch::Receiver<Option<Result<CloudUpload, String>>>,
    pub abort: tokio::task::AbortHandle,
}

static PRE: LazyLock<Mutex<HashMap<u64, Pre>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn insert(key: u64, pre: Pre) {
    if let Some(old) = PRE.lock().insert(key, pre) {
        old.abort.abort();
    }
}

pub fn take(key: u64) -> Option<Pre> {
    PRE.lock().remove(&key)
}

pub fn drop_key(key: u64) {
    if let Some(p) = take(key) {
        p.abort.abort();
    }
}

fn cancelled() -> phoenix_api::Error {
    phoenix_api::Error::Protocol(UPLOAD_CANCELLED.into())
}

async fn finished(p: &mut Pre, report: &(dyn Fn(u64) -> bool + Send + Sync)) -> Result<Option<CloudUpload>, phoenix_api::Error> {
    loop {
        if !report(p.sent.load(Ordering::Relaxed)) {
            p.abort.abort();
            return Err(cancelled());
        }
        if let Some(r) = p.rx.borrow().clone() {
            return Ok(r.ok());
        }
        if let Ok(Err(_)) = tokio::time::timeout(Duration::from_millis(50), p.rx.changed()).await
            && p.rx.borrow().is_none()
        {
            return Ok(None);
        }
    }
}

pub async fn resolve(
    rest: &RestClient,
    channel: ChannelId,
    files: &[FileUpload],
    pres: Vec<Option<Pre>>,
    progress: Option<UploadProgress>,
) -> phoenix_api::Result<Vec<CloudUpload>> {
    let total: u64 = files.iter().map(|f| f.bytes.len() as u64).sum();
    let done = AtomicU64::new(0);
    let report = |cur: u64| progress.as_ref().is_none_or(|p| p((done.load(Ordering::Relaxed) + cur).min(total), total));
    if !report(0) {
        return Err(cancelled());
    }
    let mut out = Vec::with_capacity(files.len());
    for (f, pre) in files.iter().zip(pres) {
        let ready = match pre {
            Some(mut p) => finished(&mut p, &report).await?,
            None => None,
        };
        let slot = match ready {
            Some(s) => s,
            None => rest.upload_one(channel, f, &report).await?,
        };
        done.fetch_add(f.bytes.len() as u64, Ordering::Relaxed);
        out.push(slot);
    }
    if !report(0) {
        return Err(cancelled());
    }
    Ok(out)
}
