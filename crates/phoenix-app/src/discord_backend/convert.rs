//! Pure conversion from `phoenix-api` objects/events to the UI model.
//!
//! Nothing here touches the network, so it is unit-tested with the gateway
//! fixtures from `phoenix-api/tests/fixtures`.

use std::collections::{HashMap, HashSet};

use base64::Engine;
use gpui::SharedString;
use phoenix_api::{
    Cache, GatewayEvent,
    gateway::{GuildMemberListUpdate, MemberListItem, MemberListOp, Ready},
    models as api,
};
use serde_json::Value;

use crate::{
    backend::{AppEvent, ReadyData},
    model::{self, Id},
};

/// Avatar size requested from the CDN (largest avatar drawn is 40px, x2 for HiDPI).
pub const AVATAR_SIZE: u32 = 128;
/// Guild icons are drawn at 48px.
pub const ICON_SIZE: u32 = 96;

pub fn id(s: api::Snowflake) -> Id {
    Id(s.0)
}

pub fn sf(i: Id) -> api::Snowflake {
    api::Snowflake(i.0)
}

// ------------------------------------------------------------------ users

pub fn user(u: &api::User) -> model::User {
    let avatar = if u.avatar.is_some() {
        model::ImageRef::Url(u.avatar_url(AVATAR_SIZE).into())
    } else {
        let idx = if u.discriminator.is_empty() || u.discriminator == "0" {
            (u.id.0 >> 22) % 5
        } else {
            u.discriminator.parse::<u64>().unwrap_or(0) % 5
        };
        model::ImageRef::Default(idx as u8)
    };
    model::User {
        id: id(u.id),
        name: u.username.clone().into(),
        display_name: u.display_name().to_owned().into(),
        avatar,
        bot: u.bot || u.system,
        tag: u.primary_guild.as_ref().and_then(|p| Some((p.shown_tag()?.to_owned().into(), p.badge_url().map(Into::into)))),
    }
}

pub fn presence(status: &str) -> model::Presence {
    match status {
        "online" => model::Presence::Online,
        "idle" => model::Presence::Idle,
        "dnd" => model::Presence::Dnd,
        _ => model::Presence::Offline,
    }
}

/// Short activity line for the member list: custom status text, else
/// "Playing X" style for the first activity.
pub fn activity(activities: &[Value]) -> Option<SharedString> {
    let a = activities.first()?;
    let kind = a.get("type").and_then(Value::as_u64).unwrap_or(0);
    let name = a.get("name").and_then(Value::as_str).unwrap_or_default();
    let text = match kind {
        4 => a.get("state").and_then(Value::as_str).unwrap_or_default().to_owned(),
        1 => format!("Streaming {name}"),
        2 => format!("Listening to {name}"),
        3 => format!("Watching {name}"),
        5 => format!("Competing in {name}"),
        _ => format!("Playing {name}"),
    };
    (!text.trim().is_empty()).then(|| text.into())
}

// --------------------------------------------------------------- messages

fn embed_media(m: &Option<api::EmbedMedia>) -> Option<model::EmbedMedia> {
    let m = m.as_ref()?;
    let url = m.proxy_url.clone().filter(|u| u.starts_with("https://")).or_else(|| m.url.clone())?;
    Some(model::EmbedMedia { url: url.into(), width: m.width.unwrap_or(0), height: m.height.unwrap_or(0) })
}

pub fn message(m: &api::Message) -> model::Message {
    crate::commands::store::record(m);
    let reply_to = m
        .message_reference
        .as_ref()
        .filter(|_| m.is_reply())
        .and_then(|r| r.message_id)
        .map(id);
    model::Message {
        id: id(m.id),
        channel: id(m.channel_id),
        author: id(m.author.id),
        timestamp: (m.id.timestamp_ms() / 1000) as i64,
        edited: m.edited_timestamp.is_some(),
        content: m.content.clone().into(),
        reply_to,
        embeds: m
            .embeds
            .iter()
            .filter(|e| e.title.is_some() || e.description.is_some() || e.image.is_some() || e.thumbnail.is_some() || e.author.is_some() || !e.fields.is_empty())
            .map(|e| model::Embed {
                color: e.color,
                provider: e.provider.as_ref().and_then(|p| p.name.clone()).map(Into::into),
                title: e.title.clone().map(Into::into),
                description: e.description.clone().map(Into::into),
                url: e.url.clone().map(Into::into),
                kind: e.kind.clone().map(Into::into),
                thumbnail: embed_media(&e.thumbnail),
                image: embed_media(&e.image),
                video: embed_media(&e.video),
                author: e.author.as_ref().map(|a| model::EmbedAuthor {
                    name: a.name.clone().into(),
                    url: a.url.clone().map(Into::into),
                    icon: a.proxy_icon_url.clone().or_else(|| a.icon_url.clone()).map(Into::into),
                }),
                fields: e
                    .fields
                    .iter()
                    .map(|f| model::EmbedField { name: f.name.clone().into(), value: f.value.clone().into(), inline: f.inline })
                    .collect(),
                footer: e.footer.as_ref().map(|f| model::EmbedFooter {
                    text: f.text.clone().into(),
                    icon: f.proxy_icon_url.clone().or_else(|| f.icon_url.clone()).map(Into::into),
                }),
                timestamp: e
                    .timestamp
                    .as_deref()
                    .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
                    .map(|d| d.timestamp()),
            })
            .collect(),
        attachments: m
            .attachments
            .iter()
            .map(|a| {
                let ct = a.content_type.as_deref().unwrap_or("");
                let sized = a.width.is_some() && a.height.is_some();
                let kind = if ct.starts_with("image/") && sized {
                    model::AttachmentKind::Image
                } else if ct.starts_with("video/") && sized {
                    model::AttachmentKind::Video
                } else {
                    model::AttachmentKind::File
                };
                let proxy = if a.proxy_url.starts_with("https://") { &a.proxy_url } else { &a.url };
                model::Attachment {
                    filename: a.filename.clone().into(),
                    url: if kind == model::AttachmentKind::File { a.url.clone() } else { proxy.clone() }.into(),
                    width: a.width.unwrap_or(0),
                    height: a.height.unwrap_or(0),
                    is_image: kind == model::AttachmentKind::Image,
                    size: a.size,
                    kind,
                    original: a.url.clone().into(),
                }
            })
            .chain(m.sticker_items.iter().map(|st| {
                use crate::media::StickerFormat as F;
                let format = match st.format_type {
                    api::StickerFormat::Apng => F::Apng,
                    api::StickerFormat::Lottie => F::Lottie,
                    api::StickerFormat::Gif => F::Gif,
                    _ => F::Png,
                };
                model::Attachment {
                    filename: st.name.clone().into(),
                    url: crate::media::sticker_url(st.id.0, format).unwrap_or_default(),
                    width: 160,
                    height: 160,
                    kind: model::AttachmentKind::Sticker(format),
                    ..Default::default()
                }
            }))
            .collect(),
        reactions: m
            .reactions
            .iter()
            .map(|r| model::Reaction {
                emoji: match (&r.emoji.id, &r.emoji.name) {
                    (Some(eid), Some(n)) => {
                        format!("<{}:{n}:{}>", if r.emoji.animated { "a" } else { "" }, eid.0).into()
                    }
                    (_, Some(n)) => n.clone().into(),
                    _ => "?".into(),
                },
                count: r.count,
                me: r.me || r.me_burst,
                emoji_id: r.emoji.id.map(id),
            })
            .collect(),
        kind: message_kind(m.kind),
        pinned: m.pinned,
        mention_everyone: m.mention_everyone,
        thread: m.thread.as_ref().map(|t| model::ThreadRef {
            id: id(t.id),
            name: t.name.clone().unwrap_or_default().into(),
            message_count: t.message_count.unwrap_or(0),
        }),
    }
}

fn message_kind(k: api::MessageType) -> model::MessageKind {
    use api::MessageType as T;
    use model::MessageKind as K;
    match k {
        T::Default | T::ChatInputCommand | T::ContextMenuCommand | T::ThreadStarterMessage => K::Default,
        T::Reply => K::Reply,
        T::UserJoin => K::Join,
        T::ChannelPinnedMessage => K::Pin,
        T::GuildBoost => K::Boost(0),
        T::GuildBoostTier1 => K::Boost(1),
        T::GuildBoostTier2 => K::Boost(2),
        T::GuildBoostTier3 => K::Boost(3),
        T::ThreadCreated => K::ThreadCreated,
        T::RecipientAdd => K::RecipientAdd,
        T::RecipientRemove => K::RecipientRemove,
        T::Call => K::Call,
        T::ChannelNameChange => K::ChannelNameChange,
        _ => K::OtherSystem,
    }
}

/// Users referenced by a message (author, replied-to author, mentions).
pub fn message_users(m: &api::Message, out: &mut Vec<model::User>) {
    if !m.author.username.is_empty() {
        out.push(user(&m.author));
    }
    out.extend(m.mentions.iter().map(|x| user(&x.user)));
    if let Some(r) = &m.referenced_message {
        message_users(r, out);
    }
}

// ------------------------------------------------------------ read states

/// Per-channel read marker from READY `read_state` (+ live acks).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReadMark {
    pub last_read: u64,
    pub mentions: u32,
}

fn snowflake_value(v: &Value) -> Option<u64> {
    match v {
        Value::String(s) => s.parse().ok(),
        Value::Number(n) => n.as_u64(),
        _ => None,
    }
}

pub fn parse_read_state(v: Option<&Value>) -> HashMap<u64, ReadMark> {
    let mut out = HashMap::new();
    let entries = match v {
        Some(Value::Array(a)) => a,
        Some(Value::Object(o)) => match o.get("entries") {
            Some(Value::Array(a)) => a,
            _ => return out,
        },
        _ => return out,
    };
    for e in entries {
        // read_state_type 0 (or absent) = channel.
        if e.get("read_state_type").and_then(Value::as_u64).unwrap_or(0) != 0 {
            continue;
        }
        let Some(cid) = e.get("id").and_then(snowflake_value) else { continue };
        let last_read = e.get("last_message_id").and_then(snowflake_value).unwrap_or(0);
        let mentions = e.get("mention_count").and_then(Value::as_u64).unwrap_or(0) as u32;
        out.insert(cid, ReadMark { last_read, mentions });
    }
    out
}

/// Muted guilds and channels from READY `user_guild_settings`.
#[derive(Default, Debug)]
pub struct MuteSettings {
    pub guilds: HashSet<u64>,
    pub channels: HashSet<u64>,
}

pub fn parse_mutes(v: Option<&Value>) -> MuteSettings {
    let mut out = MuteSettings::default();
    let entries = match v {
        Some(Value::Array(a)) => a,
        Some(Value::Object(o)) => match o.get("entries") {
            Some(Value::Array(a)) => a,
            _ => return out,
        },
        _ => return out,
    };
    for e in entries {
        let muted = e.get("muted").and_then(Value::as_bool).unwrap_or(false);
        if muted && let Some(g) = e.get("guild_id").and_then(snowflake_value) {
            out.guilds.insert(g);
        }
        if let Some(Value::Array(ovs)) = e.get("channel_overrides") {
            for o in ovs {
                if o.get("muted").and_then(Value::as_bool).unwrap_or(false)
                    && let Some(c) = o.get("channel_id").and_then(snowflake_value)
                {
                    out.channels.insert(c);
                }
            }
        }
    }
    out
}

// ------------------------------------------------------- guild folder order

/// Minimal protobuf reader: enough to walk `PreloadedUserSettings`.
struct Pb<'a>(&'a [u8]);

impl<'a> Pb<'a> {
    fn varint(&mut self) -> Option<u64> {
        let mut v = 0u64;
        for shift in (0..64).step_by(7) {
            let (&b, rest) = self.0.split_first()?;
            self.0 = rest;
            v |= u64::from(b & 0x7f) << shift;
            if b & 0x80 == 0 {
                return Some(v);
            }
        }
        None
    }
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.0.len() < n {
            return None;
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Some(a)
    }
    /// Next field as (number, wire type, payload for length-delimited / fixed64).
    fn field(&mut self) -> Option<(u64, u8, &'a [u8], u64)> {
        if self.0.is_empty() {
            return None;
        }
        let key = self.varint()?;
        let (num, wt) = (key >> 3, (key & 7) as u8);
        match wt {
            0 => Some((num, wt, &[][..], self.varint()?)),
            1 => {
                let b = self.take(8)?;
                Some((num, wt, b, u64::from_le_bytes(b.try_into().ok()?)))
            }
            2 => {
                let n = self.varint()? as usize;
                Some((num, wt, self.take(n)?, 0))
            }
            5 => {
                let b = self.take(4)?;
                Some((num, wt, b, u64::from(u32::from_le_bytes(b.try_into().ok()?))))
            }
            _ => None,
        }
    }
}

fn fixed64s(wt: u8, payload: &[u8], scalar: u64, out: &mut Vec<u64>) {
    match wt {
        1 => out.push(scalar),
        2 => out.extend(payload.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().unwrap()))),
        _ => {}
    }
}

/// Guild ids in sidebar order from the base64 `user_settings_proto`
/// (`PreloadedUserSettings.guild_folders` = field 14).
pub fn guild_order_from_proto(b64: &str) -> Option<Vec<u64>> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64.trim()).ok()?;
    let mut top = Pb(&bytes);
    while let Some((num, wt, payload, _)) = top.field() {
        if num != 14 || wt != 2 {
            continue;
        }
        let (mut folders, mut positions) = (Vec::new(), Vec::new());
        let mut gf = Pb(payload);
        while let Some((n, w, p, s)) = gf.field() {
            match n {
                1 if w == 2 => {
                    let mut folder = Pb(p);
                    while let Some((fnum, fw, fp, fs)) = folder.field() {
                        if fnum == 1 {
                            fixed64s(fw, fp, fs, &mut folders);
                        }
                    }
                }
                2 => fixed64s(w, p, s, &mut positions),
                _ => {}
            }
        }
        let order = if folders.is_empty() { positions } else { folders };
        return (!order.is_empty()).then_some(order);
    }
    None
}

/// Legacy JSON `user_settings.guild_folders` / `guild_positions`.
pub fn guild_order_from_json(v: Option<&Value>) -> Option<Vec<u64>> {
    let v = v?;
    if let Some(Value::Array(folders)) = v.get("guild_folders") {
        let ids: Vec<u64> = folders
            .iter()
            .filter_map(|f| f.get("guild_ids").and_then(Value::as_array))
            .flatten()
            .filter_map(snowflake_value)
            .collect();
        if !ids.is_empty() {
            return Some(ids);
        }
    }
    let ids: Vec<u64> = v.get("guild_positions")?.as_array()?.iter().filter_map(snowflake_value).collect();
    (!ids.is_empty()).then_some(ids)
}

/// Order `guilds` like the user's sidebar. Guilds missing from the saved
/// order (joined recently) go first, as in the official client.
pub fn order_guilds(guilds: &mut [api::Snowflake], order: Option<&[u64]>) {
    let Some(order) = order else { return };
    let pos: HashMap<u64, usize> = order.iter().enumerate().map(|(i, g)| (*g, i + 1)).collect();
    guilds.sort_by_key(|g| pos.get(&g.0).copied().unwrap_or(0));
}

// ---------------------------------------------------------------- channels

fn channel_kind(k: api::ChannelType) -> Option<model::ChannelKind> {
    use api::ChannelType as T;
    Some(match k {
        T::GuildText | T::Dm | T::GroupDm => model::ChannelKind::Text,
        T::GuildVoice => model::ChannelKind::Voice,
        T::GuildAnnouncement => model::ChannelKind::Announcement,
        T::GuildForum | T::GuildMedia => model::ChannelKind::Forum,
        T::GuildStageVoice => model::ChannelKind::Stage,
        T::GuildCategory => model::ChannelKind::Category,
        _ => return None,
    })
}

/// Everything needed to decorate channels with unread / mute state.
pub struct ReadCtx<'a> {
    pub reads: &'a HashMap<u64, ReadMark>,
    pub mutes: &'a MuteSettings,
}

impl ReadCtx<'_> {
    pub fn unread(&self, c: &api::Channel) -> (bool, u32) {
        if c.kind.is_voice() || c.kind == api::ChannelType::GuildCategory {
            return (false, 0);
        }
        let last = c.last_message_id.map(|m| m.0).unwrap_or(0);
        match self.reads.get(&c.id.0) {
            Some(r) => (last > r.last_read, r.mentions),
            // Never opened: only DMs count as unread.
            None => (c.kind.is_private() && last != 0, 0),
        }
    }
}

fn channel(c: &api::Channel, position: i32, rc: &ReadCtx, voice_users: Vec<Id>) -> Option<model::Channel> {
    let kind = channel_kind(c.kind)?;
    let (unread, mentions) = rc.unread(c);
    Some(model::Channel {
        id: id(c.id),
        guild: c.guild_id.map(id),
        parent: c.parent_id.map(id),
        name: c.display_name().into(),
        topic: c.topic.clone().filter(|t| !t.is_empty()).map(Into::into),
        kind,
        position,
        unread,
        muted: rc.mutes.channels.contains(&c.id.0),
        mentions,
        voice_users,
    })
}

/// A guild's visible channels in sidebar order. `position` is rewritten to
/// the display index so the UI can sort by it directly.
pub fn guild_channels(cache: &Cache, guild: api::Snowflake, rc: &ReadCtx) -> Vec<model::Channel> {
    // Without our own member object (roles) permissions can't be computed;
    // show everything rather than nothing.
    let perms_known = cache.current_user().is_some_and(|me| cache.member(guild, me.id).is_some());
    let all: Vec<_> = cache
        .guild_channels(guild)
        .into_iter()
        .filter(|c| !c.kind.is_thread() && !c.is_guild_resource() && channel_kind(c.kind).is_some())
        .filter(|c| !perms_known || cache.permissions_in(c.id).contains(api::Permissions::VIEW_CHANNEL))
        .collect();
    let key = |c: &api::Channel| (c.kind.is_voice(), c.position.unwrap_or(0), c.id.0);
    let mut top: Vec<_> = all.iter().filter(|c| c.kind != api::ChannelType::GuildCategory && c.parent_id.is_none()).collect();
    top.sort_by_key(|c| key(c));
    let mut cats: Vec<_> = all.iter().filter(|c| c.kind == api::ChannelType::GuildCategory).collect();
    cats.sort_by_key(|c| (c.position.unwrap_or(0), c.id.0));

    let mut voice: HashMap<u64, Vec<Id>> = HashMap::new();
    for v in cache.voice_states(Some(guild)) {
        if let Some(ch) = v.channel_id {
            voice.entry(ch.0).or_default().push(id(v.user_id));
        }
    }

    let mut ordered: Vec<&api::Channel> = top.iter().map(|c| &***c).collect();
    for cat in cats {
        let mut kids: Vec<_> = all.iter().filter(|c| c.parent_id == Some(cat.id) && c.kind != api::ChannelType::GuildCategory).collect();
        if kids.is_empty() {
            continue; // Discord hides categories with no visible channel.
        }
        kids.sort_by_key(|c| key(c));
        ordered.push(cat);
        ordered.extend(kids.into_iter().map(|c| &**c));
    }
    // Children of a category the user can't see: show them at the end.
    for c in &all {
        if let Some(p) = c.parent_id
            && c.kind != api::ChannelType::GuildCategory
            && !ordered.iter().any(|o| o.id == c.id)
            && !all.iter().any(|x| x.id == p)
        {
            ordered.push(c);
        }
    }
    ordered
        .into_iter()
        .enumerate()
        .filter_map(|(i, c)| channel(c, i as i32, rc, voice.remove(&c.id.0).unwrap_or_default()))
        .collect()
}

/// DMs/group DMs, newest activity first. The UI shows the first entry of
/// `voice_users` as the DM avatar, so the recipient goes there.
pub fn dm_channel(c: &api::Channel, rc: &ReadCtx) -> Option<model::Channel> {
    let recip = c.recipient_user_ids().first().copied().map(id).into_iter().collect();
    let mut ch = channel(c, 0, rc, recip)?;
    ch.guild = None;
    Some(ch)
}

pub fn dm_channels(cache: &Cache, rc: &ReadCtx) -> Vec<model::Channel> {
    let mut dms = cache.private_channels();
    dms.sort_by_key(|c| std::cmp::Reverse(c.last_message_id.unwrap_or(c.id).0));
    dms.iter().filter_map(|c| dm_channel(c, rc)).collect()
}

pub fn roles(g: &api::Guild) -> Vec<model::Role> {
    g.roles
        .iter()
        .map(|r| model::Role {
            id: id(r.id),
            name: r.name.clone().into(),
            color: (r.color != 0).then_some(r.color),
            position: r.position,
            hoist: r.hoist,
        })
        .collect()
}

pub fn guild(g: &api::Guild, channels: &[model::Channel], muted: bool) -> model::Guild {
    let visible = channels.iter().filter(|c| !c.muted);
    model::Guild {
        id: id(g.id),
        name: g.name().to_owned().into(),
        acronym: model::acronym(g.name()),
        icon: match g.icon_url(ICON_SIZE) {
            Some(u) => model::ImageRef::Url(u.into()),
            None => model::ImageRef::Default((g.id.0 % 5) as u8),
        },
        unread: !muted && visible.clone().any(|c| c.unread),
        mentions: channels.iter().map(|c| c.mentions).sum(),
        banner_color: None,
        owner: g.owner_id().map(id),
    }
}

// -------------------------------------------------------------------- READY

pub struct Prefs {
    pub reads: HashMap<u64, ReadMark>,
    pub mutes: MuteSettings,
}

impl Prefs {
    pub fn from_ready(r: &Ready) -> Self {
        Self { reads: parse_read_state(r.read_state.as_ref()), mutes: parse_mutes(r.user_guild_settings.as_ref()) }
    }
    pub fn ctx(&self) -> ReadCtx<'_> {
        ReadCtx { reads: &self.reads, mutes: &self.mutes }
    }
}

/// Initial UI state. `cache` must already have applied `ready`.
pub fn build_ready(cache: &Cache, ready: &Ready, prefs: &Prefs) -> ReadyData {
    let rc = prefs.ctx();
    let me = cache.current_user().map(|u| user(&u)).unwrap_or_else(|| user(&ready.user));

    let mut ids: Vec<api::Snowflake> = cache.guilds().iter().filter(|g| !g.unavailable).map(|g| g.id).collect();
    // Keep READY order as the base (cache order is a HashMap).
    let ready_pos: HashMap<u64, usize> = ready.guilds.iter().enumerate().map(|(i, g)| (g.id.0, i)).collect();
    ids.sort_by_key(|g| ready_pos.get(&g.0).copied().unwrap_or(usize::MAX));
    let order = ready
        .user_settings_proto
        .as_deref()
        .and_then(guild_order_from_proto)
        .or_else(|| guild_order_from_json(ready.user_settings.as_ref()));
    order_guilds(&mut ids, order.as_deref());

    let mut guilds = Vec::new();
    let mut channels = Vec::new();
    let mut roles_out = Vec::new();
    for gid in ids {
        let Some(g) = cache.guild(gid) else { continue };
        let chs = guild_channels(cache, gid, &rc);
        guilds.push(guild(&g, &chs, prefs.mutes.guilds.contains(&gid.0)));
        roles_out.push((id(gid), roles(&g)));
        channels.extend(chs);
    }

    let mut users: HashMap<u64, model::User> = HashMap::new();
    for u in ready.users.iter().chain(ready.relationships.iter().filter_map(|r| r.user.as_ref())) {
        users.insert(u.id.0, user(u));
    }
    for c in cache.private_channels() {
        for u in &c.recipients {
            users.entry(u.id.0).or_insert_with(|| user(u));
        }
    }
    users.insert(me.id.0, me.clone());

    ReadyData {
        me,
        users: users.into_values().collect(),
        guilds,
        channels,
        roles: roles_out,
        dm_channels: dm_channels(cache, &rc),
        presences: Vec::new(),
    }
}

// --------------------------------------------------------- live event map

/// One guild's lazy member list (op 37 / GUILD_MEMBER_LIST_UPDATE).
#[derive(Default)]
struct MemberList {
    id: String,
    items: Vec<Option<MemberListItem>>,
}

impl MemberList {
    fn ensure(&mut self, len: usize) {
        if self.items.len() < len {
            self.items.resize(len, None);
        }
    }
    fn apply(&mut self, ops: &[MemberListOp]) {
        for op in ops {
            match op {
                MemberListOp::Sync { range, items } => {
                    let start = range[0] as usize;
                    self.ensure(start + items.len());
                    for (i, it) in items.iter().enumerate() {
                        self.items[start + i] = Some(it.clone());
                    }
                }
                MemberListOp::Insert { index, item } => {
                    let i = (*index as usize).min(self.items.len());
                    self.items.insert(i, Some(item.clone()));
                }
                MemberListOp::Update { index, item } => {
                    let i = *index as usize;
                    self.ensure(i + 1);
                    self.items[i] = Some(item.clone());
                }
                MemberListOp::Delete { index } => {
                    let i = *index as usize;
                    if i < self.items.len() {
                        self.items.remove(i);
                    }
                }
                MemberListOp::Invalidate { range } => {
                    let end = (range[1] as usize + 1).min(self.items.len());
                    for it in self.items.iter_mut().take(end).skip(range[0] as usize) {
                        *it = None;
                    }
                }
            }
        }
    }
    fn members(&self) -> (Vec<model::Member>, Vec<model::User>) {
        let mut seen = HashSet::new();
        let (mut ms, mut us) = (Vec::new(), Vec::new());
        for it in self.items.iter().flatten() {
            let MemberListItem::Member(m) = it else { continue };
            let Some(u) = &m.member.user else { continue };
            if !seen.insert(u.id.0) {
                continue;
            }
            us.push(user(u));
            let p = m.presence.as_ref();
            ms.push(model::Member {
                user: id(u.id),
                roles: m.member.roles.iter().copied().map(id).collect(),
                presence: presence(p.and_then(|p| p.get("status")).and_then(Value::as_str).unwrap_or("offline")),
                activity: p
                    .and_then(|p| p.get("activities"))
                    .and_then(Value::as_array)
                    .and_then(|a| activity(a)),
            });
        }
        (ms, us)
    }
}

/// Stateful translation of gateway events (after they were applied to the
/// cache) into UI events.
pub struct Adapter {
    pub me: api::Snowflake,
    pub prefs: Prefs,
    lists: HashMap<u64, MemberList>,
}

impl Adapter {
    pub fn new(me: api::Snowflake, prefs: Prefs) -> Self {
        Self { me, prefs, lists: HashMap::new() }
    }

    /// Mark a channel read up to `msg` (local state; the REST ack is separate).
    pub fn mark_read(&mut self, channel: u64, msg: u64) {
        let e = self.prefs.reads.entry(channel).or_default();
        e.last_read = e.last_read.max(msg);
        e.mentions = 0;
    }

    fn guild_state(&self, cache: &Cache, g: api::Snowflake, out: &mut Vec<AppEvent>) {
        let rc = self.prefs.ctx();
        let chs = guild_channels(cache, g, &rc);
        if let Some(gg) = cache.guild(g) {
            let m = guild(&gg, &chs, self.prefs.mutes.guilds.contains(&g.0));
            out.push(AppEvent::GuildUnread { guild: m.id, unread: m.unread, mentions: m.mentions });
        }
        out.push(AppEvent::GuildChannels { guild: id(g), channels: chs });
    }

    fn list_update(&mut self, l: &GuildMemberListUpdate, out: &mut Vec<AppEvent>) {
        let has_full_sync = l.ops.iter().any(|o| matches!(o, MemberListOp::Sync { range, .. } if range[0] == 0));
        let list = self.lists.entry(l.guild_id.0).or_default();
        if list.id != l.id {
            if !has_full_sync && !list.id.is_empty() {
                return; // Update for a list we don't display.
            }
            *list = MemberList { id: l.id.clone(), items: Vec::new() };
        }
        list.apply(&l.ops);
        // Rows past the synced ranges are placeholders, so the sidebar can scroll to them.
        let total: usize = l.groups.iter().filter(|g| g.count > 0).map(|g| g.count as usize + 1).sum();
        if total > 0 {
            list.items.resize(total, None);
        }
        let (members, users) = list.members();
        out.push(AppEvent::Users(users));
        out.push(AppEvent::GuildMembers { guild: id(l.guild_id), members });
        let (rows, acts) = crate::presence::list_rows(list.items.iter());
        out.extend(acts.into_iter().map(|(user, activities)| AppEvent::Activities { user, activities }));
        out.push(AppEvent::MemberListRows { guild: id(l.guild_id), rows });
    }

    pub fn map(&mut self, cache: &Cache, ev: &GatewayEvent) -> Vec<AppEvent> {
        let mut out = Vec::new();
        match ev {
            GatewayEvent::MessageCreate(m) => {
                let mut users = Vec::new();
                message_users(m, &mut users);
                out.push(AppEvent::Users(users));
                if m.author.id == self.me {
                    self.mark_read(m.channel_id.0, m.id.0);
                } else if m.mentions_user(self.me) || m.mention_everyone {
                    self.prefs.reads.entry(m.channel_id.0).or_default().mentions += 1;
                }
                out.push(AppEvent::MessageCreate(message(m)));
                if let Some(g) = m.guild_id
                    && let Some(gg) = cache.guild(g)
                {
                    let chs = guild_channels(cache, g, &self.prefs.ctx());
                    let mg = guild(&gg, &chs, self.prefs.mutes.guilds.contains(&g.0));
                    out.push(AppEvent::GuildUnread { guild: mg.id, unread: mg.unread, mentions: mg.mentions });
                } else if m.guild_id.is_none()
                    && let Some(c) = cache.channel(m.channel_id)
                    && let Some(dm) = dm_channel(&c, &self.prefs.ctx())
                {
                    // Moves the DM to the top of the list.
                    out.push(AppEvent::DmUpsert(dm));
                }
            }
            GatewayEvent::MessageUpdate(v) => {
                let cid = v.get("channel_id").and_then(snowflake_value);
                let mid = v.get("id").and_then(snowflake_value);
                if let (Some(c), Some(mid)) = (cid, mid)
                    && let Some(m) = cache.message(api::Snowflake(c), api::Snowflake(mid))
                {
                    out.push(AppEvent::MessageUpdate(message(&m)));
                }
            }
            GatewayEvent::MessageReactionAdd(r) | GatewayEvent::MessageReactionRemove(r) => {
                if let Some(m) = cache.message(r.channel_id, r.message_id) {
                    out.push(AppEvent::MessageUpdate(message(&m)));
                }
            }
            GatewayEvent::MessageReactionRemoveAll(r) | GatewayEvent::MessageReactionRemoveEmoji(r) => {
                if let Some(m) = cache.message(r.channel_id, r.message_id) {
                    out.push(AppEvent::MessageUpdate(message(&m)));
                }
            }
            GatewayEvent::MessageDelete(d) => {
                out.push(AppEvent::MessageDelete { channel: id(d.channel_id), id: id(d.id) });
            }
            GatewayEvent::MessageDeleteBulk(d) => {
                out.extend(d.ids.iter().map(|m| AppEvent::MessageDelete { channel: id(d.channel_id), id: id(*m) }));
            }
            GatewayEvent::MessageAck(a) => {
                self.mark_read(a.channel_id.0, a.message_id.0);
                out.push(AppEvent::ChannelUnread { channel: id(a.channel_id), unread: false, mentions: 0 });
                if let Some(c) = cache.channel(a.channel_id)
                    && let Some(g) = c.guild_id
                    && let Some(gg) = cache.guild(g)
                {
                    let chs = guild_channels(cache, g, &self.prefs.ctx());
                    let mg = guild(&gg, &chs, self.prefs.mutes.guilds.contains(&g.0));
                    out.push(AppEvent::GuildUnread { guild: mg.id, unread: mg.unread, mentions: mg.mentions });
                }
            }
            GatewayEvent::TypingStart(t) => {
                if let Some(u) = t.member.as_ref().and_then(|m| m.user.as_ref()) {
                    out.push(AppEvent::Users(vec![user(u)]));
                }
                out.push(AppEvent::TypingStart { channel: id(t.channel_id), user: id(t.user_id) });
            }
            GatewayEvent::PresenceUpdate(p) => {
                out.push(AppEvent::PresenceUpdate { user: id(p.user.id), presence: presence(&p.status) });
                let activities = crate::presence::parse_activities(&p.activities);
                out.push(AppEvent::Activities { user: id(p.user.id), activities });
            }
            GatewayEvent::ReadySupplemental(r) => {
                if let Some(Value::Object(mp)) = &r.merged_presences {
                    let friends = mp.get("friends").and_then(Value::as_array).into_iter().flatten();
                    let guilds = mp.get("guilds").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_array).flatten();
                    for p in friends.chain(guilds) {
                        let uid = p.get("user_id").or_else(|| p.get("user").and_then(|u| u.get("id"))).and_then(snowflake_value);
                        if let Some(u) = uid {
                            let st = p.get("status").and_then(Value::as_str).unwrap_or("offline");
                            out.push(AppEvent::PresenceUpdate { user: Id(u), presence: presence(st) });
                            let a = p.get("activities").and_then(Value::as_array).map(|a| crate::presence::parse_activities(a));
                            out.push(AppEvent::Activities { user: Id(u), activities: a.unwrap_or_default() });
                        }
                    }
                }
                for g in &r.guilds {
                    for v in &g.voice_states {
                        if v.channel_id.is_some() {
                            out.push(AppEvent::VoiceStateUpdate { channel: v.channel_id.map(id), user: id(v.user_id) });
                            out.push(AppEvent::VoiceFlags { user: id(v.user_id), flags: voice_flags(v) });
                        }
                    }
                }
            }
            GatewayEvent::GuildMemberListUpdate(l) => self.list_update(l, &mut out),
            GatewayEvent::GuildMembersChunk(c) => {
                out.push(AppEvent::Users(c.members.iter().filter_map(|m| m.user.as_ref()).map(user).collect()));
            }
            GatewayEvent::UserUpdate(u) => out.push(AppEvent::Users(vec![user(u)])),
            GatewayEvent::GuildCreate(g) => {
                let rc = self.prefs.ctx();
                let chs = guild_channels(cache, g.id, &rc);
                if let Some(gg) = cache.guild(g.id) {
                    out.push(AppEvent::GuildCreate {
                        guild: guild(&gg, &chs, self.prefs.mutes.guilds.contains(&g.id.0)),
                        channels: chs,
                        roles: roles(&gg),
                    });
                }
            }
            GatewayEvent::GuildUpdate(g) => {
                if let Some(gg) = cache.guild(g.id) {
                    out.push(AppEvent::GuildRoles { guild: id(g.id), roles: roles(&gg) });
                }
            }
            GatewayEvent::GuildDelete(d) => {
                if !d.unavailable {
                    out.push(AppEvent::GuildDelete(id(d.id)));
                }
            }
            GatewayEvent::GuildRoleCreate(r) | GatewayEvent::GuildRoleUpdate(r) => {
                if let Some(gg) = cache.guild(r.guild_id) {
                    out.push(AppEvent::GuildRoles { guild: id(r.guild_id), roles: roles(&gg) });
                }
                // Role changes can hide/show channels.
                self.guild_state(cache, r.guild_id, &mut out);
            }
            GatewayEvent::GuildRoleDelete(r) => {
                if let Some(gg) = cache.guild(r.guild_id) {
                    out.push(AppEvent::GuildRoles { guild: id(r.guild_id), roles: roles(&gg) });
                }
                self.guild_state(cache, r.guild_id, &mut out);
            }
            GatewayEvent::GuildMemberUpdate(m) if m.user_id() == Some(self.me) => {
                if let Some(g) = m.guild_id {
                    self.guild_state(cache, g, &mut out);
                }
            }
            GatewayEvent::ChannelCreate(c) | GatewayEvent::ChannelUpdate(c) | GatewayEvent::ChannelDelete(c) => {
                match c.guild_id {
                    Some(g) => self.guild_state(cache, g, &mut out),
                    None if matches!(ev, GatewayEvent::ChannelDelete(_)) => out.push(AppEvent::DmDelete(id(c.id))),
                    None => {
                        out.push(AppEvent::Users(c.recipients.iter().map(user).collect()));
                        let c = cache.channel(c.id).unwrap_or_else(|| std::sync::Arc::new((**c).clone()));
                        if let Some(dm) = dm_channel(&c, &self.prefs.ctx()) {
                            out.push(AppEvent::DmUpsert(dm));
                        }
                    }
                }
            }
            GatewayEvent::VoiceStateUpdate(v) => {
                if let Some(u) = v.member.as_ref().and_then(|m| m.user.as_ref()) {
                    out.push(AppEvent::Users(vec![user(u)]));
                }
                out.push(AppEvent::VoiceStateUpdate { channel: v.channel_id.map(id), user: id(v.user_id) });
                out.push(AppEvent::VoiceFlags { user: id(v.user_id), flags: voice_flags(v) });
            }
            GatewayEvent::Fatal { code, reason } => {
                out.push(AppEvent::Disconnected(fatal_message(*code, reason)));
            }
            _ => {}
        }
        out
    }
}

pub fn fatal_message(code: u16, reason: &str) -> String {
    match code {
        4004 => "Your session is no longer valid. Please log in again.".into(),
        _ if reason.is_empty() => format!("Disconnected from Discord (code {code})."),
        _ => format!("Disconnected from Discord: {reason} (code {code})."),
    }
}

#[cfg(test)]
mod tests;

/// `available_tags` of a forum / media channel.
pub fn forum_tags(c: &api::Channel) -> Vec<crate::commands::ForumTag> {
    c.available_tags
        .iter()
        .map(|t| crate::commands::ForumTag {
            id: id(t.id),
            name: t.name.clone().into(),
            emoji: match (t.emoji_id, &t.emoji_name) {
                (Some(e), n) => Some(format!("<:{}:{}>", n.as_deref().unwrap_or("e"), e.0)),
                (None, Some(n)) if !n.is_empty() => Some(n.clone()),
                _ => None,
            },
            moderated: t.moderated,
        })
        .collect()
}

fn voice_flags(v: &phoenix_api::models::VoiceState) -> crate::model::VoiceFlags {
    crate::model::VoiceFlags {
        self_mute: v.self_mute,
        self_deaf: v.self_deaf,
        server_mute: v.mute,
        server_deaf: v.deaf,
        stream: v.self_stream.unwrap_or(false),
        video: v.self_video,
    }
}

/// Own activities (custom status, games) from READY `sessions`, which the
/// gateway doesn't echo back as a PRESENCE_UPDATE: the user panel subtext
/// shows the custom status from them (vanilla-guild-text user panel).
pub fn self_activities(r: &phoenix_api::gateway::events::Ready) -> Option<AppEvent> {
    // PreloadedUserSettings.status (11).custom_status (2).text (1).
    let proto = r.user_settings_proto.as_deref().and_then(|b| base64::engine::general_purpose::STANDARD.decode(b.trim()).ok());
    let from_proto = proto.as_deref().and_then(|b| {
        let st = super::nav::fields(b).into_iter().find(|f| f.0 == 11 && f.1 == 2)?.2;
        let cs = super::nav::fields(st).into_iter().find(|f| f.0 == 2 && f.1 == 2)?.2;
        let text = super::nav::fields(cs).into_iter().find(|f| f.0 == 1 && f.1 == 2)?.2;
        std::str::from_utf8(text).ok().filter(|t| !t.is_empty()).map(str::to_owned)
    });
    if let Some(text) = from_proto {
        let a = serde_json::json!({"type": 4, "name": "Custom Status", "state": text});
        return Some(AppEvent::Activities { user: id(r.user.id), activities: crate::presence::parse_activities(&[a]) });
    }
    let sessions = r.sessions.as_ref()?.as_array()?;
    let s = sessions.iter().find(|s| s.get("session_id").and_then(Value::as_str) == Some("all")).or_else(|| sessions.first())?;
    let acts = s.get("activities").and_then(Value::as_array)?;
    Some(AppEvent::Activities { user: id(r.user.id), activities: crate::presence::parse_activities(acts) })
}
