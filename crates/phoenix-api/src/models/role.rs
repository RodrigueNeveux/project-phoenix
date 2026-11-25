use serde::{Deserialize, Serialize};

use super::{Permissions, RoleId, Snowflake, null_default};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RoleTags {
    #[serde(default)]
    pub bot_id: Option<Snowflake>,
    #[serde(default)]
    pub integration_id: Option<Snowflake>,
    #[serde(default)]
    pub subscription_listing_id: Option<Snowflake>,
    #[serde(default, deserialize_with = "present", serialize_with = "as_null", skip_serializing_if = "std::ops::Not::not")]
    pub guild_connections: bool,
}

fn present<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    serde::de::IgnoredAny::deserialize(d)?;
    Ok(true)
}

fn as_null<S: serde::Serializer>(_: &bool, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_none()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Role {
    pub id: RoleId,
    #[serde(default, deserialize_with = "null_default")]
    pub name: String,
    #[serde(default, deserialize_with = "null_default")]
    pub color: u32,
    #[serde(default, deserialize_with = "null_default")]
    pub hoist: bool,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub unicode_emoji: Option<String>,
    #[serde(default, deserialize_with = "null_default")]
    pub position: i32,
    #[serde(default)]
    pub permissions: Permissions,
    #[serde(default, deserialize_with = "null_default")]
    pub managed: bool,
    #[serde(default, deserialize_with = "null_default")]
    pub mentionable: bool,
    #[serde(default)]
    pub flags: u64,
    #[serde(default)]
    pub tags: Option<RoleTags>,
    #[serde(default)]
    pub colors: Option<RoleColors>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RoleColors {
    #[serde(default, deserialize_with = "null_default")]
    pub primary_color: u32,
    #[serde(default)]
    pub secondary_color: Option<u32>,
    #[serde(default)]
    pub tertiary_color: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guild_connections_flag_is_a_present_null() {
        let r: Role = serde_json::from_str(r#"{"id":"1","tags":{"guild_connections":null}}"#).unwrap();
        assert!(r.tags.as_ref().is_some_and(|t| t.guild_connections));
        let r: Role = serde_json::from_str(r#"{"id":"1","tags":{"bot_id":"2"}}"#).unwrap();
        assert!(!r.tags.as_ref().is_some_and(|t| t.guild_connections));
        let back: Role = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert!(!back.tags.unwrap().guild_connections);
        let c: Role = serde_json::from_str(r#"{"id":"1","tags":{"guild_connections":null}}"#).unwrap();
        let back: Role = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert!(back.tags.unwrap().guild_connections);
    }
}
