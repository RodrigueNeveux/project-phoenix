//! Adapter seam between the UI and the network layer.
//!
//! The UI only talks to a `dyn Backend` and consumes `AppEvent`s. Two
//! implementations exist: [`crate::discord_backend::DiscordBackend`] (the real
//! `phoenix-api` adapter: REST calls -> futures, gateway dispatches ->
//! `AppEvent`s pushed through the [`EventSink`]) and
//! [`crate::mock::MockBackend`] (offline data for tests and screenshots).

use std::{future::Future, pin::Pin, sync::Arc};

use crate::model::{Channel, Guild, Id, Member, Message, Presence, Reaction, Role, User};

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
    /// Voice call runtime state (connection, ping, speaking, encryption).
    Voice(crate::voice::VoiceUiEvent),
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
    /// Friends, guild folders, threads, channel decorations (`ui::home`).
    Nav(crate::ui::home::data::NavEvent),
    /// Server-ordered lazy member list of a guild (groups + members).
    MemberListRows { guild: Id, rows: Vec<crate::presence::ListEntry> },
    /// Rich presence of a user (empty = no activity).
    Activities { user: Id, activities: Vec<crate::presence::Activity> },
    /// QR code login progress (see [`Backend::remote_auth`]).
    RemoteAuth(crate::auth::RemoteAuthUpdate),
    /// Fresh READY after a reconnect that could not resume.
    Resync(crate::commands::Resync),
    /// Bot modals, interaction failures, live poll votes.
    Parity(crate::commands::ParityEvent),
}

/// A file to attach to an outgoing message.
#[derive(Clone)]
pub struct Upload {
    pub filename: String,
    pub bytes: Vec<u8>,
    pub spoiler: bool,
}

impl std::fmt::Debug for Upload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Upload").field("filename", &self.filename).field("len", &self.bytes.len()).finish()
    }
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
    /// Mute/deafen, devices and volumes of the voice call.
    fn voice_control(&self, control: crate::voice::VoiceControl) -> BoxFuture<()> {
        let _ = control;
        Box::pin(async { Ok(()) })
    }
    fn delete_message(&self, channel: Id, id: Id) -> BoxFuture<()>;
    /// Send with reply options (`mention` pings the replied-to author) and
    /// file attachments. Defaults to [`Backend::send_message`] without files.
    fn send_message_ex(
        &self,
        channel: Id,
        content: String,
        reply: Option<(Id, bool)>,
        files: Vec<Upload>,
    ) -> BoxFuture<Message> {
        let _ = files;
        self.send_message(channel, content, reply.map(|r| r.0))
    }
    /// Add (`add`) or remove our own reaction.
    fn set_reaction(&self, channel: Id, msg: Id, emoji: Reaction, add: bool) -> BoxFuture<()> {
        let _ = (channel, msg, emoji, add);
        Box::pin(async { Ok(()) })
    }
    fn set_pinned(&self, channel: Id, msg: Id, pinned: bool) -> BoxFuture<()> {
        let _ = (channel, msg, pinned);
        Box::pin(async { Ok(()) })
    }
    /// Pinned messages of a channel, newest first.
    fn load_pins(&self, channel: Id) -> BoxFuture<Vec<Message>> {
        let _ = channel;
        Box::pin(async { Ok(Vec::new()) })
    }
    /// "Mark Unread": `last_read` is the message just before the one clicked.
    fn mark_unread(&self, channel: Id, last_read: Id) -> BoxFuture<()> {
        let _ = (channel, last_read);
        Box::pin(async { Ok(()) })
    }
    /// Log in from a stored session at startup, if the backend has one.
    /// The future fails with an empty message when nothing is stored.
    fn restore_session(&self) -> Option<BoxFuture<ReadyData>> {
        None
    }
    /// True after a password login asked for a TOTP code.
    fn needs_mfa(&self) -> bool {
        false
    }
    /// Custom emojis of a guild (from the cache).
    fn guild_emojis(&self, _guild: Id) -> Vec<crate::ui::pickers::types::GuildEmoji> {
        Vec::new()
    }
    fn guild_stickers(&self, _guild: Id) -> Vec<crate::ui::pickers::types::StickerData> {
        Vec::new()
    }
    fn sticker_packs(&self) -> BoxFuture<Vec<crate::ui::pickers::types::StickerPackData>> {
        Box::pin(async { Ok(Vec::new()) })
    }
    fn send_sticker(&self, _channel: Id, _sticker: Id) -> BoxFuture<()> {
        Box::pin(async { Err(BackendError("Stickers aren't available.".into())) })
    }
    fn gif_trending(&self) -> BoxFuture<crate::ui::pickers::types::GifHome> {
        Box::pin(async { Err(BackendError("GIFs aren't available.".into())) })
    }
    fn gif_trending_gifs(&self) -> BoxFuture<Vec<crate::ui::pickers::types::GifItem>> {
        Box::pin(async { Ok(Vec::new()) })
    }
    fn gif_search(&self, _query: String) -> BoxFuture<Vec<crate::ui::pickers::types::GifItem>> {
        Box::pin(async { Ok(Vec::new()) })
    }
    fn search_messages(&self, _req: crate::ui::search::SearchRequest) -> BoxFuture<crate::ui::search::SearchPage> {
        Box::pin(async { Err(BackendError("Search isn't available.".into())) })
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
    /// The account's per-guild notification levels and mutes (READY
    /// `user_guild_settings`, kept current by updates).
    fn notify_prefs(&self) -> Option<crate::notifications::NotifyPrefs> {
        None
    }
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
    /// Navigation REST calls: invites, relationships, leaving a guild...
    fn nav(&self, _req: crate::ui::home::data::NavRequest) -> BoxFuture<crate::ui::home::data::NavReply> {
        Box::pin(async { Err(BackendError("Not available offline.".into())) })
    }
    /// Op 37 member list ranges for the open channel (sidebar scrolling).
    fn subscribe_member_ranges(&self, _guild: Id, _channel: Id, _ranges: Vec<[u32; 2]>) -> BoxFuture<()> {
        Box::pin(async { Ok(()) })
    }
    /// `GET /users/{id}/profile` (+ note).
    fn user_profile(&self, _user: Id, _guild: Option<Id>) -> BoxFuture<crate::presence::Profile> {
        Box::pin(async { Err(BackendError("Profiles aren't available.".into())) })
    }
    fn set_note(&self, _user: Id, _note: String) -> BoxFuture<()> {
        Box::pin(async { Ok(()) })
    }
    /// Own status (op 3 + settings). `custom` = custom status text.
    fn set_status(&self, _status: Presence, _custom: Option<String>) -> BoxFuture<()> {
        Box::pin(async { Ok(()) })
    }
    /// Slash commands, interactions, inbox, moderation, forums, polls (`crate::commands`).
    fn parity(&self, _req: crate::commands::ParityRequest) -> BoxFuture<crate::commands::ParityReply> {
        Box::pin(async { Err(BackendError("Not available offline.".into())) })
    }
    /// Our guild-level permission bits (moderation menu entries).
    fn guild_permissions(&self, _guild: Id) -> u64 {
        0
    }
    /// Last message we read in `channel` (for the "NEW" divider).
    fn last_read(&self, _channel: Id) -> Option<Id> {
        None
    }
    /// Tags of a forum / media channel.
    fn forum_tags(&self, _forum: Id) -> Vec<crate::commands::ForumTag> {
        Vec::new()
    }
    /// Open (or create) the DM with `user`; emits `DmUpsert` first.
    fn open_dm(&self, _user: Id) -> BoxFuture<Id> {
        Box::pin(async { Err(BackendError("DMs can't be opened here.".into())) })
    }
}
