//! "Unread pins" dot on the header's pin button, as the client's ReadStateStore
//! computes it: `hasUnreadPins(c) = canBeUnread() && lastPinTimestamp > ackPinTimestamp`.
//!
//! - `lastPinTimestamp`: the channel object's `last_pin_timestamp` (READY guild
//!   channels / private channels, CHANNEL_CREATE / UPDATE, THREAD_*), then
//!   `CHANNEL_PINS_UPDATE {channel_id, last_pin_timestamp}`.
//! - `ackPinTimestamp`: the READY `read_state` entry's `last_pin_timestamp`, then
//!   `CHANNEL_PINS_ACK {channel_id, timestamp}`.
//!
//! Read-only by itself: only opening the pins popout acks (`POST
//! /channels/{id}/pins/ack`, the client's `ackPins`), and never in the
//! read-only examples (`NO_ACKS`).

use std::collections::HashMap;

use std::sync::{Mutex, MutexGuard};
use phoenix_api::{Cache, gateway::GatewayEvent, models as api};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PinMark {
    /// `lastPinTimestamp` (ms), 0 = never pinned.
    last: i64,
    /// `ackPinTimestamp` (ms).
    ack: i64,
}

static STATE: Mutex<Option<HashMap<u64, PinMark>>> = Mutex::new(None);

fn state() -> MutexGuard<'static, Option<HashMap<u64, PinMark>>> {
    STATE.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The client's `eL`: ISO-8601 to epoch ms, 0 when absent.
fn ts(v: Option<&str>) -> i64 {
    v.and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()).map_or(0, |t| t.timestamp_millis())
}

fn snowflake(v: Option<&Value>) -> Option<u64> {
    match v? {
        Value::String(s) => s.parse().ok(),
        Value::Number(n) => n.as_u64(),
        _ => None,
    }
}

fn build(channels: impl IntoIterator<Item = (u64, i64)>, read_state: Option<&Value>) -> HashMap<u64, PinMark> {
    let mut map: HashMap<u64, PinMark> = channels.into_iter().map(|(c, last)| (c, PinMark { last, ack: 0 })).collect();
    let entries = match read_state {
        Some(Value::Array(a)) => Some(a),
        Some(v) => v.get("entries").and_then(Value::as_array),
        None => None,
    };
    for e in entries.into_iter().flatten() {
        // read_state_type 0 (or absent) = channel.
        if e.get("read_state_type").and_then(Value::as_u64).unwrap_or(0) != 0 {
            continue;
        }
        let Some(c) = snowflake(e.get("id")) else { continue };
        map.entry(c).or_default().ack = ts(e.get("last_pin_timestamp").and_then(Value::as_str));
    }
    map
}

fn channel_pin(c: &api::Channel) -> (u64, i64) {
    (c.id.0, ts(c.last_pin_timestamp.as_deref()))
}

/// READY (first connect and non-resumed reconnects), after the cache applied it.
pub fn init(cache: &Cache) {
    let mut chans: Vec<(u64, i64)> = cache.private_channels().iter().map(|c| channel_pin(c)).collect();
    for g in cache.guilds() {
        chans.extend(cache.guild_channels(g.id).iter().map(|c| channel_pin(c)));
    }
    *state() = Some(build(chans, cache.read_state().as_ref()));
}

fn set_last(c: u64, last: i64) {
    if let Some(m) = state().as_mut() {
        m.entry(c).or_default().last = last;
    }
}

pub fn on_gateway(ev: &GatewayEvent) {
    match ev {
        GatewayEvent::ChannelCreate(c)
        | GatewayEvent::ChannelUpdate(c)
        | GatewayEvent::ThreadCreate(c)
        | GatewayEvent::ThreadUpdate(c) => {
            if c.last_pin_timestamp.is_some() {
                let (id, last) = channel_pin(c);
                set_last(id, last);
            }
        }
        GatewayEvent::Unknown { name, data } if name == "CHANNEL_PINS_UPDATE" => {
            if let Some(c) = snowflake(data.get("channel_id")) {
                set_last(c, ts(data.get("last_pin_timestamp").and_then(Value::as_str)));
            }
        }
        GatewayEvent::Unknown { name, data } if name == "CHANNEL_PINS_ACK" => {
            if let Some(c) = snowflake(data.get("channel_id"))
                && let Some(m) = state().as_mut()
            {
                m.entry(c).or_default().ack = ts(data.get("timestamp").and_then(Value::as_str));
            }
        }
        _ => {}
    }
}

/// `lastPinTimestamp > ackPinTimestamp`.
pub fn has_unread(channel: u64) -> bool {
    state().as_ref().and_then(|m| m.get(&channel)).is_some_and(|p| p.last > p.ack)
}

/// The client's `ackPins(null)` local half: `ackPinTimestamp = lastPinTimestamp`.
/// Returns whether there was anything to ack (then the caller POSTs).
pub fn mark_seen(channel: u64) -> bool {
    let mut g = state();
    let Some(p) = g.as_mut().and_then(|m| m.get_mut(&channel)) else { return false };
    if p.last <= p.ack {
        return false;
    }
    p.ack = p.last;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unread_until_acked() {
        let t1 = ts(Some("2026-11-20T10:00:00+00:00"));
        let rs = json!({"entries": [
            {"id": "1", "last_message_id": "5", "last_pin_timestamp": "2026-11-20T10:00:00+00:00"},
            {"id": "2", "last_message_id": "5", "last_pin_timestamp": "2026-11-19T10:00:00.000000+00:00"},
            {"id": "4", "read_state_type": 1, "last_pin_timestamp": "1970-01-01T00:00:00+00:00"}
        ]});
        let map = build([(1, t1), (2, t1), (3, t1), (4, 0)], Some(&rs));
        *state() = Some(map);
        assert!(!has_unread(1), "acked at the same pin");
        assert!(has_unread(2), "pinned after the ack");
        assert!(has_unread(3), "no read state");
        assert!(!has_unread(4), "never pinned");
        assert!(mark_seen(2) && !has_unread(2) && !mark_seen(2));
        on_gateway(&GatewayEvent::Unknown {
            name: "CHANNEL_PINS_UPDATE".into(),
            data: json!({"channel_id": "1", "last_pin_timestamp": "2026-11-21T10:00:00+00:00"}),
        });
        assert!(has_unread(1));
        on_gateway(&GatewayEvent::Unknown {
            name: "CHANNEL_PINS_ACK".into(),
            data: json!({"channel_id": "1", "timestamp": "2026-11-21T10:00:00+00:00", "version": 3}),
        });
        assert!(!has_unread(1));
    }
}
