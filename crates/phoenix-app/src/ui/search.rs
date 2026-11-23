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
    theme::{Palette, c, m as layout},
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
    _sub: Subscription,
}

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
        Ok(req) => run(app, req, cx),
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

const FILTERS: [(&str, &str); 7] = [
    ("from:", "search.hint.user"),
    ("mentions:", "search.hint.user"),
    ("has:", "search.hint.has"),
    ("in:", "search.hint.channel"),
    ("before:", "search.hint.date"),
    ("after:", "search.hint.date"),
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
        let mut help = div().flex().flex_col().p(z(16.)).gap(z(2.)).child(
            div()
                .pb(z(8.))
                .text_size(z(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c(Palette::TEXT_MUTED))
                .child(crate::t!("search.options")),
        );
        for (i, (k, hint)) in FILTERS.iter().enumerate() {
            let k: &'static str = k;
            help = help.child(
                div()
                    .id(("search-filter", i))
                    .h(z(32.))
                    .px(z(8.))
                    .rounded(z(4.))
                    .flex()
                    .items_center()
                    .gap(z(6.))
                    .cursor_pointer()
                    .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
                    .on_click(cx.listener(move |_, _, w, cx| add_filter(k, w, cx)))
                    .child(div().text_size(z(14.)).font_weight(FontWeight::SEMIBOLD).text_color(c(Palette::TEXT_HEADER)).child(k))
                    .child(div().text_size(z(14.)).text_color(c(Palette::TEXT_MUTED)).child(crate::t!(hint))),
            );
        }
        help.into_any_element()
    };

    Some(
        div()
            .id("search-popout")
            .absolute()
            .top(px(top))
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
