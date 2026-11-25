use std::cell::RefCell;
use std::mem::ManuallyDrop;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use raw_window_handle::{HasDisplayHandle as _, HasWindowHandle as _, RawDisplayHandle, RawWindowHandle};
use wayland_client::backend::{Backend, ObjectId};
use wayland_client::globals::{GlobalList, GlobalListContents, registry_queue_init};
use wayland_client::protocol::{wl_registry, wl_surface::WlSurface};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};
use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1::{self, XdgActivationV1};

pub const TOKEN_TTL: Duration = Duration::from_secs(10);

static TOKEN: Mutex<Option<(String, Instant)>> = Mutex::new(None);

pub fn provide(token: String) {
    if token.is_empty() {
        return;
    }
    *TOKEN.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some((token, Instant::now()));
}

pub fn fresh(slot: &mut Option<(String, Instant)>, now: Instant) -> Option<String> {
    let (token, at) = slot.take()?;
    (now.saturating_duration_since(at) < TOKEN_TTL).then_some(token)
}

pub fn take() -> Option<String> {
    fresh(&mut TOKEN.lock().unwrap_or_else(std::sync::PoisonError::into_inner), Instant::now())
}

struct State;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(_: &mut Self, _: &wl_registry::WlRegistry, _: wl_registry::Event, _: &GlobalListContents, _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<XdgActivationV1, ()> for State {
    fn event(_: &mut Self, _: &XdgActivationV1, _: xdg_activation_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

struct Guest {
    display: usize,
    conn: Connection,
    activation: XdgActivationV1,
    _globals: GlobalList,
    _queue: EventQueue<State>,
}

thread_local! {
    static GUEST: RefCell<Option<ManuallyDrop<Guest>>> = const { RefCell::new(None) };
}

fn guest(display: *mut std::ffi::c_void) -> Option<(Connection, XdgActivationV1)> {
    GUEST.with_borrow_mut(|g| {
        if let Some(g) = g.as_ref().filter(|g| g.display == display as usize) {
            return Some((g.conn.clone(), g.activation.clone()));
        }
        // SAFETY: `display` is the live `wl_display` of GPUI's own connection, which outlives every window and so this guest backend.
        let backend = unsafe { Backend::from_foreign_display(display.cast()) };
        let conn = Connection::from_backend(backend);
        let (globals, queue) = registry_queue_init::<State>(&conn).ok()?;
        let activation: XdgActivationV1 = globals.bind(&queue.handle(), 1..=1, ()).ok()?;
        *g = Some(ManuallyDrop::new(Guest { display: display as usize, conn: conn.clone(), activation: activation.clone(), _globals: globals, _queue: queue }));
        Some((conn, activation))
    })
}

pub fn activate(window: &gpui::Window, token: &str) -> bool {
    let (Ok(d), Ok(w)) = (window.display_handle(), window.window_handle()) else { return false };
    let (RawDisplayHandle::Wayland(d), RawWindowHandle::Wayland(w)) = (d.as_raw(), w.as_raw()) else { return false };
    let Some((conn, activation)) = guest(d.display.as_ptr()) else { return false };
    // SAFETY: the pointer is the window's live `wl_surface` proxy, used only for this request while the window exists.
    let Ok(id) = (unsafe { ObjectId::from_ptr(WlSurface::interface(), w.surface.as_ptr().cast()) }) else { return false };
    let Ok(surface) = WlSurface::from_id(&conn, id) else { return false };
    activation.activate(token.to_owned(), &surface);
    conn.flush().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_single_use_and_expire() {
        let now = Instant::now();
        let mut slot = Some(("t".to_owned(), now));
        assert_eq!(fresh(&mut slot, now + Duration::from_secs(1)).as_deref(), Some("t"));
        assert_eq!(fresh(&mut slot, now), None);
        let mut old = Some(("t".to_owned(), now));
        assert_eq!(fresh(&mut old, now + TOKEN_TTL), None);
    }
}
