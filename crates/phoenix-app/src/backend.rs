//! Adapter seam between the UI and the network layer.
//!
//! The UI only talks to a `dyn Backend` and consumes `AppEvent`s. Today the
//! implementation is [`crate::mock::MockBackend`]; a `phoenix-api` adapter
//! will implement the same trait (REST calls → futures, gateway dispatches →
//! `AppEvent`s pushed through the [`EventSink`]).

use std::{future::Future, pin::Pin, sync::Arc};

use crate::model::{Channel, Guild, Id, Member, Message, Presence, Role, User};

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = Result<T, BackendError>> + Send + 'static>>;

#[derive(Clone, Debug)]
pub struct BackendError(pub String);

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

pub enum Credentials {
    Token(String),
    Password { login: String, password: String },
}

/// Everything the UI needs once logged in ("READY").
pub struct ReadyData {
    pub me: User,
    pub users: Vec<User>,
    pub guilds: Vec<Guild>,
    pub channels: Vec<Channel>,
    pub roles: Vec<(Id, Vec<Role>)>,
    pub dm_channels: Vec<Channel>,
}

/// Pushed from the backend (gateway) to the UI, delivered on the UI thread.
#[derive(Debug)]
pub enum AppEvent {
    Ready,
    MessageCreate(Message),
    MessageUpdate(Message),
    MessageDelete { channel: Id, id: Id },
    TypingStart { channel: Id, user: Id },
    PresenceUpdate { user: Id, presence: Presence },
    GuildMembers { guild: Id, members: Vec<Member> },
    ChannelUnread { channel: Id, unread: bool, mentions: u32 },
    VoiceStateUpdate { channel: Option<Id>, user: Id },
    Disconnected(String),
}

pub type EventSink = Arc<dyn Fn(AppEvent) + Send + Sync>;

pub trait Backend: Send + Sync + 'static {
    /// Register where gateway events must be delivered.
    fn set_event_sink(&self, sink: EventSink);
    fn login(&self, creds: Credentials) -> BoxFuture<ReadyData>;
    /// Called when the user opens a guild (lazy member list subscription).
    fn select_guild(&self, guild: Option<Id>) -> BoxFuture<()>;
    fn select_channel(&self, channel: Id) -> BoxFuture<()>;
    /// Newest-last page of history, `before` a message id when paging up.
    fn load_messages(&self, channel: Id, before: Option<Id>, limit: u32) -> BoxFuture<Vec<Message>>;
    fn send_message(&self, channel: Id, content: String, reply_to: Option<Id>) -> BoxFuture<Message>;
    fn edit_message(&self, channel: Id, id: Id, content: String) -> BoxFuture<Message>;
    fn trigger_typing(&self, channel: Id) -> BoxFuture<()>;
    fn join_voice(&self, channel: Option<Id>) -> BoxFuture<()>;
}
