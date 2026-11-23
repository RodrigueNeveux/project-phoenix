//! Window-level push-to-talk and keybind recording.
//!
//! The global binding goes through the XDG GlobalShortcuts portal (see
//! `phoenix_voice::hotkey`); these key handlers also drive push-to-talk while
//! the Phoenix window has focus, which works on every desktop.

use std::cell::{Cell, RefCell};

use gpui::{Context, KeyDownEvent, KeyUpEvent, Keystroke};

use super::VoiceControl;
use super::prefs::{InputMode, prefs, update};
use crate::app::PhoenixApp;

thread_local! {
    static RECORDING: Cell<bool> = const { Cell::new(false) };
    /// Key currently held for push-to-talk.
    static HELD: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub fn is_recording() -> bool {
    RECORDING.with(Cell::get)
}

pub fn set_recording(on: bool) {
    RECORDING.with(|r| r.set(on));
}

/// Human label for a keystroke (`ctrl-shift-a` -> `CTRL + SHIFT + A`).
pub fn label(keystroke: &str) -> String {
    keystroke
        .split('-')
        .filter(|p| !p.is_empty())
        .map(|p| match p {
            "cmd" | "super" | "platform" => "META".to_string(),
            "grave" => "`".to_string(),
            k => k.to_uppercase(),
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

fn stroke_string(k: &Keystroke) -> String {
    let mut parts = Vec::new();
    if k.modifiers.control {
        parts.push("ctrl");
    }
    if k.modifiers.alt {
        parts.push("alt");
    }
    if k.modifiers.shift {
        parts.push("shift");
    }
    if k.modifiers.platform {
        parts.push("super");
    }
    parts.push(k.key.as_str());
    parts.join("-")
}

fn base_key(binding: &str) -> &str {
    binding.rsplit('-').next().unwrap_or(binding)
}

/// Root capture-phase key down hook.
pub fn on_key_down(_app: &mut PhoenixApp, ev: &KeyDownEvent, cx: &mut Context<PhoenixApp>) {
    if is_recording() {
        if ev.keystroke.key == "escape" {
            set_recording(false);
        } else {
            let k = stroke_string(&ev.keystroke);
            set_recording(false);
            update(|p| p.ptt_key = Some(k), true);
        }
        cx.stop_propagation();
        cx.notify();
        return;
    }
    if ev.is_held {
        return;
    }
    let (mode, key) = {
        let p = prefs().lock();
        (p.input_mode, p.ptt_key.clone())
    };
    let Some(key) = key.filter(|_| mode == InputMode::PushToTalk) else { return };
    if stroke_string(&ev.keystroke) == key {
        HELD.with(|h| *h.borrow_mut() = Some(key));
        super::call::manager().apply_control(None, VoiceControl::SetPttPressed(true));
    }
}

/// Root capture-phase key up hook.
pub fn on_key_up(_app: &mut PhoenixApp, ev: &KeyUpEvent, _cx: &mut Context<PhoenixApp>) {
    let released = HELD.with(|h| {
        let mut h = h.borrow_mut();
        let hit = h.as_deref().is_some_and(|k| base_key(k) == ev.keystroke.key);
        if hit {
            *h = None;
        }
        hit
    });
    if released {
        super::call::manager().apply_control(None, VoiceControl::SetPttPressed(false));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_matching() {
        assert_eq!(label("ctrl-shift-a"), "CTRL + SHIFT + A");
        assert_eq!(label("grave"), "`");
        assert_eq!(base_key("ctrl-shift-a"), "a");
        let k = Keystroke::parse("ctrl-a").unwrap();
        assert_eq!(stroke_string(&k), "ctrl-a");
    }
}
