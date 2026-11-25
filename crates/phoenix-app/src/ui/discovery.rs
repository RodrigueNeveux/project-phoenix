use std::cell::{Cell, RefCell};

use gpui::{AnyElement, Context, FontWeight, Hsla, IntoElement, ParentElement, SharedString, Styled, Window, div, img, prelude::*, rgb};
use serde_json::Value;

use crate::{
    app::PhoenixApp,
    theme::{c, generated::idx, tok, z},
};

pub const HEADER_H: f32 = 49.;
pub const HERO_H: f32 = 348.;
pub const CARD_MIN_W: f32 = 320.;
pub const CARD_H: f32 = 322.;
pub const GRID_GAP: f32 = 20.;
pub const GRID_PAD_X: f32 = 24.;
pub const BANNER_H: f32 = 146.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Apps,
    Servers,
}

pub const TABS: [Tab; 2] = [Tab::Apps, Tab::Servers];

#[derive(Clone, Debug, PartialEq)]
pub struct DiscoverableGuild {
    pub id: u64,
    pub name: SharedString,
    pub description: SharedString,
    pub icon: Option<SharedString>,
    pub banner: Option<SharedString>,
    pub online: u64,
    pub members: u64,
    pub verified: bool,
    pub partnered: bool,
}

#[derive(Default)]
struct State {
    guilds: Option<Vec<DiscoverableGuild>>,
    requested: bool,
}

thread_local! {
    static OPEN: Cell<Option<Tab>> = const { Cell::new(None) };
    static STATE: RefCell<State> = RefCell::new(State::default());
    static SCROLL: gpui::ScrollHandle = gpui::ScrollHandle::new();
}

pub fn open(tab: Tab) {
    OPEN.with(|o| o.set(Some(tab)));
}

pub fn close() {
    OPEN.with(|o| o.set(None));
}

pub fn current() -> Option<Tab> {
    OPEN.with(|o| o.get())
}

fn cdn(path: String) -> SharedString {
    format!("https://cdn.discordapp.com/{path}").into()
}

pub fn parse_guilds(v: &Value) -> Vec<DiscoverableGuild> {
    let list = v.get("guilds").and_then(Value::as_array).or_else(|| v.as_array()).cloned().unwrap_or_default();
    list.iter()
        .filter_map(|g| {
            let id = g["id"].as_str()?.parse().ok()?;
            let s = |k: &str| g[k].as_str().filter(|x| !x.is_empty()).map(str::to_owned);
            let features: Vec<&str> = g["features"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
            let ext = |h: &str| if h.starts_with("a_") { "gif" } else { "webp" };
            Some(DiscoverableGuild {
                id,
                name: s("name").unwrap_or_default().into(),
                description: s("description").unwrap_or_default().into(),
                icon: s("icon").map(|h| cdn(format!("icons/{id}/{h}.{}?size=128", ext(&h)))),
                banner: s("discovery_splash").or_else(|| s("splash")).map(|h| cdn(format!("discovery-splashes/{id}/{h}.jpg?size=600"))),
                online: g["approximate_presence_count"].as_u64().unwrap_or(0),
                members: g["approximate_member_count"].as_u64().unwrap_or(0),
                verified: features.contains(&"VERIFIED"),
                partnered: features.contains(&"PARTNERED"),
            })
        })
        .collect()
}

pub fn group_digits(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('\u{202f}');
        }
        out.push(ch);
    }
    out
}

pub fn columns(width: f32) -> usize {
    (((width - 2. * GRID_PAD_X + GRID_GAP) / (CARD_MIN_W + GRID_GAP)).floor() as usize).max(1)
}

fn ensure_loaded(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) {
    let go = STATE.with(|s| {
        let mut s = s.borrow_mut();
        let go = !s.requested;
        s.requested = true;
        go
    });
    if !go {
        return;
    }
    let fut = app.backend.discoverable_guilds(None, 0);
    cx.spawn(async move |this, cx| {
        let v = fut.await.unwrap_or(Value::Null);
        STATE.with(|s| s.borrow_mut().guilds = Some(parse_guilds(&v)));
        let _ = this.update(cx, |_, cx| cx.notify());
    })
    .detach();
}

fn tk(i: usize) -> Hsla {
    c(tok(i))
}

fn text(size: f32, line: f32, weight: FontWeight, color: Hsla, s: impl Into<SharedString>) -> gpui::Div {
    div().text_size(z(size)).line_height(z(line)).font_weight(weight).text_color(color).child(s.into())
}

fn tab_label(t: Tab) -> SharedString {
    match t {
        Tab::Apps => crate::t!("discovery.apps"),
        Tab::Servers => crate::t!("discovery.servers"),
    }
}

fn tab_icon(t: Tab) -> &'static str {
    match t {
        Tab::Apps => "apps",
        Tab::Servers => "server/server",
    }
}

pub fn sidebar(cx: &mut Context<PhoenixApp>) -> AnyElement {
    let sel = current();
    let mut nav = div().p(z(8.)).flex_1().flex().flex_col().gap(z(2.));
    for t in TABS {
        let on = sel == Some(t);
        let col = if on { tk(idx::INTERACTIVE_TEXT_ACTIVE) } else { tk(idx::INTERACTIVE_TEXT_DEFAULT) };
        nav = nav.child(
            div()
                .id(("discovery-tab", t as usize))
                .h(z(56.))
                .px(z(10.))
                .rounded(z(8.))
                .flex()
                .items_center()
                .gap(z(12.))
                .cursor_pointer()
                .text_color(col)
                .when(on, |d| d.bg(tk(idx::INTERACTIVE_BACKGROUND_ACTIVE)))
                .when(!on, |d| d.hover(|s| s.bg(tk(idx::INTERACTIVE_BACKGROUND_HOVER)).text_color(tk(idx::INTERACTIVE_TEXT_HOVER))))
                .on_click(cx.listener(move |_, _, _, cx| {
                    open(t);
                    cx.notify();
                }))
                .child(gpui::svg().path(crate::icons::path(tab_icon(t))).flex_none().size(z(24.)).text_color(col))
                .child(text(16., 20., FontWeight::MEDIUM, col, tab_label(t))),
        );
    }
    div()
        .size_full()
        .flex()
        .flex_col()
        .bg(tk(idx::BACKGROUND_BASE_LOWEST))
        .child(
            div()
                .flex_none()
                .h(z(HEADER_H))
                .p(z(16.))
                .flex()
                .items_center()
                .border_b_1()
                .border_color(tk(idx::BORDER_SUBTLE))
                .child(text(18., 22., FontWeight::SEMIBOLD, tk(idx::TEXT_STRONG), crate::t!("discovery.title"))),
        )
        .child(nav)
        .into_any_element()
}

fn verified_badge(partner: bool) -> AnyElement {
    div()
        .size(z(16.))
        .flex_none()
        .rounded_full()
        .bg(if partner { tk(idx::BRAND_500) } else { rgb(0x3d9e60).into() })
        .flex()
        .items_center()
        .justify_center()
        .child(gpui::svg().path(crate::icons::path("check")).size(z(10.)).text_color(gpui::white()))
        .into_any_element()
}

fn card(g: &DiscoverableGuild) -> AnyElement {
    let banner = div().relative().h(z(BANNER_H)).w_full().overflow_hidden().rounded_t(z(8.)).bg(tk(idx::BACKGROUND_SURFACE_HIGH)).children(g.banner.clone().map(|b| img(b).size_full().object_fit(gpui::ObjectFit::Cover)));
    let icon = div()
        .absolute()
        .left(z(12.))
        .top(z(BANNER_H - 33.))
        .size(z(56.))
        .p(z(4.))
        .rounded(z(16.))
        .bg(tk(idx::BACKGROUND_BASE_LOWER))
        .child(match g.icon.clone() {
            Some(u) => img(u).size_full().rounded(z(12.)).into_any_element(),
            None => div().size_full().rounded(z(12.)).bg(tk(idx::BACKGROUND_SURFACE_HIGH)).flex().items_center().justify_center().child(text(16., 20., FontWeight::SEMIBOLD, tk(idx::TEXT_DEFAULT), crate::model::acronym(&g.name))).into_any_element(),
        });
    let dot = |col: Hsla| div().size(z(8.)).rounded_full().flex_none().bg(col);
    let count = |col: Hsla, label: SharedString| div().flex().items_center().gap(z(4.)).child(dot(col)).child(text(12., 16., FontWeight::NORMAL, tk(idx::TEXT_DEFAULT), label));
    let details = div()
        .flex_1()
        .min_h_0()
        .pt(z(28.))
        .px(z(16.))
        .pb(z(16.))
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .items_center()
                .gap(z(4.))
                .when(g.verified || g.partnered, |d| d.child(verified_badge(g.partnered && !g.verified)))
                .child(text(16., 20., FontWeight::SEMIBOLD, tk(idx::TEXT_STRONG), g.name.clone()).min_w_0().overflow_hidden().text_ellipsis().whitespace_nowrap()),
        )
        .child(text(14., 18., FontWeight::NORMAL, tk(idx::TEXT_DEFAULT), g.description.clone()).mt(z(4.)).max_h(z(54.)).overflow_hidden())
        .child(div().flex_1())
        .child(
            div()
                .flex()
                .items_center()
                .gap(z(16.))
                .child(count(rgb(0x3d9e60).into(), crate::t!("discovery.online", n = group_digits(g.online))))
                .child(count(tk(idx::TEXT_MUTED), crate::t!("discovery.members", n = group_digits(g.members)))),
        );
    div()
        .id(("discovery-card", g.id))
        .relative()
        .h(z(CARD_H))
        .min_w_0()
        .flex()
        .flex_col()
        .rounded(z(8.))
        .bg(tk(idx::BACKGROUND_BASE_LOWER))
        .border_1()
        .border_color(tk(idx::BORDER_SUBTLE))
        .cursor_pointer()
        .hover(|s| s.bg(tk(idx::BACKGROUND_SURFACE_HIGH)))
        .child(banner)
        .child(icon)
        .child(details)
        .into_any_element()
}

fn header_bar() -> AnyElement {
    let cats = ["discovery.cat.home", "discovery.cat.gaming", "discovery.cat.music", "discovery.cat.entertainment", "discovery.cat.science", "discovery.cat.education", "discovery.cat.hubs"];
    let mut tabs = div().h_full().flex().items_center().gap(z(30.));
    for (n, k) in cats.into_iter().enumerate() {
        let on = n == 0;
        tabs = tabs.child(
            div()
                .id(("discovery-cat", n))
                .h_full()
                .relative()
                .flex()
                .items_center()
                .cursor_pointer()
                .text_size(z(14.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(if on { tk(idx::TEXT_DEFAULT) } else { tk(idx::INTERACTIVE_TEXT_DEFAULT) })
                .hover(|s| s.text_color(tk(idx::INTERACTIVE_TEXT_HOVER)))
                .child(crate::t!(k))
                .when(on, |d| d.child(div().absolute().left_0().right_0().bottom_0().h(z(2.)).bg(tk(idx::BRAND_500)))),
        );
    }
    let search = div()
        .flex_none()
        .w(z(240.))
        .h(z(32.))
        .px(z(4.))
        .rounded(z(8.))
        .bg(gpui::hsla(0., 0., 0., 0.12))
        .border_1()
        .border_color(tk(idx::INPUT_BORDER_DEFAULT))
        .flex()
        .items_center()
        .gap(z(4.))
        .child(div().size(z(22.)).flex().items_center().justify_center().child(gpui::svg().path(crate::icons::path("search")).size(z(16.)).text_color(tk(idx::TEXT_MUTED))))
        .child(text(14., 18., FontWeight::NORMAL, tk(idx::TEXT_MUTED), crate::t!("discovery.search")));
    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .h(z(HEADER_H))
        .px(z(16.))
        .flex()
        .items_center()
        .gap(z(20.))
        .border_b_1()
        .border_color(tk(idx::BORDER_SUBTLE))
        .child(gpui::svg().path(crate::icons::path("server/server")).flex_none().size(z(24.)).text_color(tk(idx::INTERACTIVE_TEXT_ACTIVE)))
        .child(div().flex_1().min_w_0().h_full().child(tabs))
        .child(search)
        .into_any_element()
}

fn hero() -> AnyElement {
    div()
        .relative()
        .w_full()
        .h(z(HERO_H))
        .overflow_hidden()
        .child(div().absolute().inset_0().bg(gpui::linear_gradient(
            115.,
            gpui::linear_color_stop(rgb(0x1a2a6c), 0.),
            gpui::linear_color_stop(rgb(0x0b1033), 1.),
        )))
        .child(
            div()
                .absolute()
                .top(z(HEADER_H + 11. + 72.))
                .left_0()
                .pl(z(32.))
                .max_w(z(772.))
                .flex()
                .flex_col()
                .gap(z(16.))
                .child(text(44., 44., FontWeight::BOLD, tk(idx::TEXT_STRONG), crate::t!("discovery.hero_title").to_uppercase()))
                .child(text(16., 20., FontWeight::MEDIUM, tk(idx::TEXT_MUTED), crate::t!("discovery.hero_desc")).max_w(z(600.))),
        )
        .into_any_element()
}

pub fn page(app: &PhoenixApp, window: &mut Window, cx: &mut Context<PhoenixApp>) -> AnyElement {
    if current() == Some(Tab::Apps) {
        return div()
            .size_full()
            .bg(tk(idx::BACKGROUND_BASE_LOWEST))
            .flex()
            .items_center()
            .justify_center()
            .child(text(16., 20., FontWeight::MEDIUM, tk(idx::TEXT_MUTED), crate::t!("discovery.apps")))
            .into_any_element();
    }
    ensure_loaded(app, cx);
    let guilds = STATE.with(|s| s.borrow().guilds.clone());
    let vw = f32::from(window.viewport_size().width) / crate::theme::zoom() - crate::ui::sidebar_resize::width() - 72.;
    let cols = columns(vw);
    let mut grid = div().px(z(GRID_PAD_X)).py(z(20.)).grid().grid_cols(cols as u16).gap(z(GRID_GAP));
    match &guilds {
        Some(list) => {
            for g in list.iter() {
                grid = grid.child(card(g));
            }
        }
        None => {
            for _ in 0..6 {
                grid = grid.child(div().h(z(CARD_H)).rounded(z(8.)).bg(tk(idx::BACKGROUND_BASE_LOWER)));
            }
        }
    }
    let body = SCROLL.with(|s| {
        div()
            .id("discovery-scroll")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(s)
            .pr(z(8.))
            .child(hero())
            .child(
                div()
                    .pt(z(24.))
                    .pl(z(32.))
                    .h(z(48.))
                    .child(text(20., 24., FontWeight::SEMIBOLD, tk(idx::TEXT_STRONG), crate::t!("discovery.popular"))),
            )
            .child(grid)
    });
    div().size_full().relative().bg(tk(idx::BACKGROUND_BASE_LOWEST)).child(body).child(header_bar()).into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_discoverable_guilds() {
        let v = json!({"total": 2, "guilds": [
            {"id": "11", "name": "A", "description": "d", "icon": "ab", "discovery_splash": "sp", "splash": "x", "features": ["VERIFIED"], "approximate_presence_count": 941116, "approximate_member_count": 4625125},
            {"id": "12", "name": "B", "description": null, "icon": null, "splash": "s2", "features": ["PARTNERED"]}
        ]});
        let g = parse_guilds(&v);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].icon.as_deref(), Some("https://cdn.discordapp.com/icons/11/ab.webp?size=128"));
        assert_eq!(g[0].banner.as_deref(), Some("https://cdn.discordapp.com/discovery-splashes/11/sp.jpg?size=600"));
        assert!(g[0].verified && !g[0].partnered);
        assert_eq!(g[1].banner.as_deref(), Some("https://cdn.discordapp.com/discovery-splashes/12/s2.jpg?size=600"));
        assert!(g[1].partnered && g[1].icon.is_none());
        assert_eq!(group_digits(941116), "941\u{202f}116");
        assert_eq!(group_digits(18149845), "18\u{202f}149\u{202f}845");
        assert_eq!(group_digits(12), "12");
    }

    #[test]
    fn three_columns_in_the_measured_width() {
        assert_eq!(columns(1111.), 3);
        assert_eq!(columns(760.), 2);
        assert_eq!(columns(300.), 1);
    }
}
