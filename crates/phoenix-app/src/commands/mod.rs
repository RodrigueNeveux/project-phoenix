//! Parity features that are not tied to one screen: slash commands and
//! interactions, the mentions inbox, saved messages, moderation, forum posts,
//! polls, group DMs, and rebuilding the UI after a fresh READY on reconnect.
//!
//! The UI sends a [`ParityRequest`] through `Backend::parity`; the real
//! backend runs it in [`exec`].

pub mod exec;
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
    ForumPosts { forum: Id, tags: Vec<Id>, offset: u32, by_creation: bool },
    CreatePost { forum: Id, title: String, content: String, tags: Vec<Id> },
    Moderate { guild: Id, user: Id, action: ModAction },
    BulkDelete { channel: Id, messages: Vec<Id> },
    CreateGroupDm(Vec<Id>),
    /// Save the guild rail order and folders (`guild_folders`).
    GuildFolders(Vec<FolderSync>),
    /// Archived public + joined private threads of a text channel (threads browser).
    ArchivedThreads { channel: Id },
}

/// (author, application, game name, first time, resurrected).
pub type InventoryEntry = (Id, Option<Id>, Option<String>, bool, bool);

#[derive(Clone, Debug)]
pub enum ParityReply {
    Done,
    Commands(Vec<SlashCommand>),
    Messages(Vec<Message>),
    ForumPosts { posts: Vec<ForumPost>, has_more: bool },
    Channel(Id),
    /// Content inventory entries: (author, application, game name, first time, resurrected).
    Inventory(Vec<InventoryEntry>),
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
}
