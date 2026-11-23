//! Plain data model used by the UI. Deliberately independent from
//! `phoenix-api` types: the adapter layer converts API objects into these.

use gpui::SharedString;

/// Discord snowflake.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Id(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Presence {
    Online,
    Idle,
    Dnd,
    #[default]
    Offline,
}

/// Where an avatar/icon comes from. `Url` is resolved lazily by the image
/// element only when the row is actually visible (virtualized lists never
/// build off-screen rows), which is our lazy-loading hook.
#[derive(Clone, Debug, PartialEq)]
pub enum ImageRef {
    /// Default avatar: one of Discord's 5 palette colours (index) + initials.
    Default(u8),
    Url(SharedString),
}

#[derive(Clone, Debug)]
pub struct User {
    pub id: Id,
    pub name: SharedString,
    pub display_name: SharedString,
    pub avatar: ImageRef,
    pub bot: bool,
}

#[derive(Clone, Debug)]
pub struct Guild {
    pub id: Id,
    pub name: SharedString,
    /// Initials shown when the guild has no icon ("Rust Programming" -> "RP").
    pub acronym: SharedString,
    pub icon: ImageRef,
    pub unread: bool,
    pub mentions: u32,
    pub banner_color: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelKind {
    Text,
    Voice,
    Announcement,
    Forum,
    Stage,
    Category,
}

#[derive(Clone, Debug)]
pub struct Channel {
    pub id: Id,
    pub guild: Option<Id>,
    pub parent: Option<Id>,
    pub name: SharedString,
    pub topic: Option<SharedString>,
    pub kind: ChannelKind,
    pub position: i32,
    pub unread: bool,
    pub muted: bool,
    pub mentions: u32,
    /// Users currently connected (voice channels only).
    pub voice_users: Vec<Id>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EmbedMedia {
    /// Proxy URL when available (resizable), else the original.
    pub url: SharedString,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Embed {
    pub color: Option<u32>,
    pub provider: Option<SharedString>,
    pub title: Option<SharedString>,
    pub description: Option<SharedString>,
    pub url: Option<SharedString>,
    /// "rich", "image", "gifv", "video", "article", ...
    pub kind: Option<SharedString>,
    pub thumbnail: Option<EmbedMedia>,
    pub image: Option<EmbedMedia>,
    pub video: Option<EmbedMedia>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AttachmentKind {
    #[default]
    File,
    Image,
    Video,
    /// Message sticker rendered as an attachment-like image.
    Sticker(crate::media::StickerFormat),
}

#[derive(Clone, Debug, Default)]
pub struct Attachment {
    pub filename: SharedString,
    pub url: SharedString,
    pub width: u32,
    pub height: u32,
    pub is_image: bool,
    pub size: u64,
    pub kind: AttachmentKind,
    /// Full-resolution URL (viewer / open in browser). Empty = same as `url`.
    pub original: SharedString,
}

#[derive(Clone, Debug)]
pub struct Reaction {
    pub emoji: SharedString,
    pub count: u32,
    pub me: bool,
}

#[derive(Clone, Debug)]
pub struct Message {
    pub id: Id,
    pub channel: Id,
    pub author: Id,
    /// Unix seconds.
    pub timestamp: i64,
    pub edited: bool,
    pub content: SharedString,
    pub reply_to: Option<Id>,
    pub embeds: Vec<Embed>,
    pub attachments: Vec<Attachment>,
    pub reactions: Vec<Reaction>,
}

#[derive(Clone, Debug)]
pub struct Role {
    pub id: Id,
    pub name: SharedString,
    pub color: Option<u32>,
    pub position: i32,
    /// Hoisted roles get their own section in the member list.
    pub hoist: bool,
}

#[derive(Clone, Debug)]
pub struct Member {
    pub user: Id,
    pub roles: Vec<Id>,
    pub presence: Presence,
    pub activity: Option<SharedString>,
}

pub fn acronym(name: &str) -> SharedString {
    let s: String = name
        .split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(4)
        .collect();
    s.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn acronyms() {
        assert_eq!(acronym("Rust Programming Language").as_ref(), "RPL");
        assert_eq!(acronym("gpui").as_ref(), "g");
    }
}
