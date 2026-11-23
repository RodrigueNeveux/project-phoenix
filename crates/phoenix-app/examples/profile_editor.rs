//! Offscreen render of the full-screen profile editor to a PNG.
//! `cargo run -p phoenix-app --example profile_editor -- out.png [dirty]`
//! (`dirty` types into the pronouns field to show the unsaved-changes bar).
//!
//! `PHX_REAL=1` renders the user's REAL account instead of the mock (token from
//! the keyring, strictly read-only: every write the backend could do is refused,
//! so nothing is ever saved). `PHX_TAB=0|1|2` picks Board / Activity / Wishlist,
//! `PHX_WAIT` seconds to wait for the profile and images (default 12).

use std::{cell::RefCell, rc::Rc, sync::Arc, time::Duration};

use gpui::{AppContext, Entity, HeadlessAppContext, px, size};
use gpui_component::Root;
use phoenix_app::{
    app::PhoenixApp,
    assets::Assets,
    backend::{
        AppEvent, Backend, BackendError, BoxFuture, Credentials, EventSink, ReadyData, Upload,
    },
    discord_backend::{
        DiscordBackend,
        store::{KeyringStore, Secret, TokenStore},
    },
    mock::MockBackend,
    model::{Id, Message, Presence, Reaction},
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
    fn load_messages(
        &self,
        channel: Id,
        before: Option<Id>,
        limit: u32,
    ) -> BoxFuture<Vec<Message>> {
        self.0.load_messages(channel, before, limit)
    }
    fn send_message(&self, _: Id, _: String, _: Option<Id>) -> BoxFuture<Message> {
        denied()
    }
    fn send_message_ex(
        &self,
        _: Id,
        _: String,
        _: Option<(Id, bool)>,
        _: Vec<Upload>,
    ) -> BoxFuture<Message> {
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
    fn save_settings(
        &self,
        _: phoenix_app::ui::settings::SettingsPatch,
    ) -> BoxFuture<phoenix_app::ui::settings::SyncedSettings> {
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
    /// Read-only GETs (profile, application identities, widget configs).
    fn profile_widgets(&self, u: Id) -> BoxFuture<Vec<phoenix_app::ui::settings::profile_widgets::AppWidget>> {
        self.0.profile_widgets(u)
    }
    fn subscribe_member_ranges(&self, g: Id, c: Id, r: Vec<[u32; 2]>) -> BoxFuture<()> {
        self.0.subscribe_member_ranges(g, c, r)
    }
    fn guild_boosts(&self, g: Id) -> Option<(u32, bool)> {
        self.0.guild_boosts(g)
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
    // nav / parity / open_dm / notes / stickers / voice_control keep the
    // trait defaults (offline errors or no-ops): they may write.
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let out = args.next().unwrap_or_else(|| "phoenix.png".into());
    let login_only = false;
    let dirty = args.next().as_deref() == Some("dirty");
    let real = std::env::var_os("PHX_REAL").is_some();
    // `PHX_GET=/path`: print one read-only GET response (endpoint shape probe), then exit.
    if let Ok(path) = std::env::var("PHX_GET") {
        let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
        let token = KeyringStore.load().map_err(anyhow::Error::msg)?.ok_or_else(|| anyhow::anyhow!("no token"))?;
        let rest = phoenix_api::RestClient::new(token.expose(), phoenix_api::ClientProperties::linux_desktop());
        let v = rt.block_on(rest.request_json(reqwest::Method::GET, &path, None))?;
        println!("{}", serde_json::to_string_pretty(&v)?);
        return Ok(());
    }
    let tab: usize = std::env::var("PHX_TAB")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let wait: u64 = std::env::var("PHX_WAIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(12);
    if real {
        phoenix_app::discord_backend::NO_ACKS.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    let text = gpui_platform::current_platform(true).text_system();
    let mut cx = HeadlessAppContext::with_platform(text, Arc::new(Assets), || {
        gpui_platform::current_headless_renderer()
    });
    // The headless text system doesn't scan system fonts: load them explicitly.
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
    let backend: Arc<dyn Backend> = if real {
        let b = DiscordBackend::with_store(Arc::new(LoadOnlyStore));
        let http = b.http_client();
        cx.update(|cx| {
            cx.set_http_client(Arc::new(phoenix_app::media::cache::CachingHttpClient::new(
                http,
            )))
        });
        Arc::new(ReadOnly(b))
    } else {
        Arc::new(MockBackend::new())
    };
    cx.update(|cx| {
        let _ = cx.text_system().add_fonts(fonts);
        phoenix_app::init(cx);
    });
    let view: Rc<RefCell<Option<Entity<PhoenixApp>>>> = Rc::default();
    let v2 = view.clone();
    let win = cx.open_window(size(px(1280.), px(700.)), move |window, cx| {
        let v = cx.new(|cx| PhoenixApp::new(backend, window, cx));
        *v2.borrow_mut() = Some(v.clone());
        cx.new(|cx| Root::new(v, window, cx))
    })?;
    let pump = |cx: &mut HeadlessAppContext, n: usize| {
        for _ in 0..n {
            std::thread::sleep(Duration::from_millis(50));
            cx.run_until_parked();
        }
    };
    pump(&mut cx, 2);
    if !login_only {
        let v = view.borrow().clone().expect("view");
        if real {
            let mut ready = false;
            for _ in 0..60 {
                pump(&mut cx, 10);
                ready = cx.update(|cx| !v.read(cx).channels.is_empty());
                if ready {
                    break;
                }
            }
            anyhow::ensure!(ready, "no READY after 30s (token missing from keyring?)");
        } else {
            cx.update_window(win.into(), |_, window, cx| {
                v.update(cx, |app, cx| {
                    app.login_with_token = true;
                    app.login_token
                        .update(cx, |s, cx| s.set_value("mock-token", window, cx));
                    app.submit_login(window, cx);
                });
            })?;
            pump(&mut cx, 30);
        }
        cx.update_window(win.into(), |_, _, cx| {
            v.update(cx, |app, cx| {
                phoenix_app::ui::settings::open(app, cx);
                if let Some(s) = app.settings.as_mut() {
                    s.page = phoenix_app::ui::settings::Page::Profiles;
                }
            });
        })?;
        pump(&mut cx, 10);
        cx.update_window(win.into(), |_, _, cx| {
            v.update(cx, |app, cx| {
                if let Some(e) = app.settings.as_mut().and_then(|s| s.profile.as_mut()) {
                    e.tab = tab;
                }
                cx.notify();
            });
        })?;
        pump(&mut cx, if real { wait as usize * 20 } else { 2 });
        if dirty {
            cx.update_window(win.into(), |_, window, cx| {
                v.update(cx, |app, cx| {
                    let i = app
                        .settings
                        .as_ref()
                        .and_then(|s| s.profile.as_ref())
                        .and_then(|e| e.pronouns.clone());
                    if let Some(i) = i {
                        i.update(cx, |s, cx| s.set_value("il/lui", window, cx));
                    }
                });
            })?;
            pump(&mut cx, 5);
        }
    }
    cx.update_window(win.into(), |_, window, _| window.refresh())?;
    pump(&mut cx, 2);
    let _ = cx.capture_screenshot(win.into())?;
    cx.update_window(win.into(), |_, window, _| window.refresh())?;
    pump(&mut cx, 2);
    let img = cx.capture_screenshot(win.into())?;
    img.save(&out)?;
    println!("wrote {out} ({}x{})", img.width(), img.height());
    Ok(())
}
