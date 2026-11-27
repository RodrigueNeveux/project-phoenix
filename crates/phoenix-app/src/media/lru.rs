use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use gpui::{AnyElement, App, AppContext, Entity, Global, ImageCache, ImageCacheError, ImageCacheItem, IntoElement, ParentElement, RenderImage, Resource, Styled, WeakEntity, Window, hash};

pub const BUDGET_BYTES: u64 = 128 * 1024 * 1024;
const KEEP_FRAMES: u64 = 2;
const MAX_FAILED_ITEMS: usize = 512;
const SWEEP_EVERY: u64 = 64;

static FRAME: AtomicU64 = AtomicU64::new(1);

pub fn next_frame() {
    FRAME.fetch_add(1, Ordering::Relaxed);
}

fn frame() -> u64 {
    FRAME.load(Ordering::Relaxed)
}

struct Entry {
    item: ImageCacheItem,
    frame: u64,
    bytes: Option<u64>,
}

pub struct LruImageCache {
    items: HashMap<u64, Entry>,
    loads: u64,
    budget: u64,
}

fn bytes_of(img: &RenderImage) -> u64 {
    (0..img.frame_count())
        .map(|i| {
            let s = img.size(i);
            s.width.0.max(0) as u64 * s.height.0.max(0) as u64 * 4
        })
        .sum()
}

impl LruImageCache {
    pub fn new(budget: u64, cx: &mut App) -> Entity<Self> {
        let e = cx.new(|_| Self { items: HashMap::new(), loads: 0, budget });
        cx.observe_release(&e, |c, cx| {
            for (_, e) in c.items.drain() {
                if let Some(Ok(img)) = e.item.get() {
                    cx.drop_image(img, None);
                }
            }
        })
        .detach();
        e
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn decoded_bytes(&self) -> u64 {
        self.items.values().filter_map(|e| e.bytes).sum()
    }

    fn evict(&mut self, protected: u64, window: &mut Window, cx: &mut App) {
        let mut entries: Vec<(u64, u64, u64)> = Vec::with_capacity(self.items.len());
        for (k, e) in &mut self.items {
            if e.bytes.is_none()
                && let Some(Ok(img)) = e.item.get()
            {
                e.bytes = Some(bytes_of(&img));
            }
            if let Some(b) = e.bytes {
                entries.push((e.frame, *k, b));
            }
        }
        let victims = select_evictions(&mut entries, self.budget, frame(), KEEP_FRAMES, protected);
        for k in victims {
            if let Some(e) = self.items.remove(&k)
                && let Some(Ok(img)) = e.item.get()
            {
                cx.drop_image(img, Some(window));
            }
        }
        if self.items.len() > MAX_FAILED_ITEMS {
            let fresh = frame().saturating_sub(KEEP_FRAMES);
            self.items.retain(|k, e| *k == protected || e.frame >= fresh || !matches!(e.item.get(), Some(Err(_))));
        }
    }
}

fn select_evictions(entries: &mut [(u64, u64, u64)], budget: u64, now: u64, keep: u64, protected: u64) -> Vec<u64> {
    let mut used: u64 = entries.iter().map(|(_, _, b)| *b).sum();
    if used <= budget {
        return Vec::new();
    }
    entries.sort_unstable_by_key(|(t, k, _)| (*t, *k));
    let fresh = now.saturating_sub(keep);
    let target = budget * 9 / 10;
    let mut victims = Vec::new();
    for &(t, k, bytes) in entries.iter() {
        if used <= target || t >= fresh {
            break;
        }
        if k == protected {
            continue;
        }
        used = used.saturating_sub(bytes);
        victims.push(k);
    }
    victims
}

impl ImageCache for LruImageCache {
    fn load(&mut self, resource: &Resource, window: &mut Window, cx: &mut App) -> Option<Result<Arc<RenderImage>, ImageCacheError>> {
        self.loads += 1;
        let now = frame();
        let key = hash(resource);
        let inserted = !self.items.contains_key(&key);
        let entry = self.items.entry(key).or_insert_with(|| Entry { item: ImageCacheItem::new(resource, cx), frame: now, bytes: None });
        entry.frame = now;
        let out = entry.item.use_image(window);
        if inserted || self.loads.is_multiple_of(SWEEP_EVERY) {
            self.evict(key, window, cx);
        }
        out
    }
}

struct SharedCache(#[allow(dead_code)] Entity<LruImageCache>);
impl Global for SharedCache {}

thread_local! {
    static SHARED: RefCell<Option<WeakEntity<LruImageCache>>> = const { RefCell::new(None) };
}

pub fn init(cx: &mut App) {
    let c = LruImageCache::new(BUDGET_BYTES, cx);
    SHARED.with_borrow_mut(|s| *s = Some(c.downgrade()));
    cx.set_global(SharedCache(c));
}

pub fn chat() -> Option<Entity<LruImageCache>> {
    SHARED.with_borrow(|s| s.as_ref()?.upgrade())
}

pub fn scope(child: impl IntoElement) -> AnyElement {
    match chat() {
        Some(cache) => gpui::image_cache(cache).size_full().child(child).into_any_element(),
        None => child.into_any_element(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_oldest_first_when_over_budget() {
        let mut entries = vec![(1, 10, 50), (2, 20, 50), (3, 30, 50)];
        let victims = select_evictions(&mut entries, 100, 500, 100, 30);
        assert_eq!(victims, vec![10, 20]);
    }

    #[test]
    fn never_evicts_images_drawn_in_the_last_frames() {
        let mut entries = vec![(100, 1, 60), (101, 2, 60), (102, 3, 60)];
        assert!(select_evictions(&mut entries, 100, 102, 2, 3).is_empty());
        let mut entries = vec![(90, 1, 60), (101, 2, 60), (102, 3, 60)];
        assert_eq!(select_evictions(&mut entries, 100, 102, 2, 3), vec![1]);
    }

    #[test]
    fn keeps_everything_under_budget() {
        let mut entries = vec![(1, 10, 40), (2, 20, 40)];
        assert!(select_evictions(&mut entries, 100, 500, 100, 20).is_empty());
    }
}
