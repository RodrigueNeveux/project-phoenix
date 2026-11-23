//! Sticker tab: guild stickers then Discord's standard packs, searchable by
//! name and tags. Clicking one sends it to the open channel.

use crate::theme::z;
use gpui::{AnyElement, Context, ParentElement, SharedString, Styled, div, img, prelude::*, px};

use super::{Pickers, close, empty_state, section_title, types::StickerData, with_app};
use crate::{
    app::PhoenixApp,
    media::sticker_url,
    theme::{Palette, c},
};

const CELL: f32 = 104.;

pub fn load_packs(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) {
    let s = &mut cx.global_mut::<Pickers>().stickers;
    if s.packs.is_some() || s.loading {
        return;
    }
    s.loading = true;
    let fut = app.backend.sticker_packs();
    cx.spawn(async move |this, cx| {
        let res = fut.await;
        let _ = this.update(cx, |_, cx| {
            let s = &mut cx.global_mut::<Pickers>().stickers;
            s.loading = false;
            s.packs = Some(res.unwrap_or_default());
            cx.notify();
        });
    })
    .detach();
}

pub fn matches(s: &StickerData, q: &str) -> bool {
    q.is_empty()
        || s.name.to_lowercase().contains(q)
        || s.tags.to_lowercase().split([',', ' ']).any(|t| !t.is_empty() && t.starts_with(q))
}

fn send(app: &mut PhoenixApp, id: crate::model::Id, window: &mut gpui::Window, cx: &mut Context<PhoenixApp>) {
    let Some(ch) = app.selected_channel else { return };
    super::detach(app.backend.send_sticker(ch, id), cx);
    close(app, window, cx);
}

fn cell(s: &StickerData) -> AnyElement {
    let id = s.id;
    let inner: AnyElement = match sticker_url(s.id.0, s.format) {
        Some(u) => img(u).size(px(CELL - 12.)).into_any_element(),
        // Lottie stickers can't be drawn yet: show the name.
        None => div()
            .size(px(CELL - 12.))
            .flex()
            .items_center()
            .justify_center()
            .text_size(z(12.))
            .text_color(c(Palette::TEXT_MUTED))
            .child(s.name.clone())
            .into_any_element(),
    };
    div()
        .id(("sticker", id.0 as usize))
        .size(px(CELL))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(z(4.))
        .cursor_pointer()
        .hover(|st| st.bg(c(Palette::BG_MOD_HOVER)))
        .on_click(move |_, w, cx| with_app(cx, w, move |a, w, cx| send(a, id, w, cx)))
        .child(inner)
        .into_any_element()
}

pub fn render_body(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let p = cx.global::<Pickers>();
    let q = p.open.as_ref().map(|o| o.input.read(cx).value().trim().to_lowercase()).unwrap_or_default();
    let mut sections: Vec<(SharedString, Vec<StickerData>)> = Vec::new();
    let mut guilds: Vec<_> = app.guilds.iter().collect();
    if let Some(sel) = app.selected_guild {
        guilds.sort_by_key(|g| g.id != sel);
    }
    for g in guilds {
        let st: Vec<_> = app.backend.guild_stickers(g.id).into_iter().filter(|s| matches(s, &q)).collect();
        if !st.is_empty() {
            sections.push((g.name.clone(), st));
        }
    }
    for pack in p.stickers.packs.iter().flatten() {
        let st: Vec<_> = pack.stickers.iter().filter(|s| matches(s, &q)).cloned().collect();
        if !st.is_empty() {
            sections.push((pack.name.clone(), st));
        }
    }
    if sections.is_empty() {
        return empty_state(if p.stickers.loading { crate::t!("picker.loading_stickers") } else { crate::t!("picker.no_stickers") });
    }
    let mut body = div().id("sticker-scroll").size_full().overflow_y_scroll().px(z(8.)).pb(z(8.));
    for (title, st) in sections {
        body = body.child(section_title(title)).child(div().flex().flex_wrap().children(st.iter().map(cell)));
    }
    body.into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::StickerFormat;

    #[test]
    fn sticker_matching() {
        let s = StickerData {
            id: crate::model::Id(1),
            name: "Wumpus Wave".into(),
            description: "".into(),
            tags: "wave, hello".into(),
            format: StickerFormat::Png,
            guild: None,
        };
        assert!(matches(&s, ""));
        assert!(matches(&s, "wump"));
        assert!(matches(&s, "hell"));
        assert!(!matches(&s, "cat"));
    }
}
