//! Keystroke-level shortcuts: the focused-message combos (E, R, P, +, F,
//! Backspace, Ctrl+C, Alt+Enter, Shift+F10), type-to-focus the composer,
//! Ctrl+1..9, and letting the app-wide combos through the focused composer
//! like the client's textarea does.

use std::cell::Cell;

use gpui::{App, Context, Keystroke, Window, point};

use crate::{
    app::{PhoenixApp, Screen},
    model::Id,
    ui::chat::state::EmojiTarget,
};

thread_local! {
    static FOCUSED: Cell<Option<(Id, Id)>> = const { Cell::new(None) };
}

/// Message with keyboard focus (channel, message), drawn like the hovered row.
pub fn focused() -> Option<(Id, Id)> {
    FOCUSED.with(Cell::get)
}

fn set_focused(v: Option<(Id, Id)>) {
    FOCUSED.with(|f| f.set(v));
}

/// Text-to-speech through the system's speech-dispatcher (`spd-say`), or
/// espeak-ng as a fallback. False when neither is installed or `text` is empty.
pub fn speak(text: &str) -> bool {
    if text.trim().is_empty() {
        return false;
    }
    let Some(bin) = tts_binary() else { return false };
    std::process::Command::new(bin)
        .arg("--")
        .arg(text)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_ok()
}

fn tts_binary() -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    ["spd-say", "espeak-ng"]
        .iter()
        .find_map(|b| std::env::split_paths(&path).map(|d| d.join(b)).find(|p| p.is_file()))
}

/// Message `step` rows away from `cur` in `ids`, clamped (no wrap, like the
/// client's list navigation). `None` cur starts from the newest message.
pub fn step_in(ids: &[Id], cur: Option<Id>, step: i32) -> Option<Id> {
    let last = ids.len().checked_sub(1)? as i32;
    let i = match cur.and_then(|c| ids.iter().position(|x| *x == c)) {
        Some(i) => (i as i32 + step).clamp(0, last),
        None => last,
    };
    Some(ids[i as usize])
}

/// Combos the composer must not swallow (the client binds them globally).
/// Text-editing keys (arrows, Home/End, Ctrl+A/C/V/X/Z…) stay with the input;
/// Ctrl+E/G/F are already routed by the pickers' interceptor.
pub fn passes_through_composer(ks: &Keystroke) -> bool {
    let m = &ks.modifiers;
    let k = ks.key.as_str();
    if matches!(k, "pageup" | "pagedown") {
        return !m.control && !m.alt;
    }
    if m.alt && matches!(k, "up" | "down" | "left" | "right") {
        return true;
    }
    if m.control && m.alt {
        return true;
    }
    if m.control && m.shift {
        return !matches!(k, "z" | "left" | "right" | "home" | "end" | "up" | "down" | "v");
    }
    m.control && matches!(k, "i" | "p" | "u" | "k" | "s" | "/" | "," | "tab")
}

/// Ctrl+N rail target: 1 = Home, N = (N-1)th server in rail order.
pub fn digit_target(rail: &[Option<Id>], key: &str) -> Option<Option<Id>> {
    let n: usize = key.parse().ok().filter(|n| (1..=9).contains(n))?;
    rail.get(n - 1).copied()
}

fn composer_focused(app: &PhoenixApp, window: &Window, cx: &App) -> bool {
    gpui::Focusable::focus_handle(app.composer.read(cx), cx).is_focused(window)
}

fn focus_composer(app: &PhoenixApp, window: &mut Window, cx: &mut Context<PhoenixApp>) {
    app.composer.update(cx, |s, cx| s.focus(window, cx));
}

/// Dispatches the app action bound to `ks` (single-keystroke bindings from
/// our own namespaces). True when one was found.
fn dispatch_global(ks: &Keystroke, window: &mut Window, cx: &mut App) -> bool {
    let action = {
        let keymap = cx.key_bindings();
        let keymap = keymap.borrow();
        keymap
            .bindings()
            .rev()
            .find(|b| {
                let s = b.keystrokes();
                s.len() == 1
                    && s[0].inner().key == ks.key
                    && s[0].inner().modifiers == ks.modifiers
                    && b.action().name().starts_with("phoenix")
            })
            .map(|b| b.action().boxed_clone())
    };
    match action {
        Some(a) => {
            window.dispatch_action(a, cx);
            true
        }
        None => false,
    }
}

impl PhoenixApp {
    fn message_ids(&self, ch: Id) -> Vec<Id> {
        self.messages.get(&ch).map(|m| m.msgs.iter().map(|m| m.id).collect()).unwrap_or_default()
    }

    /// Shift+Tab from the composer / Up when there is nothing to edit: focus
    /// the newest message.
    pub fn focus_last_message(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(ch) = self.selected_channel else { return false };
        let Some(id) = step_in(&self.message_ids(ch), None, 0) else { return false };
        set_focused(Some((ch, id)));
        window.focus(&self.focus, cx);
        self.message_list.scroll_to_reveal_item(self.messages[&ch].index_of(id).unwrap_or(0) + 1);
        cx.notify();
        true
    }

    /// Keys while a message has focus. True when consumed.
    fn focused_message_key(&mut self, ch: Id, id: Id, ks: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let m = &ks.modifiers;
        let plain = m.number_of_modifiers() == 0;
        let own = self.messages.get(&ch).and_then(|l| l.msgs.iter().find(|x| x.id == id)).is_some_and(|x| x.author == self.me.id);
        match ks.key.as_str() {
            "up" | "down" if plain => {
                let to = step_in(&self.message_ids(ch), Some(id), if ks.key == "up" { -1 } else { 1 });
                if let Some(to) = to {
                    set_focused(Some((ch, to)));
                    if let Some(ix) = self.messages.get(&ch).and_then(|l| l.index_of(to)) {
                        self.message_list.scroll_to_reveal_item(ix + 1);
                    }
                }
            }
            "escape" if plain => {
                set_focused(None);
                focus_composer(self, window, cx);
            }
            "e" if plain && own => {
                set_focused(None);
                self.start_edit(id, window, cx);
            }
            "r" if plain => {
                set_focused(None);
                self.start_reply(id, window, cx);
            }
            "p" if plain => self.toggle_pin(ch, id, cx),
            "+" | "=" if !m.control && !m.alt => self.open_emoji_picker(EmojiTarget::Reaction { channel: ch, msg: id }, cx),
            "f" if plain => self.open_forward(ch, id, cx),
            "s" if plain => {
                let text = self.messages.get(&ch).and_then(|l| l.msgs.iter().find(|x| x.id == id)).map(|x| x.content.to_string());
                if !text.as_deref().is_some_and(speak) {
                    self.parity_toast(crate::t!("shortcuts.tts_unavailable"), cx);
                }
            }
            // Shift+Backspace skips the confirmation, like Shift+click on the trash icon.
            "backspace" if own && !m.control && !m.alt => self.request_delete(ch, id, m.shift, cx),
            "c" if m.control && !m.shift && !m.alt => self.copy_message_text(ch, id, cx),
            "enter" if m.alt && !m.control => self.mark_unread_from(ch, id, cx),
            "f10" if m.shift && !m.control => {
                let b = window.bounds();
                self.open_message_menu(ch, id, point(b.size.width / 2., b.size.height / 2.), cx);
            }
            _ => return false,
        }
        cx.notify();
        true
    }

    /// Routes one keystroke before normal dispatch. True when consumed.
    pub fn shortcut_keystroke(&mut self, ks: &Keystroke, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.screen != Screen::Main || self.settings.is_some() || self.switcher.is_some() {
            return false;
        }
        if crate::ui::incoming_call::on_key(self, ks, cx) {
            return true;
        }
        if crate::ui::easter::on_key(self, ks, cx) {
            return true;
        }
        let m = ks.modifiers;
        let in_composer = composer_focused(self, window, cx);
        // A control reached with Tab counts as the root (client: type-to-focus works from any button).
        let on_ring = crate::ui::focus_ring::owns_focus(window);
        let at_root = self.focus.is_focused(window) || on_ring;
        if let Some((ch, id)) = focused() {
            if !at_root || self.selected_channel != Some(ch) {
                set_focused(None);
            } else if self.focused_message_key(ch, id, ks, window, cx) {
                return true;
            }
        }
        // Ctrl+1..9: rail items (client: `mod+${n}`).
        if m.control && !m.shift && !m.alt
            && let Some(g) = digit_target(&self.shortcut_rail(), &ks.key)
        {
            self.select_guild(g, cx);
            return true;
        }
        if in_composer {
            if ks.key == "tab" && m.shift && !m.control && self.composer.read(cx).value().is_empty() {
                return self.focus_last_message(window, cx);
            }
            if passes_through_composer(ks) {
                return dispatch_global(ks, window, cx);
            }
            return false;
        }
        // Type anywhere: printable keys go to the composer (client: any-character → TEXTAREA_FOCUS).
        // Space on a focused control clicks it (GPUI keyboard click), it is not typed.
        if at_root && !m.control && !m.alt && !m.platform && self.selected_channel.is_some() && !(on_ring && ks.key == "space")
            && let Some(ch) = ks.key_char.as_ref().filter(|c| !c.chars().any(char::is_control))
        {
            set_focused(None);
            let ch = ch.clone();
            self.composer.update(cx, |s, cx| {
                s.focus(window, cx);
                // At the caret (replacing any selection), emitting Change like real typing.
                gpui::EntityInputHandler::replace_text_in_range(s, None, &ch, window, cx);
            });
            return true;
        }
        false
    }

    /// Ctrl+Shift+E: mark the top channel of Inbox > Unreads read (inbox open
    /// only; the tip of the empty state: "mark your top message as read").
    pub fn mark_inbox_read(&mut self, cx: &mut Context<Self>) {
        let i = &self.parity.inbox;
        if !i.open || i.tab != crate::commands::state::InboxTab::Unreads {
            return;
        }
        if let Some(id) = i.unreads.iter().find(|c| c.loaded).map(|c| c.entry.channel) {
            self.mark_unread_card_read(id, cx);
        }
    }

    /// Ctrl+Shift+Alt+N: the thread form for the focused message, else the
    /// "Nouveau fil" sidebar (standalone thread) for the open channel.
    pub fn create_thread_shortcut(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match focused() {
            Some((ch, id)) if self.channels.get(&ch).is_some_and(|c| c.guild.is_some()) => {
                set_focused(None);
                self.open_create_thread_from(ch, id, window, cx);
            }
            _ => {
                if let Some(ch) = self.selected_channel {
                    self.open_create_thread(ch, window, cx);
                }
            }
        }
    }

    /// Ctrl+': call the open DM's recipient.
    pub fn call_current_dm(&mut self, cx: &mut Context<Self>) {
        let Some(ch) = self.selected_channel.and_then(|c| self.channels.get(&c)) else { return };
        if ch.guild.is_some() {
            return;
        }
        if let Some(u) = ch.voice_users.first().copied() {
            self.open_call_ui(u, cx);
        }
    }

    /// Ctrl+Shift+Alt+V: show the voice channel we're connected to.
    pub fn jump_to_call(&mut self, cx: &mut Context<Self>) {
        if let Some(v) = self.voice_channel {
            self.open_channel_chat(v, cx);
            crate::ui::call_view::show(v);
            cx.notify();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_clamps_and_starts_at_newest() {
        let ids = [Id(1), Id(2), Id(3)];
        assert_eq!(step_in(&ids, None, 0), Some(Id(3)));
        assert_eq!(step_in(&ids, Some(Id(3)), 1), Some(Id(3)));
        assert_eq!(step_in(&ids, Some(Id(1)), -1), Some(Id(1)));
        assert_eq!(step_in(&ids, Some(Id(2)), -1), Some(Id(1)));
        assert_eq!(step_in(&[], None, 0), None);
    }

    #[test]
    fn composer_keeps_editing_keys() {
        let k = |s| Keystroke::parse(s).unwrap();
        assert!(passes_through_composer(&k("ctrl-i")));
        assert!(passes_through_composer(&k("alt-up")));
        assert!(passes_through_composer(&k("pageup")));
        assert!(passes_through_composer(&k("ctrl-shift-u")));
        assert!(passes_through_composer(&k("ctrl-alt-down")));
        assert!(!passes_through_composer(&k("ctrl-a")));
        assert!(!passes_through_composer(&k("ctrl-c")));
        assert!(!passes_through_composer(&k("up")));
        assert!(!passes_through_composer(&k("ctrl-shift-z")));
        assert!(!passes_through_composer(&k("shift-left")));
    }

    #[test]
    fn digits_map_to_rail() {
        let rail = [None, Some(Id(5)), Some(Id(6))];
        assert_eq!(digit_target(&rail, "1"), Some(None));
        assert_eq!(digit_target(&rail, "3"), Some(Some(Id(6))));
        assert_eq!(digit_target(&rail, "4"), None);
        assert_eq!(digit_target(&rail, "0"), None);
        assert_eq!(digit_target(&rail, "x"), None);
    }
}
