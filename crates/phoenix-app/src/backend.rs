//! Adapter seam between the UI and the network layer.
//!
//! The UI only talks to a `dyn Backend` and consumes `AppEvent`s. Two
//! implementations exist: [`crate::discord_backend::DiscordBackend`] (the real
//! `phoenix-api` adapter: REST calls -> futures, gateway dispatches ->
//! `AppEvent`s pushed through the [`EventSink`]) and
//! [`crate::mock::MockBackend`] (offline data for tests and screenshots).

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
    /// Second step of a password login when [`Backend::needs_mfa`] is true.
    Mfa { code: String },
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Token(_) => f.write_str("Token(<redacted>)"),
            Self::Password { login, .. } => {
                f.debug_struct("Password").field("login", login).field("password", &"<redacted>").finish()
            }
            Self::Mfa { .. } => f.write_str("Mfa(<redacted>)"),
        }
    }
}

/// Everything the UI needs once logged in ("READY").
pub struct ReadyData {
    pub me: User,
    pub users: Vec<User>,
    pub guilds: Vec<Guild>,
    pub channels: Vec<Channel>,
    pub roles: Vec<(Id, Vec<Role>)>,
    pub dm_channels: Vec<Channel>,
    /// Known presences at login (friends, DM recipients).
    pub presences: Vec<(Id, Presence)>,
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
    /// New or updated user objects (message authors, member list entries...).
    Users(Vec<User>),
    /// Replace the whole (ordered) channel list of a guild.
    GuildChannels { guild: Id, channels: Vec<Channel> },
    GuildRoles { guild: Id, roles: Vec<Role> },
    GuildUnread { guild: Id, unread: bool, mentions: u32 },
    /// Joined a guild (or it became available again).
    GuildCreate { guild: Guild, channels: Vec<Channel>, roles: Vec<Role> },
    GuildDelete(Id),
    /// A DM / group DM was opened or changed.
    DmUpsert(Channel),
    DmDelete(Id),
    Disconnected(String),
    /// QR code login progress (see [`Backend::remote_auth`]).
    RemoteAuth(crate::auth::RemoteAuthUpdate),
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
    fn delete_message(&self, channel: Id, id: Id) -> BoxFuture<()>;
    /// Log in from a stored session at startup, if the backend has one.
    /// The future fails with an empty message when nothing is stored.
    fn restore_session(&self) -> Option<BoxFuture<ReadyData>> {
        None
    }
    /// True after a password login asked for a TOTP code.
    fn needs_mfa(&self) -> bool {
        false
    }
    /// Second-factor methods offered by the pending password login.
    fn mfa_options(&self) -> crate::auth::MfaOptions {
        crate::auth::MfaOptions { totp: true, ..Default::default() }
    }
    /// Finish a password login with a TOTP, SMS or backup code.
    fn login_mfa(&self, _method: crate::auth::MfaMethod, code: String) -> BoxFuture<ReadyData> {
        self.login(Credentials::Mfa { code })
    }
    /// Text an MFA code; resolves to the redacted phone number.
    fn send_mfa_sms(&self) -> BoxFuture<String> {
        Box::pin(async { Err(BackendError("SMS isn't available.".into())) })
    }
    /// Start a QR code login. Progress arrives as [`AppEvent::RemoteAuth`];
    /// resolves once the phone confirmed (empty error when cancelled/expired).
    fn remote_auth(&self) -> Option<BoxFuture<ReadyData>> {
        None
    }
    fn cancel_remote_auth(&self) {}
    /// Synced user settings (settings-proto, JSON fallback).
    fn load_settings(&self) -> BoxFuture<crate::ui::settings::SyncedSettings> {
        Box::pin(async { Ok(Default::default()) })
    }
    fn save_settings(&self, patch: crate::ui::settings::SettingsPatch) -> BoxFuture<crate::ui::settings::SyncedSettings> {
        Box::pin(async move {
            let mut s = crate::ui::settings::SyncedSettings::default();
            s.apply(&patch);
            Ok(s)
        })
    }
    /// Close the session and forget stored credentials.
    fn logout(&self) -> BoxFuture<()> {
        Box::pin(async { Ok(()) })
    }
}
