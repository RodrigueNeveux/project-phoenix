use serde::{Deserialize, Serialize};

use super::{Snowflake, UserId, null_default};

pub const CDN_BASE: &str = "https://cdn.discordapp.com";

/// A Discord user. Fields only present for the current user (email, mfa...)
/// are optional.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct User {
    pub id: UserId,
    #[serde(default, deserialize_with = "null_default")]
    pub username: String,
    #[serde(default, deserialize_with = "null_default")]
    pub discriminator: String,
    #[serde(default)]
    pub global_name: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub banner: Option<String>,
    #[serde(default)]
    pub accent_color: Option<u32>,
    #[serde(default, deserialize_with = "null_default")]
    pub bot: bool,
    #[serde(default, deserialize_with = "null_default")]
    pub system: bool,
    #[serde(default)]
    pub public_flags: Option<u64>,
    #[serde(default)]
    pub avatar_decoration_data: Option<serde_json::Value>,
    // Current-user only:
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub verified: Option<bool>,
    #[serde(default)]
    pub mfa_enabled: Option<bool>,
    #[serde(default)]
    pub premium_type: Option<u8>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub locale: Option<String>,
    #[serde(default)]
    pub flags: Option<u64>,
    #[serde(default)]
    pub bio: Option<String>,
}

impl User {
    /// Name to display: global name, falling back to username.
    pub fn display_name(&self) -> &str {
        self.global_name
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(&self.username)
    }

    /// `username` or legacy `username#1234`.
    pub fn tag(&self) -> String {
        if self.discriminator.is_empty() || self.discriminator == "0" {
            self.username.clone()
        } else {
            format!("{}#{}", self.username, self.discriminator)
        }
    }

    /// Avatar URL (or the default embed avatar when none is set).
    pub fn avatar_url(&self, size: u32) -> String {
        match &self.avatar {
            Some(hash) => {
                let ext = if hash.starts_with("a_") { "gif" } else { "png" };
                format!("{CDN_BASE}/avatars/{}/{hash}.{ext}?size={size}", self.id)
            }
            None => {
                let idx = if self.discriminator.is_empty() || self.discriminator == "0" {
                    (self.id.0 >> 22) % 6
                } else {
                    self.discriminator.parse::<u64>().unwrap_or(0) % 5
                };
                format!("{CDN_BASE}/embed/avatars/{idx}.png")
            }
        }
    }
}

int_enum! {
    pub enum RelationshipType: u8 {
        None = 0,
        Friend = 1,
        Blocked = 2,
        IncomingRequest = 3,
        OutgoingRequest = 4,
        Implicit = 5,
    }
}

/// An entry in the user's relationship list (friends, blocks, requests).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    pub id: Snowflake,
    #[serde(rename = "type")]
    pub kind: RelationshipType,
    /// Present unless the READY payload deduplicated users.
    #[serde(default)]
    pub user: Option<User>,
    #[serde(default)]
    pub user_id: Option<UserId>,
    #[serde(default)]
    pub nickname: Option<String>,
}
