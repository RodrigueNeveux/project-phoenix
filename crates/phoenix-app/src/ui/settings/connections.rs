use gpui::{AnyElement, Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, prelude::*};
use parking_lot::RwLock;
use serde_json::Value;

use crate::{
    app::PhoenixApp,
    theme::{c, generated::idx, tok, z},
    ui::controls,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LinkedAccount {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub verified: bool,
    pub visibility: u8,
    pub show_activity: bool,
    pub friend_sync: bool,
    pub metadata_visibility: u8,
}

static LIST: RwLock<Option<Vec<LinkedAccount>>> = RwLock::new(None);

pub fn get() -> Option<Vec<LinkedAccount>> {
    LIST.read().clone()
}

pub fn set(v: Vec<LinkedAccount>) {
    super::account::set_connections(v.len());
    *LIST.write() = Some(v);
}

pub fn parse(v: &Value) -> Vec<LinkedAccount> {
    let s = |o: &Value, k: &str| o.get(k).and_then(Value::as_str).unwrap_or_default().to_owned();
    let b = |o: &Value, k: &str| o.get(k).and_then(Value::as_bool).unwrap_or(false);
    let u = |o: &Value, k: &str| o.get(k).and_then(Value::as_u64).unwrap_or(0) as u8;
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|o| {
            let kind = s(o, "type");
            let name = s(o, "name");
            if kind.is_empty() && name.is_empty() {
                return None;
            }
            Some(LinkedAccount {
                id: s(o, "id"),
                kind,
                name,
                verified: b(o, "verified"),
                visibility: u(o, "visibility"),
                show_activity: b(o, "show_activity"),
                friend_sync: b(o, "friend_sync"),
                metadata_visibility: u(o, "metadata_visibility"),
            })
        })
        .collect()
}

pub fn fetch(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) {
    if LIST.read().is_some() {
        return;
    }
    let fut = app.backend.user_connections();
    cx.spawn(async move |this, cx| {
        if let Ok(Some(v)) = fut.await {
            set(v);
            let _ = this.update(cx, |_, cx| cx.notify());
        }
    })
    .detach();
}

pub fn platform_label(kind: &str) -> &'static str {
    match kind {
        "riotgames" => "Riot Games",
        "battlenet" => "Battle.net",
        "bluesky" => "Bluesky",
        "letterboxd" => "Letterboxd",
        "paypal" => "PayPal",
        "reddit" => "Reddit",
        "steam" => "Steam",
        "twitter" | "x" => "X",
        "ebay" => "eBay",
        "crunchyroll" => "Crunchyroll",
        "playstation" => "PlayStation Network",
        "spotify" => "Spotify",
        "xbox" => "Xbox",
        "amazonmusic" => "Amazon Music",
        "bungie" => "Bungie.net",
        "domain" => "Domain",
        "epicgames" => "Epic Games",
        "facebook" => "Facebook",
        "github" => "GitHub",
        "roblox" => "Roblox",
        "twitch" => "Twitch",
        "youtube" => "YouTube",
        "instagram" => "Instagram",
        "tiktok" => "TikTok",
        "leagueoflegends" => "League of Legends",
        "skype" => "Skype",
        _ => "Connection",
    }
}

fn readonly_switch_row(id: (&'static str, usize), label: SharedString, on: bool) -> gpui::Div {
    div()
        .py(z(8.))
        .flex()
        .items_center()
        .justify_between()
        .gap(z(12.))
        .child(
            div()
                .text_size(z(14.))
                .line_height(z(18.))
                .text_color(c(tok(idx::TEXT_DEFAULT)))
                .child(label),
        )
        .child(controls::switch(id, on).opacity(0.75))
}

fn card(acc: &LinkedAccount, ix: usize) -> gpui::Div {
    let icon_el: AnyElement = match super::art::connection_icon(&acc.kind) {
        Some((url, app_icon)) => gpui::img(SharedString::from(url))
            .size(z(if app_icon { 28. } else { 24. }))
            .rounded(z(if app_icon { 6. } else { 0. }))
            .object_fit(gpui::ObjectFit::Contain)
            .into_any_element(),
        None => gpui::svg()
            .path(crate::icons::path("link"))
            .size(z(20.))
            .text_color(c(tok(idx::ICON_DEFAULT)))
            .into_any_element(),
    };
    let mut header_name = div()
        .flex()
        .items_center()
        .gap(z(6.))
        .child(
            div()
                .text_size(z(16.))
                .line_height(z(20.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c(tok(idx::TEXT_STRONG)))
                .child(acc.name.clone()),
        );
    if acc.verified {
        header_name = header_name.child(
            gpui::svg()
                .path(crate::icons::path("circle-check"))
                .size(z(16.))
                .flex_none()
                .text_color(c(tok(idx::STATUS_POSITIVE))),
        );
    }
    let header = div()
        .pb(z(12.))
        .border_b_1()
        .border_color(c(tok(idx::BORDER_SUBTLE)))
        .flex()
        .items_center()
        .gap(z(12.))
        .child(
            div()
                .size(z(36.))
                .rounded(z(8.))
                .bg(c(tok(idx::BACKGROUND_MOD_NORMAL)))
                .flex()
                .items_center()
                .justify_center()
                .flex_none()
                .child(icon_el),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(z(2.))
                .child(header_name)
                .child(
                    div()
                        .text_size(z(12.))
                        .line_height(z(16.))
                        .text_color(c(tok(idx::TEXT_MUTED)))
                        .child(platform_label(&acc.kind)),
                ),
        );
    let mut rows = div()
        .pt(z(4.))
        .flex()
        .flex_col()
        .child(readonly_switch_row(("conn-vis", ix), crate::t!("sp.cn.show_profile"), acc.visibility != 0))
        .child(readonly_switch_row(("conn-act", ix), crate::t!("sp.cn.show_status"), acc.show_activity));
    if acc.friend_sync {
        rows = rows.child(readonly_switch_row(("conn-sync", ix), crate::t!("sp.cn.friend_sync"), true));
    }
    div()
        .id(("conn-card", ix))
        .p(z(16.))
        .rounded(z(12.))
        .border_1()
        .border_color(c(tok(idx::BORDER_SUBTLE)))
        .bg(c(tok(idx::BACKGROUND_MOD_SUBTLE)))
        .flex()
        .flex_col()
        .child(header)
        .child(rows)
}

pub fn render(_: &PhoenixApp, _: &mut Context<PhoenixApp>) -> AnyElement {
    let list = get().unwrap_or_default();
    if !list.is_empty() {
        return div()
            .flex()
            .flex_col()
            .gap(z(12.))
            .children(list.iter().enumerate().map(|(ix, acc)| card(acc, ix)))
            .into_any_element();
    }
    if super::account::connections().unwrap_or(0) > 0 && get().is_none() {
        return div().into_any_element();
    }
    div()
        .h(z(200.))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .w(z(440.))
                .flex()
                .flex_col()
                .gap(z(8.))
                .text_center()
                .text_color(c(tok(idx::TEXT_MUTED)))
                .child(div().text_size(z(20.)).line_height(z(24.)).font_weight(FontWeight::SEMIBOLD).child(crate::t!("sp.cn.empty")))
                .child(div().text_size(z(16.)).line_height(z(20.)).child(crate::t!("sp.cn.empty_d"))),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_linked_accounts() {
        let v = json!([
            {
                "id": "12345",
                "type": "github",
                "name": "octocat",
                "verified": true,
                "visibility": 1,
                "show_activity": true,
                "friend_sync": false,
                "metadata_visibility": 1
            },
            {
                "id": "67890",
                "type": "spotify",
                "name": "dj",
                "verified": false,
                "visibility": 0,
                "show_activity": false,
                "friend_sync": true,
                "metadata_visibility": 0
            }
        ]);
        let list = parse(&v);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].kind, "github");
        assert_eq!(list[0].name, "octocat");
        assert!(list[0].verified);
        assert_eq!(list[0].visibility, 1);
        assert!(list[0].show_activity);
        assert!(!list[0].friend_sync);
        assert_eq!(list[1].kind, "spotify");
        assert!(!list[1].verified);
        assert!(list[1].friend_sync);
        assert_eq!(platform_label(&list[0].kind), "GitHub");
        assert_eq!(platform_label(&list[1].kind), "Spotify");
    }
}
