use std::sync::LazyLock;

use phoenix_api::{
    Cache,
    models::{self as api, ChannelType, MessageReferenceType, MessageType, RelationshipType},
};
use regex::{Captures, Regex};

pub const FSI: char = '\u{2068}';
pub const PDI: char = '\u{2069}';
pub const IS_VOICE_MESSAGE: u64 = 1 << 13;
const DISCORD_EPOCH_MS: u64 = 1_420_070_400_000;

#[derive(Clone, Debug, PartialEq)]
pub struct Chan {
    pub id: u64,
    pub kind: ChannelType,
    pub name: String,
    pub parent: Option<u64>,
    pub guild: Option<u64>,
}

pub trait Lookup {
    fn channel(&self, id: u64) -> Option<Chan>;
    fn member_name(&self, guild: Option<u64>, user: &api::User) -> String;
    fn user(&self, id: u64) -> Option<api::User>;
    fn role(&self, guild: u64, id: u64) -> Option<String>;
    fn guild_exists(&self, id: u64) -> bool;
    fn guild_name(&self, _id: u64) -> Option<String> {
        None
    }
    fn me(&self) -> Option<u64> {
        None
    }
}

pub const USER_MESSAGE: [u8; 9] = [0, 19, 20, 23, 26, 41, 45, 47, 68];

pub fn is_user_message(kind: MessageType) -> bool {
    USER_MESSAGE.contains(&kind.value())
}

pub fn isolate(s: &str) -> String {
    if s.is_empty() { String::new() } else { format!("{FSI}{s}{PDI}") }
}

fn quoted(name: &str) -> String {
    format!("#\"{}\"", name.replace('\\', "\\\\").replace('"', "\\\""))
}

pub fn channel_label(c: &Chan, hash: bool, quote: bool) -> String {
    use ChannelType::*;
    match c.kind {
        GuildAnnouncement | GuildText | GuildForum | GuildMedia if hash => format!("#{}", c.name),
        GuildCategory | PublicThread | PrivateThread | AnnouncementThread | GuildVoice | GuildStageVoice if quote => quoted(&c.name),
        PublicThread | PrivateThread | AnnouncementThread if hash => format!("\"{}\"", c.name),
        _ => c.name.clone(),
    }
}

fn decorate(author: &str, c: &Chan, parent: Option<&Chan>) -> String {
    let tail = parent.map(|p| format!(", {}", isolate(&channel_label(p, false, false)))).unwrap_or_default();
    format!("{} ({}{tail})", isolate(author), isolate(&channel_label(c, true, false)))
}

pub fn title(look: &dyn Lookup, author: &str, c: &Chan, m: &api::Message) -> String {
    use ChannelType::*;
    match c.kind {
        GuildAnnouncement | GuildText | GuildVoice | AnnouncementThread | PublicThread | PrivateThread => {
            let parent = c.parent.and_then(|p| look.channel(p));
            match (&parent, m.kind == MessageType::ThreadStarterMessage) {
                (Some(o), true) => decorate(author, o, o.parent.and_then(|p| look.channel(p)).as_ref()),
                _ if !is_user_message(m.kind) && !c.guild.is_some_and(|g| look.guild_exists(g)) => author.to_owned(),
                _ => decorate(author, c, parent.as_ref()),
            }
        }
        GroupDm => format!("{} ({})", isolate(author), isolate(&c.name)),
        _ => author.to_owned(),
    }
}

static TOKENS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)```.*?```|``[^`]+``|`[^`]+`|<@!?(\d+)>|<@&(\d+)>|<#(\d+)>|<a?:(\w+):\d+>|\|\|.+?\|\||<t:(-?\d+)(?::([tTdDfFR]))?>").expect("token regex")
});

pub fn unparse(look: &dyn Lookup, c: &Chan, content: &str) -> String {
    TOKENS
        .replace_all(content, |cap: &Captures| {
            let all = &cap[0];
            if all.starts_with('`') {
                return isolate(all);
            }
            if let Some(u) = cap.get(1) {
                let name = u.as_str().parse().ok().and_then(|id| look.user(id)).map(|u| format!("@{}", look.member_name(c.guild, &u)));
                return isolate(&name.unwrap_or_else(|| all.to_owned()));
            }
            if let Some(r) = cap.get(2) {
                let name = c.guild.zip(r.as_str().parse().ok()).and_then(|(g, id)| look.role(g, id)).map(|n| format!("@{n}"));
                return isolate(&name.unwrap_or_else(|| all.to_owned()));
            }
            if let Some(ch) = cap.get(3) {
                let name = ch.as_str().parse().ok().and_then(|id| look.channel(id)).map(|c| channel_label(&c, true, true));
                return isolate(&name.unwrap_or_else(|| all.to_owned()));
            }
            if let Some(e) = cap.get(4) {
                return format!(":{}:", e.as_str());
            }
            if all.starts_with("||") {
                return format!("<{}>", crate::t!("notif.spoiler").to_lowercase());
            }
            if let Some(ts) = cap.get(5).and_then(|t| t.as_str().parse::<i64>().ok()) {
                let style = cap.get(6).and_then(|s| s.as_str().chars().next()).unwrap_or('f');
                return crate::markdown::format_timestamp(ts, style).unwrap_or_else(|| all.to_owned());
            }
            all.to_owned()
        })
        .into_owned()
}

fn attachment_name(a: &api::Attachment) -> String {
    match &a.title {
        Some(t) => {
            let ext = a.filename.rfind('.').filter(|i| *i > 0).map(|i| &a.filename[i..]).unwrap_or("");
            format!("{t}{ext}")
        }
        None => a.filename.clone(),
    }
}

pub fn fallback(m: &api::Message) -> String {
    let embed = m.embeds.first().and_then(|e| match (&e.title, &e.description) {
        (Some(t), Some(d)) => Some(format!("{t} {d}")),
        (None, Some(d)) => Some(d.clone()),
        (Some(t), None) => Some(t.clone()),
        (None, None) => e.fields.first().map(|f| format!("{} {}", f.name, f.value)),
    });
    if let Some(e) = embed {
        return e;
    }
    if m.flags & IS_VOICE_MESSAGE != 0 {
        return crate::t!("notif.voice").to_string();
    }
    match m.attachments.first() {
        Some(a) => crate::t!("notif.uploaded", filename = attachment_name(a)).to_string(),
        None => String::new(),
    }
}

fn poll_question(m: &api::Message) -> Option<String> {
    m.poll.as_ref()?.get("question")?.get("text")?.as_str().map(str::to_owned)
}

fn poll_result_question(m: &api::Message) -> String {
    m.embeds.first().and_then(|e| e.fields.iter().find(|f| f.name == "poll_question_text")).map(|f| f.value.clone()).unwrap_or_default()
}

pub const JOIN_KEYS: [&str; 13] = [
    "notif.join0", "notif.join1", "notif.join2", "notif.join3", "notif.join4", "notif.join5", "notif.join6",
    "notif.join7", "notif.join8", "notif.join9", "notif.join10", "notif.join11", "notif.join12",
];

pub fn join_key(message_id: u64) -> &'static str {
    JOIN_KEYS[((message_id >> 22) + DISCORD_EPOCH_MS) as usize % JOIN_KEYS.len()]
}

fn plain_name(u: &api::User) -> String {
    u.display_name().to_owned()
}

fn automod_notification(m: &api::Message) -> Option<Option<String>> {
    let e = m.embeds.iter().find(|e| e.kind.as_deref() == Some("auto_moderation_notification"))?;
    Some(e.fields.iter().find(|f| f.name == "notification_type").map(|f| f.value.clone()))
}

fn incident_time(content: &str) -> String {
    match chrono::DateTime::parse_from_rfc3339(content) {
        Ok(t) => crate::i18n::clock(t.with_timezone(&chrono::Local)),
        Err(_) => String::new(),
    }
}

pub fn system(look: &dyn Lookup, c: &Chan, m: &api::Message) -> Option<String> {
    let name = plain_name(&m.author);
    let other = m.mentions.first().map(|u| plain_name(&u.user));
    let guild_name = || c.guild.and_then(|g| look.guild_name(g));
    Some(match m.kind {
        MessageType::RecipientAdd => crate::t!("notif.recipient_add", username = name, other = other?).to_string(),
        MessageType::RecipientRemove => match (other, m.mentions.first().map(|u| u.user.id)) {
            (Some(o), Some(id)) if id != m.author.id => crate::t!("notif.recipient_remove", username = name, other = o).to_string(),
            (Some(_), _) => crate::t!("notif.recipient_leave", username = name).to_string(),
            (None, _) => return None,
        },
        MessageType::Call => {
            let call = m.call.as_ref()?;
            let me = look.me()?;
            if call.participants.iter().any(|p| p.0 == me) {
                return None;
            }
            crate::t!("notif.call", username = name).to_string()
        }
        MessageType::ChannelNameChange => {
            let forum_post = c.parent.and_then(|p| look.channel(p)).is_some_and(|p| matches!(p.kind, ChannelType::GuildForum | ChannelType::GuildMedia));
            let key = if forum_post { "notif.post_rename" } else { "notif.channel_rename" };
            crate::i18n::t(key, &[("username", name.as_str()), ("channel", m.content.as_str())]).to_string()
        }
        MessageType::ChannelIconChange => crate::t!("notif.group_icon", username = name).to_string(),
        MessageType::ChannelPinnedMessage => crate::t!("notif.pin", username = name).to_string(),
        MessageType::UserJoin if m.author.bot => match m.actor.as_ref() {
            Some(actor) => crate::t!("notif.app_added_by", username = plain_name(actor), app = name).to_string(),
            None => crate::t!("notif.app_added", app = name).to_string(),
        },
        MessageType::UserJoin => crate::i18n::t(join_key(m.id.0), &[("username", name.as_str())]).to_string(),
        MessageType::GuildBoost | MessageType::GuildBoostTier1 | MessageType::GuildBoostTier2 | MessageType::GuildBoostTier3 => crate::t!("notif.boost", username = name).to_string(),
        MessageType::GuildInviteReminder => crate::t!("notif.invite_reminder").to_string(),
        MessageType::ThreadStarterMessage => crate::t!("notif.thread_started", username = name, thread = c.name).to_string(),
        MessageType::AutoModerationAction => match automod_notification(m) {
            Some(kind) => {
                let g = guild_name()?;
                match kind.as_deref() {
                    Some("activity_alerts_enabled") => crate::t!("notif.automod_alerts", guild = g).to_string(),
                    Some("interaction_blocked") => crate::t!("notif.automod_blocked", guild = g).to_string(),
                    _ => crate::t!("notif.automod_unusual", guild = g).to_string(),
                }
            }
            None => m.content.clone(),
        },
        MessageType::RoleSubscriptionPurchase => return None,
        k if k.value() == 44 => {
            let product = m.purchase_notification.as_ref()?.pointer("/guild_product_purchase/product_name")?.as_str()?;
            crate::t!("notif.purchase", username = name, product = product).to_string()
        }
        k if k.value() == 36 => crate::t!("notif.security_on", username = name, guild = guild_name()?, time = incident_time(&m.content)).to_string(),
        k if k.value() == 37 => crate::t!("notif.security_off", username = name, guild = guild_name()?).to_string(),
        k if matches!(k.value(), 32..=34) => return None,
        _ => m.content.clone(),
    })
}

const PREMIUM_REFERRAL: u8 = 35;
const CHECKPOINT_CARD: u8 = 20;
const ACTIVITY_JOIN: u64 = 1;
const ACTIVITY_LISTEN: u64 = 3;
const ACTIVITY_JOIN_REQUEST: u64 = 5;

fn activity_invite(look: &dyn Lookup, c: &Chan, m: &api::Message) -> Option<String> {
    let kind = m.activity.as_ref()?.get("type").and_then(serde_json::Value::as_u64);
    let user = look.member_name(c.guild, &m.author);
    let pick = |channel: &'static str, group: &'static str, dm: &'static str| match c.kind {
        ChannelType::GroupDm => group,
        ChannelType::Dm => dm,
        _ => channel,
    };
    if let Some(app) = &m.application {
        let game = app.get("name").and_then(serde_json::Value::as_str).unwrap_or_default();
        let key = match kind {
            Some(ACTIVITY_JOIN) => pick("notif.activity_join", "notif.activity_join_group", "notif.activity_join_dm"),
            Some(ACTIVITY_JOIN_REQUEST) => "notif.activity_join_request",
            _ => return Some(String::new()),
        };
        return Some(crate::i18n::t(key, &[("user", user.as_str()), ("game", game)]).to_string());
    }
    (kind == Some(ACTIVITY_LISTEN))
        .then(|| crate::i18n::t(pick("notif.activity_listen", "notif.activity_listen_group", "notif.activity_listen_dm"), &[("user", user.as_str())]).to_string())
}

pub fn body(look: &dyn Lookup, c: &Chan, m: &api::Message) -> Option<String> {
    let forwarded = m.message_reference.as_ref().is_some_and(|r| r.kind == Some(MessageReferenceType::Forward));
    let stringified = if is_user_message(m.kind) { None } else { Some(system(look, c, m)?) };
    let mut r = if forwarded {
        crate::t!("notif.forwarded").to_string()
    } else if let Some(a) = activity_invite(look, c, m) {
        a
    } else if let Some(s) = m.sticker_items.first() {
        crate::t!("notif.sticker", name = s.name).to_string()
    } else if m.kind.value() == PREMIUM_REFERRAL {
        crate::t!("notif.premium_referral", username = plain_name(&m.author)).to_string()
    } else if let Some(q) = poll_question(m) {
        crate::t!("notif.poll", question = q).to_string()
    } else if m.kind == MessageType::PollResult {
        crate::t!("notif.poll_result", question = poll_result_question(m)).to_string()
    } else if m.components.first().is_some_and(|x| x.type_id() == CHECKPOINT_CARD) {
        crate::t!("notif.checkpoint").to_string()
    } else if let Some(text) = stringified {
        unparse(look, c, &text)
    } else {
        let mut r = unparse(look, c, &m.content);
        if c.kind == ChannelType::Dm && !m.author.bot && m.content.starts_with("> -# *") && r.starts_with("> -#") {
            r = format!(">{}", &r[4..]);
        }
        r
    };
    if r.is_empty() {
        r = fallback(m);
    }
    Some(r)
}

pub fn avatar_url(guild: Option<u64>, u: &api::User, member_avatar: Option<&str>) -> String {
    if let (Some(g), Some(hash)) = (guild, member_avatar) {
        return format!("{}/guilds/{g}/users/{}/avatars/{hash}.png?size=128", api::CDN_BASE, u.id);
    }
    u.avatar_url(128).replace(".gif?", ".png?")
}

pub struct CacheLookup<'a> {
    pub cache: &'a Cache,
}

fn group_label(cache: &Cache, c: &api::Channel, me: Option<api::Snowflake>) -> String {
    if let Some(n) = c.name.as_ref().filter(|n| !n.is_empty()) {
        return n.clone();
    }
    let rel = cache.relationships();
    let nick = |u: &api::User| {
        rel.iter().find(|r| r.user.as_ref().map(|x| x.id).or(r.user_id).unwrap_or(r.id) == u.id).and_then(|r| r.nickname.clone()).unwrap_or_else(|| u.display_name().to_owned())
    };
    let names: Vec<String> = c.recipients.iter().filter(|u| Some(u.id) != me).map(nick).collect();
    if names.is_empty() {
        let mine = cache.current_user().map(|u| u.display_name().to_owned()).unwrap_or_default();
        return crate::t!("notif.group_of", name = mine).to_string();
    }
    names.join(", ")
}

impl CacheLookup<'_> {
    pub fn chan(&self, c: &api::Channel) -> Chan {
        let name = match c.kind {
            ChannelType::GroupDm => group_label(self.cache, c, self.cache.current_user().map(|u| u.id)),
            _ => c.name.clone().unwrap_or_default(),
        };
        Chan { id: c.id.0, kind: c.kind, name, parent: c.parent_id.map(|p| p.0), guild: c.guild_id.map(|g| g.0) }
    }
}

impl Lookup for CacheLookup<'_> {
    fn channel(&self, id: u64) -> Option<Chan> {
        self.cache.channel(api::Snowflake(id)).map(|c| self.chan(&c))
    }
    fn member_name(&self, guild: Option<u64>, user: &api::User) -> String {
        let nick = guild.and_then(|g| self.cache.member(api::Snowflake(g), user.id)).and_then(|m| m.nick.clone().filter(|n| !n.is_empty()));
        let friend = || {
            self.cache
                .relationships()
                .iter()
                .find(|r| matches!(r.kind, RelationshipType::Friend) && r.user.as_ref().map(|x| x.id).or(r.user_id).unwrap_or(r.id) == user.id)
                .and_then(|r| r.nickname.clone())
        };
        nick.or_else(|| guild.is_none().then(friend).flatten()).unwrap_or_else(|| user.display_name().to_owned())
    }
    fn user(&self, id: u64) -> Option<api::User> {
        self.cache.user(api::Snowflake(id)).map(|u| (*u).clone())
    }
    fn role(&self, guild: u64, id: u64) -> Option<String> {
        self.cache.guild(api::Snowflake(guild))?.roles.iter().find(|r| r.id.0 == id).map(|r| r.name.clone())
    }
    fn guild_exists(&self, id: u64) -> bool {
        self.cache.guild(api::Snowflake(id)).is_some()
    }
    fn guild_name(&self, id: u64) -> Option<String> {
        self.cache.guild(api::Snowflake(id)).map(|g| g.properties.name.clone())
    }
    fn me(&self) -> Option<u64> {
        self.cache.current_user().map(|u| u.id.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct Fake {
        chans: HashMap<u64, Chan>,
        users: HashMap<u64, api::User>,
        nicks: HashMap<u64, String>,
    }

    impl Lookup for Fake {
        fn channel(&self, id: u64) -> Option<Chan> {
            self.chans.get(&id).cloned()
        }
        fn member_name(&self, guild: Option<u64>, user: &api::User) -> String {
            guild.and(self.nicks.get(&user.id.0).cloned()).unwrap_or_else(|| user.display_name().to_owned())
        }
        fn user(&self, id: u64) -> Option<api::User> {
            self.users.get(&id).cloned()
        }
        fn role(&self, _: u64, id: u64) -> Option<String> {
            (id == 77).then(|| "Mods".to_owned())
        }
        fn guild_exists(&self, _: u64) -> bool {
            true
        }
        fn guild_name(&self, id: u64) -> Option<String> {
            (id == 1).then(|| "Aperture".to_owned())
        }
        fn me(&self) -> Option<u64> {
            Some(99)
        }
    }

    fn chan(id: u64, kind: ChannelType, name: &str, parent: Option<u64>) -> Chan {
        Chan { id, kind, name: name.into(), parent, guild: (!kind.is_private()).then_some(1) }
    }

    fn user(id: u64, name: &str) -> api::User {
        api::User { id: api::Snowflake(id), username: name.into(), ..Default::default() }
    }

    fn fake() -> Fake {
        let mut chans = HashMap::new();
        chans.insert(1, chan(1, ChannelType::GuildCategory, "Salons", None));
        chans.insert(2, chan(2, ChannelType::GuildText, "général", Some(1)));
        chans.insert(3, chan(3, ChannelType::PublicThread, "idée", Some(2)));
        chans.insert(4, chan(4, ChannelType::GuildVoice, "Vocal", Some(1)));
        chans.insert(5, chan(5, ChannelType::GuildText, "sans-categorie", None));
        let mut users = HashMap::new();
        users.insert(10, user(10, "alice"));
        let mut nicks = HashMap::new();
        nicks.insert(10, "Alice du serveur".to_owned());
        Fake { chans, users, nicks }
    }

    fn msg(content: &str) -> api::Message {
        api::Message { content: content.into(), author: user(10, "alice"), ..Default::default() }
    }

    fn iso(s: &str) -> String {
        isolate(s)
    }

    #[test]
    fn activity_invites_referrals_and_checkpoints() {
        let f = fake();
        let dm = Chan { id: 9, kind: ChannelType::Dm, name: String::new(), parent: None, guild: None };
        let group = Chan { id: 8, kind: ChannelType::GroupDm, name: "g".into(), parent: None, guild: None };
        let game = Some(serde_json::json!({"name": "Portal 2"}));
        let invite = |t: u64, app: Option<serde_json::Value>| api::Message { activity: Some(serde_json::json!({"type": t})), application: app, ..msg("") };
        let join = invite(1, game.clone());
        assert_eq!(body(&f, &f.chans[&2], &join).unwrap(), crate::t!("notif.activity_join", user = "Alice du serveur", game = "Portal 2").to_string());
        assert_eq!(body(&f, &group, &join).unwrap(), crate::t!("notif.activity_join_group", user = "alice", game = "Portal 2").to_string());
        assert_eq!(body(&f, &dm, &join).unwrap(), crate::t!("notif.activity_join_dm", user = "alice", game = "Portal 2").to_string());
        assert_eq!(body(&f, &dm, &invite(5, game.clone())).unwrap(), crate::t!("notif.activity_join_request", user = "alice", game = "Portal 2").to_string());
        assert_eq!(body(&f, &dm, &invite(3, None)).unwrap(), crate::t!("notif.activity_listen_dm", user = "alice").to_string());
        assert_eq!(body(&f, &f.chans[&2], &invite(3, None)).unwrap(), crate::t!("notif.activity_listen", user = "Alice du serveur").to_string());
        let spectate = api::Message { content: "x".into(), ..invite(2, game) };
        assert_eq!(body(&f, &dm, &spectate).unwrap(), "", "an app activity other than join falls back to nothing");
        let referral = api::Message { kind: MessageType::from(35), ..msg("ignored") };
        assert_eq!(body(&f, &dm, &referral).unwrap(), crate::t!("notif.premium_referral", username = "alice").to_string());
        let card: api::Message = serde_json::from_value(serde_json::json!({
            "id": "1", "channel_id": "9", "content": "", "author": {"id": "10", "username": "alice"},
            "components": [{"type": 20, "id": 1}]
        }))
        .unwrap();
        assert_eq!(body(&f, &dm, &card).unwrap(), crate::t!("notif.checkpoint").to_string());
    }

    #[test]
    fn titles_follow_the_channel_kind() {
        let f = fake();
        let m = msg("salut");
        assert_eq!(title(&f, "Alice", &f.chans[&2], &m), format!("{} ({}, {})", iso("Alice"), iso("#général"), iso("Salons")));
        assert_eq!(title(&f, "Alice", &f.chans[&5], &m), format!("{} ({})", iso("Alice"), iso("#sans-categorie")));
        assert_eq!(title(&f, "Alice", &f.chans[&3], &m), format!("{} ({}, {})", iso("Alice"), iso("\"idée\""), iso("général")));
        assert_eq!(title(&f, "Alice", &f.chans[&4], &m), format!("{} ({}, {})", iso("Alice"), iso("Vocal"), iso("Salons")));
        let starter = api::Message { kind: MessageType::ThreadStarterMessage, ..msg("x") };
        assert_eq!(title(&f, "Alice", &f.chans[&3], &starter), format!("{} ({}, {})", iso("Alice"), iso("#général"), iso("Salons")));
        let dm = Chan { id: 9, kind: ChannelType::Dm, name: String::new(), parent: None, guild: None };
        assert_eq!(title(&f, "Alice", &dm, &m), "Alice");
        let group = Chan { id: 9, kind: ChannelType::GroupDm, name: "Amis".into(), parent: None, guild: None };
        assert_eq!(title(&f, "Alice", &group, &m), format!("{} ({})", iso("Alice"), iso("Amis")));
        let stage = chan(6, ChannelType::GuildStageVoice, "Scène", Some(1));
        assert_eq!(title(&f, "Alice", &stage, &m), "Alice");
    }

    #[test]
    fn unparse_resolves_mentions_like_the_client() {
        let f = fake();
        let c = &f.chans[&2];
        assert_eq!(unparse(&f, c, "hey <@10> et <@!10>"), format!("hey {} et {}", iso("@Alice du serveur"), iso("@Alice du serveur")));
        assert_eq!(unparse(&f, c, "<@&77> <@&5>"), format!("{} {}", iso("@Mods"), iso("<@&5>")));
        assert_eq!(unparse(&f, c, "<#2> <#3> <#4>"), format!("{} {} {}", iso("#général"), iso("#\"idée\""), iso("#\"Vocal\"")));
        assert_eq!(unparse(&f, c, "<:pepe:123> <a:dance:456>"), ":pepe: :dance:");
        assert_eq!(unparse(&f, c, "a ||secret|| b"), "a <spoiler> b");
        assert_eq!(unparse(&f, c, "`<@10>`"), iso("`<@10>`"));
        assert_eq!(unparse(&f, c, "plain"), "plain");
    }

    #[test]
    fn bodies_and_fallbacks() {
        let f = fake();
        let c = &f.chans[&2];
        let mut m = msg("");
        m.attachments.push(api::Attachment { filename: "photo.png".into(), ..Default::default() });
        assert_eq!(body(&f, c, &m).unwrap(), crate::t!("notif.uploaded", filename = "photo.png").to_string());
        m.attachments[0].title = Some("Vacances".into());
        assert_eq!(body(&f, c, &m).unwrap(), crate::t!("notif.uploaded", filename = "Vacances.png").to_string());
        let mut e = msg("");
        e.embeds.push(api::Embed { title: Some("Titre".into()), description: Some("desc".into()), ..Default::default() });
        assert_eq!(body(&f, c, &e).unwrap(), "Titre desc");
        e.embeds[0].description = None;
        assert_eq!(body(&f, c, &e).unwrap(), "Titre");
        e.embeds[0].title = None;
        e.embeds[0].fields.push(api::EmbedField { name: "n".into(), value: "v".into(), inline: false });
        assert_eq!(body(&f, c, &e).unwrap(), "n v");
        let v = api::Message { flags: IS_VOICE_MESSAGE, ..msg("") };
        assert_eq!(body(&f, c, &v).unwrap(), crate::t!("notif.voice").to_string());
        let mut fw = msg("texte");
        fw.message_reference = Some(api::MessageReference { kind: Some(MessageReferenceType::Forward), ..Default::default() });
        assert_eq!(body(&f, c, &fw).unwrap(), crate::t!("notif.forwarded").to_string());
        let mut p = msg("");
        p.poll = Some(serde_json::json!({"question": {"text": "Pizza ?"}}));
        assert_eq!(body(&f, c, &p).unwrap(), crate::t!("notif.poll", question = "Pizza ?").to_string());
        let dm = Chan { id: 9, kind: ChannelType::Dm, name: String::new(), parent: None, guild: None };
        assert_eq!(body(&f, &dm, &msg("> -# *note*")).unwrap(), "> *note*");
        let unknown = api::Message { kind: MessageType::Call, ..msg("") };
        assert!(body(&f, c, &unknown).is_none());
    }

    #[test]
    fn system_messages_follow_stringify() {
        let f = fake();
        let c = &f.chans[&2];
        let sys = |kind: u8, content: &str| api::Message { kind: MessageType::from(kind), ..msg(content) };
        let mut call = sys(3, "");
        call.call = Some(api::MessageCall { participants: vec![api::Snowflake(10)], ended_timestamp: None });
        assert_eq!(system(&f, c, &call).unwrap(), crate::t!("notif.call", username = "alice").to_string());
        call.call.as_mut().unwrap().participants.push(api::Snowflake(99));
        assert!(system(&f, c, &call).is_none());
        assert_eq!(system(&f, c, &sys(4, "nouveau")).unwrap(), crate::t!("notif.channel_rename", username = "alice", channel = "nouveau").to_string());
        let mut chans = f.chans.clone();
        chans.insert(7, chan(7, ChannelType::GuildForum, "forum", None));
        let post = chan(8, ChannelType::PublicThread, "post", Some(7));
        let forum = Fake { chans, users: f.users.clone(), nicks: f.nicks.clone() };
        assert_eq!(system(&forum, &post, &sys(4, "titre")).unwrap(), crate::t!("notif.post_rename", username = "alice", channel = "titre").to_string());
        assert_eq!(system(&f, c, &sys(5, "")).unwrap(), crate::t!("notif.group_icon", username = "alice").to_string());
        for tier in 9..=11 {
            assert_eq!(system(&f, c, &sys(tier, "")).unwrap(), crate::t!("notif.boost", username = "alice").to_string());
        }
        assert_eq!(system(&f, c, &sys(22, "")).unwrap(), crate::t!("notif.invite_reminder").to_string());
        assert_eq!(system(&f, &f.chans[&3], &sys(21, "")).unwrap(), crate::t!("notif.thread_started", username = "alice", thread = "idée").to_string());
        let mut bot = sys(7, "");
        bot.author.bot = true;
        assert_eq!(system(&f, c, &bot).unwrap(), crate::t!("notif.app_added", app = "alice").to_string());
        bot.actor = Some(user(11, "bob"));
        assert_eq!(system(&f, c, &bot).unwrap(), crate::t!("notif.app_added_by", username = "bob", app = "alice").to_string());
        let mut am = sys(24, "brut");
        assert_eq!(system(&f, c, &am).unwrap(), "brut");
        am.embeds.push(api::Embed {
            kind: Some("auto_moderation_notification".into()),
            fields: vec![api::EmbedField { name: "notification_type".into(), value: "interaction_blocked".into(), inline: false }],
            ..Default::default()
        });
        assert_eq!(system(&f, c, &am).unwrap(), crate::t!("notif.automod_blocked", guild = "Aperture").to_string());
        am.embeds[0].fields[0].value = "raid".into();
        assert_eq!(system(&f, c, &am).unwrap(), crate::t!("notif.automod_unusual", guild = "Aperture").to_string());
        let mut buy = sys(44, "");
        assert!(system(&f, c, &buy).is_none());
        buy.purchase_notification = Some(serde_json::json!({ "guild_product_purchase": { "product_name": "Pack" } }));
        assert_eq!(system(&f, c, &buy).unwrap(), crate::t!("notif.purchase", username = "alice", product = "Pack").to_string());
        assert_eq!(system(&f, c, &sys(37, "")).unwrap(), crate::t!("notif.security_off", username = "alice", guild = "Aperture").to_string());
        assert!(system(&f, c, &sys(36, "x")).unwrap().contains("Aperture"));
        assert!(system(&f, c, &sys(33, "")).is_none());
        assert_eq!(system(&f, c, &sys(12, "suivi")).unwrap(), "suivi");
        assert!(is_user_message(MessageType::Unknown(68)) && !is_user_message(MessageType::ThreadStarterMessage));
    }

    #[test]
    fn join_variant_uses_the_timestamp() {
        assert_eq!(join_key(0), JOIN_KEYS[(DISCORD_EPOCH_MS % 13) as usize]);
        let id = 1_000u64 << 22;
        assert_eq!(join_key(id), JOIN_KEYS[((1_000 + DISCORD_EPOCH_MS) % 13) as usize]);
        assert_eq!(avatar_url(Some(5), &user(10, "a"), Some("abc")), format!("{}/guilds/5/users/10/avatars/abc.png?size=128", api::CDN_BASE));
        let anim = api::User { avatar: Some("a_x".into()), ..user(10, "a") };
        assert!(avatar_url(None, &anim, None).ends_with("/avatars/10/a_x.png?size=128"));
    }
}
