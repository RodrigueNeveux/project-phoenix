//! Emoji tab: sectioned grid (frequently used, guild emoji, unicode
//! categories), category sidebar, skin tone selector and preview bar.

use gpui::{
    AnyElement, App, Context, FontWeight, ListOffset, ParentElement, SharedString, Styled, div,
    img, list, prelude::*, px,
};

use super::{Pickers, emoji, pick, types::GuildEmoji, with_app};
use crate::{
    app::PhoenixApp,
    media::emoji::emoji_element,
    model::{Guild, ImageRef},
    theme::{Palette, c},
};

pub const COLS: usize = 9;
const CELL: f32 = 44.;
const EMOJI_PX: f32 = 32.;
const FREQUENT: usize = 18;

#[derive(Clone, Debug, PartialEq)]
pub enum PickItem {
    /// Index into [`emoji::EMOJI`].
    Unicode(usize),
    Custom(GuildEmoji),
}

impl PickItem {
    pub fn name(&self) -> SharedString {
        match self {
            Self::Unicode(ix) => emoji::shortcode(*ix).into(),
            Self::Custom(e) => e.name.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SectionIcon {
    Frequent,
    Guild(ImageRef, SharedString),
    Category(usize),
}

#[derive(Clone, Debug)]
pub struct Section {
    pub title: SharedString,
    pub icon: SectionIcon,
    pub first_row: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    Header(usize),
    Cells { start: usize, end: usize },
}

#[derive(Default)]
pub struct Grid {
    pub items: Vec<PickItem>,
    pub rows: Vec<Row>,
    pub sections: Vec<Section>,
    /// Guild name of custom emoji, for the preview bar.
    pub guild_names: Vec<(crate::model::Id, SharedString)>,
}

impl Grid {
    fn push_section(&mut self, title: SharedString, icon: SectionIcon, items: Vec<PickItem>, header: bool) {
        if items.is_empty() {
            return;
        }
        let si = self.sections.len();
        self.sections.push(Section { title, icon, first_row: self.rows.len() });
        if header {
            self.rows.push(Row::Header(si));
        }
        let base = self.items.len();
        let n = items.len();
        self.items.extend(items);
        for chunk in (0..n).step_by(COLS) {
            self.rows.push(Row::Cells { start: base + chunk, end: base + (chunk + COLS).min(n) });
        }
    }

    pub fn row_of(&self, item: usize) -> Option<usize> {
        self.rows.iter().position(|r| matches!(r, Row::Cells { start, end } if (*start..*end).contains(&item)))
    }

    /// Item above/below `item`, keeping the column when possible.
    pub fn vertical(&self, item: usize, down: bool) -> usize {
        let Some(r) = self.row_of(item) else { return item };
        let Row::Cells { start, .. } = self.rows[r] else { return item };
        let col = item - start;
        let mut ix = r;
        loop {
            ix = if down {
                if ix + 1 >= self.rows.len() {
                    return item;
                }
                ix + 1
            } else {
                if ix == 0 {
                    return item;
                }
                ix - 1
            };
            if let Row::Cells { start, end } = self.rows[ix] {
                return (start + col).min(end - 1);
            }
        }
    }

    pub fn section_at_row(&self, row: usize) -> usize {
        self.sections.iter().rposition(|s| s.first_row <= row).unwrap_or(0)
    }
}

/// Build the grid for `query` from the app's guilds and the frequency data.
pub fn build(app: &PhoenixApp, query: &str, cx: &App) -> Grid {
    let mut guilds: Vec<&Guild> = app.guilds.iter().collect();
    if let Some(sel) = app.selected_guild {
        guilds.sort_by_key(|g| g.id != sel);
    }
    let customs: Vec<(&Guild, Vec<GuildEmoji>)> = guilds
        .into_iter()
        .map(|g| (g, app.backend.guild_emojis(g.id)))
        .filter(|(_, e)| !e.is_empty())
        .collect();
    let mut grid = Grid {
        guild_names: customs.iter().map(|(g, _)| (g.id, g.name.clone())).collect(),
        ..Default::default()
    };
    let q = query.trim().trim_matches(':').to_lowercase();
    if q.is_empty() {
        let now = chrono::Utc::now().timestamp();
        let freq = cx.try_global::<Pickers>().map(|p| p.prefs.frequent(FREQUENT, now)).unwrap_or_default();
        grid.push_section("Frequently Used".into(), SectionIcon::Frequent, freq, true);
        for (g, es) in customs {
            let items = es.into_iter().map(PickItem::Custom).collect();
            grid.push_section(g.name.clone(), SectionIcon::Guild(g.icon.clone(), g.acronym.clone()), items, true);
        }
        for (cat, title) in emoji::CATEGORIES.iter().enumerate() {
            let items = (0..emoji::EMOJI.len()).filter(|&i| emoji::EMOJI[i].3 as usize == cat).map(PickItem::Unicode).collect();
            grid.push_section((*title).into(), SectionIcon::Category(cat), items, true);
        }
    } else {
        let mut custom: Vec<(u8, GuildEmoji)> = customs
            .into_iter()
            .flat_map(|(_, es)| es)
            .filter_map(|e| emoji::name_score(&e.name.to_lowercase(), &q).map(|s| (s, e)))
            .collect();
        custom.sort_by_key(|(s, _)| *s);
        let mut items: Vec<PickItem> = custom.into_iter().map(|(_, e)| PickItem::Custom(e)).collect();
        items.extend(emoji::search(&q, 300).into_iter().map(PickItem::Unicode));
        grid.push_section("Search Results".into(), SectionIcon::Frequent, items, false);
    }
    grid
}

// ------------------------------------------------------------------ render

pub fn item_element(item: &PickItem, tone: u8, size: f32) -> AnyElement {
    match item {
        PickItem::Unicode(ix) => emoji_element(&emoji::toned(*ix, tone), size),
        PickItem::Custom(e) => img(e.url((size * 2.) as u32))
            .size(px(size))
            .flex_none()
            .when(!e.available, |i| i.opacity(0.3))
            .into_any_element(),
    }
}

fn render_row(ix: usize, cx: &mut App) -> AnyElement {
    let p = cx.global::<Pickers>();
    let Some(op) = p.open.as_ref() else { return div().into_any_element() };
    let tone = p.prefs.tone;
    match op.grid.rows.get(ix).copied() {
        Some(Row::Header(si)) => {
            let title = op.grid.sections[si].title.clone();
            super::section_title(title).pt(px(4.)).into_any_element()
        }
        Some(Row::Cells { start, end }) => {
            let selected = op.selected;
            let mut row = div().flex().px(px(8.));
            for i in start..end {
                let item = op.grid.items[i].clone();
                let is_sel = i == selected;
                row = row.child(
                    div()
                        .id(("emoji-cell", i))
                        .size(px(CELL))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.))
                        .cursor_pointer()
                        .when(is_sel, |d| d.bg(c(Palette::BG_MOD_SELECTED)))
                        .hover(|s| s.bg(c(Palette::BG_MOD_SELECTED)))
                        .on_hover(move |h, w, cx| {
                            if *h {
                                with_app(cx, w, |_, _, cx| {
                                    if let Some(op) = cx.global_mut::<Pickers>().open.as_mut()
                                        && op.hovered != Some(i)
                                    {
                                        op.hovered = Some(i);
                                        cx.notify();
                                    }
                                });
                            }
                        })
                        .on_click({
                            let item = item.clone();
                            move |ev, w, cx| {
                                let keep = ev.modifiers().shift;
                                let item = item.clone();
                                with_app(cx, w, move |a, w, cx| pick(a, item, keep, w, cx));
                            }
                        })
                        .child(item_element(&item, tone, EMOJI_PX)),
                );
            }
            row.into_any_element()
        }
        None => div().into_any_element(),
    }
}

pub fn render_body(_app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let p = cx.global::<Pickers>();
    let Some(op) = p.open.as_ref() else { return div().into_any_element() };
    let tone = p.prefs.tone;
    let list_state = op.list.clone();
    let active = op.grid.section_at_row(op.list.logical_scroll_top().item_ix);
    let preview = op.hovered.or(Some(op.selected)).and_then(|i| op.grid.items.get(i).cloned());
    let preview_from: Option<SharedString> = match &preview {
        Some(PickItem::Custom(e)) => op.grid.guild_names.iter().find(|(g, _)| *g == e.guild).map(|(_, n)| n.clone()),
        _ => None,
    };
    let empty = op.grid.items.is_empty();
    let searching = op.grid.rows.first().is_some_and(|r| matches!(r, Row::Cells { .. }));

    let mut sidebar = div()
        .id("emoji-sidebar")
        .w(px(48.))
        .flex_none()
        .h_full()
        .bg(c(Palette::BG_SECONDARY_ALT))
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .items_center()
        .py(px(4.))
        .gap(px(4.));
    if !searching {
        for (si, s) in op.grid.sections.iter().enumerate() {
            let first = s.first_row;
            let is_active = si == active;
            let content: AnyElement = match &s.icon {
                SectionIcon::Frequent => emoji_element("🕒", 20.),
                SectionIcon::Category(cat) => emoji_element(emoji::CATEGORY_ICON[*cat], 20.),
                SectionIcon::Guild(ImageRef::Url(u), _) => img(u.clone()).size(px(24.)).rounded_full().into_any_element(),
                SectionIcon::Guild(_, acr) => div()
                    .size(px(24.))
                    .rounded_full()
                    .bg(c(Palette::BG_PRIMARY))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(9.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(acr.clone())
                    .into_any_element(),
            };
            let sep = matches!(s.icon, SectionIcon::Category(0)) && si > 0;
            if sep {
                sidebar = sidebar.child(div().w(px(24.)).h(px(1.)).bg(c(Palette::DIVIDER)));
            }
            let ls = list_state.clone();
            sidebar = sidebar.child(
                div()
                    .id(("emoji-section", si))
                    .size(px(32.))
                    .flex_none()
                    .rounded(px(4.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .opacity(if is_active { 1. } else { 0.6 })
                    .when(is_active, |d| d.bg(c(Palette::BG_MOD_HOVER)))
                    .hover(|s| s.opacity(1.).bg(c(Palette::BG_MOD_HOVER)))
                    .on_click(move |_, w, cx| {
                        ls.scroll_to(ListOffset { item_ix: first, offset_in_item: px(0.) });
                        with_app(cx, w, |_, _, cx| cx.notify());
                    })
                    .child(content),
            );
        }
    }

    let grid_el = if empty {
        super::empty_state("No emoji match your search.")
    } else {
        list(list_state, |ix, _w, cx| render_row(ix, cx)).size_full().into_any_element()
    };

    let footer = div()
        .h(px(48.))
        .flex_none()
        .px(px(16.))
        .bg(c(Palette::BG_SECONDARY_ALT))
        .flex()
        .items_center()
        .gap(px(8.))
        .when_some(preview, |d, item| {
            d.child(item_element(&item, tone, 28.))
                .child(
                    div()
                        .text_size(px(16.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(Palette::TEXT_HEADER))
                        .child(SharedString::from(format!(":{}:", item.name()))),
                )
                .when_some(preview_from, |d, g| {
                    d.child(div().text_size(px(12.)).text_color(c(Palette::TEXT_MUTED)).child(SharedString::from(format!("from {g}"))))
                })
        });

    div()
        .size_full()
        .flex()
        .flex_row()
        .when(!searching, |d| d.child(sidebar))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().flex_1().min_h_0().child(grid_el))
                .child(footer),
        )
        .into_any_element()
}

/// Skin tone button next to the search box, with its 6 option popup.
pub fn tone_button(cx: &mut Context<PhoenixApp>) -> AnyElement {
    let p = cx.global::<Pickers>();
    let tone = p.prefs.tone;
    let open = p.open.as_ref().is_some_and(|o| o.tone_open);
    let mut el = div()
        .relative()
        .size(px(30.))
        .flex_none()
        .child(
            div()
                .id("tone-button")
                .size(px(30.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .cursor_pointer()
                .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
                .on_click(cx.listener(|_, _, _, cx| {
                    if let Some(op) = cx.global_mut::<Pickers>().open.as_mut() {
                        op.tone_open = !op.tone_open;
                    }
                    cx.notify();
                }))
                .child(emoji_element(&emoji::with_tone(emoji::TONE_PREVIEW, tone), 24.)),
        );
    if open {
        let mut menu = div()
            .absolute()
            .top(px(0.))
            .left(px(0.))
            .w(px(30.))
            .rounded(px(4.))
            .bg(c(Palette::BG_FLOATING))
            .flex()
            .flex_col();
        for t in 0..=5u8 {
            menu = menu.child(
                div()
                    .id(("tone", t as usize))
                    .size(px(30.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .cursor_pointer()
                    .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
                    .on_click(cx.listener(move |_, _, _, cx| {
                        let p = cx.global_mut::<Pickers>();
                        p.prefs.set_tone(t);
                        if let Some(op) = p.open.as_mut() {
                            op.tone_open = false;
                        }
                        cx.notify();
                    }))
                    .child(emoji_element(&emoji::with_tone(emoji::TONE_PREVIEW, t), 24.)),
            );
        }
        el = el.child(menu);
    }
    el.into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_of(sizes: &[usize]) -> Grid {
        let mut g = Grid::default();
        let mut n = 0;
        for &s in sizes {
            let items = (n..n + s).map(PickItem::Unicode).collect();
            n += s;
            g.push_section("x".into(), SectionIcon::Frequent, items, true);
        }
        g
    }

    #[test]
    fn rows_and_navigation() {
        let g = grid_of(&[11, 4]);
        // header, 9, 2, header, 4
        assert_eq!(g.rows.len(), 5);
        assert_eq!(g.row_of(10), Some(2));
        assert_eq!(g.vertical(1, true), 10); // col 1 -> second row has 2 items
        assert_eq!(g.vertical(8, true), 10); // clamped to the row end
        assert_eq!(g.vertical(10, true), 12); // crosses the header
        assert_eq!(g.vertical(12, false), 10);
        assert_eq!(g.vertical(0, false), 0);
        assert_eq!(g.section_at_row(3), 1);
    }
}
