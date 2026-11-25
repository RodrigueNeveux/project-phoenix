use std::cell::{Cell, RefCell};

use gpui::{AnyElement, Context, FontWeight, Hsla, IntoElement, ParentElement, SharedString, Styled, Window, div, img, prelude::*, rgb};
use serde_json::Value;

use crate::{
    app::PhoenixApp,
    theme::{c, generated::idx, tok, z},
};

pub const HEADER_H: f32 = 49.;
pub const HERO_H: f32 = 348.;
pub const CARD_MIN_W: f32 = 280.;
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
        .overflow_hidden()
        .bg(tk(idx::BACKGROUND_BASE_LOW))
        .border_1()
        .border_color(tk(idx::BORDER_SUBTLE))
        .cursor_pointer()
        .hover(|s| s.bg(tk(idx::BACKGROUND_SURFACE_HIGH)).border_color(tk(idx::BORDER_STRONG)).shadow_sm())
        .child(banner)
        .child(icon)
        .child(details)
        .into_any_element()
}

fn header_bar(icon: &'static str, cats: &[&'static str]) -> AnyElement {
    let mut tabs = div().h_full().flex().items_center().gap(z(30.));
    for (n, k) in cats.iter().copied().enumerate() {
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
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(if on { tk(idx::TEXT_DEFAULT) } else { tk(idx::TEXT_SUBTLE) })
                .hover(|s| s.text_color(tk(idx::TEXT_STRONG)))
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
        .child(gpui::svg().path(crate::icons::path(icon)).flex_none().size(z(24.)).text_color(tk(idx::INTERACTIVE_TEXT_ACTIVE)))
        .child(div().flex_1().min_w_0().h_full().child(tabs))
        .child(search)
        .into_any_element()
}

fn hero(lines: &[SharedString], desc: SharedString) -> AnyElement {
    let mut title = div().max_w(z(740.)).flex().flex_col();
    for l in lines {
        title = title.child(text(44., 44., FontWeight::BOLD, tk(idx::TEXT_STRONG), l.to_uppercase()));
    }
    div()
        .relative()
        .w_full()
        .h(z(HERO_H))
        .overflow_hidden()
        .bg(gpui::black())
        .child(div().absolute().inset_0().opacity(0.7).bg(gpui::linear_gradient(
            110.,
            gpui::linear_color_stop(rgb(0x2f72c1), 0.),
            gpui::linear_color_stop(gpui::Hsla::from(rgb(0x25229a)).opacity(0.8), 0.36),
        )))
        .child(
            div()
                .absolute()
                .top(z(60.))
                .bottom_0()
                .left_0()
                .pl(z(32.))
                .flex()
                .flex_col()
                .justify_center()
                .child(title)
                .child(text(16., 20., FontWeight::MEDIUM, tk(idx::TEXT_MUTED), desc).mt(z(16.)).max_w(z(600.))),
        )
        .into_any_element()
}

pub fn page(app: &PhoenixApp, window: &mut Window, cx: &mut Context<PhoenixApp>) -> AnyElement {
    if current() == Some(Tab::Apps) {
        return apps_page(app, window, cx);
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
            .child(hero(&[crate::t!("discovery.hero_title")], crate::t!("discovery.hero_desc")))
            .child(
                div()
                    .pt(z(24.))
                    .pl(z(32.))
                    .h(z(48.))
                    .child(text(20., 24., FontWeight::SEMIBOLD, tk(idx::TEXT_STRONG), crate::t!("discovery.popular"))),
            )
            .child(grid)
    });
    let cats = ["discovery.cat.home", "discovery.cat.gaming", "discovery.cat.music", "discovery.cat.entertainment", "discovery.cat.science", "discovery.cat.education", "discovery.cat.hubs"];
    div().size_full().relative().bg(tk(idx::BACKGROUND_BASE_LOWEST)).child(body).child(header_bar("server/server", &cats)).into_any_element()
}

#[derive(Clone, Debug, PartialEq)]
pub struct DirectoryApp {
    pub id: u64,
    pub name: SharedString,
    pub description: SharedString,
    pub icon: Option<SharedString>,
    pub banner: Option<SharedString>,
    pub category: Option<SharedString>,
    pub promoted: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Collection {
    pub title: SharedString,
    pub apps: Vec<DirectoryApp>,
}

pub const PROMOTED_FLAG: u64 = 1 << 29;

pub fn parse_collections(v: &Value) -> Vec<Collection> {
    let mut list: Vec<&Value> = v.as_array().map(|a| a.iter().collect()).unwrap_or_default();
    list.sort_by_key(|c| c["position"].as_i64().unwrap_or(i64::MAX));
    list.into_iter()
        .filter_map(|c| {
            let mut items: Vec<&Value> = c["application_directory_collection_items"].as_array()?.iter().filter(|i| i["type"].as_u64() == Some(1)).collect();
            items.sort_by_key(|i| i["position"].as_i64().unwrap_or(i64::MAX));
            let apps: Vec<DirectoryApp> = items
                .into_iter()
                .filter_map(|i| {
                    let a = &i["application"];
                    let id: u64 = a["id"].as_str()?.parse().ok()?;
                    let s = |v: &Value| v.as_str().filter(|x| !x.is_empty()).map(str::to_owned);
                    let bot = &a["bot"];
                    let banner = match (s(&bot["id"]), s(&bot["banner"])) {
                        (Some(b), Some(h)) => Some(cdn(format!("banners/{b}/{h}.png?size=600"))),
                        _ => s(&a["cover_image"]).map(|h| cdn(format!("app-icons/{id}/{h}.png?size=600"))),
                    };
                    Some(DirectoryApp {
                        id,
                        name: s(&a["name"]).unwrap_or_default().into(),
                        description: s(&a["description"]).unwrap_or_default().into(),
                        icon: s(&a["icon"]).map(|h| cdn(format!("app-icons/{id}/{h}.png?size=64"))),
                        banner,
                        category: a["categories"].get(0).and_then(|c| s(&c["name"])).map(Into::into),
                        promoted: a["flags"].as_u64().unwrap_or(0) & PROMOTED_FLAG != 0,
                    })
                })
                .collect();
            (!apps.is_empty()).then(|| Collection { title: s_of(&c["title"]), apps })
        })
        .collect()
}

fn s_of(v: &Value) -> SharedString {
    v.as_str().unwrap_or_default().to_owned().into()
}

pub fn app_columns(window_w: f32) -> usize {
    if window_w >= 1200. {
        4
    } else if window_w >= 800. {
        2
    } else {
        1
    }
}

thread_local! {
    static APPS: RefCell<(bool, Option<Vec<Collection>>)> = const { RefCell::new((false, None)) };
}

fn ensure_apps(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) {
    if APPS.with(|a| std::mem::replace(&mut a.borrow_mut().0, true)) {
        return;
    }
    let fut = app.backend.app_directory_collections(crate::i18n::locale().to_string());
    cx.spawn(async move |this, cx| {
        let v = fut.await.unwrap_or(Value::Null);
        APPS.with(|a| a.borrow_mut().1 = Some(parse_collections(&v)));
        let _ = this.update(cx, |_, cx| cx.notify());
    })
    .detach();
}

fn app_card(a: &DirectoryApp) -> AnyElement {
    let banner = div()
        .relative()
        .w_full()
        .h(z(140.))
        .overflow_hidden()
        .bg(tk(idx::BACKGROUND_SURFACE_HIGH))
        .children(a.banner.clone().map(|b| img(b).size_full().object_fit(gpui::ObjectFit::Cover)));
    let icon = div()
        .absolute()
        .left(z(12.))
        .top(z(140. - 28.))
        .size(z(56.))
        .p(z(4.))
        .rounded(z(12.))
        .bg(tk(idx::BACKGROUND_BASE_LOW))
        .child(match a.icon.clone() {
            Some(u) => img(u).size_full().rounded(z(8.)).into_any_element(),
            None => div().size_full().rounded(z(8.)).bg(tk(idx::BACKGROUND_BASE_LOWEST)).into_any_element(),
        });
    let mut info = div().mt(z(4.)).flex().gap(z(4.));
    let has_info = a.category.is_some() || a.promoted;
    if let Some(c) = a.category.clone() {
        info = info.child(text(12., 16., FontWeight::MEDIUM, tk(idx::TEXT_DEFAULT), c));
    }
    if a.category.is_some() && a.promoted {
        info = info.child(text(12., 16., FontWeight::MEDIUM, tk(idx::TEXT_SUBTLE), "\u{2022}"));
    }
    if a.promoted {
        info = info.child(text(12., 16., FontWeight::MEDIUM, tk(idx::TEXT_SUBTLE), crate::t!("discovery.promoted")));
    }
    div()
        .id(("discovery-app", a.id))
        .relative()
        .min_w_0()
        .h(z(275.))
        .flex()
        .flex_col()
        .rounded(z(8.))
        .overflow_hidden()
        .bg(tk(idx::BACKGROUND_BASE_LOW))
        .border_1()
        .border_color(tk(idx::BORDER_SUBTLE))
        .cursor_pointer()
        .hover(|s| s.bg(tk(idx::BACKGROUND_SURFACE_HIGH)).border_color(tk(idx::BORDER_STRONG)).shadow_sm())
        .child(banner)
        .child(div().h(z(28.)))
        .child(icon)
        .child(
            div()
                .flex_1()
                .min_h_0()
                .pt(z(4.))
                .px(z(16.))
                .pb(z(16.))
                .flex()
                .flex_col()
                .child(text(16., 20., FontWeight::SEMIBOLD, tk(idx::TEXT_STRONG), a.name.clone()).overflow_hidden().text_ellipsis().whitespace_nowrap())
                .when(has_info, |d| d.child(info))
                .child(text(14., 18., FontWeight::MEDIUM, tk(idx::TEXT_DEFAULT), a.description.clone()).mt(z(8.)).max_h(z(36.)).overflow_hidden()),
        )
        .into_any_element()
}

fn apps_page(app: &PhoenixApp, window: &mut Window, cx: &mut Context<PhoenixApp>) -> AnyElement {
    ensure_apps(app, cx);
    let cols = app_columns(f32::from(window.viewport_size().width) / crate::theme::zoom());
    let colls = APPS.with(|a| a.borrow().1.clone());
    let mut body = div().flex().flex_col().pb(z(16.));
    for (n, c) in colls.unwrap_or_default().iter().enumerate() {
        body = body.child(div().pt(z(if n == 0 { 24. } else { 16. })).pl(z(32.)).child(text(20., 24., FontWeight::SEMIBOLD, tk(idx::TEXT_STRONG), c.title.clone())));
        let mut grid = div().px(z(32.)).py(z(16.)).grid().grid_cols(cols as u16).gap(z(16.));
        for a in &c.apps {
            grid = grid.child(app_card(a));
        }
        body = body.child(grid);
    }
    let scroll = APP_SCROLL.with(|s| {
        div()
            .id("discovery-apps-scroll")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(s)
            .pr(z(8.))
            .child(hero(&[crate::t!("discovery.apps_title1"), crate::t!("discovery.apps_title2")], crate::t!("discovery.apps_desc")))
            .child(body)
    });
    let cats = ["discovery.apps_cat.featured", "discovery.apps_cat.games", "discovery.apps_cat.entertainment", "discovery.apps_cat.moderation", "discovery.apps_cat.social", "discovery.apps_cat.utilities"];
    div().size_full().relative().bg(tk(idx::BACKGROUND_BASE_LOWEST)).child(scroll).child(header_bar("apps", &cats)).into_any_element()
}

thread_local! {
    static APP_SCROLL: gpui::ScrollHandle = gpui::ScrollHandle::new();
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
    fn parses_app_directory_collections() {
        let v = json!([
            {"title": "Puzzle Games", "position": 2, "application_directory_collection_items": [
                {"type": 1, "position": 1, "application": {"id": "7", "name": "Wordle", "icon": "ic", "description": "d", "flags": 11665410, "categories": [{"id": 6, "name": "Jeux"}], "cover_image": "cv", "bot": {"id": "7", "banner": null}}}
            ]},
            {"title": "Promoted Games", "position": 1, "application_directory_collection_items": [
                {"type": 1, "position": 2, "application": {"id": "9", "name": "Werwulf", "flags": 537002498, "categories": [], "bot": {"id": "9", "banner": "bn"}}},
                {"type": 2, "position": 1, "application": {"id": "8", "name": "skip"}},
                {"type": 1, "position": 1, "application": {"id": "5", "name": "EXOPLANET", "flags": 537002498, "categories": [{"name": "Jeux"}]}}
            ]},
            {"title": "Empty", "position": 3, "application_directory_collection_items": []}
        ]);
        let c = parse_collections(&v);
        assert_eq!(c.iter().map(|x| x.title.as_ref()).collect::<Vec<_>>(), ["Promoted Games", "Puzzle Games"]);
        assert_eq!(c[0].apps.iter().map(|a| a.name.as_ref()).collect::<Vec<_>>(), ["EXOPLANET", "Werwulf"]);
        assert!(c[0].apps[1].promoted && c[0].apps[1].category.is_none());
        assert_eq!(c[0].apps[1].banner.as_deref(), Some("https://cdn.discordapp.com/banners/9/bn.png?size=600"));
        let w = &c[1].apps[0];
        assert!(!w.promoted);
        assert_eq!(w.category.as_deref(), Some("Jeux"));
        assert_eq!(w.banner.as_deref(), Some("https://cdn.discordapp.com/app-icons/7/cv.png?size=600"));
        assert_eq!(w.icon.as_deref(), Some("https://cdn.discordapp.com/app-icons/7/ic.png?size=64"));
        assert_eq!((app_columns(1536.), app_columns(1000.), app_columns(700.)), (4, 2, 1));
    }

    #[test]
    fn three_columns_in_the_measured_width() {
        assert_eq!(columns(1111.), 3);
        assert_eq!(columns(760.), 2);
        assert_eq!(columns(300.), 1);
    }
}
