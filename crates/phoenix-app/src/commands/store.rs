//! Per-message data the plain `model::Message` doesn't carry: bot message
//! components (buttons, selects, Components V2 layout) and polls.
//!
//! Filled by `convert::message` (one call per converted message) and read by
//! `ui::components` when a row is drawn. Only messages that actually have
//! components or a poll are kept.

use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use parking_lot::Mutex;
use phoenix_api::{models as api, rest::forum::Poll};
use serde_json::Value;

use crate::model::Id;

#[derive(Clone, Debug, PartialEq)]
pub struct Emoji {
    /// Unicode emoji, or the name of a custom one.
    pub name: String,
    pub id: Option<u64>,
    pub animated: bool,
}

impl Emoji {
    fn from_value(v: Option<&Value>) -> Option<Self> {
        let v = v?;
        let name = v.get("name").and_then(Value::as_str).unwrap_or_default().to_owned();
        let id = v.get("id").and_then(snowflake);
        (id.is_some() || !name.is_empty()).then_some(Self {
            name,
            id,
            animated: v.get("animated").and_then(Value::as_bool).unwrap_or(false),
        })
    }

    /// Token understood by `media::emoji::emoji_element`.
    pub fn token(&self) -> String {
        match self.id {
            Some(id) => format!("<{}:{}:{}>", if self.animated { "a" } else { "" }, self.name, id),
            None => self.name.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectOption {
    pub label: String,
    pub value: String,
    pub description: Option<String>,
    pub emoji: Option<Emoji>,
    pub default: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Media {
    pub url: String,
    pub description: Option<String>,
    pub spoiler: bool,
}

/// One message component (types from the Discord docs).
#[derive(Clone, Debug, PartialEq)]
pub enum Comp {
    /// 1: horizontal row of buttons or one select.
    Row(Vec<Comp>),
    /// 2
    Button {
        style: u8,
        label: Option<String>,
        emoji: Option<Emoji>,
        custom_id: Option<String>,
        url: Option<String>,
        disabled: bool,
    },
    /// 3 (string), 5 (user), 6 (role), 7 (mentionable), 8 (channel).
    Select {
        kind: u8,
        custom_id: String,
        placeholder: Option<String>,
        options: Vec<SelectOption>,
        min: u32,
        max: u32,
        disabled: bool,
    },
    /// 9: text blocks with a button or thumbnail on the right.
    Section { texts: Vec<Comp>, accessory: Option<Box<Comp>> },
    /// 10: markdown.
    Text(String),
    /// 11
    Thumbnail(Media),
    /// 12
    Gallery(Vec<Media>),
    /// 13
    File { url: String, spoiler: bool },
    /// 14: `spacing` 1 = small, 2 = large.
    Separator { divider: bool, spacing: u8 },
    /// 17
    Container { accent: Option<u32>, spoiler: bool, children: Vec<Comp> },
    Unknown,
}

fn snowflake(v: &Value) -> Option<u64> {
    match v {
        Value::String(s) => s.parse().ok(),
        Value::Number(n) => n.as_u64(),
        _ => None,
    }
}

fn s(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_owned)
}

fn media(v: Option<&Value>) -> Option<Media> {
    let v = v?;
    let m = v.get("media").unwrap_or(v);
    let url = m.get("proxy_url").or_else(|| m.get("url")).and_then(Value::as_str)?.to_owned();
    Some(Media {
        url,
        description: s(v, "description"),
        spoiler: v.get("spoiler").and_then(Value::as_bool).unwrap_or(false),
    })
}

fn list(v: Option<&Value>) -> Vec<Comp> {
    v.and_then(Value::as_array).map(|a| a.iter().map(parse_one).collect()).unwrap_or_default()
}

pub fn parse_one(v: &Value) -> Comp {
    let b = |k: &str| v.get(k).and_then(Value::as_bool).unwrap_or(false);
    match v.get("type").and_then(Value::as_u64).unwrap_or(0) {
        1 => Comp::Row(list(v.get("components"))),
        2 => Comp::Button {
            style: v.get("style").and_then(Value::as_u64).unwrap_or(1) as u8,
            label: s(v, "label"),
            emoji: Emoji::from_value(v.get("emoji")),
            custom_id: s(v, "custom_id"),
            url: s(v, "url"),
            disabled: b("disabled"),
        },
        k @ (3 | 5 | 6 | 7 | 8) => Comp::Select {
            kind: k as u8,
            custom_id: s(v, "custom_id").unwrap_or_default(),
            placeholder: s(v, "placeholder"),
            options: v
                .get("options")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .map(|o| SelectOption {
                            label: s(o, "label").unwrap_or_default(),
                            value: s(o, "value").unwrap_or_default(),
                            description: s(o, "description"),
                            emoji: Emoji::from_value(o.get("emoji")),
                            default: o.get("default").and_then(Value::as_bool).unwrap_or(false),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            min: v.get("min_values").and_then(Value::as_u64).unwrap_or(1) as u32,
            max: v.get("max_values").and_then(Value::as_u64).unwrap_or(1) as u32,
            disabled: b("disabled"),
        },
        9 => Comp::Section {
            texts: list(v.get("components")),
            accessory: v.get("accessory").map(|a| Box::new(parse_one(a))),
        },
        10 => Comp::Text(s(v, "content").unwrap_or_default()),
        11 => media(Some(v)).map(Comp::Thumbnail).unwrap_or(Comp::Unknown),
        12 => Comp::Gallery(
            v.get("items").and_then(Value::as_array).map(|a| a.iter().filter_map(|i| media(Some(i))).collect()).unwrap_or_default(),
        ),
        13 => Comp::File {
            url: v.get("file").and_then(|f| f.get("url")).and_then(Value::as_str).unwrap_or_default().to_owned(),
            spoiler: b("spoiler"),
        },
        14 => Comp::Separator {
            divider: v.get("divider").and_then(Value::as_bool).unwrap_or(true),
            spacing: v.get("spacing").and_then(Value::as_u64).unwrap_or(1) as u8,
        },
        17 => Comp::Container {
            accent: v.get("accent_color").and_then(Value::as_u64).map(|c| c as u32),
            spoiler: b("spoiler"),
            children: list(v.get("components")),
        },
        _ => Comp::Unknown,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PollAnswer {
    pub id: u32,
    pub text: String,
    pub emoji: Option<Emoji>,
    pub count: u32,
    pub me: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PollView {
    pub question: String,
    pub answers: Vec<PollAnswer>,
    pub multi: bool,
    /// Unix seconds.
    pub expiry: Option<i64>,
    pub finalized: bool,
}

impl PollView {
    pub fn from_value(v: &Value) -> Option<Self> {
        let p = Poll::from_value(v)?;
        Some(Self {
            question: p.question.text.clone().unwrap_or_default(),
            answers: p
                .answers
                .iter()
                .map(|a| {
                    let (count, me) = p.count(a.answer_id);
                    PollAnswer {
                        id: a.answer_id,
                        text: a.poll_media.text.clone().unwrap_or_default(),
                        emoji: Emoji::from_value(a.poll_media.emoji.as_ref()),
                        count,
                        me,
                    }
                })
                .collect(),
            multi: p.allow_multiselect,
            expiry: p.expiry.as_deref().and_then(|e| chrono::DateTime::parse_from_rfc3339(e).ok()).map(|d| d.timestamp()),
            finalized: p.results.as_ref().is_some_and(|r| r.is_finalized),
        })
    }

    pub fn total(&self) -> u32 {
        self.answers.iter().map(|a| a.count).sum()
    }

    pub fn voted(&self) -> bool {
        self.answers.iter().any(|a| a.me)
    }

    pub fn ended(&self, now: i64) -> bool {
        self.finalized || self.expiry.is_some_and(|e| e <= now)
    }

    /// Apply a live vote (`MESSAGE_POLL_VOTE_ADD/REMOVE`).
    pub fn apply_vote(&mut self, answer: u32, add: bool, mine: bool) {
        if let Some(a) = self.answers.iter_mut().find(|a| a.id == answer) {
            a.count = if add { a.count + 1 } else { a.count.saturating_sub(1) };
            if mine {
                a.me = add;
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Parts {
    /// Application that owns the components (interaction target).
    pub app: Id,
    pub flags: u64,
    pub components: Vec<Comp>,
    pub poll: Option<PollView>,
}

fn slot() -> &'static Mutex<HashMap<u64, Arc<Parts>>> {
    static S: OnceLock<Mutex<HashMap<u64, Arc<Parts>>>> = OnceLock::new();
    S.get_or_init(Default::default)
}

/// Remember components / poll of a message (no-op for plain messages).
pub fn record(m: &api::Message) {
    let components = list(m.components.as_ref());
    let poll = m.poll.as_ref().and_then(PollView::from_value);
    if components.is_empty() && poll.is_none() {
        return;
    }
    let app = m.application_id.or(m.webhook_id).map(|s| s.0).unwrap_or(m.author.id.0);
    let parts = Parts { app: Id(app), flags: m.flags, components, poll };
    slot().lock().insert(m.id.0, Arc::new(parts));
}

pub fn get(msg: Id) -> Option<Arc<Parts>> {
    slot().lock().get(&msg.0).cloned()
}

/// Edit the stored poll in place; returns false when there is none.
pub fn update_poll(msg: Id, f: impl FnOnce(&mut PollView)) -> bool {
    let mut map = slot().lock();
    let Some(p) = map.get_mut(&msg.0) else { return false };
    let mut parts = (**p).clone();
    let Some(poll) = parts.poll.as_mut() else { return false };
    f(poll);
    *p = Arc::new(parts);
    true
}

#[cfg(test)]
pub fn insert(msg: Id, parts: Parts) {
    slot().lock().insert(msg.0, Arc::new(parts));
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_v1_and_v2() {
        let v = json!([
            {"type":1,"components":[
                {"type":2,"style":1,"label":"Go","custom_id":"go","emoji":{"name":"🔥"}},
                {"type":2,"style":5,"label":"Site","url":"https://x"}]},
            {"type":1,"components":[{"type":3,"custom_id":"pick","max_values":2,
                "options":[{"label":"A","value":"a"},{"label":"B","value":"b","default":true}]}]},
            {"type":17,"accent_color":255,"components":[
                {"type":10,"content":"**hi**"},
                {"type":14,"divider":false,"spacing":2},
                {"type":9,"components":[{"type":10,"content":"x"}],
                 "accessory":{"type":11,"media":{"url":"https://i/1.png"}}},
                {"type":12,"items":[{"media":{"url":"https://i/2.png"},"spoiler":true}]}]}
        ]);
        let c = list(Some(&v));
        assert!(matches!(&c[0], Comp::Row(b) if b.len() == 2));
        let Comp::Row(r) = &c[0] else { panic!() };
        assert!(matches!(&r[1], Comp::Button { style: 5, url: Some(u), .. } if u == "https://x"));
        let Comp::Row(r) = &c[1] else { panic!() };
        assert!(matches!(&r[0], Comp::Select { kind: 3, max: 2, options, .. } if options[1].default));
        let Comp::Container { accent, children, .. } = &c[2] else { panic!() };
        assert_eq!(*accent, Some(255));
        assert_eq!(children[1], Comp::Separator { divider: false, spacing: 2 });
        assert!(matches!(&children[2], Comp::Section { accessory: Some(a), .. } if matches!(**a, Comp::Thumbnail(_))));
        assert!(matches!(&children[3], Comp::Gallery(g) if g[0].spoiler));
    }

    #[test]
    fn poll_counts_and_votes() {
        let v = json!({"question":{"text":"Q?"},"allow_multiselect":false,
            "answers":[{"answer_id":1,"poll_media":{"text":"Yes"}},{"answer_id":2,"poll_media":{"text":"No"}}],
            "expiry":"2030-01-01T00:00:00+00:00",
            "results":{"is_finalized":false,"answer_counts":[{"id":1,"count":3,"me_voted":true}]}});
        let mut p = PollView::from_value(&v).unwrap();
        assert_eq!(p.total(), 3);
        assert!(p.voted());
        assert!(!p.ended(0));
        p.apply_vote(2, true, false);
        assert_eq!(p.answers[1].count, 1);
        p.apply_vote(1, false, true);
        assert!(!p.voted());
    }
}
