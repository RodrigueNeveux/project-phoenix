//! Profile Board application widgets (read-only GETs), see
//! `ui::settings::profile_widgets` for the data flow.

use reqwest::Method;
use serde_json::Value;

use super::{DiscordBackend, api_err};
use crate::{
    backend::BoxFuture,
    model::Id,
    ui::settings::profile_widgets::{self as w, AppWidget},
};

impl DiscordBackend {
    pub(super) fn profile_widgets_impl(&self, user: Id) -> BoxFuture<Vec<AppWidget>> {
        self.with_session(move |_, s| async move {
            let uid = user.0;
            let prof = s.rest.request_json(Method::GET, &format!("/users/{uid}/profile"), None).await.map_err(api_err)?;
            let widgets = prof.get("widgets").and_then(Value::as_array).cloned().unwrap_or_default();
            let apps = w::app_ids(&widgets);
            if apps.is_empty() {
                return Ok(Vec::new());
            }
            let ident = s
                .rest
                .request_json(Method::GET, &format!("/users/{uid}/application-identities?with_profiles=true"), None)
                .await
                .unwrap_or(Value::Null);
            let mut out = Vec::new();
            for app in apps {
                let Ok(cfg) = s.rest.request_json(Method::GET, &format!("/applications/{app}/widget-configs"), None).await else {
                    continue;
                };
                if let Some(card) = w::build(app, &cfg, &w::identity_data(&ident, app)) {
                    out.push(card);
                }
            }
            Ok(out)
        })
    }
}
