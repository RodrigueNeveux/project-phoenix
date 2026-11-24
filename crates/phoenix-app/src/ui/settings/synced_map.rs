//! Synced settings beyond the hand-modelled ones: each entry maps a settings
//! control id to its `PreloadedUserSettings` field path, as declared by the
//! 2026 client's protobuf descriptors (`discord_protos.discord_users.v1.*`,
//! web bundle of the local Discord cache). See docs/parity/settings-wiring.md.
//!
//! Values are stored as wire integers in `SyncedSettings::proto` (strings as
//! their index in [`Kind::Str`]); the UI reads them through [`get_bool`] /
//! [`get_pick`].

use phoenix_api::rest::settings::{ProtoValue, proto_read};

/// How the wire value maps to the control.
#[derive(Clone, Copy, Debug)]
pub enum Kind {
    /// Switch; `invert` when the proto field is the negation of the label.
    Bool { invert: bool },
    /// Radio / select index `i` ↔ wire enum `opts[i]`.
    Enum(&'static [u64]),
    /// Radio / select index `i` ↔ wire string `opts[i]`.
    Str(&'static [&'static str]),
    /// Switch over bits of a flags field (`friend_source_flags`).
    Bits(u64),
}

#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub id: &'static str,
    /// Tags from `PreloadedUserSettings` down to the field.
    pub path: &'static [u32],
    /// The field is a `google.protobuf.*Value` wrapper.
    pub wrapper: bool,
    pub kind: Kind,
    /// Client default as a UI value (switch 0/1 or option index).
    pub default: u64,
}

const fn b(id: &'static str, path: &'static [u32], default: bool) -> Entry {
    Entry { id, path, wrapper: true, kind: Kind::Bool { invert: false }, default: default as u64 }
}
const fn inv(id: &'static str, path: &'static [u32], default: bool) -> Entry {
    Entry { id, path, wrapper: true, kind: Kind::Bool { invert: true }, default: default as u64 }
}
const fn en(id: &'static str, path: &'static [u32], wrapper: bool, opts: &'static [u64], default: u64) -> Entry {
    Entry { id, path, wrapper, kind: Kind::Enum(opts), default }
}

// Top-level tags: 6 text_and_images, 7 notifications, 8 privacy,
// 11 status, 13 appearance, 19 clips, 5 voice_and_video.
pub static ENTRIES: &[Entry] = &[
    // --- Text & images (6)
    b("md-preview", &[6, 2], true),      // use_rich_chat_input
    b("split-threads", &[6, 3], true),   // use_thread_sidebar
    Entry { id: "spoilers", path: &[6, 4], wrapper: true, kind: Kind::Str(&["ON_CLICK", "ALWAYS", "IF_MODERATOR"]), default: 0 }, // render_spoilers
    b("alt-text", &[6, 7], false),       // view_image_descriptions
    b("media-uploads", &[6, 9], true),   // inline_attachment_media
    b("media-links", &[6, 10], true),    // inline_embed_media
    b("gifs", &[6, 11], true),           // gif_auto_play
    b("embeds", &[6, 12], true),         // render_embeds
    b("reactions", &[6, 13], true),      // render_reactions
    // animate_stickers (UInt32Value): 0 always, 1 on interaction, 2 never
    en("stickers", &[6, 15], true, &[0, 1, 2], 0),
    b("tts", &[6, 16], true),            // enable_tts_command
    b("age-ios", &[6, 20], false),       // view_nsfw_guilds
    b("anim-emoji", &[6, 14], true),     // animate_emoji
    b("emoticons", &[6, 21], true),      // convert_emoticons
    b("age-cmds", &[6, 23], false),      // view_nsfw_commands
    b("legacy-input", &[6, 24], false),  // use_legacy_chat_input
    // dm_spam_filter_v2: 3 friends+non-friends, 2 non-friends, 1 disabled
    en("spam", &[6, 27], false, &[3, 2, 1], 1),
    b("ac-stickers", &[6, 28], true),    // include_stickers_in_autocomplete
    // explicit_content_settings (29): ExplicitContentRedaction SHOW 1 BLUR 2 BLOCK 3
    en("cf-servers", &[6, 29, 1], false, &[1, 2, 3], 0),
    en("cf-friends", &[6, 29, 2], false, &[1, 2, 3], 0),
    en("cf-others", &[6, 29, 3], false, &[1, 2, 3], 2),
    // gore_content_settings (32), same enum
    en("cfv-servers", &[6, 32, 1], false, &[1, 2, 3], 0),
    en("cfv-friends", &[6, 32, 2], false, &[1, 2, 3], 0),
    en("cfv-others", &[6, 32, 3], false, &[1, 2, 3], 1),
    en("dm-search", &[6, 36], true, &[0, 1], 0), // is_cross_dm_search_enabled
    b("ac-games", &[6, 39], true),       // include_game_mentions_in_autocomplete
    b("avatars", &[6, 41], true),        // display_compact_avatars
    // --- Notifications (7)
    b("act-stream", &[7, 2], true),      // notify_friends_on_go_live
    // reaction_notifications: 0 enabled, 1 only DMs, 2 disabled
    en("n-react", &[7, 7], false, &[0, 1, 2], 0),
    b("n-online", &[7, 12], true),      // enable_friend_online_notifications
    b("n-anniv", &[7, 14], true),        // enable_friend_anniversary_notifications
    b("n-profile", &[7, 16], true),      // enable_profile_updates_notifications
    b("n-event", &[7, 22], true),        // enable_upcoming_server_event_notifications
    b("share-update", &[7, 24], true),   // notify_friends_on_profile_update
    b("act-online", &[7, 25], true),     // notify_friends_on_come_online
    // --- Privacy (8)
    b("act-friends", &[8, 1], true),     // allow_activity_party_privacy_friends
    b("act-voice", &[8, 2], true),       // allow_activity_party_privacy_voice_channel
    // friend_source_flags (UInt32Value): MUTUAL_FRIENDS 2, MUTUAL_GUILDS 4, NO_RELATION 8
    Entry { id: "fr-everyone", path: &[8, 11], wrapper: true, kind: Kind::Bits(8), default: 1 },
    Entry { id: "fr-fof", path: &[8, 11], wrapper: true, kind: Kind::Bits(2), default: 1 },
    Entry { id: "fr-members", path: &[8, 11], wrapper: true, kind: Kind::Bits(4), default: 1 },
    inv("msg-requests", &[8, 17], true), // default_message_request_restricted
    b("game-dms", &[8, 26], true),       // allow_game_friend_dms_in_discord
    // slayer_sdk_receive_dms_in_game: 1 all, 2 users with game, 3 none
    en("ingame", &[8, 28], false, &[1, 2, 3], 0),
    // default_guilds_activity_restricted_v2: 3 on, 2 on for large guilds, 1 off
    en("act-where", &[8, 29], false, &[3, 2, 1], 0),
    inv("fr-msg", &[8, 33], true),       // hide_friend_request_notes
    // --- Status (11)
    b("act-share", &[11, 3], true),      // show_current_game
    // --- Appearance (13): timestamp_hour_cycle AUTO 0, H12 1, H23 2
    en("time-format", &[13, 9], false, &[0, 1, 2], 0),
    // --- Clips (19)
    b("data-clips", &[19, 1], true),     // allow_voice_recording
];

/// Account settings outside settings-proto (REST routes of the client).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rest {
    /// `/users/@me/consent` type.
    Consent(&'static str),
    /// `/users/@me/email-settings` category.
    Email(&'static str),
}

/// (control id, route, client default)
pub static REST: &[(&str, Rest, bool)] = &[
    ("data-improve", Rest::Consent("usage_statistics"), true),
    ("data-personalize", Rest::Consent("personalization"), true),
    ("e-comm", Rest::Email("communication"), true),
    ("e-social", Rest::Email("social"), true),
    ("e-news", Rest::Email("updates_and_announcements"), false),
    ("e-tips", Rest::Email("tips"), false),
    ("e-recs", Rest::Email("recommendations_and_events"), false),
];

pub fn rest(id: &str) -> Option<(Rest, bool)> {
    REST.iter().find(|r| r.0 == id).map(|r| (r.1, r.2))
}

/// Values found in the consent and email-settings responses.
pub fn read_rest(consent: Option<&serde_json::Value>, email: Option<&serde_json::Value>) -> std::collections::HashMap<&'static str, bool> {
    let cats = email.and_then(|e| e.get("categories").or_else(|| e.get("settings").and_then(|s| s.get("categories"))));
    REST.iter()
        .filter_map(|&(id, r, _)| {
            let v = match r {
                Rest::Consent(k) => consent?.get(k)?.get("consented")?.as_bool()?,
                Rest::Email(k) => cats?.get(k)?.as_bool()?,
            };
            Some((id, v))
        })
        .collect()
}

pub fn entry(id: &str) -> Option<&'static Entry> {
    ENTRIES.iter().find(|e| e.id == id)
}

/// Wire value for every entry present in a raw settings blob.
pub fn read_all(raw: &[u8]) -> std::collections::HashMap<&'static str, u64> {
    let mut out = std::collections::HashMap::new();
    for e in ENTRIES {
        let as_str = matches!(e.kind, Kind::Str(_));
        let v = match proto_read(raw, e.path, e.wrapper, as_str) {
            Some(ProtoValue::Int(v)) => v,
            Some(ProtoValue::Str(s)) => match e.kind {
                Kind::Str(opts) => match opts.iter().position(|o| *o == s) {
                    Some(i) => i as u64,
                    None => continue,
                },
                _ => continue,
            },
            None => continue,
        };
        out.insert(e.id, v);
    }
    out
}

/// Flags fields read as "all set" when unset (the client's default).
fn bits_of(map: &std::collections::HashMap<&'static str, u64>, id: &str) -> u64 {
    map.get(id).copied().unwrap_or(2 | 4 | 8)
}

pub fn get_bool(map: &std::collections::HashMap<&'static str, u64>, id: &str) -> bool {
    let Some(e) = entry(id) else { return false };
    match (e.kind, map.get(id)) {
        (Kind::Bits(m), _) => bits_of(map, id) & m != 0,
        (Kind::Bool { invert }, Some(v)) => (*v != 0) != invert,
        _ => e.default != 0,
    }
}

pub fn get_pick(map: &std::collections::HashMap<&'static str, u64>, id: &str) -> usize {
    let Some(e) = entry(id) else { return 0 };
    let v = map.get(id).copied();
    match e.kind {
        Kind::Enum(opts) => v.and_then(|v| opts.iter().position(|o| *o == v)).unwrap_or(e.default as usize),
        Kind::Str(_) => v.map(|v| v as usize).unwrap_or(e.default as usize),
        _ => v.unwrap_or(e.default) as usize,
    }
}

/// Wire value to store for switch `id` set to `on`. Bit switches combine
/// with the other bits of the same field ("Everyone" sets them all).
pub fn bool_wire(map: &std::collections::HashMap<&'static str, u64>, id: &str, on: bool) -> u64 {
    match entry(id).map(|e| e.kind) {
        Some(Kind::Bits(m)) => {
            let cur = bits_of(map, id);
            match (m, on) {
                (8, true) => 2 | 4 | 8,
                (_, true) => cur | m,
                (_, false) => cur & !m,
            }
        }
        Some(Kind::Bool { invert }) => (on != invert) as u64,
        _ => on as u64,
    }
}

pub fn pick_wire(id: &str, i: usize) -> u64 {
    match entry(id).map(|e| e.kind) {
        Some(Kind::Enum(opts)) => opts.get(i).copied().unwrap_or(0),
        _ => i as u64,
    }
}

/// API change for a stored wire value of `id`.
pub fn change(id: &str, wire: u64) -> Option<phoenix_api::rest::settings::SettingChange> {
    let e = entry(id)?;
    let value = match e.kind {
        Kind::Str(opts) => ProtoValue::Str(opts.get(wire as usize)?.to_string()),
        _ => ProtoValue::Int(wire),
    };
    Some(phoenix_api::rest::settings::SettingChange::Path { path: e.path.to_vec(), wrapper: e.wrapper, value })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_through_wire() {
        let mut raw = Vec::new();
        for (id, w) in [("embeds", 0), ("spoilers", 1), ("time-format", 2), ("msg-requests", 1), ("cf-others", 1)] {
            raw.extend(change(id, w).unwrap().encode());
        }
        let m = read_all(&raw);
        assert!(!get_bool(&m, "embeds"));
        assert!(get_bool(&m, "reactions"), "unset keeps the default");
        assert_eq!(get_pick(&m, "spoilers"), 1);
        assert_eq!(get_pick(&m, "time-format"), 2);
        assert!(!get_bool(&m, "msg-requests"), "inverted field");
        assert_eq!(get_pick(&m, "cf-others"), 0);
    }

    #[test]
    fn friend_source_bits() {
        let mut m = std::collections::HashMap::new();
        assert!(get_bool(&m, "fr-fof"));
        m.insert("fr-fof", bool_wire(&m, "fr-fof", false));
        assert_eq!(m["fr-fof"], 4 | 8);
        assert!(!get_bool(&m, "fr-fof"));
    }

    #[test]
    fn rest_responses() {
        let c = serde_json::json!({"usage_statistics": {"consented": false}, "personalization": {"consented": true}});
        let e = serde_json::json!({"initialized": true, "categories": {"social": false, "tips": true}});
        let m = read_rest(Some(&c), Some(&e));
        assert_eq!(m.get("data-improve"), Some(&false));
        assert_eq!(m.get("e-social"), Some(&false));
        assert_eq!(m.get("e-tips"), Some(&true));
        assert!(!m.contains_key("e-comm"));
    }

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<_> = ENTRIES.iter().map(|e| e.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), ENTRIES.len());
    }
}
