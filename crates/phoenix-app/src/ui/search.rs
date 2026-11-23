//! Message search popout (Ctrl+F): `from:`, `mentions:`, `in:`, `has:`,
//! `before:`, `after:`, `during:` and `pinned:` filters, paged results.

use crate::theme::z;
use gpui::{
    AnyElement, App, Context, Entity, FontWeight, ParentElement, SharedString, Styled, Subscription,
    Window, div, prelude::*, px,
};
use gpui_component::input::{Input, InputEvent, InputState};
use phoenix_api::rest::search::{date_to_snowflake, parse_search};

use crate::{
    app::{PhoenixApp, format_time_full},
    assets::Icon,
    model::{ChannelKind, Id, Message},
    theme::{Palette, c, generated::idx, m as layout},
    ui::{avatar, icon, icon_button},
};

pub const PAGE: u32 = 25;
const WIDTH: f32 = 420.;

/// What to search, with names already resolved to ids.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SearchRequest {
    /// Guild scope; when `None`, `channel` (a DM) is searched.
    pub guild: Option<Id>,
    pub channel: Option<Id>,
    pub content: Option<String>,
    pub authors: Vec<Id>,
    pub mentions: Vec<Id>,
    pub channels: Vec<Id>,
    /// `has:` values ("link", "image"...).
    pub has: Vec<String>,
    pub min_id: Option<Id>,
    pub max_id: Option<Id>,
    pub pinned: Option<bool>,
    pub offset: u32,
    pub oldest_first: bool,
}

#[derive(Clone, Debug, Default)]
pub struct SearchPage {
    pub total: u32,
    pub messages: Vec<Message>,
}

pub struct SearchState {
    pub input: Entity<InputState>,
    pub request: Option<SearchRequest>,
    pub page: Option<SearchPage>,
    pub loading: bool,
    pub error: Option<SharedString>,
    seq: u64,
    /// "More filters" expanded (dates and `pinned:`).
    more: bool,
    _sub: Subscription,
}

/// Queries run this session, newest first (the popout's "History" group).
#[derive(Default)]
pub struct SearchHistory(pub Vec<String>);

impl gpui::Global for SearchHistory {}

#[derive(Default)]
pub struct Search(pub Option<SearchState>);

impl gpui::Global for Search {}

const DAY: u64 = 86_400_000 << 22;

/// Turn the typed text into a request, resolving `from:`/`in:` names.
pub fn build_request(app: &PhoenixApp, text: &str) -> Result<SearchRequest, String> {
    let p = parse_search(text);
    let mut req = SearchRequest {
        guild: app.selected_guild,
        channel: if app.selected_guild.is_none() { app.selected_channel } else { None },
        content: (!p.content.trim().is_empty()).then(|| p.content.trim().to_owned()),
        has: p.has.iter().map(|h| h.as_str().to_owned()).collect(),
        pinned: p.pinned,
        ..Default::default()
    };
    if req.guild.is_none() && req.channel.is_none() {
        return Err(crate::t!("search.need_scope").into());
    }
    let find_user = |name: &str| -> Result<Id, String> {
        let n = name.to_lowercase();
        let mut best: Option<(u8, Id)> = None;
        for u in app.users.values() {
            for cand in [u.name.to_lowercase(), u.display_name.to_lowercase()] {
                let s = if cand == n { 0 } else if cand.starts_with(&n) { 1 } else { continue };
                if best.is_none_or(|(b, _)| s < b) {
                    best = Some((s, u.id));
                }
            }
        }
        best.map(|(_, id)| id).ok_or_else(|| crate::t!("search.no_user", name = name).to_string())
    };
    for f in &p.from {
        req.authors.push(find_user(f)?);
    }
    for f in &p.mentions {
        req.mentions.push(find_user(f)?);
    }
    for name in &p.in_channels {
        let n = name.to_lowercase();
        let ch = app
            .guild_channels
            .get(&req.guild.unwrap_or_default())
            .into_iter()
            .flatten()
            .filter_map(|id| app.channels.get(id))
            .find(|c| c.kind != ChannelKind::Category && c.name.to_lowercase() == n)
            .ok_or_else(|| crate::t!("search.no_channel", name = name).to_string())?;
        req.channels.push(ch.id);
    }
    let day = |d: &str| date_to_snowflake(d).map(|s| s.0).ok_or_else(|| crate::t!("search.bad_date", d = d).to_string());
    if let Some(d) = &p.before {
        req.max_id = Some(Id(day(d)?));
    }
    if let Some(d) = &p.after {
        req.min_id = Some(Id(day(d)? + DAY));
    }
    if let Some(d) = &p.during {
        let s = day(d)?;
        req.min_id = Some(Id(s));
        req.max_id = Some(Id(s + DAY));
    }
    let empty = req.content.is_none()
        && req.authors.is_empty()
        && req.mentions.is_empty()
        && req.channels.is_empty()
        && req.has.is_empty()
        && req.min_id.is_none()
        && req.max_id.is_none()
        && req.pinned.is_none();
    if empty {
        return Err(String::new());
    }
    Ok(req)
}

pub fn is_open(cx: &App) -> bool {
    cx.try_global::<Search>().is_some_and(|s| s.0.is_some())
}

pub fn is_focused(window: &Window, cx: &App) -> bool {
    cx.try_global::<Search>()
        .and_then(|s| s.0.as_ref())
        .is_some_and(|s| gpui::Focusable::focus_handle(s.input.read(cx), cx).is_focused(window))
}

pub fn open(_app: &mut PhoenixApp, window: &mut Window, cx: &mut Context<PhoenixApp>) {
    if let Some(s) = cx.try_global::<Search>().and_then(|s| s.0.as_ref()) {
        let input = s.input.clone();
        input.update(cx, |s, cx| s.focus(window, cx));
        cx.notify();
        return;
    }
    let input = cx.new(|cx| InputState::new(window, cx).placeholder(crate::t!("search.placeholder")));
    let sub = cx.subscribe_in(&input, window, |app, _, ev: &InputEvent, _, cx| {
        if let InputEvent::PressEnter { .. } = ev {
            submit(app, cx);
        }
    });
    input.update(cx, |s, cx| s.focus(window, cx));
    cx.set_global(Search(Some(SearchState {
        input,
        request: None,
        page: None,
        loading: false,
        error: None,
        seq: 0,
        more: false,
        _sub: sub,
    })));
    cx.notify();
}

pub fn close(app: &mut PhoenixApp, window: &mut Window, cx: &mut Context<PhoenixApp>) {
    if cx.has_global::<Search>() {
        cx.global_mut::<Search>().0 = None;
    }
    app.composer.update(cx, |s, cx| s.focus(window, cx));
    cx.notify();
}

pub fn on_key(app: &mut PhoenixApp, key: &str, window: &mut Window, cx: &mut Context<PhoenixApp>) -> bool {
    if key == "escape" {
        close(app, window, cx);
        return true;
    }
    false
}

fn submit(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) {
    let Some(s) = cx.global::<Search>().0.as_ref() else { return };
    let text = s.input.read(cx).value().to_string();
    match build_request(app, &text) {
        Ok(req) => {
            let t = text.trim().to_owned();
            let h = &mut cx.default_global::<SearchHistory>().0;
            h.retain(|x| *x != t);
            h.insert(0, t);
            h.truncate(5);
            run(app, req, cx)
        }
        Err(e) => {
            if let Some(s) = cx.global_mut::<Search>().0.as_mut() {
                s.error = (!e.is_empty()).then(|| e.into());
                s.page = None;
                s.request = None;
            }
            cx.notify();
        }
    }
}

pub fn run(app: &mut PhoenixApp, req: SearchRequest, cx: &mut Context<PhoenixApp>) {
    let Some(s) = cx.global_mut::<Search>().0.as_mut() else { return };
    s.seq += 1;
    let seq = s.seq;
    s.loading = true;
    s.error = None;
    s.request = Some(req.clone());
    let fut = app.backend.search_messages(req);
    cx.spawn(async move |this, cx| {
        let res = fut.await;
        let _ = this.update(cx, |_, cx| {
            let Some(s) = cx.global_mut::<Search>().0.as_mut() else { return };
            if s.seq != seq {
                return;
            }
            s.loading = false;
            match res {
                Ok(p) => s.page = Some(p),
                Err(e) => {
                    s.page = None;
                    s.error = Some(e.0.into());
                }
            }
            cx.notify();
        });
    })
    .detach();
    cx.notify();
}

fn rerun(app: &mut PhoenixApp, f: impl FnOnce(&mut SearchRequest), cx: &mut Context<PhoenixApp>) {
    let Some(mut req) = cx.global::<Search>().0.as_ref().and_then(|s| s.request.clone()) else { return };
    f(&mut req);
    run(app, req, cx);
}

/// Append a filter prefix to the box ("from: ").
fn add_filter(prefix: &'static str, window: &mut Window, cx: &mut Context<PhoenixApp>) {
    let Some(s) = cx.global::<Search>().0.as_ref() else { return };
    let input = s.input.clone();
    input.update(cx, |s, cx| {
        let v = s.value().to_string();
        let sep = if v.is_empty() || v.ends_with(' ') { "" } else { " " };
        s.set_value(format!("{v}{sep}{prefix}"), window, cx);
        s.focus(window, cx);
    });
}

/// "Filters" group of the live popout (dump search-popout `groupWrapper_lm2_KW`):
/// (inserted keyword, icon, label, shown key, shown value).
const FILTERS: [(&str, &str, &str, &str, &str); 4] = [
    ("from:", "search/user", "search.f_from", "search.k_from", "search.v_user"),
    ("in:", "hash", "search.f_in", "search.k_in", "search.v_channel"),
    ("has:", "search/attachment", "search.f_has", "search.k_has", "search.v_has"),
    ("mentions:", "search/at", "search.f_mentions", "search.k_mentions", "search.v_user"),
];

/// Behind "More filters": dates and `pinned:`.
const MORE: [(&str, &str); 4] = [
    ("before:", "search.hint.date"),
    ("after:", "search.hint.date"),
    ("during:", "search.hint.date"),
    ("pinned:", "search.hint.bool"),
];

fn result_card(app: &PhoenixApp, ix: usize, m: &Message, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let author = app.users.get(&m.author);
    let ch = app.channels.get(&m.channel);
    let ch_name: SharedString = ch.map(|c| c.name.clone()).unwrap_or_else(|| crate::t!("common.unknown_lower"));
    let channel = m.channel;
    let mut content: String = m.content.chars().take(400).collect();
    if content.is_empty() && !m.attachments.is_empty() {
        content = m.attachments.iter().map(|a| a.filename.to_string()).collect::<Vec<_>>().join(", ");
    }
    div()
        .flex()
        .flex_col()
        .gap(z(4.))
        .child(
            div()
                .flex()
                .items_center()
                .gap(z(4.))
                .text_size(z(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c(Palette::TEXT_MUTED))
                .child(icon(Icon::Hash, 14., c(Palette::CHANNEL_ICON)))
                .child(ch_name),
        )
        .child(
            div()
                .id(("search-hit", ix))
                .p(z(8.))
                .rounded(z(8.))
                .bg(c(Palette::BG_PRIMARY))
                .border_1()
                .border_color(c(Palette::BG_TERTIARY))
                .flex()
                .gap(z(12.))
                .cursor_pointer()
                .hover(|s| s.bg(c(Palette::BG_MSG_HOVER)))
                .on_click(cx.listener(move |a, _, _, cx| a.select_channel(channel, cx)))
                .child(avatar(author, 32.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(z(6.))
                                .child(
                                    div()
                                        .text_size(z(15.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(c(app.author_color(m.author).unwrap_or(Palette::TEXT_HEADER)))
                                        .child(author.map(|u| u.display_name.clone()).unwrap_or_else(|| crate::t!("common.unknown"))),
                                )
                                .child(div().text_size(z(12.)).text_color(c(Palette::TEXT_MUTED)).child(format_time_full(m.timestamp))),
                        )
                        .child(div().text_size(z(15.)).text_color(c(Palette::TEXT_NORMAL)).child(SharedString::from(content))),
                ),
        )
        .into_any_element()
}

pub fn render(app: &PhoenixApp, _window: &mut Window, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    let s = cx.try_global::<Search>()?.0.as_ref()?;
    let input = s.input.clone();
    let loading = s.loading;
    let error = s.error.clone();
    let page = s.page.clone();
    let req = s.request.clone();
    let more = s.more;
    let top = layout::HEADER_H;

    let body: AnyElement = if let Some(page) = page.filter(|_| req.is_some()) {
        let req = req.unwrap_or_default();
        let offset = req.offset;
        let total = page.total;
        let oldest = req.oldest_first;
        let mut list = div().flex().flex_col().gap(z(16.)).p(z(16.));
        for (i, m) in page.messages.iter().enumerate() {
            list = list.child(result_card(app, i, m, cx));
        }
        if page.messages.is_empty() {
            list = list.child(
                div().py(z(40.)).flex().justify_center().text_color(c(Palette::TEXT_MUTED)).child(crate::t!("search.no_results")),
            );
        }
        let pager = div()
            .flex()
            .items_center()
            .justify_center()
            .gap(z(16.))
            .pb(z(16.))
            .when(offset > 0, |d| {
                d.child(pager_button("search-prev", crate::t!("common.back"), cx.listener(move |a, _, _, cx| {
                    rerun(a, |r| r.offset = r.offset.saturating_sub(PAGE), cx)
                })))
            })
            .when(offset + PAGE < total, |d| {
                d.child(pager_button("search-next", crate::t!("common.next"), cx.listener(move |a, _, _, cx| rerun(a, |r| r.offset += PAGE, cx))))
            });
        div()
            .flex()
            .flex_col()
            .child(
                div()
                    .h(z(48.))
                    .px(z(16.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .bg(c(Palette::BG_SECONDARY_ALT))
                    .child(
                        div()
                            .text_size(z(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(c(Palette::TEXT_HEADER))
                            .child(if total == 1 { crate::t!("search.result_one") } else { crate::t!("search.results", n = total) }),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(z(4.))
                            .child(sort_button("sort-new", crate::t!("search.newest"), !oldest, cx.listener(|a, _, _, cx| {
                                rerun(a, |r| { r.oldest_first = false; r.offset = 0 }, cx)
                            })))
                            .child(sort_button("sort-old", crate::t!("search.oldest"), oldest, cx.listener(|a, _, _, cx| {
                                rerun(a, |r| { r.oldest_first = true; r.offset = 0 }, cx)
                            }))),
                    ),
            )
            .child(list)
            .child(pager)
            .into_any_element()
    } else {
        return Some(options_popout(input, error, loading, more, cx));
    };

    Some(
        div()
            .id("search-popout")
            .absolute()
            .top(px(top + layout::TITLE_BAR_H + 1.))
            .right_0()
            .bottom_0()
            .w(px(WIDTH))
            .bg(c(Palette::BG_SECONDARY))
            .border_l_1()
            .border_color(c(Palette::BG_TERTIARY))
            .occlude()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_none()
                    .p(z(12.))
                    .flex()
                    .items_center()
                    .gap(z(8.))
                    .child(
                        div()
                            .flex_1()
                            .h(z(32.))
                            .px(z(8.))
                            .rounded(z(4.))
                            .bg(c(Palette::BG_TERTIARY))
                            .flex()
                            .items_center()
                            .text_size(z(14.))
                            .child(div().flex_1().child(Input::new(&input).appearance(false).bordered(false)))
                            .child(icon(Icon::Search, 16., c(Palette::INTERACTIVE))),
                    )
                    .child(
                        icon_button("search-close", Icon::Close, c(Palette::INTERACTIVE), 18.)
                            .on_click(cx.listener(|a, _, w, cx| close(a, w, cx))),
                    ),
            )
            .when_some(error, |d, e| {
                d.child(div().px(z(16.)).pb(z(8.)).text_size(z(14.)).text_color(c(Palette::RED)).child(e))
            })
            .when(loading, |d| {
                d.child(div().px(z(16.)).pb(z(8.)).text_size(z(14.)).text_color(c(Palette::TEXT_MUTED)).child(crate::t!("search.searching")))
            })
            .child(div().id("search-scroll").flex_1().min_h_0().overflow_y_scroll().child(body))
            .into_any_element(),
    )
}

fn tk(i: usize) -> gpui::Hsla {
    c(crate::theme::tok(i))
}

fn named(name: &str, size: f32, color: gpui::Hsla) -> gpui::Svg {
    gpui::svg().path(crate::icons::path(name)).size(z(size)).flex_none().text_color(color)
}

/// `itemContainer_lm2_KW` > `item_`: radius 8, padding 4 8; 20px `itemIcon_`
/// (--text-muted, margin-end 8); hover --interactive-background-hover.
fn option_row(id: (&'static str, usize)) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .px(z(8.))
        .py(z(4.))
        .rounded(z(8.))
        .flex()
        .items_center()
        .cursor_pointer()
        .hover(|s| s.bg(tk(idx::INTERACTIVE_BACKGROUND_HOVER)))
}

/// `groupHeader_lm2_KW`: padding 4 8, body-xs/semibold 12/16 --text-subtle.
fn group_header(label: SharedString) -> gpui::Div {
    div()
        .px(z(8.))
        .py(z(4.))
        .flex()
        .items_center()
        .text_size(z(12.))
        .line_height(z(16.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(tk(idx::TEXT_SUBTLE))
        .child(div().flex_1().child(label))
}

/// The search options popout (dump search-popout): the header search bar turns
/// into the input, and a 356px dropdown opens under it (right edge 12px from
/// the window, top = header bottom − 4): `container_cbVzxY` padding 12 8,
/// radius 8, 1px --border-subtle, --background-surface-high, --shadow-high.
/// Groups (`groupWrapper_lm2_KW`, gap 4): "Filters" (44px rows: label
/// body-sm/medium --text-strong, then key body-sm/semibold --text-subtle + value
/// body-sm/medium --text-muted, gap 4), a `divider_` (margin 8 −8, 1px
/// --border-subtle), "History" (28px rows: search glyph + query
/// body-sm/semibold --text-strong; 18px clear-history trash).
fn options_popout(input: Entity<InputState>, error: Option<SharedString>, loading: bool, more: bool, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let history = cx.try_global::<SearchHistory>().map(|h| h.0.clone()).unwrap_or_default();
    let mut filters = div().flex().flex_col().gap(z(4.)).child(group_header(crate::t!("search.filters")));
    for (i, (k, glyph, label, key, value)) in FILTERS.into_iter().enumerate() {
        filters = filters.child(
            option_row(("search-filter", i))
                .on_click(cx.listener(move |_, _, w, cx| add_filter(k, w, cx)))
                .child(named(glyph, 20., tk(idx::TEXT_MUTED)).mr(z(8.)))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(div().text_size(z(14.)).line_height(z(18.)).font_weight(FontWeight::MEDIUM).text_color(tk(idx::TEXT_STRONG)).child(crate::t!(label)))
                        .child(
                            div()
                                .flex()
                                .gap(z(4.))
                                .text_size(z(14.))
                                .line_height(z(18.))
                                .child(div().font_weight(FontWeight::SEMIBOLD).text_color(tk(idx::TEXT_SUBTLE)).child(crate::t!(key)))
                                .child(div().font_weight(FontWeight::MEDIUM).text_color(tk(idx::TEXT_MUTED)).child(crate::t!(value))),
                        ),
                ),
        );
    }
    filters = filters.child(
        option_row(("search-filter", FILTERS.len()))
            .on_click(cx.listener(|_, _, _, cx| {
                if let Some(s) = cx.global_mut::<Search>().0.as_mut() {
                    s.more = !s.more;
                }
                cx.notify();
            }))
            .child(named("inbox/filter", 20., tk(idx::TEXT_MUTED)).mr(z(8.)))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .text_size(z(14.))
                    .line_height(z(18.))
                    .font_weight(FontWeight::MEDIUM)
                    .child(div().text_color(tk(idx::TEXT_STRONG)).child(crate::t!("search.f_more")))
                    .child(div().text_color(tk(idx::TEXT_MUTED)).child(crate::t!("search.v_more"))),
            ),
    );
    if more {
        for (i, (k, hint)) in MORE.into_iter().enumerate() {
            filters = filters.child(
                option_row(("search-more", i))
                    .gap(z(4.))
                    .on_click(cx.listener(move |_, _, w, cx| add_filter(k, w, cx)))
                    .child(div().w(z(20.)).mr(z(4.)))
                    .child(div().text_size(z(14.)).line_height(z(18.)).font_weight(FontWeight::SEMIBOLD).text_color(tk(idx::TEXT_SUBTLE)).child(k))
                    .child(div().text_size(z(14.)).line_height(z(18.)).font_weight(FontWeight::MEDIUM).text_color(tk(idx::TEXT_MUTED)).child(crate::t!(hint))),
            );
        }
    }
    let mut body = div().flex().flex_col().child(filters);
    if !history.is_empty() {
        let mut group = div().flex().flex_col().gap(z(4.)).child(
            group_header(crate::t!("search.history")).child(
                div()
                    .id("search-clear-history")
                    .cursor_pointer()
                    .child(named("chat/trash", 18., tk(idx::TEXT_SUBTLE)))
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.default_global::<SearchHistory>().0.clear();
                        cx.notify();
                    })),
            ),
        );
        for (i, q) in history.into_iter().enumerate() {
            let text = q.clone();
            group = group.child(
                option_row(("search-history", i))
                    .on_click(cx.listener(move |a, _, w, cx| {
                        let Some(inp) = cx.global::<Search>().0.as_ref().map(|s| s.input.clone()) else { return };
                        inp.update(cx, |s, cx| s.set_value(text.clone(), w, cx));
                        submit(a, cx);
                    }))
                    .child(named("search", 20., tk(idx::TEXT_MUTED)).mr(z(8.)))
                    .child(div().mr(z(8.)).text_size(z(14.)).line_height(z(18.)).font_weight(FontWeight::SEMIBOLD).text_color(tk(idx::TEXT_STRONG)).child(SharedString::from(q))),
            );
        }
        body = body.child(div().my(z(8.)).mx(z(-8.)).h(z(1.)).bg(tk(idx::BORDER_SUBTLE))).child(group);
    }
    if let Some(e) = error {
        body = body.child(div().px(z(8.)).pt(z(8.)).text_size(z(14.)).text_color(tk(idx::TEXT_FEEDBACK_CRITICAL)).child(e));
    }
    if loading {
        body = body.child(div().px(z(8.)).pt(z(8.)).text_size(z(14.)).text_color(tk(idx::TEXT_MUTED)).child(crate::t!("search.searching")));
    }
    // The header `searchBar_` (244×32, radius 8, --input-background-default,
    // 1px --input-border-default; editor padding 0 6, 14/22 medium) as a live input.
    let bar = div()
        .id("search-bar-live")
        .absolute()
        .top(z(layout::TITLE_BAR_H + 8.))
        .right(z(8.))
        .w(z(244.))
        .h(z(32.))
        .rounded(z(8.))
        // The input token is translucent: lay it over the header background.
        .bg(c(Palette::BG_PRIMARY))
        .border_1()
        .border_color(tk(idx::INPUT_BORDER_ACTIVE))
        .occlude()
        .flex()
        .items_center()
        .child(div().flex_1().min_w_0().px(z(6.)).text_size(z(14.)).font_weight(FontWeight::MEDIUM).child(Input::new(&input).appearance(false).bordered(false)))
        .child(
            div()
                .id("search-bar-close")
                .size(z(24.))
                .p(z(4.))
                .flex()
                .items_center()
                .cursor_pointer()
                .on_click(cx.listener(|a, _, w, cx| close(a, w, cx)))
                .child(named("close", 16., tk(idx::ICON_DEFAULT))),
        );
    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .child(bar)
        .child(
            div()
                .id("search-popout")
                .absolute()
                .top(z(layout::TITLE_BAR_H + layout::HEADER_H - 3.))
                .right(z(12.))
                .w(z(356.))
                .px(z(8.))
                .py(z(12.))
                .rounded(z(8.))
                .border_1()
                .border_color(tk(idx::BORDER_SUBTLE))
                .bg(tk(idx::BACKGROUND_SURFACE_HIGH))
                .shadow(crate::ui::inbox::popout_shadow())
                .occlude()
                .child(body),
        )
        .into_any_element()
}

fn pager_button(
    id: &'static str,
    label: SharedString,
    f: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .px(z(16.))
        .py(z(6.))
        .rounded(z(4.))
        .bg(c(Palette::BG_MOD_SELECTED))
        .text_size(z(14.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(c(Palette::TEXT_HEADER))
        .cursor_pointer()
        .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
        .on_click(f)
        .child(label)
}

fn sort_button(
    id: &'static str,
    label: SharedString,
    active: bool,
    f: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .px(z(8.))
        .py(z(2.))
        .rounded(z(4.))
        .text_size(z(14.))
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .text_color(c(if active { Palette::TEXT_HEADER } else { Palette::INTERACTIVE }))
        .when(active, |d| d.bg(c(Palette::BG_MOD_SELECTED)))
        .on_click(f)
        .child(label)
}
