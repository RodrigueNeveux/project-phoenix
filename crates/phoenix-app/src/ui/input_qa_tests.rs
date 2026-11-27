use std::sync::Arc;

use gpui::{Bounds, Modifiers, MouseButton, Pixels, Point, TestAppContext, VisualTestContext, WindowHandle, point};

use crate::{
    app::{PhoenixApp, SidebarRow},
    backend::*,
    mock::MockBackend,
    model::{ChannelKind, Id},
};

struct Qa {
    window: WindowHandle<PhoenixApp>,
    vcx: VisualTestContext,
}

fn setup(cx: &mut TestAppContext) -> Qa {
    let mock = MockBackend::synchronous();
    let ready = futures::executor::block_on(mock.login(Credentials::Token("x".into()))).unwrap();
    cx.update(gpui_component::init);
    let backend: Arc<dyn Backend> = Arc::new(mock);
    let window = cx.add_window(|w, cx| PhoenixApp::new(backend, w, cx));
    window
        .update(cx, |app, _, cx| {
            app.apply_ready(ready, cx);
            let g = app.guilds[0].id;
            app.select_guild(Some(g), cx);
            let first = app.guild_channels[&g].iter().copied().find(|c| app.channels[c].kind == ChannelKind::Text).unwrap();
            app.select_channel(first, cx);
        })
        .unwrap();
    let vcx = VisualTestContext::from_window(window.into(), cx);
    vcx.run_until_parked();
    let mut qa = Qa { window, vcx };
    qa.vcx.update(|_, cx| crate::app::bind_keys(cx));
    qa.act(|a, w, cx| a.composer.update(cx, |s, cx| s.focus(w, cx)));
    qa.vcx.refresh().unwrap();
    qa.vcx.run_until_parked();
    qa
}

impl Qa {
    fn act(&mut self, f: impl FnOnce(&mut PhoenixApp, &mut gpui::Window, &mut gpui::Context<PhoenixApp>)) {
        self.window.update(&mut self.vcx, f).unwrap();
        self.vcx.run_until_parked();
    }

    fn read<T>(&mut self, f: impl FnOnce(&PhoenixApp) -> T) -> T {
        self.window.update(&mut self.vcx, |a, _, _| f(a)).unwrap()
    }

    fn device(&mut self, at: Point<Pixels>) -> Point<Pixels> {
        let k = self.vcx.update(|w, _| w.page_zoom());
        point(at.x * k, at.y * k)
    }

    fn press(&mut self, b: Bounds<Pixels>, button: MouseButton) {
        let at = self.device(b.center());
        self.vcx.simulate_mouse_move(at, None::<MouseButton>, Modifiers::none());
        self.vcx.run_until_parked();
        self.vcx.simulate_mouse_down(at, button, Modifiers::none());
        self.vcx.run_until_parked();
        self.vcx.simulate_mouse_up(at, button, Modifiers::none());
        self.vcx.run_until_parked();
    }

    fn keys(&mut self, keys: &str) {
        self.vcx.simulate_keystrokes(keys);
        self.vcx.run_until_parked();
    }
}

#[gpui::test]
fn right_click_on_a_message_opens_its_menu_and_escape_closes_it(cx: &mut TestAppContext) {
    let mut qa = setup(cx);
    let chat = qa.read(|a| a.message_list.viewport_bounds());
    assert!(f32::from(chat.size.height) > 0., "the message list is laid out");
    qa.press(chat, MouseButton::Right);
    assert!(qa.read(|a| a.chat.menu.is_some()), "a right click on a message did not open its menu");
    qa.keys("escape");
    assert!(qa.read(|a| a.chat.menu.is_none()), "Esc did not close the message menu");
}

#[gpui::test]
fn clicking_a_channel_row_opens_that_channel(cx: &mut TestAppContext) {
    let mut qa = setup(cx);
    let target = qa.read(|a| {
        a.sidebar_rows.iter().enumerate().find_map(|(i, r)| match r {
            SidebarRow::Channel(c) if Some(*c) != a.selected_channel && a.channels.get(c).is_some_and(|ch| ch.kind == ChannelKind::Text) => {
                a.sidebar_list.bounds_for_item(i + 1).map(|b| (*c, b))
            }
            _ => None,
        })
    });
    let (channel, row): (Id, Bounds<Pixels>) = target.expect("another text channel row is laid out");
    qa.press(row, MouseButton::Left);
    assert_eq!(qa.read(|a| a.selected_channel), Some(channel), "clicking the channel row did not open it");
}

#[gpui::test]
fn ctrl_k_opens_the_switcher_and_escape_closes_it(cx: &mut TestAppContext) {
    let mut qa = setup(cx);
    qa.keys("ctrl-k");
    assert!(qa.read(|a| a.switcher.is_some()), "Ctrl+K did not open the quick switcher");
    qa.keys("escape");
    assert!(qa.read(|a| a.switcher.is_none()), "Esc did not close the quick switcher");
}

#[gpui::test]
fn typing_a_mention_opens_the_autocomplete(cx: &mut TestAppContext) {
    let mut qa = setup(cx);
    qa.vcx.simulate_input("hi @");
    qa.vcx.run_until_parked();
    let mut text = String::new();
    qa.act(|a, _, cx| text = a.composer.read(cx).value().to_string());
    assert_eq!(text, "hi @", "the typed text did not reach the composer");
    let rows = qa.vcx.update(|_, cx| cx.try_global::<crate::ui::autocomplete::Autocomplete>().and_then(|s| s.active.as_ref()).map_or(0, |s| s.items.len()));
    assert!(rows > 0, "typing @ in the composer did not open the mention autocomplete");
}
