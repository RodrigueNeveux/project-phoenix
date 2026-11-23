//! Full-screen User Settings overlay (Discord layout: left nav, content,
//! close button; Esc closes) and the settings it reads/writes.
//!
//! * [`SyncedSettings`]: account settings stored by Discord (settings-proto).
//! * [`LocalPrefs`]: client-only settings (font scale, zoom, notifications),
//!   saved as JSON in `$XDG_CONFIG_HOME/discord-phoenix/settings.json`.

mod art;
mod pages;
pub(crate) mod profile_editor;
pub mod profile_widgets;
mod view;
pub(crate) mod voice;

use std::path::PathBuf;

use gpui::{Context, SharedString, actions};
use serde_json::{Value, json};

use crate::app::PhoenixApp;

pub use view::render;

actions!(phoenix_settings, [OpenSettings]);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AppTheme {
    #[default]
    Dark,
    Light,
    Ash,
    Onyx,
}

impl AppTheme {
    pub const ALL: [AppTheme; 4] = [AppTheme::Light, AppTheme::Ash, AppTheme::Dark, AppTheme::Onyx];
    pub fn label(self) -> gpui::SharedString {
        match self {
            Self::Dark => crate::t!("settings.theme.dark"),
            Self::Light => crate::t!("settings.theme.light"),
            Self::Ash => crate::t!("settings.theme.ash"),
            Self::Onyx => crate::t!("settings.theme.onyx"),
        }
    }
    /// Preview swatch colour.
    pub fn swatch(self) -> u32 {
        match self {
            Self::Dark => 0x313338,
            Self::Light => 0xFFFFFF,
            Self::Ash => 0x2B2B30,
            Self::Onyx => 0x0C0C0E,
        }
    }
}

/// Account settings synced through Discord.
#[derive(Clone, Debug, PartialEq)]
pub struct SyncedSettings {
    pub theme: AppTheme,
    pub compact: bool,
    pub developer_mode: bool,
    pub locale: SharedString,
    /// Privacy: allow DMs from members of servers (inverse of
    /// `default_guilds_restricted`).
    pub allow_server_dms: bool,
}

impl Default for SyncedSettings {
    fn default() -> Self {
        Self {
            theme: AppTheme::Dark,
            compact: false,
            developer_mode: false,
            locale: "en-US".into(),
            allow_server_dms: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SettingsPatch {
    Theme(AppTheme),
    Compact(bool),
    DeveloperMode(bool),
    Locale(SharedString),
    AllowServerDms(bool),
}

impl SyncedSettings {
    pub fn apply(&mut self, p: &SettingsPatch) {
        match p {
            SettingsPatch::Theme(t) => self.theme = *t,
            SettingsPatch::Compact(v) => self.compact = *v,
            SettingsPatch::DeveloperMode(v) => self.developer_mode = *v,
            SettingsPatch::Locale(l) => {
                crate::i18n::set_locale(l);
                self.locale = l.clone()
            }
            SettingsPatch::AllowServerDms(v) => self.allow_server_dms = *v,
        }
    }
}

/// Client-only preferences.
#[derive(Clone, Debug, PartialEq)]
pub struct LocalPrefs {
    /// Chat font size in px (Discord: 12..=24, default 16).
    pub font_size: f32,
    /// Zoom percent (50..=200, default 100).
    pub zoom: u32,
    pub desktop_notifications: bool,
    pub notification_sounds: bool,
    pub reduced_motion: bool,
    pub saturation: u32,
    /// UI density (`--space-*` scale), default cozy like the user's client.
    pub density: crate::theme::Density,
}

impl Default for LocalPrefs {
    fn default() -> Self {
        Self {
            font_size: 16.,
            zoom: 100,
            desktop_notifications: true,
            notification_sounds: true,
            reduced_motion: false,
            saturation: 100,
            density: crate::theme::Density::default(),
        }
    }
}

fn config_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join(crate::APP_ID).join("settings.json"))
}

impl LocalPrefs {
    pub fn from_json(v: &Value) -> Self {
        let d = Self::default();
        let f = |k: &str| v.get(k).and_then(Value::as_f64);
        let b = |k: &str, def: bool| v.get(k).and_then(Value::as_bool).unwrap_or(def);
        Self {
            font_size: f("font_size").map(|x| (x as f32).clamp(12., 24.)).unwrap_or(d.font_size),
            zoom: f("zoom").map(|x| (x as u32).clamp(50, 200)).unwrap_or(d.zoom),
            desktop_notifications: b("desktop_notifications", d.desktop_notifications),
            notification_sounds: b("notification_sounds", d.notification_sounds),
            reduced_motion: b("reduced_motion", d.reduced_motion),
            saturation: f("saturation").map(|x| (x as u32).min(100)).unwrap_or(d.saturation),
            density: v.get("density").and_then(Value::as_str).and_then(crate::theme::Density::from_key).unwrap_or(d.density),
        }
    }
    pub fn to_json(&self) -> Value {
        json!({
            "font_size": self.font_size,
            "zoom": self.zoom,
            "desktop_notifications": self.desktop_notifications,
            "notification_sounds": self.notification_sounds,
            "reduced_motion": self.reduced_motion,
            "saturation": self.saturation,
            "density": self.density.key(),
        })
    }
    pub fn load() -> Self {
        config_path()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .map(|v| Self::from_json(&v))
            .unwrap_or_default()
    }
    pub fn save(&self) {
        let Some(p) = config_path() else { return };
        let data = serde_json::to_vec_pretty(&self.to_json()).unwrap_or_default();
        std::thread::spawn(move || {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let tmp = p.with_extension("json.tmp");
            if std::fs::write(&tmp, data).is_ok() {
                let _ = std::fs::rename(tmp, p);
            }
        });
    }
}

/// Everything other views may read (compact mode, font size, dev mode...).
#[derive(Clone, Debug, Default)]
pub struct Prefs {
    pub synced: SyncedSettings,
    pub local: LocalPrefs,
}

impl Prefs {
    pub fn load() -> Self {
        let local = LocalPrefs::load();
        crate::notifications::set_enabled(local.desktop_notifications);
        Self { synced: SyncedSettings::default(), local }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    MyAccount,
    Profiles,
    /// "Données et confidentialité".
    DataPrivacy,
    /// "Permissions de messagerie" (content & social).
    PrivacySafety,
    Devices,
    Connections,
    Notifications,
    Keybinds,
    Language,
    Appearance,
    Accessibility,
    VoiceVideo,
    Advanced,
    System,
    ActivityPrivacy,
    /// "Jeux enregistrés" (Games & Apps).
    RegisteredGames,
    /// Billing section (vanilla sidebar): read-only pointers to discord.com.
    Nitro,
    ServerBoost,
    Subscriptions,
    GiftInventory,
    Billing,
}

impl Page {
    pub fn label(self) -> gpui::SharedString {
        match self {
            Self::MyAccount => crate::t!("settings.page.my_account"),
            Self::Profiles => crate::t!("settings.page.profiles"),
            Self::DataPrivacy => crate::t!("settings.page.privacy"),
            Self::PrivacySafety => crate::t!("settings.page.content_social"),
            Self::System => crate::t!("settings.page.system"),
            Self::ActivityPrivacy => crate::t!("settings.page.activity"),
            Self::Devices => crate::t!("settings.page.devices"),
            Self::Connections => crate::t!("settings.page.connections"),
            Self::Notifications => crate::t!("settings.page.notifications"),
            Self::Keybinds => crate::t!("settings.page.keybinds"),
            Self::Language => crate::t!("settings.page.language"),
            Self::Appearance => crate::t!("settings.page.appearance"),
            Self::Accessibility => crate::t!("settings.page.accessibility"),
            Self::VoiceVideo => crate::t!("settings.page.voice_video"),
            Self::Advanced => crate::t!("settings.page.advanced"),
            Self::RegisteredGames => crate::t!("sp.games"),
            Self::Nitro => crate::t!("settings.page.nitro"),
            Self::ServerBoost => crate::t!("settings.page.server_boost"),
            Self::Subscriptions => crate::t!("settings.page.subscriptions"),
            Self::GiftInventory => crate::t!("settings.page.gift_inventory"),
            Self::Billing => crate::t!("settings.page.billing"),
        }
    }
}

/// Nav sections in Discord order.
/// Sidebar sections in the 2026 order (live settings-home dump): the
/// profile tab sits above them; the first section's label is visually hidden.
pub const NAV: &[(&str, &[Page])] = &[
    ("settings.nav.user", &[Page::MyAccount, Page::DataPrivacy, Page::PrivacySafety, Page::Notifications]),
    ("settings.nav.billing", &[Page::Nitro, Page::ServerBoost, Page::Subscriptions, Page::GiftInventory, Page::Billing]),
    ("settings.nav.app", &[Page::VoiceVideo, Page::Appearance, Page::Accessibility, Page::System, Page::Language]),
    ("settings.nav.apps", &[Page::RegisteredGames, Page::ActivityPrivacy, Page::Connections]),
    // last section: no label, "Développeur" then "Déconnexion"
    ("", &[Page::Advanced]),
];

#[derive(Default)]
pub struct SettingsState {
    pub page: Page,
    pub confirm_logout: bool,
    pub error: Option<SharedString>,
    pub reveal_email: bool,
    /// Session-only values of settings Phoenix doesn't back yet.
    pub toggles: std::collections::HashMap<&'static str, bool>,
    pub picks: std::collections::HashMap<&'static str, usize>,
    /// Open "Show advanced ..." expanders.
    pub expanded: std::collections::HashSet<&'static str>,
    /// Dropdown whose list is open.
    pub open_select: Option<&'static str>,
    /// Sidebar search box (created on first render).
    pub search: Option<gpui::Entity<gpui_component::input::InputState>>,
    pub search_sub: Option<gpui::Subscription>,
    /// Full-screen profile editor (Page::Profiles).
    pub profile: Option<profile_editor::EditorState>,
    /// Content panel scroll (lets offscreen shots match scrolled dumps).
    pub scroll: gpui::ScrollHandle,
}

pub fn open(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) {
    if app.screen == crate::app::Screen::Main {
        app.settings = Some(SettingsState::default());
        cx.notify();
    }
}

pub fn open_action(app: &mut PhoenixApp, _: &OpenSettings, _: &mut gpui::Window, cx: &mut Context<PhoenixApp>) {
    open(app, cx);
}

/// Esc: close the logout dialog, then the overlay. True when handled.
pub fn on_escape(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) -> bool {
    let Some(s) = app.settings.as_mut() else { return false };
    if s.page == Page::Profiles {
        profile_editor::on_escape(s);
        cx.notify();
        return true;
    }
    if s.confirm_logout {
        s.confirm_logout = false;
    } else if s.open_select.is_some() {
        s.open_select = None;
    } else {
        app.settings = None;
    }
    cx.notify();
    true
}

/// After login: fetch the synced settings.
pub fn on_ready(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) {
    let fut = app.backend.load_settings();
    cx.spawn(async move |this, cx| {
        let res = fut.await;
        let _ = this.update(cx, |this, cx| {
            if let Ok(s) = res {
                crate::i18n::set_locale(&s.locale);
                this.prefs.synced = s;
                cx.notify();
            }
        });
    })
    .detach();
}

/// Optimistically apply then save a synced setting; reverts on failure.
pub fn save(app: &mut PhoenixApp, patch: SettingsPatch, cx: &mut Context<PhoenixApp>) {
    let before = app.prefs.synced.clone();
    app.prefs.synced.apply(&patch);
    let fut = app.backend.save_settings(patch);
    cx.notify();
    cx.spawn(async move |this, cx| {
        let res = fut.await;
        let _ = this.update(cx, |this, cx| {
            match res {
                Ok(s) => {
                    crate::i18n::set_locale(&s.locale);
                    this.prefs.synced = s
                }
                Err(e) => {
                    crate::i18n::set_locale(&before.locale);
                    this.prefs.synced = before;
                    if let Some(st) = this.settings.as_mut() {
                        st.error = Some(e.0.into());
                    }
                }
            }
            cx.notify();
        });
    })
    .detach();
}

pub fn update_local(app: &mut PhoenixApp, f: impl FnOnce(&mut LocalPrefs), cx: &mut Context<PhoenixApp>) {
    f(&mut app.prefs.local);
    crate::notifications::set_enabled(app.prefs.local.desktop_notifications);
    app.prefs.local.save();
    cx.notify();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_prefs_roundtrip_and_clamp() {
        let p = LocalPrefs { font_size: 20., zoom: 125, desktop_notifications: false, ..Default::default() };
        assert_eq!(LocalPrefs::from_json(&p.to_json()), p);
        let c = LocalPrefs::from_json(&json!({"font_size": 99, "zoom": 10}));
        assert_eq!((c.font_size, c.zoom), (24., 50));
        assert_eq!(LocalPrefs::from_json(&json!(null)), LocalPrefs::default());
    }

    #[test]
    fn patches_apply() {
        let mut s = SyncedSettings::default();
        s.apply(&SettingsPatch::Theme(AppTheme::Onyx));
        s.apply(&SettingsPatch::DeveloperMode(true));
        assert_eq!(s.theme, AppTheme::Onyx);
        assert!(s.developer_mode);
    }
}
