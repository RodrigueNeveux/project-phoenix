//! Offscreen render of Phoenix on the user's REAL account, strictly read-only.
//!
//! `cargo run -p phoenix-app --example real_shots -- out.png`
//!
//! The token comes from the keyring (never printed, never rewritten). The
//! backend is wrapped so nothing can be sent, acked, typed, reacted, joined or
//! changed: only READY, channel selection, message history, member list
//! subscription and media downloads go out.
//!
//! Location: env `PHX_GUILD` + `PHX_CHANNEL`, or `PHX_DM=<channel id>`, or by
//! default the location shown by the user's Discord (`cdp.mjs eval location.pathname`).
//! Size / theme / density: env `PHX_W`, `PHX_H`, `PHX_THEME` (dark|light|ash|onyx),
//! `PHX_COMPACT`, otherwise read from the live Discord through CDP.
//! `PHX_AUDIO` = two digits mute/deaf (e.g. `11`), otherwise read from the live user panel.
//! `PHX_WAIT` = seconds to wait for messages and images (default 15).

use std::{cell::RefCell, rc::Rc, sync::Arc, time::Duration};

use gpui::{AppContext, Entity, HeadlessAppContext, px, size};
use gpui_component::Root;
use phoenix_app::{
    app::PhoenixApp,
    assets::Assets,
    backend::{AppEvent, Backend, BackendError, BoxFuture, Credentials, EventSink, ReadyData, Upload},
    discord_backend::{
        DiscordBackend,
        store::{KeyringStore, Secret, TokenStore},
    },
    model::{Id, Message, Presence, Reaction},
    ui::settings::AppTheme,
};

/// Keyring store that can load but never save or delete.
struct LoadOnlyStore;

impl TokenStore for LoadOnlyStore {
    fn load(&self) -> Result<Option<Secret>, String> {
        KeyringStore.load()
    }
    fn save(&self, _: &Secret) -> Result<(), String> {
        Ok(())
    }
    fn clear(&self) -> Result<(), String> {
        Ok(())
    }
}

fn denied<T: Send + 'static>() -> BoxFuture<T> {
    Box::pin(async { Err(BackendError("read-only capture".into())) })
}

/// Delegates reads to the real backend; every write is refused.
struct ReadOnly(DiscordBackend);

impl Backend for ReadOnly {
    fn set_event_sink(&self, sink: EventSink) {
        // Drop events that would make the app answer with a write (none today, but keep the hook).
        self.0.set_event_sink(Arc::new(move |e: AppEvent| sink(e)));
    }
    fn login(&self, _: Credentials) -> BoxFuture<ReadyData> {
        denied()
    }
    fn restore_session(&self) -> Option<BoxFuture<ReadyData>> {
        self.0.restore_session()
    }
    fn select_guild(&self, guild: Option<Id>) -> BoxFuture<()> {
        self.0.select_guild(guild)
    }
    fn select_channel(&self, channel: Id) -> BoxFuture<()> {
        self.0.select_channel(channel) // acks disabled via NO_ACKS
    }
    fn load_messages(&self, channel: Id, before: Option<Id>, limit: u32) -> BoxFuture<Vec<Message>> {
        self.0.load_messages(channel, before, limit)
    }
    fn send_message(&self, _: Id, _: String, _: Option<Id>) -> BoxFuture<Message> {
        denied()
    }
    fn send_message_ex(&self, _: Id, _: String, _: Option<(Id, bool)>, _: Vec<Upload>) -> BoxFuture<Message> {
        denied()
    }
    fn edit_message(&self, _: Id, _: Id, _: String) -> BoxFuture<Message> {
        denied()
    }
    fn delete_message(&self, _: Id, _: Id) -> BoxFuture<()> {
        denied()
    }
    fn trigger_typing(&self, _: Id) -> BoxFuture<()> {
        denied()
    }
    fn join_voice(&self, _: Option<Id>) -> BoxFuture<()> {
        denied()
    }
    fn set_reaction(&self, _: Id, _: Id, _: Reaction, _: bool) -> BoxFuture<()> {
        denied()
    }
    fn set_pinned(&self, _: Id, _: Id, _: bool) -> BoxFuture<()> {
        denied()
    }
    fn mark_unread(&self, _: Id, _: Id) -> BoxFuture<()> {
        denied()
    }
    fn save_settings(&self, _: phoenix_app::ui::settings::SettingsPatch) -> BoxFuture<phoenix_app::ui::settings::SyncedSettings> {
        denied()
    }
    fn set_status(&self, _: Presence, _: Option<String>) -> BoxFuture<()> {
        denied()
    }
    fn logout(&self) -> BoxFuture<()> {
        denied()
    }
    // Reads.
    fn notify_prefs(&self) -> Option<phoenix_app::notifications::NotifyPrefs> {
        self.0.notify_prefs()
    }
    fn is_media_channel(&self, c: Id) -> bool {
        self.0.is_media_channel(c)
    }
    fn load_settings(&self) -> BoxFuture<phoenix_app::ui::settings::SyncedSettings> {
        self.0.load_settings()
    }
    fn guild_emojis(&self, g: Id) -> Vec<phoenix_app::ui::pickers::types::GuildEmoji> {
        self.0.guild_emojis(g)
    }
    fn has_premium(&self) -> bool {
        self.0.has_premium()
    }
    /// Read-only `GET /users/{id}/profile` (DM profile panel, popouts).
    fn user_profile(&self, u: Id, g: Option<Id>) -> BoxFuture<phoenix_app::presence::Profile> {
        self.0.user_profile(u, g)
    }
    fn subscribe_member_ranges(&self, g: Id, c: Id, r: Vec<[u32; 2]>) -> BoxFuture<()> {
        self.0.subscribe_member_ranges(g, c, r)
    }
    fn guild_boosts(&self, g: Id) -> Option<(u32, bool)> {
        self.0.guild_boosts(g)
    }
    fn channel_user_limit(&self, c: Id) -> Option<u32> {
        self.0.channel_user_limit(c)
    }
    fn guild_features(&self, g: Id) -> Vec<String> {
        self.0.guild_features(g)
    }
    fn guild_permissions(&self, g: Id) -> u64 {
        self.0.guild_permissions(g)
    }
    fn last_read(&self, c: Id) -> Option<Id> {
        self.0.last_read(c)
    }
    fn forum_tags(&self, f: Id) -> Vec<phoenix_app::commands::ForumTag> {
        self.0.forum_tags(f)
    }
    /// Only the forum post listing (a read-only thread search) goes through.
    fn parity(&self, req: phoenix_app::commands::ParityRequest) -> BoxFuture<phoenix_app::commands::ParityReply> {
        match req {
            phoenix_app::commands::ParityRequest::ForumPosts { .. } => self.0.parity(req),
            _ => denied(),
        }
    }
    // nav / other parity / open_dm / notes / stickers / voice_control keep the
    // trait defaults (offline errors or no-ops): they may write.
}

fn cdp(expr: &str) -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let out = std::process::Command::new("node")
        .arg(format!("{home}/discord-phoenix-ref/cdp.mjs"))
        .args(["eval", expr])
        .output()
        .ok()?;
    let s = String::from_utf8(out.stdout).ok()?;
    Some(s.trim().trim_matches('"').to_string())
}

fn env_or_cdp(var: &str, expr: &str) -> Option<String> {
    std::env::var(var).ok().filter(|v| !v.is_empty()).or_else(|| cdp(expr))
}

fn parse_id(s: &str) -> Option<Id> {
    s.parse().ok().map(Id)
}

fn main() -> anyhow::Result<()> {
    let out = std::env::args().nth(1).unwrap_or_else(|| "real.png".into());
    phoenix_app::discord_backend::NO_ACKS.store(true, std::sync::atomic::Ordering::Relaxed);

    // Where: explicit env, else what the user's Discord shows.
    let (guild, channel) = if let Some(dm) = std::env::var("PHX_DM").ok().and_then(|v| parse_id(&v)) {
        (None, Some(dm))
    } else if let Some(c) = std::env::var("PHX_CHANNEL").ok().and_then(|v| parse_id(&v)) {
        (std::env::var("PHX_GUILD").ok().and_then(|v| parse_id(&v)), Some(c))
    } else {
        let path = cdp("location.pathname").unwrap_or_default();
        let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        match parts.as_slice() {
            ["channels", "@me", c, ..] => (None, parse_id(c)),
            ["channels", g, c, ..] => (parse_id(g), parse_id(c)),
            _ => (None, None),
        }
    };
    let w: f32 = env_or_cdp("PHX_W", "innerWidth").and_then(|v| v.parse().ok()).unwrap_or(1280.);
    let h: f32 = env_or_cdp("PHX_H", "innerHeight").and_then(|v| v.parse().ok()).unwrap_or(800.);
    let classes = cdp("document.documentElement.className").unwrap_or_default();
    let theme = match std::env::var("PHX_THEME").ok().as_deref() {
        Some("light") => AppTheme::Light,
        Some("ash") => AppTheme::Ash,
        Some("onyx") => AppTheme::Onyx,
        Some(_) => AppTheme::Dark,
        None if classes.contains("theme-midnight") => AppTheme::Onyx,
        None if classes.contains("theme-darker") => AppTheme::Ash,
        None if classes.contains("theme-light") => AppTheme::Light,
        None => AppTheme::Dark,
    };
    let compact = std::env::var_os("PHX_COMPACT").is_some() || classes.contains("density-compact");
    // Self mute / deafen are client-side state in Discord (not on the account):
    // read the user panel switches (`audioButtonWithMenu_` role=switch, aria-checked).
    let audio = env_or_cdp(
        "PHX_AUDIO",
        "[...document.querySelectorAll('button[role=switch][class*=audioButtonWithMenu]')].map(b=>b.getAttribute('aria-checked')==='true'?1:0).join('')",
    )
    .unwrap_or_default();
    let (self_mute, self_deaf) = (audio.starts_with('1'), audio.get(1..2) == Some("1"));
    // Channel list width the user dragged to (sidebarList_ box), so compares line up.
    if std::env::var_os("PHX_SIDEBAR_W").is_none()
        && let Some(sw) = cdp("(document.querySelector('[class*=sidebarList_]')?.getBoundingClientRect().width||'')+''")
        && sw.parse::<f32>().is_ok()
    {
        // SAFETY: single-threaded, before any other thread starts.
        unsafe { std::env::set_var("PHX_SIDEBAR_W", sw) };
    }
    let wait: u64 = std::env::var("PHX_WAIT").ok().and_then(|v| v.parse().ok()).unwrap_or(15);
    println!("target guild={guild:?} channel={channel:?} size={w}x{h} theme={theme:?} compact={compact} mute={self_mute} deaf={self_deaf}");

    let text = gpui_platform::current_platform(true).text_system();
    let mut cx = HeadlessAppContext::with_platform(text, Arc::new(Assets), || {
        gpui_platform::current_headless_renderer()
    });
    let fonts: Vec<std::borrow::Cow<'static, [u8]>> = [
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Medium.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-SemiBold.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Italic.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    ]
    .iter()
    .filter_map(|p| std::fs::read(p).ok().map(Into::into))
    .collect();
    let backend = DiscordBackend::with_store(Arc::new(LoadOnlyStore));
    let http = backend.http_client();
    cx.update(|cx| {
        let _ = cx.text_system().add_fonts(fonts);
        cx.set_http_client(Arc::new(phoenix_app::media::cache::CachingHttpClient::new(http)));
        phoenix_app::init(cx);
    });
    let backend: Arc<dyn Backend> = Arc::new(ReadOnly(backend));
    let view: Rc<RefCell<Option<Entity<PhoenixApp>>>> = Rc::default();
    let v2 = view.clone();
    let win = cx.open_window(size(px(w), px(h)), move |window, cx| {
        let v = cx.new(|cx| PhoenixApp::new(backend, window, cx));
        *v2.borrow_mut() = Some(v.clone());
        cx.new(|cx| Root::new(v, window, cx))
    })?;
    let pump = |cx: &mut HeadlessAppContext, ms: u64| {
        for _ in 0..ms / 50 {
            std::thread::sleep(Duration::from_millis(50));
            cx.run_until_parked();
        }
    };
    let v = view.borrow().clone().expect("view");
    // Wait for READY (restore_session from the keyring).
    let mut ready = false;
    for _ in 0..60 {
        pump(&mut cx, 500);
        ready = cx.update(|cx| !v.read(cx).channels.is_empty());
        if ready {
            break;
        }
    }
    anyhow::ensure!(ready, "no READY after 30s (token missing from keyring?)");
    cx.update_window(win.into(), |_, _, cx| {
        v.update(cx, |app, cx| {
            app.prefs.synced.theme = theme;
            app.prefs.local.zoom = 100;
            app.prefs.synced.compact = compact;
            // In-memory only: the read-only capture never writes voice.json.
            app.mic_muted = self_mute || self_deaf;
            app.deafened = self_deaf;
            if let Some(c) = channel {
                let g = guild.or_else(|| app.channels.get(&c).and_then(|ch| ch.guild));
                app.select_guild(g, cx);
                app.select_channel(c, cx);
            }
            cx.notify();
        });
    })?;
    pump(&mut cx, wait * 1000);
    let mut img = None;
    for _ in 0..2 {
        cx.update_window(win.into(), |_, window, _| window.refresh())?;
        pump(&mut cx, 200);
        img = Some(cx.capture_screenshot(win.into())?);
    }
    let img = img.expect("frame");
    img.save(&out)?;
    println!("wrote {out} ({}x{})", img.width(), img.height());
    Ok(())
}
