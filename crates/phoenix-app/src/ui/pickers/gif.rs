//! GIF tab: Tenor categories / trending home, search, masonry results.
//! Clicking a GIF sends its URL to the open channel (Discord embeds it).

use gpui::{
    AnyElement, Context, FontWeight, ObjectFit, ParentElement, SharedString, Styled, StyledImage,
    div, img, prelude::*, px,
};

use super::{Pickers, PICKER_W, close, debounce, empty_state, types::GifItem, with_app};
use crate::{
    app::PhoenixApp,
    theme::{Palette, c, ca},
};

const GAP: f32 = 8.;
const PAD: f32 = 16.;

pub fn load_home(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) {
    let g = &mut cx.global_mut::<Pickers>().gifs;
    if g.home.is_some() || g.home_loading {
        return;
    }
    g.home_loading = true;
    let fut = app.backend.gif_trending();
    cx.spawn(async move |this, cx| {
        let res = fut.await;
        let _ = this.update(cx, |_, cx| {
            let g = &mut cx.global_mut::<Pickers>().gifs;
            g.home_loading = false;
            match res {
                Ok(h) => g.home = Some(h),
                Err(e) => g.error = Some(e.0.into()),
            }
            cx.notify();
        });
    })
    .detach();
}

/// Search after the user stopped typing for a moment.
pub fn search_debounced(_app: &mut PhoenixApp, q: String, cx: &mut Context<PhoenixApp>) {
    let g = &mut cx.global_mut::<Pickers>().gifs;
    g.seq += 1;
    let seq = g.seq;
    if q.trim().is_empty() {
        g.shown = None;
        g.results.clear();
        g.loading = false;
        return;
    }
    debounce(cx, 350, move |app, cx| {
        if cx.global::<Pickers>().gifs.seq == seq {
            run(app, q, cx);
        }
    });
}

/// Load results for `q` ("" = trending GIFs).
pub fn run(app: &mut PhoenixApp, q: String, cx: &mut Context<PhoenixApp>) {
    let g = &mut cx.global_mut::<Pickers>().gifs;
    g.seq += 1;
    let seq = g.seq;
    g.loading = true;
    g.error = None;
    let fut = if q.is_empty() { app.backend.gif_trending_gifs() } else { app.backend.gif_search(q.clone()) };
    cx.spawn(async move |this, cx| {
        let res = fut.await;
        let _ = this.update(cx, |_, cx| {
            let g = &mut cx.global_mut::<Pickers>().gifs;
            if g.seq != seq {
                return;
            }
            g.loading = false;
            g.shown = Some(q);
            match res {
                Ok(v) => g.results = v,
                Err(e) => {
                    g.results.clear();
                    g.error = Some(e.0.into());
                }
            }
            cx.notify();
        });
    })
    .detach();
    cx.notify();
}

fn send(app: &mut PhoenixApp, gif: &GifItem, window: &mut gpui::Window, cx: &mut Context<PhoenixApp>) {
    let Some(ch) = app.selected_channel else { return };
    let fut = app.backend.send_message(ch, gif.url.to_string(), None);
    super::detach(fut, cx);
    close(app, window, cx);
}

fn tile(id: (&'static str, usize), name: SharedString, preview: Option<SharedString>, w: f32) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .relative()
        .w(px(w))
        .h(px(110.))
        .rounded(px(4.))
        .overflow_hidden()
        .bg(c(Palette::BG_TERTIARY))
        .cursor_pointer()
        .when_some(preview, |d, u| d.child(img(u).size_full().object_fit(ObjectFit::Cover)))
        .child(
            div()
                .absolute()
                .inset_0()
                .bg(ca(0x00000080))
                .hover(|s| s.bg(ca(0x00000040)))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(16.))
                .font_weight(FontWeight::BOLD)
                .text_color(gpui::white())
                .child(name),
        )
}

pub fn render_body(_app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let p = cx.global::<Pickers>();
    let g = &p.gifs;
    let typed = p.open.as_ref().map(|o| !o.input.read(cx).value().trim().is_empty()).unwrap_or(false);
    let col_w = (PICKER_W - 2. * PAD - GAP - 8.) / 2.;
    let content: AnyElement = if g.shown.is_none() && !typed {
        // Home: trending tile + categories.
        let Some(home) = g.home.clone() else {
            return empty_state(if g.home_loading { "Loading GIFs…" } else { "GIFs are unavailable." });
        };
        let mut tiles: Vec<AnyElement> = vec![
            tile(("gif-trending", 0), "Trending GIFs".into(), home.trending_preview.clone(), col_w)
                .on_click(|_, w, cx| with_app(cx, w, |a, _, cx| run(a, String::new(), cx)))
                .into_any_element(),
        ];
        for (i, cat) in home.categories.iter().enumerate() {
            let name = cat.name.clone();
            tiles.push(
                tile(("gif-cat", i), cat.name.clone(), Some(cat.preview.clone()), col_w)
                    .on_click(move |_, w, cx| {
                        let name = name.clone();
                        with_app(cx, w, move |a, w, cx| {
                            if let Some(op) = cx.global::<Pickers>().open.as_ref() {
                                let input = op.input.clone();
                                // Fills the box; the Change event runs the search.
                                input.update(cx, |s, cx| s.set_value(name.to_string(), w, cx));
                            }
                            run(a, name.to_string(), cx);
                        })
                    })
                    .into_any_element(),
            );
        }
        div().flex().flex_wrap().gap(px(GAP)).children(tiles).into_any_element()
    } else if g.loading && g.results.is_empty() {
        return empty_state("Loading GIFs…");
    } else if g.results.is_empty() {
        return empty_state(if g.error.is_some() { "GIFs are unavailable." } else { "No GIFs found." });
    } else {
        // Two-column masonry: each GIF goes to the shorter column.
        let mut cols: [(f32, Vec<AnyElement>); 2] = [(0., Vec::new()), (0., Vec::new())];
        for (i, gif) in g.results.iter().enumerate() {
            let h = if gif.width > 0 { col_w * gif.height as f32 / gif.width as f32 } else { col_w * 0.75 };
            let h = h.clamp(60., 400.);
            let k = if cols[0].0 <= cols[1].0 { 0 } else { 1 };
            cols[k].0 += h + GAP;
            let item = gif.clone();
            cols[k].1.push(
                div()
                    .id(("gif", i))
                    .w(px(col_w))
                    .h(px(h))
                    .rounded(px(4.))
                    .overflow_hidden()
                    .bg(c(Palette::BG_TERTIARY))
                    .cursor_pointer()
                    .hover(|s| s.border_2().border_color(c(Palette::BLURPLE)))
                    .on_click(move |_, w, cx| {
                        let item = item.clone();
                        with_app(cx, w, move |a, w, cx| send(a, &item, w, cx));
                    })
                    .child(img(gif.preview.clone()).size_full().object_fit(ObjectFit::Cover))
                    .into_any_element(),
            );
        }
        let [(_, a), (_, b)] = cols;
        div()
            .flex()
            .gap(px(GAP))
            .child(div().flex().flex_col().gap(px(GAP)).children(a))
            .child(div().flex().flex_col().gap(px(GAP)).children(b))
            .into_any_element()
    };
    div()
        .id("gif-scroll")
        .size_full()
        .overflow_y_scroll()
        .px(px(PAD))
        .pb(px(PAD))
        .child(content)
        .into_any_element()
}
