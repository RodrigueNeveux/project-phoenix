//! Offscreen frames of animated custom emoji as Phoenix draws them (message size,
//! jumbo, reaction, picker tile at rest and hovered-off):
//! `anim_emoji_shot <out-prefix> <emoji-id>...` writes `<out-prefix>-<n>.png`,
//! one frame every 150 ms. Ids come from the command line, none are bundled.

use std::{sync::Arc, time::Duration};

use gpui::{AppContext, Context, HeadlessAppContext, IntoElement, ParentElement, Render, Styled, Window, div, px, rgb, size};
use phoenix_app::{assets::Assets, media::emoji};

struct Shot {
    ids: Vec<String>,
}

impl Render for Shot {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        emoji::sync_motion(true, true);
        let row = |size: f32, hover_only: bool, ids: &[String]| {
            let mut r = div().flex().items_center().gap(px(8.));
            for id in ids {
                let t = format!("<a:e:{id}>");
                r = r.child(if hover_only { emoji::emoji_element_hover(&t, size) } else { emoji::emoji_element(&t, size) });
            }
            r
        };
        div()
            .size_full()
            .bg(rgb(0x1a1a1e))
            .p(px(16.))
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(row(22., false, &self.ids)) // inline 1.375em at 16px
            .child(row(48., false, &self.ids)) // jumbo 3em
            .child(row(40., true, &self.ids)) // picker tile, not hovered: still
    }
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let out = args.next().unwrap_or_else(|| "anim".into());
    let ids: Vec<String> = args.filter(|a| a.chars().all(|c| c.is_ascii_digit())).collect();
    let text = gpui_platform::current_platform(true).text_system();
    let mut cx = HeadlessAppContext::with_platform(text, Arc::new(Assets), || gpui_platform::current_headless_renderer());
    let rt = tokio::runtime::Runtime::new()?;
    let http = phoenix_app::discord_backend::http::CdnHttpClient::new(rt.handle().clone(), "Mozilla/5.0");
    cx.update(|cx| cx.set_http_client(Arc::new(phoenix_app::media::cache::CachingHttpClient::new(Arc::new(http)))));
    let win = cx.open_window(size(px(420.), px(180.)), move |_, cx| cx.new(|_| Shot { ids }))?;
    let pump = |cx: &mut HeadlessAppContext, n: usize| {
        for _ in 0..n {
            std::thread::sleep(Duration::from_millis(50));
            cx.run_until_parked();
        }
    };
    pump(&mut cx, 60);
    for n in 0..6 {
        cx.update_window(win.into(), |_, window, _| window.refresh())?;
        pump(&mut cx, 3);
        let img = cx.capture_screenshot(win.into())?;
        img.save(format!("{out}-{n}.png"))?;
    }
    println!("wrote {out}-0..5.png");
    Ok(())
}
