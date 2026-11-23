//! Parity features that are not tied to one screen: slash commands and
//! interactions, the mentions inbox, saved messages, moderation, forum posts,
//! polls, group DMs, and rebuilding the UI after a fresh READY on reconnect.
//!
//! The UI sends a [`ParityRequest`] through `Backend::parity`; the real
//! backend runs it in [`exec`].

pub mod exec;
pub mod slash;

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
    SetSaved { channel: Id, message: Id, saved: bool },
    VotePoll { channel: Id, message: Id, answers: Vec<u32> },
    ForumPosts { forum: Id, tags: Vec<Id>, offset: u32 },
    CreatePost { forum: Id, title: String, content: String, tags: Vec<Id> },
    Moderate { guild: Id, user: Id, action: ModAction },
    BulkDelete { channel: Id, messages: Vec<Id> },
    CreateGroupDm(Vec<Id>),
}

#[derive(Clone, Debug)]
pub enum ParityReply {
    Done,
    Commands(Vec<SlashCommand>),
    Messages(Vec<Message>),
    ForumPosts { posts: Vec<ForumPost>, has_more: bool },
    Channel(Id),
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
