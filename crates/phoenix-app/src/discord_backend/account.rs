//! Account features of the real backend: MFA methods, QR (remote auth)
//! login, synced settings and desktop notifications.

use phoenix_api::{
    remote_auth::{self, RemoteAuthEvent},
    rest::settings::{self as api_settings, PreloadedUserSettings, SettingChange, Theme},
};

use super::*;
use crate::{
    auth::{MfaMethod, MfaOptions, RemoteAuthUpdate},
    ui::settings::{AppTheme, SettingsPatch, SyncedSettings},
};

/// Password login answered with an MFA ticket.
pub(super) fn mfa_required(inner: &Inner, ticket: String, totp: bool, sms: bool, backup: bool) -> Result<ReadyData, BackendError> {
    if !totp && !sms && !backup {
        return Err(err(
            "This account only allows a security key for 2FA, which Phoenix can't use. Log in with the QR code or a token instead.",
        ));
    }
    *inner.mfa_ticket.lock() = Some(Secret::new(ticket));
    *inner.mfa_opts.lock() = MfaOptions { totp, sms, backup };
    Err(err(if totp {
        "Enter the 6-digit code from your authenticator app."
    } else {
        "Choose how to receive your login code below."
    }))
}

impl DiscordBackend {
    pub(super) fn mfa_options_impl(&self) -> MfaOptions {
        *self.inner.mfa_opts.lock()
    }

    pub(super) fn login_mfa_impl(&self, method: MfaMethod, code: String) -> BoxFuture<ReadyData> {
        self.spawn(move |inner| async move {
            let ticket = inner.mfa_ticket.lock().clone().ok_or_else(|| err("Log in with your password first."))?;
            let code: String = code.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();
            if code.is_empty() {
                return Err(err("Enter your code."));
            }
            let rest = RestClient::unauthenticated(inner.props.clone());
            let token = match method {
                MfaMethod::Totp => rest.login_mfa_totp(ticket.expose(), &code).await,
                MfaMethod::Backup => rest.login_mfa_backup(ticket.expose(), &code).await,
                MfaMethod::Sms => rest.login_mfa_sms(ticket.expose(), &code).await,
            }
            .map_err(api_err)?;
            inner.mfa_ticket.lock().take();
            connect(inner, Secret::new(token), true).await
        })
    }

    pub(super) fn send_mfa_sms_impl(&self) -> BoxFuture<String> {
        self.spawn(move |inner| async move {
            let ticket = inner.mfa_ticket.lock().clone().ok_or_else(|| err("Log in with your password first."))?;
            let rest = RestClient::unauthenticated(inner.props.clone());
            rest.login_mfa_sms_send(ticket.expose()).await.map_err(api_err)
        })
    }

    pub(super) fn cancel_remote_auth_impl(&self) {
        if let Some(h) = self.inner.remote_auth.lock().take() {
            h.abort();
        }
    }

    pub(super) fn remote_auth_impl(&self) -> BoxFuture<ReadyData> {
        self.cancel_remote_auth_impl();
        self.spawn(move |inner| async move {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let rest = RestClient::unauthenticated(inner.props.clone());
            let task = tokio::spawn(remote_auth::run(rest, tx));
            *inner.remote_auth.lock() = Some(task.abort_handle());
            while let Some(ev) = rx.recv().await {
                match ev {
                    RemoteAuthEvent::Qr(url) => inner.emit(AppEvent::RemoteAuth(RemoteAuthUpdate::Qr(url))),
                    RemoteAuthEvent::Scanned { username, .. } => {
                        inner.emit(AppEvent::RemoteAuth(RemoteAuthUpdate::Scanned { username }))
                    }
                    RemoteAuthEvent::Token(t) => {
                        inner.remote_auth.lock().take();
                        return connect(inner, Secret::new(t), true).await;
                    }
                    RemoteAuthEvent::Cancelled => return Err(err("")),
                }
            }
            match task.await {
                Ok(Err(e)) => Err(err(format!("Couldn't reach Discord's QR login ({}).", short(&e)))),
                _ => Err(err("")),
            }
        })
    }

    pub(super) fn load_settings_impl(&self) -> BoxFuture<SyncedSettings> {
        self.with_session(move |_, s| async move {
            match s.rest.get_settings_proto().await {
                Ok(p) => Ok(synced(&p)),
                Err(_) => {
                    let v = s.rest.get_settings_json().await.map_err(api_err)?;
                    Ok(synced(&api_settings::from_json(&v)))
                }
            }
        })
    }

    pub(super) fn save_settings_impl(&self, patch: SettingsPatch) -> BoxFuture<SyncedSettings> {
        self.with_session(move |_, s| async move {
            let change = match &patch {
                SettingsPatch::Theme(t) => SettingChange::Theme(theme_to_api(*t)),
                SettingsPatch::Compact(b) => SettingChange::Compact(*b),
                SettingsPatch::DeveloperMode(b) => SettingChange::DeveloperMode(*b),
                SettingsPatch::Locale(l) => SettingChange::Locale(l.to_string()),
                SettingsPatch::AllowServerDms(b) => SettingChange::DefaultGuildsRestricted(!*b),
            };
            let p = s.rest.patch_setting(&change).await.map_err(api_err)?;
            Ok(synced(&p))
        })
    }
}

fn short(e: &phoenix_api::Error) -> String {
    let s = e.to_string();
    s.chars().take(120).collect()
}

fn theme_to_api(t: AppTheme) -> Theme {
    match t {
        AppTheme::Dark => Theme::Dark,
        AppTheme::Light => Theme::Light,
        AppTheme::Ash => Theme::Ash,
        AppTheme::Onyx => Theme::Onyx,
    }
}

pub fn synced(p: &PreloadedUserSettings) -> SyncedSettings {
    let d = SyncedSettings::default();
    crate::ui::settings::account::set_activity_restricted(p.privacy.as_ref().map(|x| x.activity_restricted_guild_ids.clone()).unwrap_or_default());
    SyncedSettings {
        theme: match p.theme() {
            Theme::Dark => AppTheme::Dark,
            Theme::Light => AppTheme::Light,
            Theme::Ash => AppTheme::Ash,
            Theme::Onyx => AppTheme::Onyx,
        },
        compact: p.compact(),
        developer_mode: p.developer_mode(),
        locale: p.locale().filter(|l| !l.is_empty()).map(|l| l.to_owned().into()).unwrap_or(d.locale),
        allow_server_dms: !p.default_guilds_restricted(),
    }
}

/// Gateway hook (pump): desktop notifications + notification settings updates.
pub(super) fn on_gateway(s: &Session, ev: &GatewayEvent) {
    match ev {
        GatewayEvent::MessageCreate(m) => {
            let me = s.adapter.lock().me;
            let viewing = *s.selected_channel.lock();
            crate::notifications::on_message(&s.notify.lock(), &s.cache, me, viewing, m);
        }
        GatewayEvent::Unknown { name, data } if name == "USER_GUILD_SETTINGS_UPDATE" => s.notify.lock().update(data),
        _ => {}
    }
}

impl DiscordBackend {
    /// `GET /auth/sessions`: number of logged-in devices (Settings > Account).
    pub(super) fn session_count_impl(&self) -> BoxFuture<Option<usize>> {
        self.with_session(move |_, s| async move {
            let v = s.rest.request_json(reqwest::Method::GET, "/auth/sessions", None).await.map_err(api_err)?;
            Ok(v.get("user_sessions").and_then(serde_json::Value::as_array).map(Vec::len))
        })
    }
}

impl DiscordBackend {
    /// Settings > Billing: three read-only GETs.
    pub(super) fn billing_impl(&self) -> BoxFuture<Option<crate::ui::settings::billing::Billing>> {
        use crate::ui::settings::billing as b;
        self.with_session(move |_, s| async move {
            let get = |p: &'static str| {
                let rest = s.rest.clone();
                async move { rest.request_json(reqwest::Method::GET, p, None).await.ok() }
            };
            let sources = get("/users/@me/billing/payment-sources").await.map(|v| b::parse_sources(&v)).unwrap_or_default();
            let payments = get("/users/@me/billing/payments?limit=10").await.map(|v| b::parse_payments(&v)).unwrap_or_default();
            let country = get("/users/@me/billing/country-code")
                .await
                .and_then(|v| v.get("country_code").and_then(serde_json::Value::as_str).map(str::to_owned));
            Ok(Some(b::Billing { country, sources, payments }))
        })
    }
}

impl DiscordBackend {
    /// Settings > Connections: `GET /oauth2/tokens` (read-only).
    pub(super) fn authorized_apps_impl(&self) -> BoxFuture<Option<Vec<crate::ui::settings::authorized_apps::AuthorizedApp>>> {
        self.with_session(move |_, s| async move {
            let v = s.rest.request_json(reqwest::Method::GET, "/oauth2/tokens", None).await.map_err(api_err)?;
            Ok(Some(crate::ui::settings::authorized_apps::parse(&v)))
        })
    }
}
