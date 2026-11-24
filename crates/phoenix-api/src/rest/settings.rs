//! User settings: the protobuf `settings-proto/1` blob (preloaded user
//! settings) plus the legacy JSON fallback.
//!
//! Only the fields Phoenix reads or writes are modelled; prost skips the rest
//! on decode. PATCH sends only the changed fields, which Discord merges.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use reqwest::Method;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{Body, RestClient};
use crate::error::{Error, Result};

#[derive(Clone, PartialEq, prost::Message)]
pub struct BoolValue {
    #[prost(bool, tag = "1")]
    pub value: bool,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct StringValue {
    #[prost(string, tag = "1")]
    pub value: String,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct TextAndImagesSettings {
    #[prost(message, optional, tag = "10")]
    pub inline_embed_media: Option<BoolValue>,
    #[prost(message, optional, tag = "11")]
    pub gif_auto_play: Option<BoolValue>,
    #[prost(message, optional, tag = "14")]
    pub animate_emoji: Option<BoolValue>,
    #[prost(message, optional, tag = "17")]
    pub message_display_compact: Option<BoolValue>,
    #[prost(message, optional, tag = "21")]
    pub convert_emoticons: Option<BoolValue>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct NotificationSettings {
    #[prost(message, optional, tag = "1")]
    pub show_in_app_notifications: Option<BoolValue>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct PrivacySettings {
    #[prost(fixed64, repeated, tag = "3")]
    pub restricted_guild_ids: Vec<u64>,
    #[prost(bool, tag = "4")]
    pub default_guilds_restricted: bool,
    /// Servers the game/app activity is not shared with.
    #[prost(fixed64, repeated, tag = "13")]
    pub activity_restricted_guild_ids: Vec<u64>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct StatusSettings {
    #[prost(message, optional, tag = "1")]
    pub status: Option<StringValue>,
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct LocalizationSettings {
    #[prost(message, optional, tag = "1")]
    pub locale: Option<StringValue>,
}

/// `Theme` enum values of the appearance settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
    /// "Ash" in the client (proto DARKER).
    Ash,
    /// "Onyx" in the client (proto MIDNIGHT).
    Onyx,
}

impl Theme {
    pub fn from_proto(v: i32) -> Self {
        match v {
            2 => Self::Light,
            3 => Self::Ash,
            4 => Self::Onyx,
            _ => Self::Dark,
        }
    }
    pub fn to_proto(self) -> i32 {
        match self {
            Self::Dark => 1,
            Self::Light => 2,
            Self::Ash => 3,
            Self::Onyx => 4,
        }
    }
    /// Legacy JSON `theme` string.
    pub fn from_json(s: &str) -> Self {
        match s {
            "light" => Self::Light,
            "darker" => Self::Ash,
            "midnight" => Self::Onyx,
            _ => Self::Dark,
        }
    }
}

#[derive(Clone, PartialEq, prost::Message)]
pub struct AppearanceSettings {
    #[prost(int32, tag = "1")]
    pub theme: i32,
    #[prost(bool, tag = "2")]
    pub developer_mode: bool,
}

/// The subset of `PreloadedUserSettings` Phoenix uses.
#[derive(Clone, PartialEq, prost::Message)]
pub struct PreloadedUserSettings {
    #[prost(message, optional, tag = "6")]
    pub text_and_images: Option<TextAndImagesSettings>,
    #[prost(message, optional, tag = "7")]
    pub notifications: Option<NotificationSettings>,
    #[prost(message, optional, tag = "8")]
    pub privacy: Option<PrivacySettings>,
    #[prost(message, optional, tag = "11")]
    pub status: Option<StatusSettings>,
    #[prost(message, optional, tag = "12")]
    pub localization: Option<LocalizationSettings>,
    #[prost(message, optional, tag = "13")]
    pub appearance: Option<AppearanceSettings>,
}

fn b(v: &Option<BoolValue>) -> Option<bool> {
    v.as_ref().map(|b| b.value)
}

impl PreloadedUserSettings {
    pub fn decode_base64(s: &str) -> Result<Self> {
        let bytes = STANDARD.decode(s.trim()).map_err(|e| Error::Protocol(format!("settings base64: {e}")))?;
        <Self as prost::Message>::decode(bytes.as_slice()).map_err(|e| Error::Protocol(format!("settings proto: {e}")))
    }
    pub fn encode_base64(&self) -> String {
        STANDARD.encode(prost::Message::encode_to_vec(self))
    }

    pub fn theme(&self) -> Theme {
        self.appearance.as_ref().map(|a| Theme::from_proto(a.theme)).unwrap_or_default()
    }
    pub fn developer_mode(&self) -> bool {
        self.appearance.as_ref().is_some_and(|a| a.developer_mode)
    }
    pub fn compact(&self) -> bool {
        self.text_and_images.as_ref().and_then(|t| b(&t.message_display_compact)).unwrap_or(false)
    }
    pub fn locale(&self) -> Option<&str> {
        self.localization.as_ref()?.locale.as_ref().map(|s| s.value.as_str())
    }
    pub fn status(&self) -> Option<&str> {
        self.status.as_ref()?.status.as_ref().map(|s| s.value.as_str())
    }
    pub fn default_guilds_restricted(&self) -> bool {
        self.privacy.as_ref().is_some_and(|p| p.default_guilds_restricted)
    }

    // Patch builders (only the given field is set).
    pub fn patch_appearance(theme: Theme, developer_mode: bool) -> Self {
        Self {
            appearance: Some(AppearanceSettings { theme: theme.to_proto(), developer_mode }),
            ..Default::default()
        }
    }
    pub fn patch_compact(compact: bool) -> Self {
        Self {
            text_and_images: Some(TextAndImagesSettings {
                message_display_compact: Some(BoolValue { value: compact }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }
    pub fn patch_locale(locale: &str) -> Self {
        Self {
            localization: Some(LocalizationSettings { locale: Some(StringValue { value: locale.to_owned() }) }),
            ..Default::default()
        }
    }
    /// Note: Discord replaces the whole privacy message's repeated fields.
    pub fn patch_default_guilds_restricted(v: bool) -> Self {
        Self {
            privacy: Some(PrivacySettings { default_guilds_restricted: v, ..Default::default() }),
            ..Default::default()
        }
    }
}

/// One change, encoded with explicit field values (so `false` / `0` are
/// really sent, which prost's proto3 encoding would omit).
#[derive(Clone, Debug, PartialEq)]
pub enum SettingChange {
    Theme(Theme),
    DeveloperMode(bool),
    Compact(bool),
    Locale(String),
    DefaultGuildsRestricted(bool),
}

fn varint(mut v: u64, out: &mut Vec<u8>) {
    while v >= 0x80 {
        out.push((v as u8) | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

fn field_varint(tag: u32, v: u64, out: &mut Vec<u8>) {
    varint((tag as u64) << 3, out);
    varint(v, out);
}

fn field_bytes(tag: u32, data: &[u8], out: &mut Vec<u8>) {
    varint(((tag as u64) << 3) | 2, out);
    varint(data.len() as u64, out);
    out.extend_from_slice(data);
}

impl SettingChange {
    /// Top-level `PreloadedUserSettings` bytes carrying only this change.
    pub fn encode(&self) -> Vec<u8> {
        let mut inner = Vec::new();
        let mut out = Vec::new();
        match self {
            Self::Theme(t) => {
                field_varint(1, t.to_proto() as u64, &mut inner);
                field_bytes(13, &inner, &mut out);
            }
            Self::DeveloperMode(b) => {
                field_varint(2, *b as u64, &mut inner);
                field_bytes(13, &inner, &mut out);
            }
            Self::Compact(b) => {
                let mut w = Vec::new();
                field_varint(1, *b as u64, &mut w);
                field_bytes(17, &w, &mut inner);
                field_bytes(6, &inner, &mut out);
            }
            Self::Locale(l) => {
                let mut w = Vec::new();
                field_bytes(1, l.as_bytes(), &mut w);
                field_bytes(1, &w, &mut inner);
                field_bytes(12, &inner, &mut out);
            }
            Self::DefaultGuildsRestricted(b) => {
                field_varint(4, *b as u64, &mut inner);
                field_bytes(8, &inner, &mut out);
            }
        }
        out
    }
}

#[derive(Deserialize)]
struct ProtoBody {
    settings: String,
}

impl RestClient {
    /// `GET /users/@me/settings-proto/1`
    pub async fn get_settings_proto(&self) -> Result<PreloadedUserSettings> {
        let b: ProtoBody = self.call(Method::GET, "/users/@me/settings-proto/1", Body::None).await?;
        PreloadedUserSettings::decode_base64(&b.settings)
    }

    /// `PATCH /users/@me/settings-proto/1` with a partial message; returns the
    /// merged settings.
    pub async fn patch_settings_proto(&self, patch: &PreloadedUserSettings) -> Result<PreloadedUserSettings> {
        let b: ProtoBody = self
            .call(
                Method::PATCH,
                "/users/@me/settings-proto/1",
                Body::Json(json!({ "settings": patch.encode_base64() })),
            )
            .await?;
        PreloadedUserSettings::decode_base64(&b.settings)
    }

    /// Apply one change. The full current blob is sent back with the change
    /// appended: protobuf merge semantics make the appended fields win while
    /// every field Phoenix doesn't model is preserved.
    pub async fn patch_setting(&self, change: &SettingChange) -> Result<PreloadedUserSettings> {
        let b: ProtoBody = self.call(Method::GET, "/users/@me/settings-proto/1", Body::None).await?;
        let mut raw = STANDARD.decode(b.settings.trim()).map_err(|e| Error::Protocol(format!("settings base64: {e}")))?;
        raw.extend(change.encode());
        let b: ProtoBody = self
            .call(Method::PATCH, "/users/@me/settings-proto/1", Body::Json(json!({ "settings": STANDARD.encode(raw) })))
            .await?;
        PreloadedUserSettings::decode_base64(&b.settings)
    }

    /// Legacy JSON `GET /users/@me/settings` (fallback).
    pub async fn get_settings_json(&self) -> Result<Value> {
        self.call(Method::GET, "/users/@me/settings", Body::None).await
    }

    /// Legacy JSON `PATCH /users/@me/settings` (fallback).
    pub async fn patch_settings_json(&self, patch: Value) -> Result<Value> {
        self.call(Method::PATCH, "/users/@me/settings", Body::Json(patch)).await
    }
}

/// Build proto-equivalent settings from the legacy JSON object.
pub fn from_json(v: &Value) -> PreloadedUserSettings {
    let mut s = PreloadedUserSettings::patch_appearance(
        Theme::from_json(v.get("theme").and_then(Value::as_str).unwrap_or("dark")),
        v.get("developer_mode").and_then(Value::as_bool).unwrap_or(false),
    );
    s.text_and_images = Some(TextAndImagesSettings {
        message_display_compact: v.get("message_display_compact").and_then(Value::as_bool).map(|value| BoolValue { value }),
        ..Default::default()
    });
    if let Some(l) = v.get("locale").and_then(Value::as_str) {
        s.localization = Some(LocalizationSettings { locale: Some(StringValue { value: l.to_owned() }) });
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_unknown_fields() {
        let mut s = PreloadedUserSettings::patch_appearance(Theme::Onyx, true);
        s.text_and_images = PreloadedUserSettings::patch_compact(true).text_and_images;
        let enc = s.encode_base64();
        let d = PreloadedUserSettings::decode_base64(&enc).unwrap();
        assert_eq!(d.theme(), Theme::Onyx);
        assert!(d.developer_mode());
        assert!(d.compact());

        // Unknown top-level field 14 (guild folders) is skipped.
        let mut bytes = STANDARD.decode(&enc).unwrap();
        bytes.extend_from_slice(&[0x72, 0x02, 0x08, 0x01]);
        let d = PreloadedUserSettings::decode_base64(&STANDARD.encode(bytes)).unwrap();
        assert_eq!(d.theme(), Theme::Onyx);
    }

    #[test]
    fn theme_wire_values() {
        // appearance { theme: LIGHT }
        let bytes = [0x6a, 0x02, 0x08, 0x02];
        let d = PreloadedUserSettings::decode_base64(&STANDARD.encode(bytes)).unwrap();
        assert_eq!(d.theme(), Theme::Light);
        assert!(!d.compact());
    }

    #[test]
    fn appended_changes_override_including_false() {
        let base = PreloadedUserSettings::patch_appearance(Theme::Light, true);
        let mut raw = prost::Message::encode_to_vec(&base);
        raw.extend(SettingChange::DeveloperMode(false).encode());
        raw.extend(SettingChange::Compact(true).encode());
        raw.extend(SettingChange::Locale("fr".into()).encode());
        raw.extend(SettingChange::DefaultGuildsRestricted(true).encode());
        let d = PreloadedUserSettings::decode_base64(&STANDARD.encode(&raw)).unwrap();
        assert_eq!(d.theme(), Theme::Light, "untouched field kept");
        assert!(!d.developer_mode());
        assert!(d.compact());
        assert_eq!(d.locale(), Some("fr"));
        assert!(d.default_guilds_restricted());
        raw.extend(SettingChange::Theme(Theme::Ash).encode());
        let d = PreloadedUserSettings::decode_base64(&STANDARD.encode(&raw)).unwrap();
        assert_eq!(d.theme(), Theme::Ash);
    }

    #[test]
    fn json_fallback() {
        let s = from_json(&json!({"theme": "darker", "message_display_compact": true, "locale": "fr"}));
        assert_eq!(s.theme(), Theme::Ash);
        assert!(s.compact());
        assert_eq!(s.locale(), Some("fr"));
    }
}
