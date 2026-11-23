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
    /// Only the forum post listing and the content inventory (read-only GETs) go through.
    fn parity(&self, req: phoenix_app::commands::ParityRequest) -> BoxFuture<phoenix_app::commands::ParityReply> {
        match req {
            phoenix_app::commands::ParityRequest::ForumPosts { .. } => {
                let what = format!("{req:?}");
                let fut = self.0.parity(req);
                Box::pin(async move {
                    let r = fut.await;
                    if let Err(e) = &r {
                        eprintln!("real_shots: {what} failed: {}", e.0);
                    }
                    r
                })
            }
            // GET /content-inventory/users/@me (member list Activity feed).
            phoenix_app::commands::ParityRequest::ContentInventory => self.0.parity(req),
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

/// Live scroll state of the user's Discord (read-only `scrollTop` reads).
#[derive(Debug, Default)]
struct Scroll {
    /// Channel list (`sidebarList_` scroller) scrollTop, CSS px.
    channels: f32,
    /// First channel-list row (category / channel / thread id) still in view,
    /// and how far the scroller's top edge sits inside its `li`.
    top_channel: Option<(Id, f32)>,
    /// Member list (`membersWrap_` scroller) scrollTop, CSS px.
    members: f32,
    /// First message whose bottom is below the top of the message scroller,
    /// and how far (CSS px) the scroller's top edge sits inside it.
    top_message: Option<(Id, f32)>,
    /// Message scroller distance from the bottom (fallback when the anchor
    /// message is not in our loaded history).
    from_bottom: f32,
}

const SCROLL_JS: &str = r"(()=>{const q=s=>document.querySelector(s);
const ch=q('[class*=sidebarList_] [class*=scroller_]'),mb=q('[class*=membersWrap_] [class*=scroller_]'),
ms=q('ol[data-list-id=chat-messages]')?.closest('[class*=scroller_]')||q('[class*=messagesWrapper_] > [class*=scroller_]');let top='',off=0,fb=0,ct='',co=0;
if(ch){const r=ch.getBoundingClientRect();for(const e of ch.querySelectorAll('[data-list-item-id^=channels___]')){
const b=(e.closest('li')||e).getBoundingClientRect();if(b.bottom>r.top+1){ct=e.getAttribute('data-list-item-id').split('___').pop();co=r.top-b.top;break}}}
if(ms){const r=ms.getBoundingClientRect();fb=ms.scrollHeight-ms.scrollTop-ms.clientHeight;
for(const li of ms.querySelectorAll('li[id^=chat-messages-]')){const b=li.getBoundingClientRect();
if(b.bottom>r.top+1){top=li.id.split('-').pop();off=r.top-b.top;break}}}
return [ch?ch.scrollTop:0,mb?mb.scrollTop:0,top,off,fb,ct,co].join(' ')})()";

fn read_scroll() -> Option<Scroll> {
    if std::env::var_os("PHX_NO_SCROLL").is_some() {
        return None;
    }
    let raw = cdp(SCROLL_JS)?;
    let f: Vec<&str> = raw.split(' ').collect();
    let num = |i: usize| f.get(i).and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.);
    Some(Scroll {
        channels: num(0),
        top_channel: f.get(5).and_then(|v| parse_id(v)).map(|id| (id, num(6))),
        members: num(1),
        top_message: f.get(2).and_then(|v| parse_id(v)).map(|id| (id, num(3))),
        from_bottom: num(4),
    })
}

/// Scrolls our three lists to the Discord offsets. Pixel offsets are replayed
/// in viewport-sized steps with a frame in between, because `ListState`
/// only knows the heights of items it has laid out.
fn apply_scroll(
    cx: &mut HeadlessAppContext,
    win: gpui::AnyWindowHandle,
    v: &Entity<PhoenixApp>,
    sc: &Scroll,
    pump: &dyn Fn(&mut HeadlessAppContext, u64),
) -> anyhow::Result<()> {
    use gpui::ListOffset;
    let top = ListOffset { item_ix: 0, offset_in_item: px(0.) };
    // Message list: anchor on the same message, else same distance from the bottom.
    let anchor = cx.update(|cx| {
        let app = v.read(cx);
        let (id, off) = sc.top_message?;
        let ix = app.messages.get(&app.selected_channel?)?.index_of(id)?;
        Some((ix + 1, off)) // item 0 is the channel header / loader
    });
    // Channel list: anchor on the same row (item 0 is the guild banner / top spacer).
    let ch_anchor = cx.update(|cx| {
        use phoenix_app::app::SidebarRow::*;
        let (id, off) = sc.top_channel?;
        let ix = v.read(cx).sidebar_rows.iter().position(|r| matches!(r, Category(c) | Channel(c) | Thread(c) if *c == id))?;
        Some((ix + 1, off))
    });
    cx.update(|cx| {
        let app = v.read(cx);
        match ch_anchor {
            Some((ix, off)) => app.sidebar_list.scroll_to(ListOffset { item_ix: ix, offset_in_item: px(off.max(0.)) }),
            None => app.sidebar_list.scroll_to(top),
        }
        app.member_list.scroll_to(top);
        match anchor {
            Some((ix, off)) => app.message_list.scroll_to(ListOffset { item_ix: ix, offset_in_item: px(off.max(0.)) }),
            None => app.message_list.scroll_to_end(),
        }
    });
    if anchor.is_none() {
        eprintln!("top message not in loaded history, using distance from bottom");
    }
    let mut left = [if ch_anchor.is_none() { sc.channels } else { 0. }, sc.members, if anchor.is_none() { sc.from_bottom } else { 0. }];
    while left.iter().any(|l| *l > 0.5) {
        cx.update_window(win, |_, window, _| window.refresh())?;
        pump(cx, 100);
        cx.update(|cx| {
            let app = v.read(cx);
            let lists = [&app.sidebar_list, &app.member_list, &app.message_list];
            for (i, l) in lists.into_iter().enumerate() {
                let step = left[i].min(300.);
                if step > 0. {
                    // Messages scroll up from the bottom, the other lists down from the top.
                    l.scroll_by(px(if i == 2 { -step } else { step }));
                    left[i] -= step;
                }
            }
        });
    }
    Ok(())
}

/// Window height while READY and the history load (see `open_window`).
const LOAD_H: f32 = 800.;

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
    let scroll = read_scroll();
    println!("scroll {scroll:?}");
    // Open guild folders are client-side state in Discord: read the rail's
    // `aria-expanded` folder buttons (`data-list-item-id=guildsnav___<folder id>`).
    let open_folders: Vec<u64> = env_or_cdp(
        "PHX_FOLDERS",
        "[...document.querySelectorAll('nav[class*=guilds_] [aria-expanded=true][data-list-item-id^=guildsnav___]')].map(e=>e.getAttribute('data-list-item-id').slice(12)).join(',')",
    )
    .unwrap_or_default()
    .split(',')
    .filter_map(|s| s.trim().parse().ok())
    .collect();
    let wait: u64 = std::env::var("PHX_WAIT").ok().and_then(|v| v.parse().ok()).unwrap_or(15);
    println!("target guild={guild:?} channel={channel:?} size={w}x{h} theme={theme:?} compact={compact} mute={self_mute} deaf={self_deaf}");

    let t0 = std::time::Instant::now();
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
    // Load small, capture big: in headless (test) mode gpui draws the window on
    // every effect flush, i.e. once per gateway event. READY and the presence
    // storm that follows meant thousands of full-size draws, so tall captures
    // (PHX_H=1600+) took minutes. The real size is applied after READY.
    let win = cx.open_window(size(px(w), px(h.min(LOAD_H))), move |window, cx| {
        let v = cx.new(|cx| PhoenixApp::new(backend, window, cx));
        *v2.borrow_mut() = Some(v.clone());
        cx.new(|cx| Root::new(v, window, cx))
    })?;
    // Time-based, not iteration-based: at tall sizes one parked run can draw a
    // large frame (animated avatars/GIFs keep requesting frames), so a fixed
    // `ms / 50` iteration count stretched a 15 s wait to minutes.
    let pump = |cx: &mut HeadlessAppContext, ms: u64| {
        let end = std::time::Instant::now() + Duration::from_millis(ms);
        while std::time::Instant::now() < end {
            std::thread::sleep(Duration::from_millis(50));
            let t = std::time::Instant::now();
            cx.run_until_parked();
            if std::env::var_os("PHX_TRACE").is_some() && t.elapsed() > Duration::from_millis(300) {
                eprintln!("slow parked run: {:?}", t.elapsed());
            }
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
    eprintln!("[{:.1}s] ready", t0.elapsed().as_secs_f32());
    anyhow::ensure!(ready, "no READY after 30s (token missing from keyring?)");
    // `PHX_LIST=1`: print DM / group DM ids and the open threads of the
    // current guild (for picking capture targets), then exit.
    if std::env::var_os("PHX_LIST").is_some() {
        cx.update(|cx| {
            use phoenix_app::app::SidebarRow;
            let app = v.read(cx);
            for d in &app.dms {
                // A 1:1 DM is named after its recipient; anything else is a group.
                let group = app.channels.get(d).is_some_and(|c| {
                    c.voice_users.first().and_then(|u| app.users.get(u)).is_none_or(|u| u.display_name != c.name)
                });
                println!("dm {} group={group}", d.0);
            }
            for r in &app.sidebar_rows {
                if let SidebarRow::Thread(t) = r {
                    println!("thread {} guild={:?}", t.0, app.selected_guild.map(|g| g.0));
                }
            }
        });
        return Ok(());
    }
    cx.update_window(win.into(), |_, _, cx| {
        v.update(cx, |app, cx| {
            app.prefs.synced.theme = theme;
            app.prefs.local.zoom = 100;
            app.prefs.synced.compact = compact;
            // In-memory only: the read-only capture never writes voice.json.
            app.mic_muted = self_mute || self_deaf;
            app.deafened = self_deaf;
            app.nav.expanded_folders.clear();
            app.nav.expanded_folders.extend(open_folders.iter().copied());
            if let Some(c) = channel {
                let g = guild.or_else(|| app.channels.get(&c).and_then(|ch| ch.guild));
                app.select_guild(g, cx);
                app.select_channel(c, cx);
            }
            cx.notify();
        });
    })?;
    pump(&mut cx, wait * 1000 / 2);
    // Full size for the rest: images in view start loading now.
    cx.update_window(win.into(), |_, window, cx| {
        window.resize(size(px(w), px(h)));
        window.bounds_changed(cx);
    })?;
    pump(&mut cx, wait * 1000 / 2);
    // Server nicks of authors outside the synced member-list range come from op 8
    // (GUILD_MEMBERS_CHUNK): wait until every author of the page has a guild member
    // (members who left the guild never get one, hence the timeout).
    let missing = |cx: &mut HeadlessAppContext| {
        cx.update(|cx| {
            let app = v.read(cx);
            let (Some(g), Some(c)) = (app.selected_guild, app.selected_channel) else { return 0 };
            let members = app.members.get(&g);
            let mut authors: Vec<Id> = app.messages.get(&c).map(|m| m.msgs.iter().map(|m| m.author).collect()).unwrap_or_default();
            authors.sort_unstable_by_key(|a| a.0);
            authors.dedup();
            authors.iter().filter(|a| !members.is_some_and(|ms| ms.iter().any(|m| m.user == **a))).count()
        })
    };
    for _ in 0..20 {
        if missing(&mut cx) == 0 {
            break;
        }
        pump(&mut cx, 500);
    }
    println!("authors without guild member: {}", missing(&mut cx));
    if std::env::var("PHX_DEBUG_NICKS").is_ok() {
        cx.update(|cx| {
            let app = v.read(cx);
            for ch in app.channels.values().filter(|c| !c.voice_users.is_empty() && c.guild == app.selected_guild) {
                let names: Vec<_> = ch.voice_users.iter().map(|u| app.users.get(u).map(|u| u.display_name.clone())).collect();
                println!("voice {} {:?}", ch.name, names);
                for u in &ch.voice_users {
                    let m = app.selected_guild.and_then(|g| app.members.get(&g)).and_then(|ms| ms.iter().find(|m| m.user == *u));
                    println!("  vu {} member={:?}", u.0, m.map(|m| (m.nick.clone(), m.presence, m.activity.clone())));
                }
            }
            println!("feed {:?}", phoenix_app::ui::activity_traits::feed_authors().map(|v| v.len()));
            for a in phoenix_app::ui::activity_traits::feed_authors().unwrap_or_default() {
                println!("feed author {} {:?}", a.0, app.users.get(&a).map(|u| u.display_name.clone()));
            }
            if let Some(c) = app.selected_channel.and_then(|c| app.messages.get(&c)) {
                for m in c.msgs.iter().rev().take(12) {
                    let u = app.users.get(&m.author);
                    println!("msg {} author {} dn={:?} author_nick={:?} member_nick={:?} reply_to={:?}", m.id.0, m.author.0, u.map(|u| u.display_name.clone()), m.author_nick, app.member_nick(m.author), m.reply_to.map(|r| r.0));
                }
            }
        });
    }
    eprintln!("[{:.1}s] waited", t0.elapsed().as_secs_f32());
    // Same rows as the user's Discord: replay its scroll offsets on our lists.
    if let Some(sc) = scroll {
        apply_scroll(&mut cx, win.into(), &v, &sc, &pump)?;
        eprintln!("[{:.1}s] scrolled", t0.elapsed().as_secs_f32());
    }
    let mut img = None;
    for _ in 0..2 {
        cx.update_window(win.into(), |_, window, _| window.refresh())?;
        pump(&mut cx, 200);
        img = Some(cx.capture_screenshot(win.into())?);
    }
    eprintln!("[{:.1}s] captured", t0.elapsed().as_secs_f32());
    let img = img.expect("frame");
    img.save(&out)?;
    println!("wrote {out} ({}x{})", img.width(), img.height());
    Ok(())
}
