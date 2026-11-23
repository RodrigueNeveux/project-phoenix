use super::*;
use phoenix_api::gateway::events::parse_dispatch;

const FIX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../phoenix-api/tests/fixtures/");

fn ev(name: &str, file: &str) -> GatewayEvent {
    let raw = std::fs::read_to_string(format!("{FIX}{file}")).unwrap();
    parse_dispatch(name, &raw)
}

fn ready_cache() -> (Cache, Box<Ready>) {
    let cache = Cache::default();
    let e = ev("READY", "ready_user.json");
    cache.apply(&e);
    let GatewayEvent::Ready(r) = e else { panic!("not ready") };
    (cache, r)
}

#[test]
fn ready_builds_guilds_and_dms() {
    let (cache, r) = ready_cache();
    let prefs = Prefs::from_ready(&r);
    let d = build_ready(&cache, &r, &prefs);
    assert_eq!(d.me.display_name.as_ref(), "Phoenix");
    assert!(d.guilds.iter().any(|g| g.name.as_ref() == "Rustaceans"));
    assert_eq!(d.dm_channels.len(), 2);
    // Newest DM first.
    assert_eq!(d.dm_channels[0].id, Id(900000000000000001));
    assert!(d.dm_channels.iter().all(|c| c.guild.is_none() && !c.voice_users.is_empty()));
    let me = &d.users.iter().find(|u| u.id == d.me.id).unwrap().avatar;
    assert!(matches!(me, model::ImageRef::Url(u) if u.contains("size=128")));
}

#[test]
fn message_create_maps() {
    let (cache, r) = ready_cache();
    let mut a = Adapter::new(r.user.id, Prefs::from_ready(&r));
    let e = ev("MESSAGE_CREATE", "message_create.json");
    cache.apply(&e);
    let out = a.map(&cache, &e);
    let m = out.iter().find_map(|x| match x { AppEvent::MessageCreate(m) => Some(m), _ => None }).unwrap();
    assert_eq!(m.id, Id(700000000000000051));
    // Sizing (?width=assert!(m.attachments[0].is_image && m.attachments[0].url.contains("width="));height=) is now applied at render time by `media`, so the stored URL is the raw proxy URL.
    assert!(m.attachments[0].is_image && !m.attachments[0].url.is_empty());
    assert!(out.iter().any(|x| matches!(x, AppEvent::Users(u) if u.iter().any(|u| u.name.as_ref() == "bob"))));
}

#[test]
fn member_list_update_maps() {
    let (cache, r) = ready_cache();
    let mut a = Adapter::new(r.user.id, Prefs::from_ready(&r));
    let e = ev("GUILD_MEMBER_LIST_UPDATE", "member_list_update.json");
    cache.apply(&e);
    let out = a.map(&cache, &e);
    let ms = out.iter().find_map(|x| match x { AppEvent::GuildMembers { members, .. } => Some(members), _ => None }).unwrap();
    // The synced member, plus my own member (READY merged_members) for my server nick.
    assert_eq!(ms[0].presence, model::Presence::Idle);
    assert!(ms.len() == 1 || (ms.len() == 2 && ms[1].user == Id(r.user.id.0)));
}

#[test]
fn read_state_and_proto_order() {
    let rs = serde_json::json!({"entries": [{"id": "5", "last_message_id": "10", "mention_count": 2}]});
    let m = parse_read_state(Some(&rs));
    assert_eq!(m[&5], ReadMark { last_read: 10, mentions: 2 });
    // field 14 { field 1 (folder) { field 1 packed fixed64 [7, 3] } }
    let mut folder = vec![0x0a, 16];
    folder.extend(7u64.to_le_bytes());
    folder.extend(3u64.to_le_bytes());
    let mut gf = vec![0x0a, folder.len() as u8];
    gf.extend(folder);
    let mut top = vec![0x72, gf.len() as u8];
    top.extend(gf);
    let b64 = base64::engine::general_purpose::STANDARD.encode(top);
    assert_eq!(guild_order_from_proto(&b64), Some(vec![7, 3]));
}

#[test]
fn guild_resource_channels_are_hidden_and_not_unread() {
    let (cache, r) = ready_cache();
    let g = r.guilds.iter().find(|g| !g.unavailable).unwrap().id;
    let raw = format!(
        r#"{{"id":"700000000000000077","type":0,"guild_id":"{g}","name":"guide-page","position":0,"flags":128,"last_message_id":"700000000000000099"}}"#
    );
    cache.apply(&parse_dispatch("CHANNEL_CREATE", &raw));
    let prefs = Prefs::from_ready(&r);
    let chs = guild_channels(&cache, g, &prefs.ctx());
    assert!(!chs.iter().any(|c| c.id == Id(700000000000000077)));
}
