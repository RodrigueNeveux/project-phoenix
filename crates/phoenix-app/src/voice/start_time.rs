use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};

use phoenix_api::gateway::events::GatewayEvent;
use serde_json::Value;

#[derive(Default)]
struct Store {
    start: HashMap<u64, i64>,
    status: HashMap<u64, String>,
    requested: HashSet<u64>,
}

static STORE: LazyLock<Mutex<Store>> = LazyLock::new(Default::default);

pub fn request(guild: u64) -> Option<(u8, Value)> {
    STORE
        .lock()
        .ok()?
        .requested
        .insert(guild)
        .then(|| (43, serde_json::json!({ "guild_id": guild.to_string(), "fields": ["status", "voice_start_time"] })))
}

fn id(v: &Value) -> Option<u64> {
    v.get("id")?.as_str()?.parse().ok()
}

fn set_status(s: &mut Store, channel: u64, v: &Value) {
    match v.get("status").and_then(Value::as_str).filter(|t| !t.is_empty()) {
        Some(t) => s.status.insert(channel, t.to_owned()),
        None => s.status.remove(&channel),
    };
}

fn ts(v: &Value) -> Option<i64> {
    v.get("voice_start_time").and_then(Value::as_i64)
}

pub fn apply(name: &str, data: &Value) -> bool {
    let Ok(mut s) = STORE.lock() else { return false };
    match name {
        "CHANNEL_INFO" => {
            for c in data.get("channels").and_then(Value::as_array).into_iter().flatten() {
                if let Some(i) = id(c) {
                    set_status(&mut s, i, c);
                    match ts(c) {
                        Some(t) => s.start.insert(i, t),
                        None => s.start.remove(&i),
                    };
                }
            }
            true
        }
        "VOICE_CHANNEL_STATUS_UPDATE" => {
            let Some(i) = id(data) else { return false };
            set_status(&mut s, i, data);
            true
        }
        "VOICE_CHANNEL_START_TIME_UPDATE" => {
            let Some(i) = id(data) else { return false };
            match ts(data) {
                Some(t) => s.start.insert(i, t),
                None => s.start.remove(&i),
            };
            true
        }
        _ => false,
    }
}

pub fn on_gateway(ev: &GatewayEvent) -> bool {
    match ev {
        GatewayEvent::Unknown { name, data } => apply(name, data),
        _ => false,
    }
}

pub fn get(channel: u64) -> Option<i64> {
    STORE.lock().ok()?.start.get(&channel).copied()
}

pub fn status(channel: u64) -> Option<String> {
    STORE.lock().ok()?.status.get(&channel).cloned()
}

pub fn set_channel_status(channel: u64, status: Option<&str>) {
    if let Ok(mut s) = STORE.lock() {
        match status.filter(|t| !t.is_empty()) {
            Some(t) => s.status.insert(channel, t.to_owned()),
            None => s.status.remove(&channel),
        };
    }
}

pub fn ticking() -> bool {
    STORE.lock().is_ok_and(|s| !s.start.is_empty())
}

pub fn set(channel: u64, start: Option<i64>) {
    if let Ok(mut s) = STORE.lock() {
        match start {
            Some(t) => s.start.insert(channel, t),
            None => s.start.remove(&channel),
        };
    }
}

pub fn format(elapsed_s: i64) -> String {
    let e = elapsed_s.max(0);
    let (h, m, s) = (e / 3600, e / 60 % 60, e % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

pub fn now_s() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_like_the_client() {
        assert_eq!(format(47 * 3600 + 50 * 60 + 7), "47:50:07");
        assert_eq!(format(65), "1:05");
    }

    #[test]
    fn parses_channel_info() {
        apply("CHANNEL_INFO", &serde_json::json!({"guild_id":"1","channels":[{"id":"77","voice_start_time":1700000000}]}));
        assert_eq!(get(77), Some(1_700_000_000));
        apply("VOICE_CHANNEL_START_TIME_UPDATE", &serde_json::json!({"id":"77","guild_id":"1","voice_start_time":null}));
        assert_eq!(get(77), None);
        assert!(request(5).is_some());
        assert!(request(5).is_none());
    }

    #[test]
    fn keeps_voice_channel_statuses() {
        apply("CHANNEL_INFO", &serde_json::json!({"guild_id":"1","channels":[{"id":"78","status":"on code <:ferris:42> 🦀"},{"id":"79","status":""}]}));
        assert_eq!(status(78).as_deref(), Some("on code <:ferris:42> 🦀"));
        assert_eq!(status(79), None);
        apply("VOICE_CHANNEL_STATUS_UPDATE", &serde_json::json!({"id":"78","guild_id":"1","status":null}));
        assert_eq!(status(78), None);
        apply("VOICE_CHANNEL_STATUS_UPDATE", &serde_json::json!({"id":"79","guild_id":"1","status":"pause"}));
        assert_eq!(status(79).as_deref(), Some("pause"));
    }
}
