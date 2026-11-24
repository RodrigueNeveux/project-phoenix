//! Parity features that are not tied to one screen: slash commands and
//! interactions, the mentions inbox, saved messages, moderation, forum posts,
//! polls, group DMs, and rebuilding the UI after a fresh READY on reconnect.
//!
//! The UI sends a [`ParityRequest`] through `Backend::parity`; the real
//! backend runs it in [`exec`].

pub mod exec;
pub mod guide;
pub mod slash;
pub mod state;
pub mod store;

pub use state::{ParityEvent, ParityUi, from_gateway, on_event};

use std::sync::OnceLock;

use gpui::Context;
use parking_lot::Mutex;
use serde_json::Value;

use crate::{
    app::PhoenixApp as App,
    backend::ReadyData,
    model::{Id, Message},
};

/// Gateway session id, needed by `POST /interactions`.
fn session_slot() -> &'static Mutex<String> {
    static S: OnceLock<Mutex<String>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(String::new()))
}

pub fn set_session_id(id: &str) {
    *session_slot().lock() = id.to_owned();
}

pub fn session_id() -> String {
    session_slot().lock().clone()
}

/// One chat-input command offered by the composer autocomplete.
#[derive(Clone, Debug)]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
    pub app_name: String,
    /// `app-icons/{app}/{icon}.png` of the command's application, if it has one.
    pub app_icon: Option<String>,
    /// Raw command, sent back as is to run it.
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct ForumPost {
    pub thread: Id,
    pub title: String,
    pub tags: Vec<Id>,
    pub message_count: u32,
    pub first_message: Option<Message>,
    pub owner: Option<Id>,
    /// Latest message id (activity time), else the thread id.
    pub last_message: Option<Id>,
    /// Thread `member_ids_preview` / `member_count` (threads browser facepile).
    pub member_ids_preview: Vec<Id>,
    pub member_count: u32,
    /// Thread flag PINNED (1 << 1): pinned post shown first with a pin badge.
    pub pinned: bool,
    /// Came from the archived search (forum "Older posts" section).
    pub archived: bool,
    /// `thread_metadata.locked`: only moderators can reopen or reply.
    pub locked: bool,
}

/// Tag of a forum / media channel (`available_tags`).
#[derive(Clone, Debug, PartialEq)]
pub struct ForumTag {
    pub id: Id,
    pub name: gpui::SharedString,
    /// Unicode emoji or custom emoji token.
    pub emoji: Option<String>,
    pub moderated: bool,
}

/// One entry of the synced guild list: a folder (`id` set) or a lone guild.
#[derive(Clone, Debug, PartialEq)]
pub struct FolderSync {
    pub id: Option<u64>,
    pub name: Option<String>,
    pub color: Option<u32>,
    pub guilds: Vec<Id>,
}

#[derive(Clone, Debug)]
pub enum ModAction {
    /// Seconds; 0 removes the timeout.
    Timeout(u64),
    Kick,
    /// Seconds of history to delete.
    Ban(u32),
    Unban,
}

#[derive(Clone, Debug)]
pub enum ParityRequest {
    /// Guild (or DM channel) + user-installed commands.
    Commands { guild: Option<Id>, channel: Id },
    /// Run `command` (raw from [`SlashCommand`]) with the text typed after its name.
    RunCommand { guild: Option<Id>, channel: Id, command: Value, args: String },
    /// Button (type 2) or select (3, 5..8) click on a bot message.
    Component {
        guild: Option<Id>,
        channel: Id,
        message: Id,
        app: Id,
        flags: u64,
        custom_id: String,
        kind: u8,
        values: Option<Vec<String>>,
    },
    /// Modal submit: `fields` = (custom_id, value) of each text input.
    ModalSubmit { guild: Option<Id>, channel: Id, app: Id, modal: Id, custom_id: String, fields: Vec<(String, String)> },
    Mentions { before: Option<Id>, guild: Option<Id> },
    DismissMention(Id),
    SavedMessages,
    /// `GET /users/@me/scheduled-messages` (Inbox > Scheduled), read-only.
    ScheduledMessages,
    /// `GET /content-inventory/users/@me`: activity card traits.
    ContentInventory,
    SetSaved { channel: Id, message: Id, saved: bool },
    VotePoll { channel: Id, message: Id, answers: Vec<u32> },
    /// `by_creation` sorts by post date instead of recent activity.
    ForumPosts { forum: Id, tags: Vec<Id>, offset: u32, by_creation: bool, archived: bool },
    CreatePost { forum: Id, title: String, content: String, tags: Vec<Id> },
    Moderate { guild: Id, user: Id, action: ModAction },
    BulkDelete { channel: Id, messages: Vec<Id> },
    CreateGroupDm(Vec<Id>),
    /// Save the guild rail order and folders (`guild_folders`).
    GuildFolders(Vec<FolderSync>),
    /// Archived public + joined private threads of a text channel (threads browser).
    ArchivedThreads { channel: Id },
    /// `GET /guilds/{g}/scheduled-events` (Events modal), read-only.
    ScheduledEvents { guild: Id },
    /// `GET /guilds/{g}/onboarding` (Channels & Roles), read-only.
    Onboarding { guild: Id },
    /// `GET /guilds/{g}/new-member-welcome` + `GET /guilds/{g}/profile` (Server Guide), read-only.
    ServerGuide { guild: Id },
    /// `GET /channels/{id}` for a thread / forum post opened by link.
    FetchThread { id: Id },
}

/// One onboarding question (Channels & Roles "Customize" tab).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OnboardingPrompt {
    pub id: Id,
    pub title: String,
    pub single_select: bool,
    pub options: Vec<OnboardingOption>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OnboardingOption {
    pub id: Id,
    pub title: String,
    pub description: String,
    /// Custom emoji id, or a unicode emoji in `emoji_name`.
    pub emoji_id: Option<Id>,
    pub emoji_name: Option<String>,
    pub animated: bool,
}

/// A guild scheduled event (Events modal card).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GuildEvent {
    pub id: Id,
    pub name: String,
    pub description: String,
    /// Unix seconds.
    pub start: i64,
    pub end: Option<i64>,
    /// 1 scheduled, 2 active, 3 completed, 4 canceled.
    pub status: u8,
    /// Voice/stage channel, or `location` for external events.
    pub channel: Option<Id>,
    pub location: Option<String>,
    pub user_count: u32,
    pub creator: Option<Id>,
    /// Cover image hash (`guild-events/{id}/{hash}.png`).
    pub image: Option<String>,
}

/// One content inventory feed entry (member list Activity card).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InventoryEntry {
    pub id: Id,
    pub author: Id,
    pub app: Option<Id>,
    pub game: Option<String>,
    pub content_type: u64,
    pub first_time: bool,
    pub resurrected: bool,
    pub trending: bool,
    pub streak_days: Option<u64>,
    /// `app-icons/{app}/{icon}.png` from `/applications/public`.
    pub icon: Option<gpui::SharedString>,
    /// Live session (trait `is_live`): start and expiry, unix ms.
    pub live: bool,
    pub started_ms: Option<i64>,
    pub expires_ms: Option<i64>,
}

#[derive(Clone, Debug)]
pub enum ParityReply {
    Done,
    Commands(Vec<SlashCommand>),
    Messages(Vec<Message>),
    ForumPosts { posts: Vec<ForumPost>, has_more: bool },
    Channel(Id),
    /// Content inventory feed entries.
    Inventory(Vec<InventoryEntry>),
    Events(Vec<GuildEvent>),
    Onboarding(Vec<OnboardingPrompt>),
    Guide(Box<guide::GuildGuide>),
    Thread(crate::model::Channel),
}

/// Fresh READY after a non-resumable reconnect (carried by `AppEvent::Resync`).
pub struct Resync(pub ReadyData);

impl std::fmt::Debug for Resync {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Resync")
    }
}

/// Rebuild all state from the new READY, then return to where the user was.
pub fn on_resync(app: &mut App, r: Resync, cx: &mut Context<App>) {
    let (guild, channel) = (app.selected_guild, app.selected_channel);
    app.apply_ready(r.0, cx);
    if guild.is_none() || app.guilds.iter().any(|g| Some(g.id) == guild) {
        app.select_guild(guild, cx);
    }
    if let Some(c) = channel
        && app.channels.contains_key(&c)
    {
        app.select_channel(c, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{backend::Backend, mock::MockBackend};

    #[gpui::test]
    fn resync_rebuilds_and_keeps_selection(cx: &mut gpui::TestAppContext) {
        let backend = std::sync::Arc::new(MockBackend::synchronous());
        let ready = || futures::executor::block_on(backend.login(crate::backend::Credentials::Token("x".into()))).unwrap();
        let first = ready();
        cx.update(gpui_component::init);
        let window = cx.add_window(|w, cx| App::new(backend.clone(), w, cx));
        window
            .update(cx, |app, _, cx| {
                app.apply_ready(first, cx);
                let g = app.guilds[0].id;
                app.select_guild(Some(g), cx);
                let ch = app.guild_channels[&g]
                    .iter()
                    .copied()
                    .find(|c| app.channels[c].kind == crate::model::ChannelKind::Text)
                    .unwrap();
                app.select_channel(ch, cx);
                app.guilds.clear();
                on_resync(app, Resync(ready()), cx);
                assert!(!app.guilds.is_empty());
                assert_eq!(app.selected_guild, Some(g));
                assert_eq!(app.selected_channel, Some(ch));
            })
            .unwrap();
    }

    /// Start-up order seen on the real account: the disk snapshot is shown,
    /// READY_SUPPLEMENTAL puts a user in a voice channel, then the live
    /// ReadyData (built from READY, before the supplemental, so without voice
    /// users) replaces the snapshot. The voice row must survive.
    #[gpui::test]
    fn voice_users_survive_ready_after_supplemental(cx: &mut gpui::TestAppContext) {
        use crate::backend::AppEvent;
        let backend = std::sync::Arc::new(MockBackend::synchronous());
        let ready = || futures::executor::block_on(backend.login(crate::backend::Credentials::Token("x".into()))).unwrap();
        let first = ready();
        cx.update(gpui_component::init);
        let window = cx.add_window(|w, cx| App::new(backend.clone(), w, cx));
        window
            .update(cx, |app, _, cx| {
                app.apply_ready(first, cx);
                let vc = app.channels.values().find(|c| c.kind == crate::model::ChannelKind::Voice && c.guild.is_some()).map(|c| c.id).unwrap();
                let user = crate::model::Id(4242);
                app.handle_event(AppEvent::VoiceStateUpdate { channel: Some(vc), user }, cx);
                let flags = crate::model::VoiceFlags { self_mute: true, self_deaf: true, ..Default::default() };
                app.handle_event(AppEvent::VoiceFlags { user, flags }, cx);
                let mut live = ready();
                for c in live.channels.iter_mut() {
                    c.voice_users.retain(|u| *u != user);
                }
                on_resync(app, Resync(live), cx);
                assert!(app.channels[&vc].voice_users.contains(&user), "voice row lost by the ReadyData after READY_SUPPLEMENTAL");
                let f = crate::ui::voice_panel::flags(user);
                assert!(f.self_mute && f.self_deaf);
                // A later leave still removes it.
                app.handle_event(AppEvent::VoiceStateUpdate { channel: None, user }, cx);
                assert!(!app.channels[&vc].voice_users.contains(&user));
            })
            .unwrap();
    }
}

/// One `scheduled-events` entry; `None` without an id.
pub fn parse_event(v: &serde_json::Value) -> Option<GuildEvent> {
    let sid = |k: &str| v.get(k).and_then(|x| x.as_str()).and_then(|x| x.parse::<u64>().ok()).map(Id);
    let time = |k: &str| {
        v.get(k).and_then(|x| x.as_str()).and_then(|x| chrono::DateTime::parse_from_rfc3339(x).ok()).map(|d| d.timestamp())
    };
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).map(str::to_owned);
    Some(GuildEvent {
        id: sid("id")?,
        name: s("name").unwrap_or_default(),
        description: s("description").unwrap_or_default(),
        start: time("scheduled_start_time").unwrap_or(0),
        end: time("scheduled_end_time"),
        status: v.get("status").and_then(|x| x.as_u64()).unwrap_or(1) as u8,
        channel: sid("channel_id"),
        location: v.get("entity_metadata").and_then(|m| m.get("location")).and_then(|x| x.as_str()).map(str::to_owned),
        user_count: v.get("user_count").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
        creator: sid("creator_id"),
        image: s("image"),
    })
}

#[cfg(test)]
mod event_tests {
    #[test]
    fn parses_scheduled_event() {
        let v = serde_json::json!({
            "id": "12", "name": "Game night", "description": "", "status": 1,
            "scheduled_start_time": "2026-11-30T20:00:00+00:00", "scheduled_end_time": null,
            "channel_id": null, "entity_metadata": { "location": "Paris" }, "user_count": 3
        });
        let ev = super::parse_event(&v).unwrap();
        assert_eq!(ev.id, super::Id(12));
        assert_eq!(ev.location.as_deref(), Some("Paris"));
        assert_eq!(ev.user_count, 3);
        assert_eq!(ev.start, 1_796_068_800);
        assert!(ev.end.is_none());
    }
}

/// `prompts` of `GET /guilds/{g}/onboarding`, in order.
pub fn parse_onboarding(v: &serde_json::Value) -> Vec<OnboardingPrompt> {
    let sid = |v: &serde_json::Value, k: &str| v.get(k).and_then(|x| x.as_str()).and_then(|x| x.parse::<u64>().ok()).map(Id);
    let s = |v: &serde_json::Value, k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_owned();
    let Some(prompts) = v.get("prompts").and_then(|p| p.as_array()) else { return Vec::new() };
    prompts
        .iter()
        .filter_map(|p| {
            let options = p
                .get("options")
                .and_then(|o| o.as_array())
                .map(|o| {
                    o.iter()
                        .filter_map(|o| {
                            let emoji = o.get("emoji");
                            Some(OnboardingOption {
                                id: sid(o, "id")?,
                                title: s(o, "title"),
                                description: s(o, "description"),
                                emoji_id: emoji.and_then(|e| sid(e, "id")),
                                emoji_name: emoji.and_then(|e| e.get("name")).and_then(|x| x.as_str()).map(str::to_owned),
                                animated: emoji.and_then(|e| e.get("animated")).and_then(|x| x.as_bool()).unwrap_or(false),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            Some(OnboardingPrompt {
                id: sid(p, "id")?,
                title: s(p, "title"),
                single_select: p.get("single_select").and_then(|x| x.as_bool()).unwrap_or(false),
                options,
            })
        })
        .collect()
}

#[cfg(test)]
mod onboarding_tests {
    #[test]
    fn parses_prompts() {
        let v = serde_json::json!({ "prompts": [{ "id": "1", "title": "Topics", "single_select": false, "options": [
            { "id": "2", "title": "AI", "description": "", "emoji": { "id": null, "name": "🧠" } },
            { "id": "3", "title": "Games", "description": "Fun", "emoji": { "id": "44", "name": "pad", "animated": true } }
        ]}]});
        let p = super::parse_onboarding(&v);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].options[0].emoji_name.as_deref(), Some("🧠"));
        assert_eq!(p[0].options[1].emoji_id, Some(super::Id(44)));
        assert!(p[0].options[1].animated);
    }
}
