//! Root view: owns UI state, talks to the `Backend`, applies `AppEvent`s.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::{Datelike, Local, TimeZone};
use futures::StreamExt;
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, KeyBinding,
    ListAlignment, ListState, ParentElement, Render, SharedString, Styled, Subscription, Window,
    actions, div, prelude::*, px,
};
use gpui_component::input::{InputEvent, InputState};

use crate::{
    backend::{AppEvent, Backend, Credentials, EventSink, ReadyData},
    grouping,
    markdown::MentionResolver,
    model::*,
    richtext::{self, RenderedMessage},
    theme::{FONT_UI, Palette, c},
    ui,
};

actions!(phoenix, [QuickSwitcher, Escape, EditLast, ToggleMembers]);

pub const CONTEXT: &str = "PhoenixApp";

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-k", QuickSwitcher, Some(CONTEXT)),
        KeyBinding::new("escape", Escape, Some(CONTEXT)),
        KeyBinding::new("ctrl-u", ToggleMembers, Some(CONTEXT)),
    ]);
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Login,
    Main,
}

#[derive(Clone, Copy, Debug)]
pub enum SidebarRow {
    Category(Id),
    Channel(Id),
    VoiceUser { user: Id },
}

#[derive(Clone, Debug)]
pub enum MemberRow {
    Header(SharedString),
    Member(usize),
}

#[derive(Default)]
pub struct ChannelMessages {
    pub msgs: Vec<Message>,
    pub rendered: Vec<RenderedMessage>,
    pub loading: bool,
    pub loaded_once: bool,
    pub reached_start: bool,
}

pub struct Switcher {
    pub input: Entity<InputState>,
    pub results: Vec<Id>,
    pub _sub: Subscription,
}

pub struct PhoenixApp {
    pub backend: Arc<dyn Backend>,
    pub screen: Screen,
    pub focus: FocusHandle,

    // Login
    pub login_email: Entity<InputState>,
    pub login_password: Entity<InputState>,
    pub login_token: Entity<InputState>,
    pub login_with_token: bool,
    pub login_error: Option<SharedString>,
    pub logging_in: bool,

    // Data
    pub me: User,
    pub users: HashMap<Id, User>,
    pub guilds: Vec<Guild>,
    pub channels: HashMap<Id, Channel>,
    pub guild_channels: HashMap<Id, Vec<Id>>,
    pub dms: Vec<Id>,
    pub roles: HashMap<Id, Vec<Role>>,
    pub members: HashMap<Id, Vec<Member>>,
    pub messages: HashMap<Id, ChannelMessages>,

    // Selection
    pub selected_guild: Option<Id>,
    pub selected_channel: Option<Id>,
    pub last_channel: HashMap<Option<Id>, Id>,
    pub collapsed: HashSet<Id>,

    // Derived rows (rebuilt on change, not per frame)
    pub sidebar_rows: Vec<SidebarRow>,
    pub member_rows: Vec<MemberRow>,

    // Virtualized list states
    pub guild_list: ListState,
    pub sidebar_list: ListState,
    pub message_list: ListState,
    pub member_list: ListState,

    // Composer
    pub composer: Entity<InputState>,
    pub replying_to: Option<Id>,
    pub editing: Option<Id>,
    pub typing: HashMap<Id, Vec<(Id, Instant)>>,
    pub typing_label: Option<SharedString>,

    // Self state
    pub presence: Presence,
    pub mic_muted: bool,
    pub deafened: bool,
    pub voice_channel: Option<Id>,
    pub show_members: bool,

    pub switcher: Option<Switcher>,
    pub revealed: HashSet<(Id, usize)>,
    pub utc_offset: i64,
    _subs: Vec<Subscription>,
}

struct Resolver<'a> {
    users: &'a HashMap<Id, User>,
    channels: &'a HashMap<Id, Channel>,
}
impl MentionResolver for Resolver<'_> {
    fn user(&self, id: u64) -> Option<String> {
        self.users.get(&Id(id)).map(|u| u.display_name.to_string())
    }
    fn channel(&self, id: u64) -> Option<String> {
        self.channels.get(&Id(id)).map(|c| c.name.to_string())
    }
}

pub fn format_time_full(ts: i64) -> SharedString {
    let Some(dt) = Local.timestamp_opt(ts, 0).single() else { return "".into() };
    let today = Local::now().date_naive();
    let d = dt.date_naive();
    let t = dt.format("%-I:%M %p");
    if d == today {
        format!("Today at {t}").into()
    } else if today.pred_opt() == Some(d) {
        format!("Yesterday at {t}").into()
    } else {
        format!("{:02}/{:02}/{} {t}", d.month(), d.day(), d.year()).into()
    }
}

fn format_time_short(ts: i64) -> SharedString {
    Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|d| d.format("%-I:%M %p").to_string().into())
        .unwrap_or_default()
}

fn format_day(ts: i64) -> SharedString {
    Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|d| d.format("%B %-d, %Y").to_string().into())
        .unwrap_or_default()
}

impl PhoenixApp {
    pub fn new(backend: Arc<dyn Backend>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let login_email = cx.new(|cx| InputState::new(window, cx));
        let login_password = cx.new(|cx| InputState::new(window, cx).masked(true));
        let login_token = cx.new(|cx| InputState::new(window, cx).masked(true));
        let composer = cx.new(|cx| InputState::new(window, cx));

        let mut subs = Vec::new();
        subs.push(cx.subscribe_in(&composer, window, |this, _, ev: &InputEvent, window, cx| {
            match ev {
                InputEvent::PressEnter { shift: false, .. } => this.submit_composer(window, cx),
                InputEvent::Change => this.on_composer_change(cx),
                _ => {}
            }
        }));
        for input in [&login_email, &login_password, &login_token] {
            subs.push(cx.subscribe_in(input, window, |this, _, ev: &InputEvent, window, cx| {
                if let InputEvent::PressEnter { .. } = ev {
                    this.submit_login(window, cx);
                }
            }));
        }

        // Gateway event pump: backend threads -> channel -> UI thread.
        let (tx, mut rx) = futures::channel::mpsc::unbounded::<AppEvent>();
        let sink: EventSink = Arc::new(move |ev| {
            let _ = tx.unbounded_send(ev);
        });
        backend.set_event_sink(sink);
        cx.spawn(async move |this, cx| {
            while let Some(ev) = rx.next().await {
                if this.update(cx, |this, cx| this.handle_event(ev, cx)).is_err() {
                    break;
                }
            }
        })
        .detach();

        // Typing indicator expiry tick.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                if this.update(cx, |this, cx| this.expire_typing(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();

        let message_list = ListState::new(0, ListAlignment::Bottom, px(800.));
        let weak = cx.entity().downgrade();
        message_list.set_scroll_handler(move |ev: &gpui::ListScrollEvent, _w, cx: &mut App| {
            if ev.visible_range.start == 0 {
                let _ = weak.update(cx, |this, cx| this.load_older(cx));
            }
        });

        let utc_offset = Local::now().offset().local_minus_utc() as i64;
        Self {
            backend,
            screen: Screen::Login,
            focus: cx.focus_handle(),
            login_email,
            login_password,
            login_token,
            login_with_token: false,
            login_error: None,
            logging_in: false,
            me: User {
                id: Id(0),
                name: "".into(),
                display_name: "".into(),
                avatar: ImageRef::Default(0),
                bot: false,
            },
            users: HashMap::new(),
            guilds: Vec::new(),
            channels: HashMap::new(),
            guild_channels: HashMap::new(),
            dms: Vec::new(),
            roles: HashMap::new(),
            members: HashMap::new(),
            messages: HashMap::new(),
            selected_guild: None,
            selected_channel: None,
            last_channel: HashMap::new(),
            collapsed: HashSet::new(),
            sidebar_rows: Vec::new(),
            member_rows: Vec::new(),
            guild_list: ListState::new(0, ListAlignment::Top, px(200.)),
            sidebar_list: ListState::new(0, ListAlignment::Top, px(300.)),
            message_list,
            member_list: ListState::new(0, ListAlignment::Top, px(300.)),
            composer,
            replying_to: None,
            editing: None,
            typing: HashMap::new(),
            typing_label: None,
            presence: Presence::Online,
            mic_muted: false,
            deafened: false,
            voice_channel: None,
            show_members: true,
            switcher: None,
            revealed: HashSet::new(),
            utc_offset,
            _subs: subs,
        }
    }

    // ---------------------------------------------------------------- login

    pub fn submit_login(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.logging_in {
            return;
        }
        let creds = if self.login_with_token {
            Credentials::Token(self.login_token.read(cx).value().to_string())
        } else {
            Credentials::Password {
                login: self.login_email.read(cx).value().to_string(),
                password: self.login_password.read(cx).value().to_string(),
            }
        };
        self.logging_in = true;
        self.login_error = None;
        cx.notify();
        let fut = self.backend.login(creds);
        cx.spawn(async move |this, cx| {
            let res = fut.await;
            let _ = this.update(cx, |this, cx| {
                this.logging_in = false;
                match res {
                    Ok(ready) => this.apply_ready(ready, cx),
                    Err(e) => this.login_error = Some(e.0.into()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn apply_ready(&mut self, r: ReadyData, cx: &mut Context<Self>) {
        self.me = r.me;
        self.users = r.users.into_iter().map(|u| (u.id, u)).collect();
        self.guilds = r.guilds;
        for (g, roles) in r.roles {
            self.roles.insert(g, roles);
        }
        for ch in r.channels {
            if let Some(g) = ch.guild {
                self.guild_channels.entry(g).or_default().push(ch.id);
            }
            self.channels.insert(ch.id, ch);
        }
        for list in self.guild_channels.values_mut() {
            list.sort_by_key(|id| self.channels[id].position);
        }
        for ch in r.dm_channels {
            self.dms.push(ch.id);
            self.channels.insert(ch.id, ch);
        }
        // Home + separator + guilds + separator + add + discover
        self.guild_list.reset(self.guilds.len() + 5);
        self.screen = Screen::Main;
        let first = self.guilds.first().map(|g| g.id);
        self.select_guild(first, cx);
    }

    // ----------------------------------------------------------- selection

    pub fn select_guild(&mut self, guild: Option<Id>, cx: &mut Context<Self>) {
        self.selected_guild = guild;
        if let Some(g) = guild
            && let Some(gg) = self.guilds.iter_mut().find(|x| x.id == g)
        {
            gg.mentions = 0;
        }
        self.rebuild_sidebar();
        self.rebuild_members();
        self.backend.select_guild(guild).detach_ok();
        let target = self.last_channel.get(&guild).copied().or_else(|| match guild {
            Some(g) => self.guild_channels.get(&g).and_then(|l| {
                l.iter().copied().find(|id| {
                    matches!(self.channels[id].kind, ChannelKind::Text | ChannelKind::Announcement)
                        && self.channels[id].name.as_ref() == "general"
                })
                .or_else(|| {
                    l.iter().copied().find(|id| self.channels[id].kind == ChannelKind::Text)
                })
            }),
            None => self.dms.first().copied(),
        });
        if let Some(t) = target {
            self.select_channel(t, cx);
        }
        cx.notify();
    }

    pub fn select_channel(&mut self, id: Id, cx: &mut Context<Self>) {
        let Some(ch) = self.channels.get_mut(&id) else { return };
        if matches!(ch.kind, ChannelKind::Voice | ChannelKind::Stage) {
            self.join_voice(Some(id), cx);
            return;
        }
        ch.unread = false;
        ch.mentions = 0;
        let guild = ch.guild;
        if guild != self.selected_guild {
            self.selected_guild = guild;
            self.rebuild_sidebar();
            self.rebuild_members();
        }
        self.selected_channel = Some(id);
        self.last_channel.insert(guild, id);
        self.replying_to = None;
        self.editing = None;
        self.update_typing_label();
        self.backend.select_channel(id).detach_ok();
        let entry = self.messages.entry(id).or_default();
        self.message_list.reset(entry.msgs.len() + 1);
        if !entry.loaded_once && !entry.loading {
            entry.loading = true;
            let fut = self.backend.load_messages(id, None, 50);
            cx.spawn(async move |this, cx| {
                let res = fut.await;
                let _ = this.update(cx, |this, cx| {
                    let msgs = res.unwrap_or_default();
                    this.insert_history(id, msgs, false, cx);
                });
            })
            .detach();
        }
        cx.notify();
    }

    fn load_older(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected_channel else { return };
        let Some(entry) = self.messages.get_mut(&id) else { return };
        if entry.loading || entry.reached_start || !entry.loaded_once {
            return;
        }
        let before = entry.msgs.first().map(|m| m.id);
        entry.loading = true;
        let fut = self.backend.load_messages(id, before, 50);
        cx.spawn(async move |this, cx| {
            let res = fut.await;
            let _ = this.update(cx, |this, cx| {
                this.insert_history(id, res.unwrap_or_default(), true, cx)
            });
        })
        .detach();
    }

    fn resolver(&self) -> Resolver<'_> {
        Resolver { users: &self.users, channels: &self.channels }
    }

    fn render_msg(&self, m: &Message, prev: Option<&Message>) -> RenderedMessage {
        let revealed = |i: usize| self.revealed.contains(&(m.id, i));
        let (parsed, blocks, mentions_me, preview) =
            richtext::build(&m.content, &self.resolver(), self.me.id, &revealed);
        RenderedMessage {
            group_start: grouping::starts_group(prev, m, self.utc_offset),
            day_divider: (prev.is_none() || grouping::day_divider(prev, m, self.utc_offset))
                .then(|| format_day(m.timestamp)),
            time_full: format_time_full(m.timestamp),
            time_short: format_time_short(m.timestamp),
            blocks,
            mentions_me,
            preview,
            parsed,
        }
    }

    fn insert_history(&mut self, ch: Id, page: Vec<Message>, older: bool, cx: &mut Context<Self>) {
        let mut entry = self.messages.remove(&ch).unwrap_or_default();
        entry.loading = false;
        entry.loaded_once = true;
        if page.len() < 50 {
            entry.reached_start = true;
        }
        let n = page.len();
        let mut all = page;
        if older {
            all.extend(std::mem::take(&mut entry.msgs));
        }
        // Re-render only the new rows plus the first old row (its grouping may change).
        let rendered: Vec<RenderedMessage> = if older {
            let mut r: Vec<_> = (0..n.min(all.len()))
                .map(|i| self.render_msg(&all[i], i.checked_sub(1).map(|p| &all[p])))
                .collect();
            let mut old = std::mem::take(&mut entry.rendered);
            if let Some(first) = old.first_mut()
                && n < all.len()
            {
                *first = self.render_msg(&all[n], n.checked_sub(1).map(|p| &all[p]));
            }
            r.extend(old);
            r
        } else {
            (0..all.len())
                .map(|i| self.render_msg(&all[i], i.checked_sub(1).map(|p| &all[p])))
                .collect()
        };
        entry.msgs = all;
        entry.rendered = rendered;
        let len = entry.msgs.len();
        self.messages.insert(ch, entry);
        if self.selected_channel == Some(ch) {
            if older {
                self.message_list.splice(1..1, n);
                if n < len {
                    // The previously-first message may have changed grouping.
                    self.message_list.splice(n + 1..n + 2, 1);
                }
            } else {
                self.message_list.reset(len + 1);
            }
        }
        cx.notify();
    }

    fn append_message(&mut self, m: Message, cx: &mut Context<Self>) {
        let ch = m.channel;
        let selected = self.selected_channel == Some(ch);
        let mention = m.content.contains(&format!("<@{}>", self.me.id.0));
        if let Some(t) = self.typing.get_mut(&ch) {
            t.retain(|(u, _)| *u != m.author);
        }
        let Some(mut entry) = self.messages.remove(&ch) else {
            if let Some(c) = self.channels.get_mut(&ch) {
                c.unread = true;
                if mention {
                    c.mentions += 1;
                }
            }
            return;
        };
        if entry.msgs.iter().any(|x| x.id == m.id) {
            self.messages.insert(ch, entry);
            return;
        }
        let r = self.render_msg(&m, entry.msgs.last());
        entry.msgs.push(m);
        entry.rendered.push(r);
        let len = entry.msgs.len();
        self.messages.insert(ch, entry);
        if selected {
            self.message_list.splice(len..len, 1);
        } else if let Some(c) = self.channels.get_mut(&ch) {
            c.unread = true;
        }
        self.update_typing_label();
        cx.notify();
    }

    fn replace_message(&mut self, m: Message, cx: &mut Context<Self>) {
        let ch = m.channel;
        let Some(mut entry) = self.messages.remove(&ch) else { return };
        if let Some(ix) = entry.msgs.iter().position(|x| x.id == m.id) {
            let prev = ix.checked_sub(1).map(|p| entry.msgs[p].clone());
            entry.rendered[ix] = self.render_msg(&m, prev.as_ref());
            entry.msgs[ix] = m;
            if self.selected_channel == Some(ch) {
                self.message_list.splice(ix + 1..ix + 2, 1);
            }
        }
        self.messages.insert(ch, entry);
        cx.notify();
    }

    pub fn reveal_spoiler(&mut self, msg: Id, spoiler: usize, cx: &mut Context<Self>) {
        self.revealed.insert((msg, spoiler));
        let Some(ch) = self.selected_channel else { return };
        if let Some(m) = self
            .messages
            .get(&ch)
            .and_then(|e| e.msgs.iter().find(|x| x.id == msg).cloned())
        {
            self.replace_message(m, cx);
        }
    }

    // ------------------------------------------------------------- events

    pub fn handle_event(&mut self, ev: AppEvent, cx: &mut Context<Self>) {
        match ev {
            AppEvent::Ready => {}
            AppEvent::MessageCreate(m) => self.append_message(m, cx),
            AppEvent::MessageUpdate(m) => self.replace_message(m, cx),
            AppEvent::MessageDelete { channel, id } => {
                if let Some(e) = self.messages.get_mut(&channel)
                    && let Some(ix) = e.msgs.iter().position(|m| m.id == id)
                {
                    e.msgs.remove(ix);
                    e.rendered.remove(ix);
                    if self.selected_channel == Some(channel) {
                        self.message_list.splice(ix + 1..ix + 2, 0);
                    }
                }
            }
            AppEvent::TypingStart { channel, user } => {
                if user != self.me.id {
                    let list = self.typing.entry(channel).or_default();
                    list.retain(|(u, _)| *u != user);
                    list.push((user, Instant::now()));
                    self.update_typing_label();
                }
            }
            AppEvent::PresenceUpdate { user, presence } => {
                for ms in self.members.values_mut() {
                    for m in ms.iter_mut().filter(|m| m.user == user) {
                        m.presence = presence;
                    }
                }
                self.rebuild_members();
            }
            AppEvent::GuildMembers { guild, members } => {
                self.members.insert(guild, members);
                if self.selected_guild == Some(guild) {
                    self.rebuild_members();
                }
            }
            AppEvent::ChannelUnread { channel, unread, mentions } => {
                if let Some(c) = self.channels.get_mut(&channel) {
                    c.unread = unread;
                    c.mentions = mentions;
                }
                self.rebuild_sidebar();
            }
            AppEvent::VoiceStateUpdate { channel, user } => {
                for ch in self.channels.values_mut() {
                    ch.voice_users.retain(|u| *u != user);
                }
                if let Some(c) = channel.and_then(|c| self.channels.get_mut(&c)) {
                    c.voice_users.push(user);
                }
                if user == self.me.id {
                    self.voice_channel = channel;
                }
                self.rebuild_sidebar();
            }
            AppEvent::Disconnected(reason) => {
                self.login_error = Some(reason.into());
                self.screen = Screen::Login;
            }
        }
        cx.notify();
    }

    fn expire_typing(&mut self, cx: &mut Context<Self>) {
        let before = self.typing.values().map(Vec::len).sum::<usize>();
        for v in self.typing.values_mut() {
            v.retain(|(_, t)| t.elapsed() < Duration::from_secs(10));
        }
        if before != self.typing.values().map(Vec::len).sum::<usize>() {
            self.update_typing_label();
            cx.notify();
        }
    }

    fn update_typing_label(&mut self) {
        let names: Vec<&str> = self
            .selected_channel
            .and_then(|c| self.typing.get(&c))
            .map(|v| {
                v.iter()
                    .filter_map(|(u, _)| self.users.get(u).map(|u| u.display_name.as_ref()))
                    .collect()
            })
            .unwrap_or_default();
        self.typing_label = match names.as_slice() {
            [] => None,
            [a] => Some(format!("{a} is typing...").into()),
            [a, b] => Some(format!("{a} and {b} are typing...").into()),
            [a, b, c] => Some(format!("{a}, {b}, and {c} are typing...").into()),
            _ => Some("Several people are typing...".into()),
        };
    }

    // ---------------------------------------------------------- composer

    fn on_composer_change(&mut self, cx: &mut Context<Self>) {
        if let Some(ch) = self.selected_channel
            && !self.composer.read(cx).value().is_empty()
        {
            self.backend.trigger_typing(ch).detach_ok();
        }
    }

    pub fn submit_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ch) = self.selected_channel else { return };
        let text = self.composer.read(cx).value().trim().to_string();
        if text.is_empty() {
            return;
        }
        self.composer.update(cx, |s, cx| s.set_value("", window, cx));
        if let Some(id) = self.editing.take() {
            let fut = self.backend.edit_message(ch, id, text);
            cx.spawn(async move |this, cx| {
                if let Ok(m) = fut.await {
                    let _ = this.update(cx, |this, cx| this.replace_message(m, cx));
                }
            })
            .detach();
        } else {
            let reply = self.replying_to.take();
            let fut = self.backend.send_message(ch, text, reply);
            cx.spawn(async move |this, cx| {
                if let Ok(m) = fut.await {
                    let _ = this.update(cx, |this, cx| this.append_message(m, cx));
                }
            })
            .detach();
        }
        cx.notify();
    }

    /// Up arrow in an empty composer: edit own last message.
    pub fn edit_last(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.composer.read(cx).value().is_empty() {
            return false;
        }
        let Some(ch) = self.selected_channel else { return false };
        let Some(m) = self
            .messages
            .get(&ch)
            .and_then(|e| e.msgs.iter().rev().find(|m| m.author == self.me.id))
        else {
            return false;
        };
        self.editing = Some(m.id);
        let content = m.content.clone();
        self.composer.update(cx, |s, cx| s.set_value(content, window, cx));
        cx.notify();
        true
    }

    pub fn start_reply(&mut self, id: Id, window: &mut Window, cx: &mut Context<Self>) {
        self.replying_to = Some(id);
        self.editing = None;
        self.composer.update(cx, |s, cx| s.focus(window, cx));
        cx.notify();
    }

    pub fn join_voice(&mut self, ch: Option<Id>, cx: &mut Context<Self>) {
        self.backend.join_voice(ch).detach_ok();
        cx.notify();
    }

    // -------------------------------------------------------- derived rows

    pub fn toggle_category(&mut self, id: Id, cx: &mut Context<Self>) {
        if !self.collapsed.remove(&id) {
            self.collapsed.insert(id);
        }
        self.rebuild_sidebar();
        cx.notify();
    }

    pub fn rebuild_sidebar(&mut self) {
        self.sidebar_rows.clear();
        match self.selected_guild {
            None => {
                self.sidebar_rows.extend(self.dms.iter().map(|id| SidebarRow::Channel(*id)));
            }
            Some(g) => {
                let ids = self.guild_channels.get(&g).cloned().unwrap_or_default();
                let mut current_cat: Option<Id> = None;
                for id in ids {
                    let ch = &self.channels[&id];
                    if ch.kind == ChannelKind::Category {
                        current_cat = Some(id);
                        self.sidebar_rows.push(SidebarRow::Category(id));
                        continue;
                    }
                    let collapsed = current_cat.is_some_and(|c| self.collapsed.contains(&c));
                    // Collapsed categories still show the selected / unread channels.
                    if collapsed && Some(id) != self.selected_channel && !ch.unread {
                        continue;
                    }
                    self.sidebar_rows.push(SidebarRow::Channel(id));
                    if !collapsed {
                        self.sidebar_rows
                            .extend(ch.voice_users.iter().map(|u| SidebarRow::VoiceUser { user: *u }));
                    }
                }
            }
        }
        self.sidebar_list.reset(self.sidebar_rows.len() + 1);
    }

    pub fn rebuild_members(&mut self) {
        self.member_rows.clear();
        let Some(g) = self.selected_guild else {
            self.member_list.reset(0);
            return;
        };
        let roles = self.roles.get(&g).cloned().unwrap_or_default();
        let Some(members) = self.members.get(&g) else {
            self.member_list.reset(0);
            return;
        };
        let mut hoisted: Vec<&Role> = roles.iter().filter(|r| r.hoist).collect();
        hoisted.sort_by_key(|r| -r.position);
        let mut buckets: Vec<Vec<usize>> = vec![Vec::new(); hoisted.len() + 2];
        for (i, m) in members.iter().enumerate() {
            let b = if m.presence == Presence::Offline {
                hoisted.len() + 1
            } else {
                hoisted
                    .iter()
                    .position(|r| m.roles.contains(&r.id))
                    .unwrap_or(hoisted.len())
            };
            buckets[b].push(i);
        }
        let users = &self.users;
        for (bi, mut b) in buckets.into_iter().enumerate() {
            if b.is_empty() {
                continue;
            }
            b.sort_by(|x, y| {
                let a = users.get(&members[*x].user).map(|u| u.display_name.to_lowercase());
                let bb = users.get(&members[*y].user).map(|u| u.display_name.to_lowercase());
                a.cmp(&bb)
            });
            let title = if bi < hoisted.len() {
                hoisted[bi].name.to_uppercase()
            } else if bi == hoisted.len() {
                "ONLINE".to_string()
            } else {
                "OFFLINE".to_string()
            };
            self.member_rows
                .push(MemberRow::Header(format!("{title} — {}", b.len()).into()));
            self.member_rows.extend(b.into_iter().map(MemberRow::Member));
        }
        self.member_list.reset(self.member_rows.len());
    }

    pub fn member_color(&self, guild: Id, m: &Member) -> Option<u32> {
        let roles = self.roles.get(&guild)?;
        roles
            .iter()
            .filter(|r| m.roles.contains(&r.id) && r.color.is_some())
            .max_by_key(|r| r.position)
            .and_then(|r| r.color)
    }

    pub fn author_color(&self, user: Id) -> Option<u32> {
        let g = self.selected_guild?;
        let m = self.members.get(&g)?.iter().find(|m| m.user == user)?;
        self.member_color(g, m)
    }

    pub fn presence_of(&self, user: Id) -> Presence {
        if user == self.me.id {
            return self.presence;
        }
        self.selected_guild
            .and_then(|g| self.members.get(&g))
            .and_then(|ms| ms.iter().find(|m| m.user == user))
            .map(|m| m.presence)
            .unwrap_or(Presence::Online)
    }

    // ------------------------------------------------------ quick switcher

    pub fn open_switcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.screen != Screen::Main {
            return;
        }
        let input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Where would you like to go?")
        });
        let sub = cx.subscribe_in(&input, window, |this, _, ev: &InputEvent, window, cx| match ev {
            InputEvent::Change => this.update_switcher(cx),
            InputEvent::PressEnter { .. } => {
                if let Some(first) = this.switcher.as_ref().and_then(|s| s.results.first().copied()) {
                    this.close_switcher(window, cx);
                    this.select_channel(first, cx);
                }
            }
            _ => {}
        });
        input.update(cx, |s, cx| s.focus(window, cx));
        self.switcher = Some(Switcher { input, results: Vec::new(), _sub: sub });
        self.update_switcher(cx);
    }

    fn update_switcher(&mut self, cx: &mut Context<Self>) {
        let Some(sw) = &self.switcher else { return };
        let q = sw.input.read(cx).value().to_lowercase();
        let q = q.trim_start_matches(['#', '@', '!', '*']);
        let mut res: Vec<(i32, Id)> = self
            .channels
            .values()
            .filter(|c| c.kind != ChannelKind::Category)
            .filter_map(|c| {
                let name = c.name.to_lowercase();
                let score = if q.is_empty() {
                    if c.unread { 0 } else { 2 }
                } else if name.starts_with(q) {
                    0
                } else if name.contains(q) {
                    1
                } else {
                    return None;
                };
                Some((score, c.id))
            })
            .collect();
        res.sort_by_key(|(s, id)| (*s, self.channels[id].name.clone()));
        res.truncate(12);
        if let Some(sw) = &mut self.switcher {
            sw.results = res.into_iter().map(|(_, id)| id).collect();
        }
        cx.notify();
    }

    pub fn close_switcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.switcher = None;
        self.composer.update(cx, |s, cx| s.focus(window, cx));
        cx.notify();
    }

    fn on_escape(&mut self, _: &Escape, window: &mut Window, cx: &mut Context<Self>) {
        if self.switcher.is_some() {
            self.close_switcher(window, cx);
        } else if self.editing.take().is_some() {
            self.composer.update(cx, |s, cx| s.set_value("", window, cx));
        } else if self.replying_to.take().is_none() {
            self.message_list.scroll_to_end();
        }
        cx.notify();
    }
}

trait DetachOk {
    fn detach_ok(self);
}
impl<T: Send + 'static> DetachOk for crate::backend::BoxFuture<T> {
    fn detach_ok(self) {
        // Fire-and-forget; executed on a helper thread so we don't need a
        // context. Real adapters can return immediately-ready futures.
        std::thread::spawn(move || {
            let _ = futures::executor::block_on(self);
        });
    }
}

impl Focusable for PhoenixApp {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for PhoenixApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match self.screen {
            Screen::Login => ui::login::render(self, window, cx).into_any_element(),
            Screen::Main => div()
                .size_full()
                .flex()
                .flex_row()
                .child(ui::server_bar::render(self, cx))
                .child(ui::sidebar::render(self, cx))
                .child(ui::chat::render(self, window, cx))
                .into_any_element(),
        };
        div()
            .id("phoenix-root")
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &QuickSwitcher, w, cx| this.open_switcher(w, cx)))
            .on_action(cx.listener(Self::on_escape))
            .on_action(cx.listener(|this, _: &ToggleMembers, _, cx| {
                this.show_members = !this.show_members;
                cx.notify();
            }))
            .size_full()
            .relative()
            .font_family(FONT_UI)
            .text_color(c(Palette::TEXT_NORMAL))
            .bg(c(Palette::BG_TERTIARY))
            .child(body)
            .when_some(self.switcher.as_ref(), |d, sw| d.child(ui::switcher::render(self, sw, cx)))
    }
}
