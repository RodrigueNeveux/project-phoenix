use std::cell::{Cell, RefCell};

use chrono::{DateTime, Utc};
use gpui::{AnyElement, Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, Window, div, img, prelude::*};
use serde_json::Value;

use crate::{
    app::PhoenixApp,
    theme::{c, generated::idx, tok, z},
};

pub const CDN: &str = "https://cdn.discordapp.com/";
const TILE_MIN_W: f32 = 336.;
const GAP: f32 = 24.;

#[derive(Clone, Debug, PartialEq)]
pub struct Quest {
    pub id: u64,
    pub name: SharedString,
    pub game: SharedString,
    pub publisher: SharedString,
    pub hero: Option<SharedString>,
    pub logotype: Option<SharedString>,
    pub expires: Option<DateTime<Utc>>,
    pub reward: SharedString,
    pub orbs: Option<u64>,
    pub enrolled: bool,
    pub completed: bool,
    pub claimed: bool,
    pub progress: f32,
}

thread_local! {
    static QUESTS: RefCell<Option<Vec<Quest>>> = const { RefCell::new(None) };
    static REQUESTED: Cell<bool> = const { Cell::new(false) };
}

fn asset(v: &Value) -> Option<SharedString> {
    let p = v.as_str()?;
    if p.is_empty() || p == "PLACEHOLDER" {
        return None;
    }
    Some(if p.starts_with("http") { p.to_owned().into() } else { format!("{CDN}{p}").into() })
}

fn date(v: &Value) -> Option<DateTime<Utc>> {
    v.as_str().and_then(|s| DateTime::parse_from_rfc3339(s).ok()).map(|d| d.with_timezone(&Utc))
}

pub fn parse(v: &Value, now: DateTime<Utc>) -> Vec<Quest> {
    let list = v.get("quests").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut out = Vec::new();
    for q in &list {
        let cfg = &q["config"];
        let expires = date(&cfg["expires_at"]);
        if expires.is_some_and(|e| e <= now) {
            continue;
        }
        let msgs = &cfg["messages"];
        let reward = &cfg["rewards_config"]["rewards"][0];
        let us = &q["user_status"];
        let tasks = cfg["task_config_v2"]["tasks"].as_object().cloned().unwrap_or_default();
        let progress = tasks
            .iter()
            .filter_map(|(k, t)| {
                let target = t["target"].as_f64().filter(|t| *t > 0.)?;
                let value = us["progress"][k]["value"].as_f64().unwrap_or(0.);
                Some((value / target).clamp(0., 1.) as f32)
            })
            .fold(0f32, f32::max);
        out.push(Quest {
            id: q["id"].as_str().and_then(|s| s.parse().ok()).unwrap_or_default(),
            name: msgs["quest_name"].as_str().unwrap_or_default().to_owned().into(),
            game: msgs["game_title"].as_str().unwrap_or_default().to_owned().into(),
            publisher: msgs["game_publisher"].as_str().unwrap_or_default().to_owned().into(),
            hero: asset(&cfg["assets"]["hero"]),
            logotype: asset(&cfg["assets"]["logotype_dark"]).or_else(|| asset(&cfg["assets"]["logotype"])),
            expires,
            reward: reward["messages"]["name"].as_str().unwrap_or_default().to_owned().into(),
            orbs: reward["orb_quantity"].as_u64(),
            enrolled: !us["enrolled_at"].is_null(),
            completed: !us["completed_at"].is_null(),
            claimed: !us["claimed_at"].is_null(),
            progress,
        });
    }
    out.sort_by_key(|q| (q.claimed, q.completed, std::cmp::Reverse(q.id)));
    out
}

fn ensure_loaded(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) {
    if REQUESTED.with(|r| r.replace(true)) {
        return;
    }
    let fut = app.backend.quests();
    cx.spawn(async move |this, cx| {
        let v = fut.await.unwrap_or(Value::Null);
        QUESTS.with(|q| *q.borrow_mut() = Some(parse(&v, Utc::now())));
        let _ = this.update(cx, |_, cx| cx.notify());
    })
    .detach();
}

fn tk(i: usize) -> gpui::Hsla {
    c(tok(i))
}

fn text(size: f32, line: f32, weight: FontWeight, color: gpui::Hsla, t: impl Into<SharedString>) -> gpui::Div {
    div().text_size(z(size)).line_height(z(line)).font_weight(weight).text_color(color).child(t.into())
}

pub fn badge_text(q: &Quest) -> SharedString {
    if q.claimed {
        return crate::t!("quests.claimed");
    }
    if q.completed {
        return crate::t!("quests.completed");
    }
    match q.expires {
        Some(e) => crate::t!("quests.ends", date = e.with_timezone(&chrono::Local).format("%d/%m").to_string()),
        None => SharedString::default(),
    }
}

fn tile(q: &Quest) -> AnyElement {
    let white = gpui::white();
    let reward = match q.orbs {
        Some(n) => crate::t!("quests.orbs", n = n),
        None => q.reward.clone(),
    };
    let mut el = div().id(("quest-tile", q.id)).relative().w_full().min_w_0().h(z(260.)).rounded(z(16.)).overflow_hidden().bg(tk(idx::BACKGROUND_BASE_LOWEST)).border_1().border_color(tk(idx::BORDER_SUBTLE));
    if let Some(h) = q.hero.clone() {
        el = el.child(img(h).absolute().inset_0().size_full().object_fit(gpui::ObjectFit::Cover));
    }
    el.child(
        div()
            .absolute()
            .inset_0()
            .bg(gpui::linear_gradient(
                0.,
                gpui::linear_color_stop(tk(idx::BACKGROUND_BASE_LOW), 0.15),
                gpui::linear_color_stop(tk(idx::BACKGROUND_BASE_LOW).opacity(0.), 0.7),
            )),
    )
    .child(
        div().absolute().top(z(16.)).left(z(16.)).child(
            div()
                .px(z(8.))
                .py(z(4.))
                .rounded(z(16.))
                .border_1()
                .border_color(tk(idx::BORDER_MUTED))
                .bg(tk(idx::CONTROL_OVERLAY_SECONDARY_BACKGROUND_DEFAULT))
                .child(text(12., 16., FontWeight::SEMIBOLD, white, badge_text(q).to_uppercase())),
        ),
    )
    .child(
        div()
            .absolute()
            .left(z(16.))
            .right(z(16.))
            .bottom(z(16.))
            .flex()
            .flex_col()
            .gap(z(16.))
            .children(q.logotype.clone().map(|l| div().px(z(24.)).w_full().h(z(80.)).flex().items_center().justify_center().child(img(l).max_h(z(80.)).max_w_full().object_fit(gpui::ObjectFit::Contain))))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(z(6.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(text(12., 16., FontWeight::MEDIUM, gpui::hsla(0., 0., 1., 0.8), q.game.clone()))
                            .child(text(16., 20., FontWeight::SEMIBOLD, white, reward)),
                    ),
            )
            .when(q.enrolled && !q.completed, |d| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(z(8.))
                        .child(text(12., 16., FontWeight::SEMIBOLD, white, format!("{}%", (q.progress * 100.).round() as u32)))
                        .child(div().flex_1().h(z(8.)).rounded_full().bg(gpui::hsla(0., 0., 1., 0.2)).child(div().h_full().w(gpui::relative(q.progress)).rounded_full().bg(tk(idx::BRAND_500)))),
                )
            }),
    )
    .into_any_element()
}

pub fn columns(width: f32) -> usize {
    (((width + GAP) / (TILE_MIN_W + GAP)).floor() as usize).max(1)
}

pub fn render(app: &PhoenixApp, window: &mut Window, cx: &mut Context<PhoenixApp>) -> AnyElement {
    ensure_loaded(app, cx);
    let quests = QUESTS.with(|q| q.borrow().clone());
    let header = div()
        .flex_none()
        .h(z(49.))
        .px(z(8.))
        .border_b_1()
        .border_color(tk(idx::BORDER_SUBTLE))
        .bg(tk(idx::BACKGROUND_BASE_LOWER))
        .flex()
        .items_center()
        .child(div().size(z(32.)).flex().items_center().justify_center().child(gpui::svg().path(crate::icons::path("quests")).size(z(20.)).text_color(tk(idx::ICON_DEFAULT))))
        .child(text(16., 20., FontWeight::SEMIBOLD, tk(idx::TEXT_DEFAULT), crate::t!("dm.quests")).mr(z(8.)));
    let page_w = f32::from(window.viewport_size().width) / crate::theme::zoom() - crate::ui::sidebar_resize::width() - 72.;
    let inner = page_w.min(1300.) - 40.;
    let cols = columns(inner);
    let mut content = div().w_full().max_w(z(1300.)).mx_auto().px(z(20.)).py(z(24.)).flex().flex_col();
    match quests {
        None => content = content.child(text(16., 20., FontWeight::MEDIUM, tk(idx::TEXT_MUTED), crate::t!("common.loading"))),
        Some(list) if list.is_empty() => {
            content = content.child(
                div()
                    .py(z(80.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(z(8.))
                    .child(text(24., 32., FontWeight::SEMIBOLD, tk(idx::TEXT_STRONG), crate::t!("quests.none_title"))),
            )
        }
        Some(list) => {
            content = content.child(text(20., 24., FontWeight::SEMIBOLD, tk(idx::TEXT_STRONG), crate::t!("quests.available", n = list.len())));
            let mut grid = div().mt(z(GAP)).grid().grid_cols(cols as u16).gap(z(GAP));
            for q in &list {
                grid = grid.child(tile(q));
            }
            content = content.child(grid);
        }
    }
    div()
        .size_full()
        .flex()
        .flex_col()
        .bg(tk(idx::BACKGROUND_BASE_LOWER))
        .child(header)
        .child(div().id("quests-scroll").flex_1().min_h_0().overflow_y_scroll().child(content))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_active_quests_only() {
        let v = serde_json::json!({"quests": [
            {"id": "2", "config": {"expires_at": "2030-01-01T00:00:00+00:00", "messages": {"quest_name": "A", "game_title": "G", "game_publisher": "P"},
              "assets": {"hero": "quests/2/h.jpg", "logotype_dark": "PLACEHOLDER", "logotype": "quests/2/l.png"},
              "rewards_config": {"rewards": [{"type": 4, "messages": {"name": "700 Orbs"}, "orb_quantity": 700}]},
              "task_config_v2": {"tasks": {"PLAY_ON_DESKTOP": {"target": 900}}}},
             "user_status": {"enrolled_at": "x", "completed_at": null, "claimed_at": null, "progress": {"PLAY_ON_DESKTOP": {"value": 450}}}},
            {"id": "1", "config": {"expires_at": "2020-01-01T00:00:00+00:00"}, "user_status": null}
        ]});
        let q = parse(&v, Utc::now());
        assert_eq!(q.len(), 1);
        assert_eq!(q[0].hero.as_deref(), Some("https://cdn.discordapp.com/quests/2/h.jpg"));
        assert_eq!(q[0].logotype.as_deref(), Some("https://cdn.discordapp.com/quests/2/l.png"));
        assert_eq!(q[0].orbs, Some(700));
        assert!(q[0].enrolled && !q[0].completed);
        assert!((q[0].progress - 0.5).abs() < 1e-6);
        assert_eq!(columns(1260.), 3);
        assert_eq!(columns(300.), 1);
    }
}
