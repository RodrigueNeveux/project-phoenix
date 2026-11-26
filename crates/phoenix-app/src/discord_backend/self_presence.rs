use phoenix_api::{
    Cache,
    gateway::{GatewayCommand, UpdatePresence},
    rest::settings::CustomStatusProto,
};
use serde_json::{Value, json};

use super::nav::fields;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Custom {
    pub proto: CustomStatusProto,
    pub label: Option<String>,
    pub animated: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Local {
    pub status: Option<String>,
    pub custom: Option<Custom>,
}

fn sub(b: &[u8], n: u64) -> Option<&[u8]> {
    fields(b).into_iter().rev().find(|f| f.0 == n && f.1 == 2).map(|f| f.2)
}

fn text(b: &[u8], n: u64) -> String {
    sub(b, n).and_then(|v| std::str::from_utf8(v).ok()).unwrap_or_default().to_owned()
}

fn fixed(b: &[u8], n: u64) -> u64 {
    fields(b).into_iter().rev().find(|f| f.0 == n && f.1 == 1).map_or(0, |f| f.3)
}

fn custom(b: &[u8]) -> Custom {
    Custom {
        proto: CustomStatusProto {
            text: text(b, 1),
            emoji_id: fixed(b, 2),
            emoji_name: text(b, 3),
            expires_at_ms: fixed(b, 4),
            created_at_ms: fixed(b, 5),
        },
        label: sub(b, 6).map(|l| text(l, 1)),
        animated: None,
    }
}

impl Local {
    pub fn from_proto(raw: &[u8]) -> Self {
        let mut l = Self::default();
        l.apply(raw, false);
        l
    }

    pub fn from_base64(b64: Option<&str>) -> Self {
        use base64::Engine;
        b64.and_then(|b| base64::engine::general_purpose::STANDARD.decode(b.trim()).ok()).map(|b| Self::from_proto(&b)).unwrap_or_default()
    }

    pub fn apply(&mut self, raw: &[u8], partial: bool) {
        let st = sub(raw, 11);
        if partial && st.is_none() {
            return;
        }
        let st = st.unwrap_or_default();
        let status = sub(st, 1).map(|v| text(v, 1));
        let cs = sub(st, 2).map(custom);
        if partial {
            if status.is_some() {
                self.status = status;
            }
            if cs.is_some() {
                self.custom = cs;
            }
        } else {
            self.status = status;
            self.custom = cs;
        }
    }

    pub fn set_status(&mut self, status: &str) {
        self.status = Some(status.to_owned());
    }

    pub fn status(&self) -> &str {
        match self.status.as_deref() {
            None | Some("" | "unknown") => "online",
            Some(s) => s,
        }
    }

    pub fn payload(&self, now_ms: u64, animated: impl Fn(u64) -> bool) -> UpdatePresence {
        let status = self.status().to_owned();
        let activities = match &self.custom {
            Some(c) if status != "invisible" && (c.proto.expires_at_ms == 0 || c.proto.expires_at_ms > now_ms) => {
                vec![activity(c, &animated)]
            }
            _ => Vec::new(),
        };
        UpdatePresence { since: 0, activities, status, afk: false }
    }
}

fn activity(c: &Custom, animated: &dyn Fn(u64) -> bool) -> Value {
    let p = &c.proto;
    let mut a = json!({"name": "Custom Status", "type": 4, "emoji": null, "metadata": {}});
    if !p.text.is_empty() {
        a["state"] = json!(p.text);
    }
    if p.expires_at_ms > 0 {
        a["timestamps"] = json!({"end": p.expires_at_ms});
    }
    if p.emoji_id != 0 {
        let anim = c.animated.unwrap_or_else(|| animated(p.emoji_id));
        a["emoji"] = json!({"id": p.emoji_id.to_string(), "name": p.emoji_name, "animated": anim});
    } else if !p.emoji_name.is_empty() {
        a["emoji"] = json!({"id": null, "name": p.emoji_name, "animated": false});
    }
    if let Some(l) = &c.label {
        a["details"] = json!(l);
        a["metadata"] = json!({"label": l});
    }
    a
}

fn emoji_animated(cache: &Cache, emoji: u64) -> bool {
    cache.guilds().iter().flat_map(|g| g.emojis.iter()).find(|e| e.id.map(|i| i.0) == Some(emoji)).is_some_and(|e| e.animated)
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

pub(super) fn send(s: &super::Session) {
    let p = s.presence.lock().payload(now_ms(), |e| emoji_animated(&s.cache, e));
    let _ = s.gateway.send(GatewayCommand::UpdatePresence(p));
}

pub(super) fn on_settings_update(s: &super::Session, data: &Value) {
    let Some((raw, partial)) = settings_update(data) else { return };
    let changed = {
        let mut l = s.presence.lock();
        let before = l.clone();
        l.apply(&raw, partial);
        *l != before
    };
    if changed {
        send(s);
    }
}

fn settings_update(data: &Value) -> Option<(Vec<u8>, bool)> {
    use base64::Engine;
    let s = data.get("settings")?;
    if s.get("type").and_then(Value::as_u64) != Some(1) {
        return None;
    }
    let raw = base64::engine::general_purpose::STANDARD.decode(s.get("proto")?.as_str()?.trim()).ok()?;
    Some((raw, data.get("partial").and_then(Value::as_bool).unwrap_or(false)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use phoenix_api::rest::settings::encode_custom_status;

    fn fb(tag: u8, data: &[u8], out: &mut Vec<u8>) {
        out.push((tag << 3) | 2);
        out.push(data.len() as u8);
        out.extend_from_slice(data);
    }

    fn settings(status: Option<&str>, cs: Option<&CustomStatusProto>) -> Vec<u8> {
        let mut st = Vec::new();
        if let Some(s) = status {
            let mut sv = Vec::new();
            fb(1, s.as_bytes(), &mut sv);
            fb(1, &sv, &mut st);
        }
        if let Some(c) = cs {
            fb(2, &encode_custom_status(c), &mut st);
        }
        let mut out = Vec::new();
        fb(11, &st, &mut out);
        out
    }

    #[test]
    fn ready_status_is_sent_as_is() {
        let l = Local::from_proto(&settings(Some("dnd"), None));
        let p = l.payload(0, |_| false);
        assert_eq!(serde_json::to_value(&p).unwrap(), json!({"since": 0, "activities": [], "status": "dnd", "afk": false}));
    }

    #[test]
    fn missing_or_unknown_status_means_online() {
        assert_eq!(Local::from_proto(&[]).status(), "online");
        assert_eq!(Local::from_proto(&settings(Some("unknown"), None)).status(), "online");
        assert_eq!(Local::from_base64(None).status(), "online");
    }

    #[test]
    fn custom_status_activity_matches_the_client() {
        let c = CustomStatusProto { text: "Lunch".into(), emoji_id: 42, emoji_name: "dance".into(), expires_at_ms: 2_000, created_at_ms: 1_000 };
        let l = Local::from_proto(&settings(Some("idle"), Some(&c)));
        let p = l.payload(1_500, |id| id == 42);
        assert_eq!(
            serde_json::to_value(&p).unwrap(),
            json!({"since": 0, "status": "idle", "afk": false, "activities": [{
                "name": "Custom Status", "type": 4, "state": "Lunch", "timestamps": {"end": 2_000},
                "emoji": {"id": "42", "name": "dance", "animated": true}, "metadata": {}
            }]})
        );
        assert!(l.payload(2_000, |_| false).activities.is_empty());
    }

    #[test]
    fn unicode_emoji_only_and_invisible() {
        let c = CustomStatusProto { emoji_name: "\u{1f354}".into(), ..Default::default() };
        let mut l = Local::from_proto(&settings(Some("online"), Some(&c)));
        assert_eq!(
            l.payload(0, |_| false).activities,
            vec![json!({"name": "Custom Status", "type": 4, "emoji": {"id": null, "name": "\u{1f354}", "animated": false}, "metadata": {}})]
        );
        l.set_status("invisible");
        assert!(l.payload(0, |_| false).activities.is_empty());
    }

    #[test]
    fn partial_updates_merge_and_full_updates_replace() {
        let c = CustomStatusProto { text: "hi".into(), ..Default::default() };
        let mut l = Local::from_proto(&settings(Some("dnd"), Some(&c)));
        l.apply(&settings(Some("idle"), None), true);
        assert_eq!((l.status(), l.custom.as_ref().map(|c| c.proto.text.as_str())), ("idle", Some("hi")));
        l.apply(&[], true);
        assert_eq!(l.status(), "idle");
        l.apply(&settings(Some("online"), None), false);
        assert_eq!((l.status(), l.custom.is_none()), ("online", true));
    }

    #[test]
    fn settings_update_reads_preloaded_proto_only() {
        use base64::Engine;
        let b = base64::engine::general_purpose::STANDARD.encode(settings(Some("dnd"), None));
        let d = json!({"settings": {"type": 1, "proto": b}, "partial": true});
        let (raw, partial) = settings_update(&d).unwrap();
        assert!(partial);
        assert_eq!(Local::from_proto(&raw).status(), "dnd");
        assert!(settings_update(&json!({"settings": {"type": 2, "proto": ""}})).is_none());
    }
}
