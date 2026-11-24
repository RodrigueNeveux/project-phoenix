//! Offscreen render of the Ctrl+/ keyboard combos modal (mock data) at the
//! live dump's 1536x840 viewport (card only: the headless renderer skips deferred overlays): `cargo run -p phoenix-app --example shortcuts_shot -- out.png`

use std::{sync::Arc, time::Duration};

use gpui::{AppContext, HeadlessAppContext, px, size};
use phoenix_app::assets::Assets;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let out = args.next().unwrap_or_else(|| "phoenix.png".into());

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
    // PHOENIX_SHOT_NET=1 loads remote art (Discord media hosts only) like the app.
    let _rt = std::env::var_os("PHOENIX_SHOT_NET").map(|_| tokio::runtime::Runtime::new().expect("runtime"));
    if let Some(rt) = &_rt {
        let http = phoenix_app::discord_backend::http::CdnHttpClient::new(rt.handle().clone(), "Mozilla/5.0");
        cx.update(|cx| cx.set_http_client(std::sync::Arc::new(phoenix_app::media::cache::CachingHttpClient::new(std::sync::Arc::new(http)))));
    }
    cx.update(|cx| {
        let _ = cx.text_system().add_fonts(fonts);
        phoenix_app::init(cx);
    });
    struct Shot;
    impl gpui::Render for Shot {
        fn render(&mut self, window: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
            use gpui::{ParentElement, Styled};
            use phoenix_app::theme::{c, generated::idx, tok};
            gpui::div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .font_family(phoenix_app::theme::FONT_UI())
                .bg(c(tok(idx::BACKGROUND_BASE_LOW)))
                .child(phoenix_app::shortcuts::help_card(window.viewport_size()))
        }
    }
    let env = |k: &str, d: f32| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    let win = cx.open_window(size(px(env("PHOENIX_SHOT_W", 1536.)), px(env("PHOENIX_SHOT_H", 840.))), |_, cx| cx.new(|_| Shot))?;
    for _ in 0..6 {
        std::thread::sleep(Duration::from_millis(50));
        cx.run_until_parked();
    }
    cx.update_window(win.into(), |_, window, _| window.refresh())?;
    cx.run_until_parked();
    let _ = cx.capture_screenshot(win.into())?;
    let img = cx.capture_screenshot(win.into())?;
    img.save(&out)?;
    println!("wrote {out} ({}x{})", img.width(), img.height());
    Ok(())
}
