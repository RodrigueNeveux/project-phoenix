use phoenix_api::{
    gateway::GuildSubscription,
    rest::{profile::CustomStatus, settings::CustomStatusProto},
};

use super::{DiscordBackend, api_err, convert, self_presence::Custom};
use crate::{
    backend::{AppEvent, BoxFuture},
    model::{Id, Presence},
    presence::{Profile, profile_from_api, status_str},
    ui::modals::custom_status::StatusDraft,
};
use convert::{id, sf};

impl DiscordBackend {
    pub(super) fn people_ranges(&self, guild: Id, channel: Id, ranges: Vec<[u32; 2]>) -> BoxFuture<()> {
        self.with_session(move |_, s| async move {
            let mut sub = GuildSubscription::focused(None);
            sub.channels.insert(sf(channel), ranges);
            let _ = s.gateway.subscribe_guild(sf(guild), sub);
            Ok(())
        })
    }

    pub(super) fn people_profile(&self, user: Id, guild: Option<Id>, with_note: bool) -> BoxFuture<Profile> {
        self.with_session(move |inner, s| async move {
            let note = async {
                if with_note { s.rest.get_user_note(sf(user)).await } else { Ok(String::new()) }
            };
            let (p, note) = tokio::join!(s.rest.get_user_profile(sf(user), guild.map(sf)), note);
            let p = p.map_err(api_err)?;
            inner.emit(AppEvent::Users(vec![convert::user(&p.user)]));
            Ok(profile_from_api(&p, note.unwrap_or_default()))
        })
    }

    pub(super) fn people_note(&self, user: Id, note: String) -> BoxFuture<()> {
        self.with_session(move |_, s| async move { s.rest.set_user_note(sf(user), &note).await.map_err(api_err) })
    }

    pub(super) fn people_status(&self, status: Presence, custom: Option<String>) -> BoxFuture<()> {
        self.with_session(move |_, s| async move {
            let custom = custom.filter(|t| !t.trim().is_empty());
            {
                let mut l = s.presence.lock();
                l.set_status(status_str(status));
                if l.custom.as_ref().map(|c| c.proto.text.as_str()) != custom.as_deref() {
                    l.custom = custom.clone().map(|t| Custom { proto: CustomStatusProto { text: t, ..Default::default() }, ..Default::default() });
                }
            }
            super::self_presence::send(&s);
            let cs = custom.map(|t| CustomStatus { text: Some(t), ..Default::default() });
            s.rest.update_status_settings(Some(status_str(status)), Some(cs.as_ref())).await.map_err(api_err)
        })
    }

    pub(super) fn people_custom_status(&self, status: Presence, draft: Option<StatusDraft>) -> BoxFuture<()> {
        self.with_session(move |_, s| async move {
            {
                let mut l = s.presence.lock();
                l.set_status(status_str(status));
                l.custom = draft.as_ref().map(custom_local);
            }
            super::self_presence::send(&s);
            let proto = draft.as_ref().map(custom_status_proto);
            s.rest.save_custom_status(proto.as_ref()).await.map(|_| ()).map_err(api_err)
        })
    }

    pub(super) fn people_open_dm(&self, user: Id) -> BoxFuture<Id> {
        self.with_session(move |inner, s| async move {
            let c = s.rest.create_dm(sf(user)).await.map_err(api_err)?;
            let dm = {
                let a = s.adapter.lock();
                convert::dm_channel(&c, &a.prefs.ctx())
            };
            if let Some(dm) = dm {
                inner.emit(AppEvent::DmUpsert(dm));
            }
            Ok(id(c.id))
        })
    }
}

pub(crate) fn custom_status_proto(d: &StatusDraft) -> CustomStatusProto {
    CustomStatusProto {
        text: d.text.clone(),
        emoji_id: d.emoji.as_ref().and_then(|e| e.id).map(|i| i.0).unwrap_or(0),
        emoji_name: d.emoji.as_ref().map(|e| e.name.to_string()).unwrap_or_default(),
        expires_at_ms: d.expires_at_ms,
        created_at_ms: d.created_at_ms,
    }
}

fn custom_local(d: &StatusDraft) -> Custom {
    Custom { proto: custom_status_proto(d), label: None, animated: d.emoji.as_ref().map(|e| e.animated) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discord_backend::self_presence::Local;
    use crate::ui::modals::custom_status::{ClearAfter, StatusEmoji, draft};
    use serde_json::{Value, json};

    fn activity(d: &StatusDraft, now: u64) -> Value {
        let l = Local { status: Some("online".into()), custom: Some(custom_local(d)) };
        l.payload(now, |_| false).activities.remove(0)
    }

    #[test]
    fn custom_status_body_matches_the_client() {
        let now = 1_764_085_600_000;
        let e = StatusEmoji { id: Some(Id(42)), name: "dance".into(), animated: true };
        let d = draft("Lunch", Some(e), ClearAfter::Minutes30, now).unwrap();
        let p = custom_status_proto(&d);
        assert_eq!(
            p,
            CustomStatusProto { text: "Lunch".into(), emoji_id: 42, emoji_name: "dance".into(), expires_at_ms: now + 1_800_000, created_at_ms: now }
        );
        assert_eq!(
            activity(&d, now),
            json!({"name": "Custom Status", "type": 4, "state": "Lunch", "timestamps": {"end": now + 1_800_000}, "emoji": {"id": "42", "name": "dance", "animated": true}, "metadata": {}})
        );
        let u = draft("", Some(StatusEmoji { id: None, name: "\u{1f354}".into(), animated: false }), ClearAfter::Never, now).unwrap();
        assert_eq!(activity(&u, now), json!({"name": "Custom Status", "type": 4, "emoji": {"id": null, "name": "\u{1f354}", "animated": false}, "metadata": {}}));
        let p = custom_status_proto(&u);
        assert_eq!((p.emoji_id, p.expires_at_ms, p.text.as_str()), (0, 0, ""));
    }
}
