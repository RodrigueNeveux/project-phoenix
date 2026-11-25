use phoenix_api::rest::guild_server::{AnalyticsKind, ServerCall};
use serde_json::{Value, json};

pub fn reply(c: &ServerCall) -> Value {
    use ServerCall as S;
    match c {
        S::Guild { guild } => json!({
            "id": guild, "name": "Phoenix Demo", "verification_level": 1, "explicit_content_filter": 2, "mfa_level": 1,
            "features": ["COMMUNITY", "NEWS"], "premium_tier": 2, "premium_subscription_count": 9, "premium_progress_bar_enabled": true,
            "rules_channel_id": "10", "public_updates_channel_id": "10", "safety_alerts_channel_id": "10", "preferred_locale": "fr",
            "description": "Un serveur de démonstration pour Phoenix.", "banner": null, "splash": null, "discovery_splash": null,
            "roles": [{ "id": guild, "permissions": "1024" }]
        }),
        S::AutomodRules { .. } => json!([
            { "id": "31", "name": "Block Words in Member Profile Names", "event_type": 2, "trigger_type": 6, "enabled": true,
              "actions": [{ "type": 4, "metadata": {} }], "trigger_metadata": { "keyword_filter": ["spam"] }, "exempt_roles": [], "exempt_channels": [] },
            { "id": "32", "name": "Block Mention Spam", "event_type": 1, "trigger_type": 5, "enabled": true,
              "actions": [{ "type": 1, "metadata": {} }], "trigger_metadata": { "mention_total_limit": 20 }, "exempt_roles": [], "exempt_channels": [] },
            { "id": "33", "name": "Mots personnalisés", "event_type": 1, "trigger_type": 1, "enabled": false,
              "actions": [{ "type": 1, "metadata": {} }, { "type": 2, "metadata": { "channel_id": "10" } }, { "type": 3, "metadata": { "duration_seconds": 300 } }],
              "trigger_metadata": { "keyword_filter": ["*crab*", "rouille"], "regex_patterns": ["fe+r+is"], "allow_list": [] }, "exempt_roles": [], "exempt_channels": ["10"] }
        ]),
        S::Integrations { .. } => json!([
            { "id": "1450000000000000000", "type": "discord", "name": "Phoenix Bot", "scopes": ["applications.commands", "bot"], "user": { "username": "ferris" },
              "application": { "id": "40", "name": "Phoenix Bot", "icon": null, "description": "Un bot de démonstration.", "is_verified": true, "bot": { "id": "40", "avatar": null } } }
        ]),
        S::Webhooks { .. } => json!([
            { "id": "1440000000000000000", "type": 1, "name": "Captain Hook", "channel_id": "10", "application_id": "40", "avatar": null, "user": { "username": "ferris" } }
        ]),
        S::AppCommands { .. } => json!({ "application_commands": [{ "name": "ping", "type": 1 }, { "name": "help", "type": 1 }] }),
        S::VanityUrl { .. } => json!({ "code": "phoenix", "uses": 12 }),
        S::Templates { .. } => json!([]),
        S::Onboarding { guild } => json!({ "guild_id": guild, "enabled": false, "mode": 1, "default_channel_ids": ["10"],
            "prompts": [{ "id": "1", "title": "Que veux-tu faire ?", "options": [{ "id": "2", "title": "Discuter" }, { "id": "3", "title": "Coder" }] }] }),
        S::Analytics { kind, .. } => match kind {
            AnalyticsKind::Engagement => json!([{ "visitors": 120, "communicators": 30 }, { "visitors": 100, "communicators": 30 }]),
            AnalyticsKind::Growth => json!([{ "new_members": 8 }, { "new_members": 10 }]),
            AnalyticsKind::Retention => json!([{ "pct_retained": 25.0 }, { "pct_retained": 20.0 }]),
        },
        S::DiscoveryMetadata { .. } => json!({ "primary_category_id": 49, "category_ids": [14], "reasons_to_join": [{ "reason": "Rust", "emoji_name": "🦀" }],
            "about": "Phoenix est un client natif.", "social_links": ["https://github.com/phoenix"] }),
        S::DiscoveryCategories { .. } => json!([{ "id": 49, "name": "Bots" }, { "id": 14, "name": "Discussion générale" }]),
        S::DiscoverableContent { .. } => json!({ "channel_summaries": { "enabled": true, "channels": [{ "channel_id": "10", "included": true }] },
            "channel_messages": { "enabled": false, "channels": [] } }),
        S::ServerIndex { .. } => json!({ "seo": { "canonical_url": "https://discord.com/servers/phoenix-demo-1" } }),
        S::Profile { .. } => json!({ "tag": "PHX", "badge": 8, "badge_color_primary": "#ff9b00", "badge_color_secondary": "#008dff", "badge_hash": "" }),
        _ => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phoenix_api::models::Snowflake;

    #[test]
    fn mock_rules_parse() {
        let rules = phoenix_api::rest::guild_server::parse_rules(&reply(&ServerCall::AutomodRules { guild: Snowflake(1) }));
        assert_eq!(rules.len(), 3);
        assert!(reply(&ServerCall::DeleteGuild { guild: Snowflake(1) }).is_null());
    }
}
