use std::sync::{Condvar, Mutex, Once, OnceLock};
use std::time::{Duration, Instant};

const SETTLE: Duration = Duration::from_secs(2);
const PERIOD: Duration = Duration::from_secs(90);

struct Trimmer {
    due: Mutex<Option<Instant>>,
    wake: Condvar,
}

fn trimmer() -> &'static Trimmer {
    static T: OnceLock<Trimmer> = OnceLock::new();
    T.get_or_init(|| {
        std::thread::Builder::new()
            .name("phoenix-trim".into())
            .stack_size(64 * 1024)
            .spawn(run)
            .ok();
        Trimmer { due: Mutex::new(None), wake: Condvar::new() }
    })
}

pub fn init() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        tune();
        trimmer();
    });
}

pub fn release_soon() {
    let t = trimmer();
    let Ok(mut due) = t.due.lock() else { return };
    *due = Some(Instant::now() + SETTLE);
    t.wake.notify_one();
}

fn run() {
    let t = trimmer();
    let mut last = Instant::now();
    loop {
        let Ok(mut due) = t.due.lock() else { return };
        loop {
            let now = Instant::now();
            let deadline = due.unwrap_or(last + PERIOD);
            if now >= deadline {
                break;
            }
            due = match t.wake.wait_timeout(due, deadline - now) {
                Ok((g, _)) => g,
                Err(_) => return,
            };
        }
        *due = None;
        drop(due);
        let t0 = Instant::now();
        trim();
        crate::perf::span("malloc_trim", t0);
        last = Instant::now();
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn tune() {
    // SAFETY: mallopt only changes glibc allocator parameters and is safe to call at any time.
    unsafe {
        libc::mallopt(libc::M_MMAP_THRESHOLD, 256 * 1024);
        libc::mallopt(libc::M_TRIM_THRESHOLD, 8 * 1024 * 1024);
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn trim() {
    // SAFETY: malloc_trim only returns free heap pages to the kernel.
    unsafe {
        libc::malloc_trim(0);
    }
}

#[cfg(target_os = "linux")]
pub fn map_file(path: &std::path::Path) -> Option<&'static [u8]> {
    use std::os::fd::AsRawFd;
    let file = std::fs::File::open(path).ok()?;
    let len = usize::try_from(file.metadata().ok()?.len()).ok().filter(|l| *l > 0)?;
    // SAFETY: a read-only private mapping of a file we just opened; the mapping is never unmapped,
    // so the slice stays valid for the rest of the process even after the descriptor is closed.
    unsafe {
        let ptr = libc::mmap(std::ptr::null_mut(), len, libc::PROT_READ, libc::MAP_PRIVATE, file.as_raw_fd(), 0);
        if ptr == libc::MAP_FAILED {
            return None;
        }
        Some(std::slice::from_raw_parts(ptr.cast::<u8>(), len))
    }
}

#[cfg(not(target_os = "linux"))]
pub fn map_file(_: &std::path::Path) -> Option<&'static [u8]> {
    None
}

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
fn tune() {}

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
fn trim() {}
