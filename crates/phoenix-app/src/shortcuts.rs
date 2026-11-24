//! Global keyboard shortcuts of the official client.
//!
//! The list and the bindings come from the live 2026 client's keyboard
//! shortcuts modal (Ctrl+/) and its keybind table (`binds:[...]` entries in
//! the local web bundles); see docs/parity/shortcuts.md. Navigation follows the
//! client's behaviour: Alt+Up/Down cycle through the sidebar's channels with
//! wrap-around, Alt+Shift skips to unread channels, Ctrl+Shift+Alt to channels
//! with mentions, Ctrl+Alt+Up/Down cycle through the server rail (Home first).

use std::cell::RefCell;

use gpui::{
    App, ClipboardItem, Context, FontWeight, InteractiveElement, IntoElement, KeyBinding,
    ParentElement, Pixels, SharedString, Stateful, Styled, actions, div, prelude::*, px,
};

pub mod message;

use crate::{
    app::{CONTEXT, PhoenixApp},
    model::{ChannelKind, Id},
    theme::{c, generated::idx, tok, z},
    ui,
};

actions!(
    phoenix_shortcuts,
    [
        ToggleHelp,
        PrevChannel,
        NextChannel,
        PrevUnread,
        NextUnread,
        PrevMention,
        NextMention,
        PrevGuild,
        NextGuild,
        ToggleLastGuild,
        HistoryBack,
        HistoryForward,
        ToggleInbox,
        TogglePins,
        UploadFile,
        FocusSearch,
        ToggleMute,
        ToggleDeafen,
        ToggleEmoji,
        ToggleGif,
        ToggleSticker,
        PageUp,
        PageDown,
        JumpOldestUnread,
        MarkServerRead,
        CreateGuild,
        CreateGroupDm,
        CopyChannelLink,
        GetHelp,
        MarkInboxRead,
        CallDm,
        JumpToCall,
    ]
);

/// One row of the shortcuts modal: GPUI keystrokes, the client's group and
/// the i18n key of its description.
pub struct Shortcut {
    pub keys: &'static [&'static str],
    pub group: Group,
    pub label: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Group {
    Navigation,
    Voice,
    Chat,
    Misc,
}

impl Group {
    fn title(self) -> SharedString {
        match self {
            Group::Navigation => crate::t!("shortcuts.group.navigation"),
            Group::Voice => crate::t!("shortcuts.group.voice"),
            Group::Chat => crate::t!("shortcuts.group.chat"),
            Group::Misc => crate::t!("shortcuts.group.misc"),
        }
    }
}

macro_rules! sc {
    ($group:ident, $label:literal, [$($k:literal),+]) => {
        Shortcut { keys: &[$($k),+], group: Group::$group, label: $label }
    };
}

/// Implemented shortcuts, in the order of the client's modal.
pub const SHORTCUTS: &[Shortcut] = &[
    sc!(Navigation, "shortcuts.guilds", ["ctrl-alt-up", "ctrl-alt-down"]),
    sc!(Navigation, "shortcuts.channels", ["alt-up", "alt-down"]),
    sc!(Navigation, "shortcuts.history", ["alt-left", "alt-right"]),
    sc!(Navigation, "shortcuts.unread", ["alt-shift-up", "alt-shift-down"]),
    sc!(Navigation, "shortcuts.mentions", ["ctrl-shift-alt-up", "ctrl-shift-alt-down"]),
    sc!(Navigation, "shortcuts.jump_call", ["ctrl-shift-alt-v"]),
    sc!(Navigation, "shortcuts.last_guild", ["ctrl-alt-right"]),
    sc!(Navigation, "shortcuts.switcher", ["ctrl-k"]),
    sc!(Navigation, "shortcuts.create_guild", ["ctrl-shift-n"]),
    sc!(Voice, "shortcuts.mute", ["ctrl-shift-m"]),
    sc!(Voice, "shortcuts.deafen", ["ctrl-shift-d"]),
    sc!(Voice, "shortcuts.call", ["ctrl-'"]),
    sc!(Chat, "shortcuts.mark_server_read", ["shift-escape"]),
    sc!(Chat, "shortcuts.mark_channel_read", ["escape"]),
    sc!(Chat, "shortcuts.group_dm", ["ctrl-shift-t"]),
    sc!(Chat, "shortcuts.pins", ["ctrl-p"]),
    sc!(Chat, "shortcuts.inbox", ["ctrl-i"]),
    sc!(Chat, "shortcuts.inbox_read", ["ctrl-shift-e"]),
    sc!(Chat, "shortcuts.members", ["ctrl-u"]),
    sc!(Chat, "shortcuts.emoji", ["ctrl-e"]),
    sc!(Chat, "shortcuts.gif", ["ctrl-g"]),
    sc!(Chat, "shortcuts.sticker", ["ctrl-s"]),
    sc!(Chat, "shortcuts.scroll", ["pageup", "pagedown"]),
    sc!(Chat, "shortcuts.oldest_unread", ["shift-pageup"]),
    sc!(Chat, "shortcuts.upload", ["ctrl-shift-u"]),
    sc!(Chat, "shortcuts.copy_link", ["ctrl-shift-l"]),
    sc!(Misc, "shortcuts.help", ["ctrl-shift-h"]),
    sc!(Misc, "shortcuts.search", ["ctrl-f"]),
    sc!(Misc, "shortcuts.modal", ["ctrl-/"]),
];

pub fn bind_keys(cx: &mut App) {
    let ctx = Some(CONTEXT);
    cx.bind_keys([
        KeyBinding::new("ctrl-/", ToggleHelp, ctx),
        KeyBinding::new("ctrl-shift-/", ToggleHelp, ctx),
        KeyBinding::new("alt-up", PrevChannel, ctx),
        KeyBinding::new("alt-down", NextChannel, ctx),
        KeyBinding::new("alt-shift-up", PrevUnread, ctx),
        KeyBinding::new("alt-shift-down", NextUnread, ctx),
        KeyBinding::new("ctrl-shift-alt-up", PrevMention, ctx),
        KeyBinding::new("ctrl-shift-alt-down", NextMention, ctx),
        // Client: ["mod+alt+up","mod+shift+[","ctrl+shift+tab"] and the mirror.
        KeyBinding::new("ctrl-alt-up", PrevGuild, ctx),
        KeyBinding::new("ctrl-shift-[", PrevGuild, ctx),
        KeyBinding::new("ctrl-shift-tab", PrevGuild, ctx),
        KeyBinding::new("ctrl-alt-down", NextGuild, ctx),
        KeyBinding::new("ctrl-shift-]", NextGuild, ctx),
        KeyBinding::new("ctrl-tab", NextGuild, ctx),
        KeyBinding::new("ctrl-alt-right", ToggleLastGuild, ctx),
        KeyBinding::new("alt-left", HistoryBack, ctx),
        KeyBinding::new("alt-right", HistoryForward, ctx),
        KeyBinding::new("ctrl-i", ToggleInbox, ctx),
        KeyBinding::new("ctrl-p", TogglePins, ctx),
        KeyBinding::new("ctrl-shift-u", UploadFile, ctx),
        KeyBinding::new("ctrl-f", FocusSearch, ctx),
        KeyBinding::new("ctrl-shift-f", FocusSearch, ctx),
        KeyBinding::new("ctrl-shift-m", ToggleMute, ctx),
        KeyBinding::new("ctrl-shift-d", ToggleDeafen, ctx),
        KeyBinding::new("ctrl-e", ToggleEmoji, ctx),
        KeyBinding::new("ctrl-g", ToggleGif, ctx),
        KeyBinding::new("ctrl-s", ToggleSticker, ctx),
        KeyBinding::new("pageup", PageUp, ctx),
        KeyBinding::new("pagedown", PageDown, ctx),
        KeyBinding::new("shift-pageup", JumpOldestUnread, ctx),
        KeyBinding::new("shift-escape", MarkServerRead, ctx),
        KeyBinding::new("ctrl-shift-n", CreateGuild, ctx),
        KeyBinding::new("ctrl-shift-t", CreateGroupDm, ctx),
        KeyBinding::new("ctrl-shift-l", CopyChannelLink, ctx),
        KeyBinding::new("ctrl-shift-h", GetHelp, ctx),
        KeyBinding::new("f1", GetHelp, ctx),
        KeyBinding::new("ctrl-shift-e", MarkInboxRead, ctx),
        KeyBinding::new("ctrl-'", CallDm, ctx),
        KeyBinding::new("ctrl-shift-'", CallDm, ctx),
        KeyBinding::new("ctrl-shift-alt-v", JumpToCall, ctx),
    ]);
}

// ------------------------------------------------------------ pure helpers

/// Item `step` places away from `cur` in `list`, wrapping around; the first
/// (or last, going up) item when `cur` is not in the list.
pub fn cycle<T: Copy + PartialEq>(list: &[T], cur: Option<T>, step: i32) -> Option<T> {
    if list.is_empty() {
        return None;
    }
    let n = list.len() as i32;
    let i = match cur.and_then(|c| list.iter().position(|x| *x == c)) {
        Some(i) => (i as i32 + step).rem_euclid(n),
        None if step < 0 => n - 1,
        None => 0,
    };
    Some(list[i as usize])
}

/// Next item after `cur` (in `step` direction, wrapping, never `cur` itself)
/// that satisfies `pred`.
pub fn seek<T: Copy + PartialEq>(list: &[T], cur: Option<T>, step: i32, pred: impl Fn(T) -> bool) -> Option<T> {
    let n = list.len() as i32;
    let start = cur.and_then(|c| list.iter().position(|x| *x == c)).map(|i| i as i32);
    (1..=n)
        .map(|k| match start {
            Some(s) => (s + step.signum() * k).rem_euclid(n),
            None if step < 0 => n - k,
            None => k - 1,
        })
        .map(|i| list[i as usize])
        .find(|x| Some(*x) != cur && pred(*x))
}

/// Server rail order: Home, guilds outside folders, then folder contents.
pub fn rail_order(guilds: &[Id], folders: &[Vec<Id>]) -> Vec<Option<Id>> {
    let in_folder = |g: &Id| folders.iter().any(|f| f.contains(g));
    let mut out = vec![None];
    out.extend(guilds.iter().filter(|g| !in_folder(g)).map(|g| Some(*g)));
    for f in folders {
        out.extend(f.iter().filter(|g| guilds.contains(g)).map(|g| Some(*g)));
    }
    out
}

/// Back/forward stack of visited channels (the client's route history).
#[derive(Default, Debug)]
pub struct History {
    pub past: Vec<Id>,
    pub future: Vec<Id>,
    pub current: Option<Id>,
}

impl History {
    /// Records a navigation to `ch` (no-op when it is already current).
    pub fn visit(&mut self, ch: Id) {
        if self.current == Some(ch) {
            return;
        }
        if let Some(c) = self.current.replace(ch) {
            self.past.push(c);
            if self.past.len() > 50 {
                self.past.remove(0);
            }
        }
        self.future.clear();
    }

    pub fn back(&mut self) -> Option<Id> {
        let to = self.past.pop()?;
        if let Some(c) = self.current.replace(to) {
            self.future.push(c);
        }
        Some(to)
    }

    pub fn forward(&mut self) -> Option<Id> {
        let to = self.future.pop()?;
        if let Some(c) = self.current.replace(to) {
            self.past.push(c);
        }
        Some(to)
    }
}

#[derive(Default)]
struct State {
    help_open: bool,
    keys: Option<gpui::Subscription>,
    history: History,
    last_guild: Option<Id>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::default();
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

pub fn help_open() -> bool {
    with(|s| s.help_open)
}

/// Escape closes the modal first. True when consumed.
pub fn on_escape() -> bool {
    with(|s| std::mem::take(&mut s.help_open))
}

// ------------------------------------------------------------ app glue

fn texty(kind: ChannelKind) -> bool {
    !matches!(kind, ChannelKind::Voice | ChannelKind::Stage | ChannelKind::Category)
}

impl PhoenixApp {
    /// Channels Alt+Up/Down walk through: the sidebar's text channels and
    /// threads (plus the joined voice channel), or the DM list at Home.
    fn shortcut_channels(&self) -> Vec<Id> {
        let Some(g) = self.selected_guild else { return self.dms.clone() };
        let ok = |id: &Id| {
            self.channels.get(id).is_some_and(|c| c.guild == Some(g) && (texty(c.kind) || self.voice_channel == Some(*id)))
        };
        let rows: Vec<Id> = self
            .sidebar_rows
            .iter()
            .filter_map(|r| match r {
                crate::app::SidebarRow::Channel(id) | crate::app::SidebarRow::Thread(id) => Some(*id),
                _ => None,
            })
            .filter(ok)
            .collect();
        if rows.is_empty() { self.guild_channels.get(&g).into_iter().flatten().copied().filter(ok).collect() } else { rows }
    }

    fn shortcut_rail(&self) -> Vec<Option<Id>> {
        let guilds: Vec<Id> = self.guilds.iter().map(|g| g.id).collect();
        let folders: Vec<Vec<Id>> = self.nav.folders.iter().map(|f| f.guilds.clone()).collect();
        rail_order(&guilds, &folders)
    }

    /// Every navigable channel across the rail, current guild's order first.
    fn shortcut_all_channels(&self) -> Vec<Id> {
        let mut out = Vec::new();
        for g in self.shortcut_rail() {
            match g {
                _ if g == self.selected_guild => out.extend(self.shortcut_channels()),
                None => out.extend(self.dms.iter().copied()),
                Some(g) => out.extend(
                    self.guild_channels.get(&g).into_iter().flatten().copied().filter(|id| self.channels.get(id).is_some_and(|c| texty(c.kind))),
                ),
            }
        }
        out
    }

    fn goto_channel(&mut self, id: Id, cx: &mut Context<Self>) {
        // Showing the chat only: never joins voice (the client routes, it doesn't connect).
        self.open_channel_chat(id, cx);
        cx.notify();
    }

    fn step_channel(&mut self, step: i32, cx: &mut Context<Self>) {
        if let Some(id) = cycle(&self.shortcut_channels(), self.selected_channel, step) {
            self.goto_channel(id, cx);
        }
    }

    fn step_unread(&mut self, step: i32, mentions_only: bool, cx: &mut Context<Self>) {
        let list = self.shortcut_all_channels();
        let pred = |id: Id| {
            self.channels.get(&id).is_some_and(|c| if mentions_only { c.mentions > 0 } else { (c.unread && !c.muted) || c.mentions > 0 })
        };
        if let Some(id) = seek(&list, self.selected_channel, step, pred) {
            self.goto_channel(id, cx);
        }
    }

    fn step_guild(&mut self, step: i32, cx: &mut Context<Self>) {
        if let Some(g) = cycle(&self.shortcut_rail(), Some(self.selected_guild), step) {
            self.select_guild(g, cx);
        }
    }

    fn toggle_last_guild(&mut self, cx: &mut Context<Self>) {
        let target = match self.selected_guild {
            Some(g) => {
                with(|s| s.last_guild = Some(g));
                None
            }
            None => match with(|s| s.last_guild) {
                Some(g) => Some(g),
                None => return,
            },
        };
        self.select_guild(target, cx);
    }

    fn history_step(&mut self, back: bool, cx: &mut Context<Self>) {
        let to = with(|s| if back { s.history.back() } else { s.history.forward() });
        if let Some(id) = to.filter(|id| self.channels.contains_key(id)) {
            self.goto_channel(id, cx);
        }
    }

    fn page(&mut self, up: bool) {
        let h = self.message_list.viewport_bounds().size.height;
        let d: Pixels = if h > px(0.) { h - px(60.) } else { px(400.) };
        self.message_list.scroll_by(if up { -d } else { d });
    }

    fn copy_channel_link(&self, cx: &mut Context<Self>) {
        let Some(ch) = self.selected_channel else { return };
        let guild = self.channels.get(&ch).and_then(|c| c.guild).map_or("@me".to_string(), |g| g.0.to_string());
        cx.write_to_clipboard(ClipboardItem::new_string(format!("https://discord.com/channels/{guild}/{}", ch.0)));
    }
}

/// Registers every shortcut handler on the root element and records history.
pub fn handlers(app: &PhoenixApp, root: Stateful<gpui::Div>, cx: &mut Context<PhoenixApp>) -> Stateful<gpui::Div> {
    if let Some(ch) = app.selected_channel {
        with(|s| s.history.visit(ch));
    }
    if let Some(g) = app.selected_guild {
        with(|s| s.last_guild = Some(g));
    }
    if with(|s| s.keys.is_none()) {
        let weak = cx.entity().downgrade();
        let sub = cx.intercept_keystrokes(move |ev, window, cx| {
            let Some(app) = weak.upgrade() else { return };
            app.update(cx, |app, cx| {
                if app.shortcut_keystroke(&ev.keystroke, window, cx) {
                    cx.stop_propagation();
                }
            });
        });
        with(|s| s.keys = Some(sub));
    }
    use ui::pickers::{EmojiTarget, Tab};
    root.on_action(cx.listener(|_, _: &ToggleHelp, _, cx| {
        with(|s| s.help_open = !s.help_open);
        cx.notify();
    }))
    .on_action(cx.listener(|a, _: &PrevChannel, _, cx| a.step_channel(-1, cx)))
    .on_action(cx.listener(|a, _: &NextChannel, _, cx| a.step_channel(1, cx)))
    .on_action(cx.listener(|a, _: &PrevUnread, _, cx| a.step_unread(-1, false, cx)))
    .on_action(cx.listener(|a, _: &NextUnread, _, cx| a.step_unread(1, false, cx)))
    .on_action(cx.listener(|a, _: &PrevMention, _, cx| a.step_unread(-1, true, cx)))
    .on_action(cx.listener(|a, _: &NextMention, _, cx| a.step_unread(1, true, cx)))
    .on_action(cx.listener(|a, _: &PrevGuild, _, cx| a.step_guild(-1, cx)))
    .on_action(cx.listener(|a, _: &NextGuild, _, cx| a.step_guild(1, cx)))
    .on_action(cx.listener(|a, _: &ToggleLastGuild, _, cx| a.toggle_last_guild(cx)))
    .on_action(cx.listener(|a, _: &HistoryBack, _, cx| a.history_step(true, cx)))
    .on_action(cx.listener(|a, _: &HistoryForward, _, cx| a.history_step(false, cx)))
    .on_action(cx.listener(|a, _: &ToggleInbox, _, cx| a.toggle_inbox(cx)))
    .on_action(cx.listener(|a, _: &TogglePins, _, cx| a.toggle_pins(cx)))
    .on_action(cx.listener(|a, _: &UploadFile, _, cx| {
        if a.selected_channel.is_some() {
            a.prompt_attach(cx)
        }
    }))
    .on_action(cx.listener(|a, _: &FocusSearch, w, cx| ui::search::open(a, w, cx)))
    .on_action(cx.listener(|a, _: &ToggleMute, _, cx| {
        a.mic_muted = !a.mic_muted;
        crate::voice::sync_self_state(a);
        cx.notify();
    }))
    .on_action(cx.listener(|a, _: &ToggleDeafen, _, cx| {
        a.deafened = !a.deafened;
        crate::voice::sync_self_state(a);
        cx.notify();
    }))
    .on_action(cx.listener(|a, _: &ToggleEmoji, w, cx| ui::pickers::toggle(a, Tab::Emoji, EmojiTarget::Composer, w, cx)))
    .on_action(cx.listener(|a, _: &ToggleGif, w, cx| ui::pickers::toggle(a, Tab::Gif, EmojiTarget::Composer, w, cx)))
    .on_action(cx.listener(|a, _: &ToggleSticker, w, cx| ui::pickers::toggle(a, Tab::Sticker, EmojiTarget::Composer, w, cx)))
    .on_action(cx.listener(|a, _: &PageUp, _, cx| {
        a.page(true);
        cx.notify();
    }))
    .on_action(cx.listener(|a, _: &PageDown, _, cx| {
        a.page(false);
        cx.notify();
    }))
    .on_action(cx.listener(|a, _: &JumpOldestUnread, _, cx| {
        if let Some((ch, msg)) = a.chat.unread_marker {
            a.jump_to_message(ch, msg, cx);
        }
    }))
    .on_action(cx.listener(|a, _: &MarkServerRead, _, cx| {
        if let Some(g) = a.selected_guild {
            a.mark_nav_read(ui::nav_menus::Target::Guild(g), cx);
        }
    }))
    .on_action(cx.listener(|a, _: &CreateGuild, w, cx| a.open_join_server(w, cx)))
    .on_action(cx.listener(|a, _: &CreateGroupDm, _, cx| a.open_group_dm_picker(cx)))
    .on_action(cx.listener(|a, _: &CopyChannelLink, _, cx| a.copy_channel_link(cx)))
    .on_action(cx.listener(|a, _: &MarkInboxRead, _, cx| a.mark_inbox_read(cx)))
    .on_action(cx.listener(|a, _: &CallDm, _, cx| a.call_current_dm(cx)))
    .on_action(cx.listener(|a, _: &JumpToCall, _, cx| a.jump_to_call(cx)))
    .on_action(cx.listener(|_, _: &GetHelp, _, cx| cx.open_url("https://support.discord.com")))
}

// ------------------------------------------------------------ Ctrl+/ modal

/// "ctrl-shift-u" -> ["CTRL", "MAJ", "U"] (the client's key caps, localized).
pub fn key_caps(keys: &str) -> Vec<SharedString> {
    keys.split('-')
        .map(|k| match k {
            "ctrl" => crate::t!("shortcuts.key.ctrl"),
            "shift" => crate::t!("shortcuts.key.shift"),
            "alt" => "ALT".into(),
            "escape" => crate::t!("shortcuts.key.escape"),
            "pageup" => crate::t!("shortcuts.key.pageup"),
            "pagedown" => crate::t!("shortcuts.key.pagedown"),
            "up" => "↑".into(),
            "down" => "↓".into(),
            "left" => "←".into(),
            "right" => "→".into(),
            k => k.to_uppercase().into(),
        })
        .collect()
}

fn cap(text: SharedString) -> impl IntoElement {
    div()
        .px(z(6.))
        .h(z(22.))
        .min_w(z(22.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(z(4.))
        .bg(c(tok(idx::BACKGROUND_MOD_NORMAL)))
        .text_size(z(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(c(tok(idx::TEXT_DEFAULT)))
        .child(text)
}

pub fn render_help(_app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<gpui::AnyElement> {
    if !help_open() {
        return None;
    }
    let entity = cx.entity().downgrade();
    let close: ui::modal::Action = std::rc::Rc::new(move |_, cx| {
        with(|s| s.help_open = false);
        let _ = entity.update(cx, |_, cx| cx.notify());
    });
    let mut body = div().id("shortcuts-list").flex().flex_col().gap(z(4.)).max_h(px(560.)).overflow_y_scroll();
    for g in [Group::Navigation, Group::Voice, Group::Chat, Group::Misc] {
        body = body.child(
            div().pt(z(16.)).pb(z(4.)).text_size(z(12.)).font_weight(FontWeight::BOLD).text_color(c(tok(idx::TEXT_MUTED))).child(g.title()),
        );
        for s in SHORTCUTS.iter().filter(|s| s.group == g) {
            let mut keys = div().flex().gap(z(4.)).items_center();
            for (i, k) in s.keys.iter().enumerate() {
                if i > 0 {
                    keys = keys.child(div().w(z(8.)));
                }
                keys = keys.children(key_caps(k).into_iter().map(cap));
            }
            body = body.child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .py(z(6.))
                    .text_size(z(14.))
                    .text_color(c(tok(idx::TEXT_DEFAULT)))
                    .child(crate::t!(s.label))
                    .child(keys),
            );
        }
    }
    let card = div()
        .w(px(720.))
        .p(z(24.))
        .rounded(z(8.))
        .bg(c(tok(idx::MODAL_BACKGROUND)))
        .border_1()
        .border_color(c(tok(idx::BORDER_SUBTLE)))
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .gap(z(8.))
                .items_center()
                .text_size(z(20.))
                .font_weight(FontWeight::BOLD)
                .text_color(c(tok(idx::TEXT_STRONG)))
                .child(crate::t!("shortcuts.title"))
                .children(key_caps("ctrl-/").into_iter().map(cap)),
        )
        .child(body);
    Some(ui::modal::frame("shortcuts-modal", close, card))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_wraps_both_ways() {
        let l = [1, 2, 3];
        assert_eq!(cycle(&l, Some(3), 1), Some(1));
        assert_eq!(cycle(&l, Some(1), -1), Some(3));
        assert_eq!(cycle(&l, Some(2), 1), Some(3));
        assert_eq!(cycle(&l, None, 1), Some(1));
        assert_eq!(cycle(&l, None, -1), Some(3));
        assert_eq!(cycle::<i32>(&[], None, 1), None);
    }

    #[test]
    fn seek_skips_current_and_wraps() {
        let l = [1, 2, 3, 4];
        let odd = |x: i32| x % 2 == 1;
        assert_eq!(seek(&l, Some(1), 1, odd), Some(3));
        assert_eq!(seek(&l, Some(3), 1, odd), Some(1));
        assert_eq!(seek(&l, Some(1), -1, odd), Some(3));
        assert_eq!(seek(&l, Some(1), 1, |x| x == 1), None);
        assert_eq!(seek(&l, None, 1, |x| x > 1), Some(2));
    }

    #[test]
    fn rail_puts_home_first_and_folders_last() {
        let (a, b, c, d) = (Id(1), Id(2), Id(3), Id(4));
        let order = rail_order(&[a, b, c, d], &[vec![c, Id(99)], vec![a]]);
        assert_eq!(order, vec![None, Some(b), Some(d), Some(c), Some(a)]);
    }

    #[test]
    fn history_back_and_forward() {
        let mut h = History::default();
        h.visit(Id(1));
        h.visit(Id(2));
        h.visit(Id(2));
        h.visit(Id(3));
        assert_eq!(h.back(), Some(Id(2)));
        assert_eq!(h.back(), Some(Id(1)));
        assert_eq!(h.back(), None);
        assert_eq!(h.forward(), Some(Id(2)));
        h.visit(Id(5));
        assert_eq!(h.forward(), None);
        assert_eq!(h.back(), Some(Id(2)));
    }

    #[test]
    fn every_listed_key_is_bound() {
        let cx = gpui::TestAppContext::single();
        cx.update(|cx| {
            bind_keys(cx);
            crate::app::bind_keys(cx);
            for s in SHORTCUTS {
                for k in s.keys {
                    let ks = gpui::Keystroke::parse(k).unwrap();
                    let keymap = cx.key_bindings();
                    let keymap = keymap.borrow();
                    assert!(
                        keymap.bindings().any(|b| b.keystrokes().len() == 1 && b.keystrokes()[0].inner().modifiers == ks.modifiers && b.keystrokes()[0].inner().key == ks.key),
                        "{k} not bound"
                    );
                }
            }
        });
    }

    #[test]
    fn caps_are_localized_upper() {
        let caps = key_caps("ctrl-shift-u");
        assert_eq!(caps.len(), 3);
        assert_eq!(caps[2].as_ref(), "U");
    }
}
