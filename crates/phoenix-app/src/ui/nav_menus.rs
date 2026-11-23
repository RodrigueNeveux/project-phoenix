//! Right-click menus on the navigation: channel rows, DM rows and guild icons.
//!
//! Item order observed in the official client:
//! - text channel: Mark As Read | Invite People, Copy Link | Mute Channel >,
//!   Notification Settings > | Copy Channel ID (developer mode)
//! - DM: Mark As Read | Profile, Close DM | Mute @user > | Copy Channel ID
//! - guild: Mark As Read | Invite People | Mute Server >, Notification
//!   Settings >, Hide Muted Channels | Privacy Settings | Leave Server |
//!   Copy Server ID
//!
//! Mute durations: 15 minutes, 1 hour, 3 hours, 8 hours, 24 hours, until
//! turned back on.

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
};

use gpui::{AnyElement, App, ClipboardItem, Context, Pixels, Point, Window};

use crate::{
    app::PhoenixApp,
    model::{ChannelKind, Id},
    ui::{
        context_menu::{self, MenuItem},
        home::data::{NavRequest, Overlay},
    },
};

#[derive(Clone, Copy, PartialEq)]
pub enum Target {
    Channel(Id),
    Guild(Id),
}

type Scope = (Option<Id>, Option<Id>);

thread_local! {
    static OPEN: RefCell<Option<(Target, Point<Pixels>)>> = const { RefCell::new(None) };
    /// Mutes and levels set from these menus this session (the READY copy
    /// lives in the backend), so radios and labels reflect the last choice.
    static MUTED_GUILDS: RefCell<HashMap<Id, bool>> = RefCell::new(HashMap::new());
    static LEVELS: RefCell<HashMap<Scope, u8>> = RefCell::new(HashMap::new());
}

pub fn open(target: Target, pos: Point<Pixels>, window: &mut Window) {
    OPEN.with(|o| *o.borrow_mut() = Some((target, pos)));
    window.refresh();
}

const DURATIONS: [(&str, Option<u32>); 6] = [
    ("menu.mute.15m", Some(900)),
    ("menu.mute.1h", Some(3600)),
    ("menu.mute.3h", Some(10800)),
    ("menu.mute.8h", Some(28800)),
    ("menu.mute.24h", Some(86400)),
    ("menu.mute.forever", None),
];

type Ent = gpui::Entity<PhoenixApp>;

fn act(e: &Ent, label: gpui::SharedString, f: impl Fn(&mut PhoenixApp, &mut Window, &mut Context<PhoenixApp>) + 'static) -> MenuItem {
    let e = e.clone();
    MenuItem::action(label, None, move |_, w, cx: &mut App| e.update(cx, |a, cx| f(a, w, cx)))
}

fn copy(label: gpui::SharedString, text: String) -> MenuItem {
    MenuItem::action(label, None, move |_, _, cx: &mut App| cx.write_to_clipboard(ClipboardItem::new_string(text.clone())))
}

/// "Mute X >" submenu, or a plain "Unmute X" row when already muted.
fn mute_item(e: &Ent, label: gpui::SharedString, unmute: gpui::SharedString, muted: bool, guild: Option<Id>, channel: Option<Id>) -> MenuItem {
    if muted {
        let e = e.clone();
        return MenuItem::action(unmute, None, move |_, _, cx: &mut App| {
            e.update(cx, |a, cx| a.set_nav_mute(guild, channel, false, None, cx))
        });
    }
    let items = DURATIONS
        .iter()
        .map(|&(l, secs)| {
            let e = e.clone();
            MenuItem::action(crate::t!(l), None, move |_, _, cx: &mut App| e.update(cx, |a, cx| a.set_nav_mute(guild, channel, true, secs, cx)))
        })
        .collect();
    MenuItem::submenu(label, items)
}

fn level_code(l: crate::notifications::Level) -> u8 {
    use crate::notifications::Level::*;
    match l {
        All => 0,
        Mentions => 1,
        Nothing => 2,
        Inherit => 3,
    }
}

/// The account's stored level for this scope (`user_guild_settings`).
fn stored_level(app: &PhoenixApp, guild: Option<Id>, channel: Option<Id>) -> Option<u8> {
    let p = app.backend.notify_prefs()?;
    let g = p.guilds.get(&guild.map_or(0, |g| g.0))?;
    Some(match channel {
        Some(ch) => g.channels.get(&ch.0).map_or(3, |c| level_code(c.level)),
        None => level_code(g.level),
    })
}

fn guild_muted(app: &PhoenixApp, g: Id) -> bool {
    MUTED_GUILDS.with(|m| m.borrow().get(&g).copied()).unwrap_or_else(|| {
        app.backend.notify_prefs().and_then(|p| p.guilds.get(&g.0).map(|x| x.muted)).unwrap_or(false)
    })
}

fn notify_item(app: &PhoenixApp, e: &Ent, guild: Option<Id>, channel: Option<Id>) -> MenuItem {
    let default = if channel.is_some() { 3 } else { 1 };
    let cur = LEVELS
        .with(|l| l.borrow().get(&(guild, channel)).copied())
        .or_else(|| stored_level(app, guild, channel))
        .unwrap_or(default);
    let mut opts: Vec<(&str, u8)> = vec![("menu.notify.all", 0), ("menu.notify.mentions", 1), ("menu.notify.nothing", 2)];
    if channel.is_some() {
        opts.insert(0, ("menu.notify.default", 3));
    }
    let items = opts
        .into_iter()
        .map(|(l, lv)| {
            let e = e.clone();
            MenuItem::radio(crate::t!(l), cur == lv, move |_, _, cx: &mut App| {
                LEVELS.with(|m| m.borrow_mut().insert((guild, channel), lv));
                e.update(cx, |a, cx| a.nav_request(NavRequest::NotifyLevel { guild, channel, level: lv }, cx, |_, _, _| {}))
            })
        })
        .collect();
    MenuItem::submenu(crate::t!("menu.notify_settings"), items)
}

pub fn render(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    let (target, pos) = OPEN.with(|o| *o.borrow())?;
    let e = cx.entity();
    let dev = app.prefs.synced.developer_mode;
    let mut items = Vec::new();
    match target {
        Target::Channel(id) => {
            let ch = app.channels.get(&id)?;
            let (guild, muted) = (ch.guild, ch.muted);
            items.push(act(&e, crate::t!("menu.mark_read"), move |a, _, cx| a.mark_nav_read(Target::Channel(id), cx)));
            items.push(MenuItem::Separator);
            if let Some(g) = guild {
                items.push(act(&e, crate::t!("menu.invite"), move |a, _, cx| a.open_invite_people(g, cx)));
                items.push(copy(crate::t!("menu.copy_link"), format!("https://discord.com/channels/{}/{}", g.0, id.0)));
                items.push(MenuItem::Separator);
                let cat = ch.kind == ChannelKind::Category;
                let (mute, unmute) = if cat {
                    (crate::t!("menu.mute_category"), crate::t!("menu.unmute_category"))
                } else {
                    (crate::t!("menu.mute_channel"), crate::t!("menu.unmute_channel"))
                };
                items.push(mute_item(&e, mute, unmute, muted, guild, Some(id)));
                items.push(notify_item(app, &e, guild, Some(id)));
            } else {
                let peer = ch.voice_users.first().copied();
                let is_group = app.nav.extras.get(&id).is_some_and(|x| x.recipients > 1);
                if let Some(u) = peer.filter(|_| !is_group) {
                    items.push(act(&e, crate::t!("menu.profile"), move |a, _, cx| a.open_profile_modal(u, cx)));
                }
                items.push(act(&e, if is_group { crate::t!("menu.leave_group") } else { crate::t!("menu.close_dm") }, move |a, _, cx| a.close_dm(id, cx)));
                items.push(MenuItem::Separator);
                let who = if is_group { ch.name.to_string() } else { format!("@{}", ch.name) };
                items.push(mute_item(&e, crate::t!("menu.mute_who", who = who), crate::t!("menu.unmute_who", who = who), muted, None, Some(id)));
            }
            if dev {
                items.push(MenuItem::Separator);
                items.push(copy(crate::t!("menu.copy_channel_id"), id.0.to_string()));
            }
        }
        Target::Guild(g) => {
            let muted = guild_muted(app, g);
            let hide = app.nav.hide_muted.contains(&g);
            items.push(act(&e, crate::t!("menu.mark_read"), move |a, _, cx| a.mark_nav_read(Target::Guild(g), cx)));
            items.push(MenuItem::Separator);
            items.push(act(&e, crate::t!("menu.invite"), move |a, _, cx| a.open_invite_people(g, cx)));
            items.push(MenuItem::Separator);
            items.push(mute_item(&e, crate::t!("menu.mute_server"), crate::t!("menu.unmute_server"), muted, Some(g), None));
            items.push(notify_item(app, &e, Some(g), None));
            items.push(MenuItem::check(crate::t!("menu.hide_muted"), hide, {
                let e = e.clone();
                move |_, _, cx: &mut App| {
                    e.update(cx, |a, cx| {
                        if !a.nav.hide_muted.remove(&g) {
                            a.nav.hide_muted.insert(g);
                        }
                        a.rebuild_sidebar();
                        a.save_nav_prefs();
                        cx.notify();
                    })
                }
            }));
            items.push(MenuItem::Separator);
            items.push(act(&e, crate::t!("menu.privacy_settings"), |a, _, cx| a.stub(&crate::t!("menu.privacy_settings"), cx)));
            items.push(MenuItem::Separator);
            items.push(MenuItem::danger(crate::t!("menu.leave_server"), None, {
                let e = e.clone();
                move |_, _, cx: &mut App| e.update(cx, |a, cx| a.show_overlay(Overlay::LeaveGuild(g), cx))
            }));
            if dev {
                items.push(MenuItem::Separator);
                items.push(copy(crate::t!("menu.copy_server_id"), g.0.to_string()));
            }
        }
    }
    let dismiss: context_menu::Dismiss = Rc::new(|w, _| {
        OPEN.with(|o| *o.borrow_mut() = None);
        w.refresh();
    });
    Some(context_menu::render(pos, items, dismiss))
}

impl PhoenixApp {
    fn mark_nav_read(&mut self, target: Target, cx: &mut Context<Self>) {
        let req = match target {
            Target::Channel(id) => {
                if let Some(ch) = self.channels.get_mut(&id) {
                    ch.unread = false;
                    ch.mentions = 0;
                }
                NavRequest::AckChannel(id)
            }
            Target::Guild(g) => {
                for id in self.guild_channels.get(&g).cloned().unwrap_or_default() {
                    if let Some(ch) = self.channels.get_mut(&id) {
                        ch.unread = false;
                        ch.mentions = 0;
                    }
                }
                if let Some(guild) = self.guilds.iter_mut().find(|x| x.id == g) {
                    guild.unread = false;
                    guild.mentions = 0;
                }
                NavRequest::AckGuild(g)
            }
        };
        self.rebuild_sidebar();
        self.nav_request(req, cx, |_, _, _| {});
        cx.notify();
    }

    fn set_nav_mute(&mut self, guild: Option<Id>, channel: Option<Id>, muted: bool, secs: Option<u32>, cx: &mut Context<Self>) {
        match channel {
            Some(id) => {
                if let Some(ch) = self.channels.get_mut(&id) {
                    ch.muted = muted;
                }
            }
            None => {
                if let Some(g) = guild {
                    MUTED_GUILDS.with(|m| m.borrow_mut().insert(g, muted));
                }
            }
        }
        self.rebuild_sidebar();
        self.nav_request(NavRequest::Mute { guild, channel, muted, secs }, cx, |_, _, _| {});
        cx.notify();
    }
}
