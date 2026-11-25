pub mod analytics;
pub mod automod;
pub mod boost;
pub mod community;
pub mod delete;
pub mod integrations;
pub mod mock;
pub mod presence;
pub mod safety;
pub mod tag;
pub mod template;

use std::{cell::RefCell, collections::HashMap};

use gpui::{AnyElement, Context, FontWeight, Hsla, IntoElement, ParentElement, SharedString, Styled, div, prelude::*};
use phoenix_api::{
    models::Snowflake,
    rest::{guild_manage::AdminCall, guild_server::ServerCall},
};
use serde_json::Value;

use crate::{
    actions::{ActionReply, ActionRequest},
    app::PhoenixApp,
    model::Id,
    theme::{c, generated::idx, tok, z},
};

#[derive(Clone, Debug, PartialEq, Default)]
pub enum Sub {
    #[default]
    Root,
    RaidCaptcha,
    DmSpam,
    Automod,
    Permissions,
    Webhooks,
    App(String),
}

#[derive(Clone, Debug)]
pub enum Load {
    Pending,
    Ready(Value),
    Failed(String),
}

#[derive(Default)]
pub struct Store {
    pub data: HashMap<(Id, String), Load>,
    pub sub: HashMap<(Id, &'static str), Sub>,
    pub open: HashMap<Id, String>,
    pub flags: HashMap<(Id, String), bool>,
    pub text: HashMap<(Id, String), String>,
    pub modal: Option<(Id, &'static str)>,
    pub rule_draft: Option<(Id, phoenix_api::rest::guild_server::AutomodRule)>,
    pub menu: Option<(Id, String)>,
}

thread_local!(static STORE: RefCell<Store> = RefCell::new(Store::default()));

pub fn with<R>(f: impl FnOnce(&mut Store) -> R) -> R {
    STORE.with(|s| f(&mut s.borrow_mut()))
}

pub fn sf(id: Id) -> Snowflake {
    Snowflake(id.0)
}

pub fn sub(guild: Id, section: &'static str) -> Sub {
    with(|s| s.sub.get(&(guild, section)).cloned().unwrap_or_default())
}

pub fn set_sub(guild: Id, section: &'static str, v: Sub) {
    with(|s| {
        s.sub.insert((guild, section), v);
    });
}

pub fn flag(guild: Id, key: &str) -> Option<bool> {
    with(|s| s.flags.get(&(guild, key.to_owned())).copied())
}

pub fn set_flag(guild: Id, key: &str, v: bool) {
    with(|s| {
        s.flags.insert((guild, key.to_owned()), v);
    });
}

pub fn fetch(app: &PhoenixApp, guild: Id, call: ServerCall, cx: &mut Context<PhoenixApp>) -> Option<Value> {
    debug_assert!(call.is_read());
    let key = phoenix_api::rest::guild_server::build(&call).path;
    let k = (guild, key);
    let cur = with(|s| s.data.get(&k).cloned());
    match cur {
        Some(Load::Ready(v)) => return Some(v),
        Some(_) => return None,
        None => {}
    }
    with(|s| {
        s.data.insert(k.clone(), Load::Pending);
    });
    let fut = app.backend.action(ActionRequest::Admin(AdminCall::Server(call)));
    cx.spawn(async move |this, cx| {
        let r = fut.await;
        with(|s| {
            s.data.insert(
                k,
                match r {
                    Ok(ActionReply::Json(v)) => Load::Ready(v),
                    Ok(_) => Load::Ready(Value::Null),
                    Err(e) => Load::Failed(e.0),
                },
            );
        });
        let _ = this.update(cx, |_, cx| cx.notify());
    })
    .detach();
    None
}

pub fn failed(guild: Id, call: &ServerCall) -> Option<String> {
    let key = phoenix_api::rest::guild_server::build(call).path;
    with(|s| match s.data.get(&(guild, key)) {
        Some(Load::Failed(e)) => Some(e.clone()),
        _ => None,
    })
}

pub fn write(app: &mut PhoenixApp, call: ServerCall, ok: Option<SharedString>, cx: &mut Context<PhoenixApp>) {
    app.admin_toast(AdminCall::Server(call), ok, cx);
}

pub fn content(app: &PhoenixApp, guild: Id, key: &str, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    let page = match key {
        "menu.gs.tag" => tag::render(app, guild, cx),
        "menu.gs.boost_perks" => boost::render(app, guild, cx),
        "menu.gs.integrations" => integrations::render(app, guild, cx),
        "menu.gs.safety" => safety::render(app, guild, cx),
        "menu.gs.community" => community::render(app, guild, cx),
        "menu.gs.onboarding" => community::onboarding(app, guild, cx),
        "menu.gs.analytics" => analytics::render(app, guild, cx),
        "menu.gs.discovery" => presence::render(app, guild, cx),
        "menu.gs.templates" => template::render(app, guild, cx),
        "menu.gs.delete" => delete::render(app, guild, cx),
        _ => return None,
    };
    Some(page)
}

pub fn modal(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    let (guild, which) = with(|s| s.modal)?;
    match which {
        "delete" => Some(delete::modal(app, guild, cx)),
        "verification" => Some(safety::verification_modal(app, guild, cx)),
        _ => None,
    }
}

pub fn open_modal(guild: Id, which: &'static str) {
    with(|s| s.modal = Some((guild, which)));
}

pub fn close_modal() {
    with(|s| s.modal = None);
}

pub fn t(i: usize) -> Hsla {
    c(tok(i))
}

pub fn text(size: f32, lh: f32, w: FontWeight, col: usize, s: impl Into<SharedString>) -> gpui::Div {
    div().text_size(z(size)).line_height(z(lh)).font_weight(w).text_color(t(col)).child(s.into())
}

pub fn title(s: impl Into<SharedString>) -> gpui::Div {
    text(20., 24., FontWeight::SEMIBOLD, idx::TEXT_STRONG, s)
}

pub fn lead(s: impl Into<SharedString>) -> gpui::Div {
    text(14., 18., FontWeight::NORMAL, idx::TEXT_DEFAULT, s)
}

pub fn label(s: impl Into<SharedString>) -> gpui::Div {
    text(16., 20., FontWeight::MEDIUM, idx::TEXT_STRONG, s)
}

pub fn description(s: impl Into<SharedString>) -> gpui::Div {
    text(14., 18., FontWeight::NORMAL, idx::TEXT_SUBTLE, s)
}

pub fn link(s: impl Into<SharedString>) -> gpui::Div {
    text(14., 18., FontWeight::MEDIUM, idx::TEXT_LINK, s).cursor_pointer().hover(|s| s.underline())
}

pub fn page() -> gpui::Div {
    div().w_full().flex().flex_col()
}

pub fn divider(my: f32) -> gpui::Div {
    div().my(z(my)).h(z(1.)).flex_none().bg(t(idx::BORDER_SUBTLE))
}

pub fn card() -> gpui::Div {
    div().bg(t(idx::CARD_BACKGROUND_DEFAULT)).border_1().border_color(t(idx::BORDER_SUBTLE)).rounded(z(8.))
}

pub fn icon(name: &str, size: f32, col: usize) -> gpui::Svg {
    gpui::svg().path(crate::icons::path(name)).flex_none().size(z(size)).text_color(t(col))
}

pub fn pill(s: impl Into<SharedString>) -> gpui::Div {
    div()
        .mt(z(4.))
        .px(z(8.))
        .py(z(4.))
        .rounded(z(16.))
        .bg(t(idx::BACKGROUND_MOD_NORMAL))
        .flex()
        .items_center()
        .gap(z(4.))
        .child(text(12., 16., FontWeight::MEDIUM, idx::TEXT_SUBTLE, s))
}

pub fn back(guild: Id, section: &'static str, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("gs-back-{section}")))
        .h(z(24.))
        .mb(z(32.))
        .flex()
        .items_center()
        .gap(z(4.))
        .cursor_pointer()
        .on_click(cx.listener(move |_, _, _, cx| {
            set_sub(guild, section, Sub::Root);
            cx.notify();
        }))
        .child(icon("chevron-right", 24., idx::ICON_SUBTLE).with_transformation(gpui::Transformation::rotate(gpui::radians(std::f32::consts::PI))))
        .child(text(14., 18., FontWeight::SEMIBOLD, idx::TEXT_SUBTLE, crate::t!("gs2.back")))
}

pub fn loading() -> AnyElement {
    div().py(z(40.)).flex().justify_center().child(description(crate::t!("gs2.loading"))).into_any_element()
}

pub fn error(e: String) -> AnyElement {
    div().py(z(16.)).child(text(14., 18., FontWeight::MEDIUM, idx::TEXT_FEEDBACK_CRITICAL, e)).into_any_element()
}

pub fn str_of<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}

pub fn id_of(v: &Value, k: &str) -> Option<Id> {
    v.get(k).and_then(Value::as_str).and_then(|s| s.parse().ok()).map(Id)
}

pub fn features(v: &Value) -> Vec<String> {
    v.get("features").and_then(Value::as_array).into_iter().flatten().filter_map(|x| x.as_str().map(str::to_owned)).collect()
}

pub fn channel_label(app: &PhoenixApp, ch: Option<Id>) -> SharedString {
    match ch.and_then(|i| app.channels.get(&i)) {
        Some(c) => format!("#{}", c.name).into(),
        None => crate::t!("gs2.no_channel"),
    }
}

pub fn date_fr(iso: &str) -> String {
    let Ok(d) = chrono::DateTime::parse_from_rfc3339(iso) else { return String::new() };
    let d = d.with_timezone(&chrono::Local);
    use chrono::Datelike;
    let months = ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."];
    format!("{} {} {}", d.day(), months[d.month0() as usize], d.year())
}

pub fn snowflake_date(id: u64) -> String {
    let ms = (id >> 22) + phoenix_api::models::DISCORD_EPOCH_MS;
    chrono::DateTime::from_timestamp_millis(ms as i64).map(|d| date_fr(&d.to_rfc3339())).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(date_fr("2026-10-06T12:00:00+00:00"), "6 oct. 2026");
        assert!(snowflake_date(1_369_382_919_948_210_248).ends_with("2025"));
    }

    #[test]
    fn sub_pages() {
        let g = Id(77);
        assert_eq!(sub(g, "menu.gs.safety"), Sub::Root);
        set_sub(g, "menu.gs.safety", Sub::Automod);
        assert_eq!(sub(g, "menu.gs.safety"), Sub::Automod);
        assert_eq!(sub(g, "menu.gs.integrations"), Sub::Root);
    }

    #[test]
    fn keys() {
        assert_eq!(features(&serde_json::json!({ "features": ["COMMUNITY", "NEWS"] })), vec!["COMMUNITY", "NEWS"]);
        assert_eq!(id_of(&serde_json::json!({ "x": "12" }), "x"), Some(Id(12)));
    }
}
