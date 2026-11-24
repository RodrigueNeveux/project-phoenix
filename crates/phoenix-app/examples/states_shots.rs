//! Offscreen renders of hover / pressed states (docs/parity/states.md).
//! `cargo run -p phoenix-app --example states_shots -- out_dir x,y[,down] ...`
//! writes one PNG per point with the mouse moved there (and pressed with `,down`).

use std::{cell::RefCell, rc::Rc, sync::Arc, time::Duration};

use gpui::{
    AppContext, Entity, HeadlessAppContext, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PlatformInput, px, size,
};
use gpui_component::Root;
use phoenix_app::{app::PhoenixApp, assets::Assets, mock::MockBackend};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let out = args.next().unwrap_or_else(|| "states".into());
    let points: Vec<String> = args.collect();
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
    let win = cx.open_window(size(px(1280.), px(800.)), move |window, cx| {
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
    }
    std::fs::create_dir_all(&out)?;
    for (n, p) in std::iter::once("0,0".to_string()).chain(points).enumerate() {
        let parts: Vec<&str> = p.split(',').collect();
        let (x, y): (f32, f32) = (parts[0].parse()?, parts[1].parse()?);
        let down = parts.get(2) == Some(&"down");
        let position = gpui::point(px(x), px(y));
        cx.update_window(win.into(), |_, window, cx| {
            window.dispatch_event(
                PlatformInput::MouseMove(MouseMoveEvent { position, pressed_button: None, modifiers: Modifiers::default() }),
                cx,
            );
            if down {
                window.dispatch_event(
                    PlatformInput::MouseDown(MouseDownEvent {
                        button: MouseButton::Left,
                        position,
                        modifiers: Modifiers::default(),
                        click_count: 1,
                        first_mouse: false,
                    }),
                    cx,
                );
            }
            window.refresh();
        })?;
        // Past every 200ms fade.
        pump(&mut cx, 6);
        cx.update_window(win.into(), |_, window, _| window.refresh())?;
        pump(&mut cx, 2);
        let img = cx.capture_screenshot(win.into())?;
        let path = format!("{out}/state-{n:02}.png");
        img.save(&path)?;
        println!("wrote {path} ({p})");
        if down {
            cx.update_window(win.into(), |_, window, cx| {
                window.dispatch_event(
                    PlatformInput::MouseUp(MouseUpEvent { button: MouseButton::Left, position, modifiers: Modifiers::default(), click_count: 1 }),
                    cx,
                );
            })?;
            pump(&mut cx, 2);
        }
    }
    Ok(())
}
