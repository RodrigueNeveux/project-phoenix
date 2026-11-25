use gpui::{AnyElement, Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, prelude::*};
use phoenix_api::rest::guild_server::ServerCall;
use serde_json::Value;

use super::{description, divider, fetch, icon, label, link, page, sf, str_of, t, text, title};
use crate::{
    app::PhoenixApp,
    model::Id,
    theme::{generated::idx, z},
    ui::controls::{BtnKind, button_sm, switch},
};

fn field(v: impl Into<SharedString>, h: f32) -> gpui::Div {
    div()
        .min_h(z(h))
        .px(z(12.))
        .py(z(9.))
        .rounded(z(8.))
        .border_1()
        .border_color(t(idx::INPUT_BORDER_DEFAULT))
        .bg(t(idx::INPUT_BACKGROUND_DEFAULT))
        .flex()
        .items_start()
        .child(text(16., 20., FontWeight::NORMAL, idx::TEXT_DEFAULT, v))
}

fn chip(s: String) -> gpui::Div {
    div().px(z(8.)).py(z(4.)).rounded(z(4.)).bg(t(idx::BACKGROUND_MOD_SUBTLE)).border_1().border_color(t(idx::BORDER_MUTED)).flex().items_center().gap(z(4.)).child(text(14., 18., FontWeight::NORMAL, idx::TEXT_DEFAULT, s)).child(icon("close-small", 16., idx::ICON_SUBTLE))
}

pub fn category_name(cats: &Value, id: u64) -> String {
    cats.as_array().into_iter().flatten().find(|c| c.get("id").and_then(Value::as_u64) == Some(id)).map(|c| str_of(c, "name").to_owned()).unwrap_or_default()
}

pub fn channel_summary(app: &PhoenixApp, list: &[Id]) -> String {
    let names: Vec<String> = list.iter().take(2).map(|c| app.channels.get(c).map(|c| c.name.to_string()).unwrap_or_default()).collect();
    let more = list.len().saturating_sub(2);
    if more > 0 { format!("{}, +\u{a0}{more} en plus", names.join(", ")) } else { names.join(", ") }
}

fn content_row(app: &PhoenixApp, head: SharedString, desc: SharedString, on: bool, ids: &[Id], key: &'static str) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .justify_between()
                .gap(z(24.))
                .child(div().w(z(453.)).flex().flex_col().gap(z(4.)).child(text(16., 20., FontWeight::MEDIUM, idx::TEXT_STRONG, head)).child(text(14., 18., FontWeight::NORMAL, idx::TEXT_SUBTLE, desc)).child(link(crate::t!("gs2.learn_more"))))
                .child(switch(key, on)),
        )
        .child(
            div()
                .mt(z(12.))
                .px(z(12.))
                .py(z(15.))
                .rounded(z(8.))
                .bg(t(idx::BACKGROUND_MOD_SUBTLE))
                .flex()
                .gap(z(12.))
                .child(text(14., 18., FontWeight::MEDIUM, idx::TEXT_STRONG, crate::t!("gs2.selected_channels")))
                .child(div().flex_1().child(text(14., 18., FontWeight::MEDIUM, idx::TEXT_DEFAULT, channel_summary(app, ids))))
                .child(icon("chevron-right", 16., idx::ICON_DEFAULT)),
        )
}

pub fn render(app: &PhoenixApp, guild: Id, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let meta = fetch(app, guild, ServerCall::DiscoveryMetadata { guild: sf(guild) }, cx);
    let cats = fetch(app, guild, ServerCall::DiscoveryCategories { locale: "fr".into() }, cx);
    let content = fetch(app, guild, ServerCall::DiscoverableContent { guild: sf(guild) }, cx);
    let index = fetch(app, guild, ServerCall::ServerIndex { guild: sf(guild) }, cx);
    let g = fetch(app, guild, ServerCall::Guild { guild: sf(guild) }, cx);
    let mut p = page().child(title(crate::t!("menu.gs.discovery"))).child(div().mt(z(8.)).child(text(14., 18., FontWeight::NORMAL, idx::TEXT_DEFAULT, crate::t!("gs2.presence_lead"))));
    let (Some(meta), Some(cats), Some(content), Some(g)) = (meta, cats, content, g) else { return p.child(super::loading()).into_any_element() };
    let url = index.as_ref().and_then(|i| i.get("seo")).map(|s| str_of(s, "canonical_url").to_owned()).unwrap_or_default();
    p = p
        .child(div().mt(z(24.)).child(label(crate::t!("gs2.page_url"))))
        .child(
            div()
                .mt(z(8.))
                .h(z(40.))
                .pl(z(12.))
                .pr(z(4.))
                .rounded(z(8.))
                .border_1()
                .border_color(t(idx::INPUT_BORDER_DEFAULT))
                .bg(t(idx::INPUT_BACKGROUND_DEFAULT))
                .flex()
                .items_center()
                .child(div().flex_1().min_w_0().child(text(16., 20., FontWeight::NORMAL, idx::TEXT_DEFAULT, url.clone()).truncate()))
                .child(button_sm("gs2-copy-url", crate::t!("gs2.copy"), BtnKind::Secondary).on_click(move |_, _, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string(url.clone())))),
        )
        .child(div().mt(z(16.)).flex().child(button_sm("gs2-open-page", crate::t!("gs2.view_page"), BtnKind::Primary)))
        .child(divider(80.).my(z(0.)).mt(z(80.)).mb(z(80.)));
    let primary = meta.get("primary_category_id").and_then(Value::as_u64).unwrap_or(0);
    let mut subs = div().flex().flex_wrap().gap(z(4.));
    for c in meta.get("category_ids").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_u64) {
        subs = subs.child(chip(category_name(&cats, c)));
    }
    p = p
        .child(label(crate::t!("gs2.category")))
        .child(div().mt(z(4.)).child(description(crate::t!("gs2.category_desc"))))
        .child(div().mt(z(8.)).child(crate::ui::controls::select_field("gs2-category", category_name(&cats, primary))))
        .child(div().mt(z(40.)).child(label(crate::t!("gs2.subcategories"))))
        .child(div().mt(z(8.)).min_h(z(40.)).p(z(6.)).rounded(z(8.)).border_1().border_color(t(idx::INPUT_BORDER_DEFAULT)).bg(t(idx::INPUT_BACKGROUND_DEFAULT)).child(subs))
        .child(divider(60.));
    let splash = g.get("discovery_splash").and_then(Value::as_str).map(|h| format!("https://cdn.discordapp.com/discovery-splashes/{}/{h}.jpg?size=512", guild.0));
    p = p.child(
        div()
            .flex()
            .gap(z(24.))
            .child(div().w(z(324.)).flex().flex_col().gap(z(4.)).child(label(crate::t!("gs2.discovery_cover"))).child(description(crate::t!("gs2.discovery_cover_desc"))))
            .child(
                div()
                    .w(z(279.))
                    .flex()
                    .flex_col()
                    .gap(z(12.))
                    .child(div().h(z(157.)).rounded(z(8.)).overflow_hidden().bg(t(idx::BACKGROUND_MOD_STRONG)).children(splash.map(|u| gpui::img(SharedString::from(u)).size_full().object_fit(gpui::ObjectFit::Cover))))
                    .child(div().flex().justify_center().child(text(14., 14., FontWeight::NORMAL, idx::TEXT_LINK, crate::t!("gs2.remove"))))
                    .child(button_sm("gs2-upload-cover", crate::t!("gs2.upload_bg"), BtnKind::Secondary)),
            ),
    );
    let sect = |k: &str| content.get(k).cloned().unwrap_or(Value::Null);
    let ids = |v: &Value| -> Vec<Id> {
        v.get("channels").and_then(Value::as_array).into_iter().flatten().filter(|c| c.get("included").and_then(Value::as_bool).unwrap_or(false)).filter_map(|c| super::id_of(c, "channel_id")).collect()
    };
    let sums = sect("channel_summaries");
    let msgs = sect("channel_messages");
    p = p
        .child(divider(60.))
        .child(div().flex().items_center().gap(z(16.)).child(text(16., 20., FontWeight::MEDIUM, idx::TEXT_STRONG, crate::t!("gs2.web_content"))).child(div().px(z(4.)).rounded(z(4.)).bg(t(idx::BACKGROUND_BRAND)).child(text(12., 16., FontWeight::SEMIBOLD, idx::WHITE, crate::t!("gs2.new")))))
        .child(div().mt(z(4.)).child(text(14., 18., FontWeight::NORMAL, idx::TEXT_DEFAULT, crate::t!("gs2.web_content_desc"))))
        .child(div().mt(z(32.)).child(content_row(app, crate::t!("gs2.channel_summaries"), crate::t!("gs2.channel_summaries_desc"), sums.get("enabled").and_then(Value::as_bool).unwrap_or(false), &ids(&sums), "gs2-sum-sw")))
        .child(div().mt(z(32.)).child(content_row(app, crate::t!("gs2.channel_messages"), crate::t!("gs2.channel_messages_desc"), msgs.get("enabled").and_then(Value::as_bool).unwrap_or(false), &ids(&msgs), "gs2-msg-sw")))
        .child(divider(80.));
    p = p
        .child(label(crate::t!("gs2.server_description")))
        .child(div().mt(z(4.)).child(description(crate::t!("gs2.presence_description_desc"))))
        .child(div().mt(z(8.)).child(field(str_of(&g, "description").to_owned(), 88.)))
        .child(divider(80.))
        .child(label(crate::t!("gs2.reasons")))
        .child(div().mt(z(4.)).child(description(crate::t!("gs2.reasons_desc"))));
    let reasons: Vec<Value> = meta.get("reasons_to_join").and_then(Value::as_array).cloned().unwrap_or_default();
    for i in 0..4 {
        let r = reasons.get(i);
        p = p.child(
            div()
                .mt(z(if i == 0 { 8. } else { 16. }))
                .h(z(40.))
                .px(z(10.))
                .rounded(z(8.))
                .border_1()
                .border_color(t(idx::INPUT_BORDER_DEFAULT))
                .bg(t(idx::INPUT_BACKGROUND_DEFAULT))
                .flex()
                .items_center()
                .gap(z(10.))
                .child(div().w(z(19.)).child(text(16., 20., FontWeight::NORMAL, idx::TEXT_DEFAULT, r.map(|r| str_of(r, "emoji_name").to_owned()).unwrap_or_default())))
                .child(div().flex_1().child(text(16., 20., FontWeight::NORMAL, idx::TEXT_DEFAULT, r.map(|r| str_of(r, "reason").to_owned()).unwrap_or_default())))
                .child(icon("close-small", 16., idx::ICON_SUBTLE)),
        );
    }
    let about = str_of(&meta, "about").to_owned();
    let n = about.chars().count();
    p = p
        .child(divider(80.))
        .child(label(crate::t!("gs2.about")))
        .child(div().mt(z(4.)).child(description(crate::t!("gs2.about_desc"))))
        .child(div().mt(z(8.)).child(field(about, 232.)))
        .child(div().mt(z(4.)).flex().justify_end().child(text(12., 16., FontWeight::NORMAL, idx::TEXT_MUTED, format!("{n} / 2400"))))
        .child(divider(80.))
        .child(label(crate::t!("gs2.social")))
        .child(div().mt(z(4.)).child(description(crate::t!("gs2.social_desc"))));
    for (i, l) in meta.get("social_links").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).enumerate() {
        let (plat, prefix) = social_platform(l);
        p = p.child(
            div()
                .id(("gs2-social", i))
                .mt(z(8.))
                .flex()
                .gap(z(12.))
                .child(div().w(z(186.)).child(crate::ui::controls::select_field(("gs2-social-plat", i), plat)))
                .child(field(format!("{prefix}{}", l.trim_start_matches("https://").split_once('/').map(|x| x.1).unwrap_or("")), 40.).flex_1()),
        );
    }
    p.child(div().mt(z(20.)).flex().child(button_sm("gs2-social-add", crate::t!("gs2.add_another"), BtnKind::Secondary)))
        .child(div().mt(z(80.)).child(text(14., 18., FontWeight::NORMAL, idx::TEXT_DEFAULT, crate::t!("gs2.presence_remove_hint"))))
        .into_any_element()
}

pub fn social_platform(url: &str) -> (&'static str, &'static str) {
    let host = url.trim_start_matches("https://").trim_start_matches("www.");
    for (h, name) in [("instagram.com", "Instagram"), ("x.com", "X"), ("twitter.com", "X"), ("youtube.com", "YouTube"), ("tiktok.com", "TikTok"), ("twitch.tv", "Twitch"), ("facebook.com", "Facebook"), ("reddit.com", "Reddit"), ("github.com", "GitHub")] {
        if host.starts_with(h) {
            return (name, "");
        }
    }
    ("Site web", "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn presence_helpers() {
        let cats = json!([{ "id": 49, "name": "Bots" }, { "id": 14, "name": "Discussion générale" }]);
        assert_eq!(category_name(&cats, 49), "Bots");
        assert_eq!(social_platform("https://instagram.com/gladosofficiel").0, "Instagram");
    }
}
