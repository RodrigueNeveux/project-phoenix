//! The client's sound effects (Settings > Notifications > Sounds and
//! Voice & Video > Sounds), with the same gates as its `SoundUtils.playSound`:
//!
//! - nothing while Streamer Mode "Disable sounds" is on (`StreamerModeStore.disableSounds`,
//!   checked first in `playSound`);
//! - `NotificationSettingsStore.isSoundDisabled(name)` = "Disable all notification
//!   sounds" || the sound's own switch is off (checked by every caller);
//! - volume `min(outputVolume / 100 * volume, 1)` (`WebAudioSound.ensureAudio`), on the
//!   system default output (`setSinkId("default")`);
//! - sounds whose name contains "message" are throttled to one per second, leading edge
//!   (`throttle(B, 1e3, {leading: true})` in the notification utils).
//!
//! The mp3 files are not bundled: [`assets`] takes them from the local Discord cache or
//! from `discord.com/assets` (like the client) into `~/.cache/discord-phoenix/sounds`.
//! Triggers live in [`watch`] (voice / call cues) and [`on_message`].

pub mod assets;
pub mod watch;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use phoenix_voice::audio::effects::{self, Handle, Pcm};

use crate::ui::settings::LocalPrefs;

/// Volume the client passes for message, voice and call cues (`.4`).
pub const VOLUME: f32 = 0.4;

/// Sounds of Voice & Video > Sounds, in the client's order (`aJ` in the settings bundle).
pub const VOICE_LIST: &[&str] = &[
    "deafen",
    "undeafen",
    "mute",
    "unmute",
    "camera_on",
    "camera_off",
    "disconnect",
    "ptt_start",
    "ptt_stop",
    "user_join",
    "user_leave",
    "user_moved",
    "call_calling",
    "stream_started",
    "stream_ended",
    "stream_user_joined",
    "stream_user_left",
    "activity_launch",
    "activity_end",
    "activity_user_join",
    "activity_user_left",
    "reconnect",
];

/// Settings switch of a sound in `LocalPrefs::ui` (the client's `disabledSounds` list).
/// Notifications > Sounds keeps its existing ids.
pub fn setting_id(name: &str) -> String {
    match name {
        "message1" => "snd-msg".into(),
        "call_ringing" => "snd-ring".into(),
        n => format!("snd-{n}"),
    }
}

#[derive(Clone, Default)]
struct Gates {
    prefs: Option<LocalPrefs>,
}

fn gates() -> MutexGuard<'static, Gates> {
    static G: Mutex<Gates> = Mutex::new(Gates { prefs: None });
    G.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Keep the gates in step with the settings (called wherever local prefs change).
pub fn sync_prefs(p: &LocalPrefs) {
    gates().prefs = Some(p.clone());
}

/// `NotificationSettingsStore.isSoundDisabled`.
pub fn is_disabled(name: &str) -> bool {
    let g = gates();
    let Some(p) = g.prefs.as_ref() else { return false };
    !p.notification_sounds || !p.flag(&setting_id(name), true)
}

/// Settings > Notifications "New message in the channel I'm currently in"
/// (`notifyMessagesInSelectedChannel`, default off), greyed out by "disable all".
pub fn notify_selected_channel() -> bool {
    let g = gates();
    g.prefs.as_ref().is_some_and(|p| p.notification_sounds && p.flag("snd-msg-cur", false))
}

/// Streamer Mode on with "Disable sounds" (default on).
fn streamer_mutes() -> bool {
    let g = gates();
    g.prefs.as_ref().is_some_and(|p| p.flag("streamer", false) && p.flag("st-sounds", true))
}

/// `min(outputVolume / 100 * volume, 1)`, `outputVolume` = Voice & Video > Output volume (0..=200).
fn scaled(output_volume: u32, volume: f32) -> f32 {
    (output_volume as f32 / 100. * volume).min(1.)
}

fn effective_volume(volume: f32) -> f32 {
    scaled(crate::voice::prefs::prefs().lock().output_volume, volume)
}

static MUTED_FOR_TESTS: AtomicBool = AtomicBool::new(cfg!(test));

/// Log of what played (tests and `PHOENIX_SOUND_LOG`).
fn log(name: &str, volume: f32, looped: bool) {
    if std::env::var_os("PHOENIX_SOUND_LOG").is_some() {
        eprintln!("phoenix-sound: {name} volume={volume:.3} loop={looped}");
    }
    #[cfg(test)]
    tests::PLAYED.with(|p| p.borrow_mut().push(name.to_owned()));
}

fn memory() -> &'static Mutex<HashMap<String, Arc<Pcm>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Arc<Pcm>>>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// Decoded sound (blocking: may read, extract or download the file).
fn pcm(name: &str) -> Option<Arc<Pcm>> {
    if let Some(p) = memory().lock().ok()?.get(name) {
        return Some(p.clone());
    }
    let p = Arc::new(assets::decode(&assets::load(name)?)?);
    memory().lock().ok()?.insert(name.to_owned(), p.clone());
    Some(p)
}

/// Start `name` under a fresh handle: right away when decoded, else on a loader thread.
fn start(name: &str, volume: f32, looped: bool) -> Option<Handle> {
    log(name, volume, looped);
    if MUTED_FOR_TESTS.load(Ordering::Relaxed) {
        return None;
    }
    let h = Handle::new(volume);
    let ready = memory().lock().ok().and_then(|m| m.get(name).cloned());
    match ready {
        Some(p) => effects::start(&h, p, looped),
        None => {
            let (n, h2) = (name.to_owned(), h.clone());
            let _ = std::thread::Builder::new().name("sound-load".into()).spawn(move || {
                if let Some(p) = pcm(&n) {
                    effects::start(&h2, p, looped);
                }
            });
        }
    }
    Some(h)
}

fn throttled(name: &str) -> bool {
    if !name.contains("message") {
        return false;
    }
    static LAST: Mutex<Option<Instant>> = Mutex::new(None);
    let mut last = LAST.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let now = Instant::now();
    if last.is_some_and(|t| now.duration_since(t) < Duration::from_secs(1)) {
        return true;
    }
    *last = Some(now);
    false
}

/// `SoundUtils.playSound(name, volume)`; callers check [`is_disabled`] first like the client.
pub fn play(name: &str, volume: f32) -> Option<Handle> {
    if streamer_mutes() || throttled(name) {
        return None;
    }
    start(name, effective_volume(volume), false)
}

/// A looping cue (`sound.loop()`: incoming / outgoing call rings).
pub fn play_loop(name: &str, volume: f32) -> Option<Handle> {
    if streamer_mutes() {
        return None;
    }
    start(name, effective_volume(volume), true)
}

/// The "Sound preview" link of a settings row (`aQ`: stops the previous preview).
pub fn preview(name: &str) {
    static CUR: Mutex<Option<Handle>> = Mutex::new(None);
    let mut cur = CUR.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(h) = cur.take() {
        h.stop();
    }
    *cur = start(name, effective_volume(1.), false);
}

/// Decode the given sounds into memory (loader thread), so their first cue is instant.
pub fn warm(names: &'static [&'static str]) {
    let _ = std::thread::Builder::new().name("sound-warm".into()).spawn(move || {
        for n in names {
            let _ = pcm(n);
        }
    });
}

/// Start the background asset extraction.
pub fn init() {
    assets::init();
}

// ------------------------------------------------------------ messages

/// What the message sound rule needs besides the notification prefs.
#[derive(Clone, Copy, Debug, Default)]
pub struct MessageCtx {
    /// The window has focus (`document.hasFocus()` / WINDOW_FOCUS).
    pub focused: bool,
    /// Channel open in the chat view.
    pub selected: Option<u64>,
    /// Own status is Do Not Disturb.
    pub dnd: bool,
    /// Notifications "quiet mode" (settings proto `notifications.quiet_mode`).
    pub quiet: bool,
}

static CTX: Mutex<MessageCtx> = Mutex::new(MessageCtx { focused: true, selected: None, dnd: false, quiet: false });

fn ctx() -> MutexGuard<'static, MessageCtx> {
    CTX.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The main window gained / lost focus (read at render time).
pub fn set_focused(on: bool) {
    ctx().focused = on;
}

/// `MESSAGE_SUPPRESS_NOTIFICATIONS` (@silent).
const SUPPRESS_NOTIFICATIONS: u64 = 1 << 12;

/// What the client decides for MESSAGE_CREATE (NotificationStore): which sounds play.
/// `notify` = the mute / level part of `shouldNotify` ([`crate::notifications::should_notify`]);
/// `blocked` = author blocked or ignored.
/// `allow_any` = the channel is not muted / set to "nothing" (`!allowNoMessages`).
pub fn message_sounds(c: &MessageCtx, f: &crate::notifications::MessageFacts, flags: u64, blocked: bool, notify: bool, allow_any: bool) -> Vec<&'static str> {
    // `K`: not me, not blocked, not DND, not quiet mode, not muted / "nothing".
    let base = !f.from_me && !blocked && !c.dnd && !c.quiet;
    let selected = c.selected == Some(f.channel);
    // `lx(message, channel, includeSelectedChannel = !focused)`.
    let p = base && flags & SUPPRESS_NOTIFICATIONS == 0 && notify && (!c.focused || !selected);
    // `kY`: the selected channel (only while it is not set to "nothing" / muted).
    let m = notify_selected_channel() && base && selected && allow_any;
    if !p && !m {
        return Vec::new();
    }
    // Both read the "New message" switch (`g = !isSoundDisabled("message1")`).
    let g = !is_disabled("message1");
    let mut out = Vec::new();
    if m {
        if g {
            out.push("message3");
        }
        if !c.focused {
            return out;
        }
    }
    if p && g {
        out.push("message1");
    }
    out
}

/// Gateway hook next to the desktop notification one.
pub fn on_message(prefs: &crate::notifications::NotifyPrefs, cache: &phoenix_api::Cache, me: phoenix_api::models::Snowflake, m: &phoenix_api::models::Message) {
    let blocked = cache.relationships().iter().any(|r| {
        matches!(r.kind, phoenix_api::models::RelationshipType::Blocked) && r.user.as_ref().map(|u| u.id).or(r.user_id).unwrap_or(r.id) == m.author.id
    });
    let f = crate::notifications::facts(cache, me, m);
    let notify = crate::notifications::should_notify(prefs, &f);
    // Probe with a direct mention: true unless muted or set to "nothing".
    let allow_any = crate::notifications::should_notify(prefs, &crate::notifications::MessageFacts { mentions_me: true, ..f });
    let c = *ctx();
    for s in message_sounds(&c, &f, m.flags, blocked, notify, allow_any) {
        play(s, VOLUME);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notifications::MessageFacts;
    use std::cell::RefCell;

    thread_local! {
        pub static PLAYED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    /// The gates are process-wide: tests touching them run one at a time.
    static SERIAL: Mutex<()> = Mutex::new(());

    fn prefs(f: impl FnOnce(&mut LocalPrefs)) {
        let mut p = LocalPrefs::default();
        f(&mut p);
        sync_prefs(&p);
    }

    #[test]
    fn gates_and_message_rules() {
        let _g = SERIAL.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        prefs(|_| {});
        let f = MessageFacts { channel: 7, ..Default::default() };
        let c = MessageCtx { focused: true, selected: Some(1), ..Default::default() };
        assert_eq!(message_sounds(&c, &f, 0, false, true, true), ["message1"]);
        assert!(message_sounds(&c, &f, SUPPRESS_NOTIFICATIONS, false, true, true).is_empty(), "@silent");
        assert!(message_sounds(&c, &f, 0, true, true, true).is_empty(), "blocked");
        assert!(message_sounds(&c, &f, 0, false, false, true).is_empty(), "muted / not notified");
        assert!(message_sounds(&MessageCtx { dnd: true, ..c }, &f, 0, false, true, true).is_empty(), "DND");
        assert!(message_sounds(&MessageCtx { quiet: true, ..c }, &f, 0, false, true, true).is_empty(), "quiet mode");
        assert!(message_sounds(&c, &MessageFacts { from_me: true, ..f }, 0, false, true, true).is_empty());
        // Viewing the channel with focus: nothing; unfocused: the normal sound.
        let viewing = MessageCtx { selected: Some(7), ..c };
        assert!(message_sounds(&viewing, &f, 0, false, true, true).is_empty());
        assert_eq!(message_sounds(&MessageCtx { focused: false, ..viewing }, &f, 0, false, true, true), ["message1"]);
        // "New message" switch off, then "disable all".
        prefs(|p| {
            p.ui.insert("snd-msg".into(), 0);
        });
        assert!(message_sounds(&c, &f, 0, false, true, true).is_empty());
        prefs(|p| p.notification_sounds = false);
        assert!(is_disabled("deafen") && is_disabled("message1"));
        // Selected-channel sound.
        prefs(|p| {
            p.ui.insert("snd-msg-cur".into(), 1);
        });
        assert_eq!(message_sounds(&viewing, &f, 0, false, true, true), ["message3"]);
        assert_eq!(message_sounds(&MessageCtx { focused: false, ..viewing }, &f, 0, false, true, true), ["message3"]);
        prefs(|_| {});
    }

    #[test]
    fn streamer_mode_and_volume() {
        let _g = SERIAL.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        prefs(|p| {
            p.ui.insert("streamer".into(), 1);
        });
        PLAYED.with(|p| p.borrow_mut().clear());
        assert!(play("deafen", VOLUME).is_none());
        assert!(PLAYED.with(|p| p.borrow().is_empty()), "streamer mode mutes");
        prefs(|_| {});
        assert!((scaled(200, 0.4) - 0.8).abs() < 1e-6);
        assert_eq!(scaled(200, 1.), 1.);
        assert!((scaled(50, 0.4) - 0.2).abs() < 1e-6);
        play("mute", VOLUME);
        assert_eq!(PLAYED.with(|p| p.borrow().clone()), ["mute"]);
    }

    #[test]
    fn setting_ids() {
        assert_eq!(setting_id("message1"), "snd-msg");
        assert_eq!(setting_id("call_ringing"), "snd-ring");
        assert_eq!(setting_id("user_join"), "snd-user_join");
        assert_eq!(VOICE_LIST.len(), 22);
    }
}
