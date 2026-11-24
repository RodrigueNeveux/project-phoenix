//! Offscreen render of the mock #showcase channel (every kind of message content).
//! `content_shots out.png [dark|light|ash|onyx]`; PHOENIX_SHOT_H sets the window height.
//! (scene arg unused) Scenes of views.rs: friends-all, friends-pending, inbox, pins, threads, search, profile, profile-modal, appearance, account, status, switcher, emoji, gif, sticker, friends, forum, dm-menu, dm-submenu.

use std::{cell::RefCell, rc::Rc, sync::Arc, time::Duration};

use gpui::{AppContext, Entity, HeadlessAppContext, px, size};
use gpui_component::Root;
use phoenix_app::{app::PhoenixApp, assets::Assets, mock::MockBackend, ui::settings::AppTheme};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let out = args.next().unwrap_or_else(|| "phoenix.png".into());
    let theme = match args.next().as_deref() {
        Some("light") => AppTheme::Light,
        Some("ash") => AppTheme::Ash,
        Some("onyx") => AppTheme::Onyx,
        _ => AppTheme::Dark,
    };
    let scene = args.next().unwrap_or_else(|| "appearance".into());
    let login_only = false;

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
    cx.update(|cx| {
        let _ = cx.text_system().add_fonts(fonts);
        phoenix_app::init(cx);
    });
    let view: Rc<RefCell<Option<Entity<PhoenixApp>>>> = Rc::default();
    let v2 = view.clone();
    // PHOENIX_SHOT_H=700 matches the live client dumps (1280x700 viewport).
    let h: f32 = std::env::var("PHOENIX_SHOT_H").ok().and_then(|v| v.parse().ok()).unwrap_or(800.);
    let w: f32 = std::env::var("PHOENIX_SHOT_W").ok().and_then(|v| v.parse().ok()).unwrap_or(1280.);
    let win = cx.open_window(size(px(w), px(h)), move |window, cx| {
        let v = cx.new(|cx| PhoenixApp::new(Arc::new(MockBackend::new()), window, cx));
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
        cx.update_window(win.into(), |_, window, cx| {
            v.update(cx, |app, cx| {
                app.login_with_token = true;
                app.login_token.update(cx, |s, cx| s.set_value("mock-token", window, cx));
                app.submit_login(window, cx);
            });
        })?;
        pump(&mut cx, 30);
        cx.update_window(win.into(), |_, window, cx| {
            v.update(cx, |app, cx| {
                app.prefs.synced.theme = theme;
                let g = app.guilds.first().map(|g| g.id);
                app.select_guild(g, cx);
                let id = app.channels.values().find(|c| c.guild == g && c.name.as_ref() == "showcase").map(|c| c.id);
                if let Some(id) = id {
                    app.select_channel(id, cx);
                }
                let _ = (&scene, window);
                cx.notify();
            });
        })?;
        pump(&mut cx, 4);
    }
    // Images (avatars, decorations) load over several frames: with
    // PHX_MOCK_DECO or PHX_WAIT_IMAGES, keep painting ~2 s before capturing.
    let frames = 10;
    for _ in 0..frames {
        cx.update_window(win.into(), |_, window, _| window.refresh())?;
        pump(&mut cx, 2);
    }
    let _ = cx.capture_screenshot(win.into())?;
    cx.update_window(win.into(), |_, window, _| window.refresh())?;
    pump(&mut cx, 2);
    let img = cx.capture_screenshot(win.into())?;
    img.save(&out)?;
    println!("wrote {out} ({}x{})", img.width(), img.height());
    Ok(())
}
