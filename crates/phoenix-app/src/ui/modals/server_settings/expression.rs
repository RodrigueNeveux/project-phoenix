use gpui::{AnyElement, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, prelude::*};
use serde_json::Value;

use super::{
    State,
    kit::{self, Cx, text},
};
use crate::{
    app::PhoenixApp,
    theme::{c, generated::idx, tok, z},
    ui::controls::BtnKind,
};

pub fn tier(s: &State) -> u8 {
    kit::n(s.guild_json(), "premium_tier").min(3) as u8
}

pub fn emoji_slots(tier: u8) -> usize {
    [50, 100, 150, 250][tier.min(3) as usize]
}

pub fn sticker_slots(tier: u8) -> usize {
    [5, 15, 30, 60][tier.min(3) as usize]
}

pub fn sound_slots(tier: u8) -> usize {
    [8, 24, 36, 48][tier.min(3) as usize]
}

fn header(title: SharedString, desc: SharedString, upload: SharedString, id: &'static str) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap(z(20.))
        .child(div().flex().flex_col().gap(z(4.)).child(kit::h2(title)).child(text(desc, 16., 20., FontWeight::NORMAL, idx::TEXT_DEFAULT)))
        .child(div().flex().child(crate::ui::controls::button(id, upload, BtnKind::Primary)))
}

fn uploader(u: Option<&Value>) -> AnyElement {
    let Some(u) = u else {
        return div().into_any_element();
    };
    div()
        .flex()
        .items_center()
        .min_w_0()
        .child(div().mr(z(12.)).child(kit::avatar(kit::user_avatar(u, 40), 20.)))
        .child(
            div()
                .flex()
                .flex_col()
                .min_w_0()
                .child(text(kit::user_name(u), 14., 18., FontWeight::NORMAL, idx::TEXT_DEFAULT).truncate())
                .child(text(kit::s(u, "username"), 12., 16., FontWeight::NORMAL, idx::TEXT_MUTED).mt(z(-2.)).truncate()),
        )
        .into_any_element()
}

fn emoji_table(title: SharedString, list: &[&Value], slots: usize, cx: &mut Cx) -> AnyElement {
    let _ = cx;
    let head = div()
        .py(z(8.))
        .px(z(16.))
        .mb(z(4.))
        .rounded_t(z(12.))
        .bg(c(tok(idx::BACKGROUND_MOD_MUTED)))
        .flex()
        .child(text(crate::t!("gs.expr.image"), 14., 18., FontWeight::SEMIBOLD, idx::TEXT_DEFAULT).w(z(49.3)))
        .child(text(crate::t!("gs.expr.name"), 14., 18., FontWeight::SEMIBOLD, idx::TEXT_DEFAULT).flex_1())
        .child(text(crate::t!("gs.expr.uploader"), 14., 18., FontWeight::SEMIBOLD, idx::TEXT_DEFAULT).flex_1())
        .child(div().w(z(62.4)));
    let mut body = div().py(z(4.)).px(z(24.)).flex().flex_col();
    for (i, e) in list.iter().enumerate() {
        let id = kit::s(e, "id");
        let anim = e.get("animated").and_then(Value::as_bool).unwrap_or(false);
        let url: SharedString = format!("https://cdn.discordapp.com/emojis/{id}.{}?size=64", if anim { "gif" } else { "webp" }).into();
        let group: SharedString = format!("ss-emoji-{id}").into();
        body = body.child(
            div()
                .id(("ss-emoji", i))
                .group(group.clone())
                .h(z(64.))
                .flex()
                .items_center()
                .when(i > 0, |d| d.border_t_1().border_color(c(tok(idx::BORDER_SUBTLE))))
                .child(div().w(z(48.1)).flex_none().child(gpui::img(url).size(z(32.)).object_fit(gpui::ObjectFit::Contain)))
                .child(div().flex_1().min_w_0().flex().items_center().text_size(z(16.)).line_height(z(20.)).text_color(c(tok(idx::TEXT_DEFAULT))).child(format!(":{}:", kit::s(e, "name"))))
                .child(div().flex_1().min_w_0().child(uploader(e.get("user"))))
                .child(
                    div()
                        .flex()
                        .gap(z(10.))
                        .invisible()
                        .group_hover(group.clone(), |s| s.visible())
                        .child(kit::icon_btn(("ss-emoji-edit", i), "pen", 32., 16., false))
                        .child(kit::icon_btn(("ss-emoji-del", i), "close-small", 32., 16., true)),
                ),
        );
    }
    div()
        .flex()
        .flex_col()
        .gap(z(16.))
        .child(kit::legend(title, Some(crate::t!("gs.expr.slots_left", n = slots.saturating_sub(list.len())))))
        .when(!list.is_empty(), |d| d.child(div().mb(z(48.)).rounded(z(12.)).border_1().border_color(c(tok(idx::BORDER_SUBTLE))).flex().flex_col().child(head).child(body)))
        .into_any_element()
}

pub fn emoji(_app: &PhoenixApp, s: &State, cx: &mut Cx) -> AnyElement {
    let all = s.list("emojis");
    let stat: Vec<&Value> = all.iter().filter(|e| !e.get("animated").and_then(Value::as_bool).unwrap_or(false)).collect();
    let anim: Vec<&Value> = all.iter().filter(|e| e.get("animated").and_then(Value::as_bool).unwrap_or(false)).collect();
    let slots = emoji_slots(tier(s));
    div()
        .flex()
        .flex_col()
        .gap(z(20.))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(z(24.))
                .pb(z(24.))
                .mb(z(24.))
                .border_b_1()
                .border_color(c(tok(idx::BORDER_SUBTLE)))
                .child(header(crate::t!("menu.gs.emoji"), crate::t!("gs.expr.emoji_desc", n = slots), crate::t!("gs.expr.emoji_upload"), "ss-emoji-upload"))
                .child(text(crate::t!("gs.expr.emoji_drop"), 12., 16., FontWeight::NORMAL, idx::TEXT_DEFAULT)),
        )
        .child(emoji_table(crate::t!("menu.gs.emoji"), &stat, slots, cx))
        .child(emoji_table(crate::t!("gs.expr.animated"), &anim, slots, cx))
        .into_any_element()
}

pub fn stickers(_app: &PhoenixApp, s: &State, cx: &mut Cx) -> AnyElement {
    let _ = cx;
    let list = s.list("stickers");
    let slots = sticker_slots(tier(s));
    let mut grid = div().flex().flex_wrap().gap(z(16.));
    for (i, st) in list.iter().enumerate() {
        let id = kit::s(st, "id");
        let ext = match kit::n(st, "format_type") {
            3 => "json",
            4 => "gif",
            _ => "png",
        };
        grid = grid.child(
            div()
                .id(("ss-sticker", i))
                .w(z(140.))
                .h(z(180.))
                .rounded(z(8.))
                .bg(c(tok(idx::BACKGROUND_BASE_LOWER)))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(z(8.))
                .child(if ext == "json" {
                    div().size(z(96.)).into_any_element()
                } else {
                    gpui::img(SharedString::from(format!("https://media.discordapp.net/stickers/{id}.{ext}?size=160"))).size(z(96.)).object_fit(gpui::ObjectFit::Contain).into_any_element()
                })
                .child(text(kit::s(st, "name"), 14., 18., FontWeight::MEDIUM, idx::TEXT_STRONG).truncate().px(z(8.))),
        );
    }
    div()
        .flex()
        .flex_col()
        .gap(z(20.))
        .child(kit::h2(crate::t!("menu.gs.stickers")))
        .child(div().flex().flex_col().gap(z(8.)).child(text(crate::t!("gs.expr.sticker_rules"), 16., 20., FontWeight::MEDIUM, idx::TEXT_DEFAULT)).child(text(crate::t!("gs.expr.sticker_rules_desc"), 14., 18., FontWeight::NORMAL, idx::TEXT_DEFAULT)))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(text(crate::t!("gs.expr.slots_of", used = list.len(), n = slots), 16., 20., FontWeight::SEMIBOLD, idx::TEXT_STRONG))
                .child(kit::btn_sm("ss-sticker-upload", crate::t!("gs.expr.sticker_upload"), BtnKind::Primary)),
        )
        .child(grid)
        .into_any_element()
}

pub fn soundboard(_app: &PhoenixApp, s: &State, cx: &mut Cx) -> AnyElement {
    let _ = cx;
    let items: Vec<Value> = s.get("sounds").and_then(|v| v.get("items")).and_then(Value::as_array).cloned().unwrap_or_default();
    let t = tier(s);
    let slots = sound_slots(t);
    let head = div()
        .flex()
        .pb(z(8.))
        .child(text(crate::t!("gs.expr.emoji_col"), 14., 18., FontWeight::SEMIBOLD, idx::TEXT_STRONG).w(z(60.)).mr(z(8.)))
        .child(text(crate::t!("gs.expr.name"), 14., 18., FontWeight::SEMIBOLD, idx::TEXT_STRONG).w(z(328.)).mr(z(8.)))
        .child(text(crate::t!("gs.expr.uploader"), 14., 18., FontWeight::SEMIBOLD, idx::TEXT_STRONG).w(z(200.)));
    let mut rows = div().flex().flex_col();
    for (i, snd) in items.iter().enumerate() {
        let emoji: AnyElement = match (kit::id_of(snd, "emoji_id"), snd.get("emoji_name").and_then(Value::as_str)) {
            (Some(id), _) => gpui::img(SharedString::from(format!("https://cdn.discordapp.com/emojis/{id}.webp?size=48"))).size(z(24.)).into_any_element(),
            (None, Some(n)) => crate::media::emoji::emoji_element(n, 24.),
            _ => div().size(z(24.)).into_any_element(),
        };
        let user = snd.get("user").cloned().unwrap_or(Value::Null);
        rows = rows.child(
            div()
                .id(("ss-sound", i))
                .h(z(60.8))
                .flex()
                .items_center()
                .border_t_1()
                .border_color(c(tok(idx::BORDER_SUBTLE)))
                .child(div().w(z(60.)).mr(z(8.)).flex().justify_center().child(emoji))
                .child(text(kit::s(snd, "name"), 14., 18., FontWeight::NORMAL, idx::TEXT_DEFAULT).w(z(328.)).mr(z(8.)).truncate())
                .child(div().w(z(200.)).flex().items_center().gap(z(8.)).child(kit::avatar(kit::user_avatar(&user, 40), 20.)).child(text(kit::s(&user, "username"), 14., 18., FontWeight::NORMAL, idx::TEXT_DEFAULT).truncate()))
                .child(div().flex_1().flex().justify_end().child(kit::icon_btn(("ss-sound-play", i), "play", 32., 16., false))),
        );
    }
    div()
        .flex()
        .flex_col()
        .child(kit::h2(crate::t!("menu.gs.soundboard")))
        .child(text(crate::t!("gs.expr.sound_desc"), 14., 18., FontWeight::NORMAL, idx::TEXT_DEFAULT).mt(z(16.)))
        .child(div().mt(z(24.)).flex().child(crate::ui::controls::button("ss-sound-upload", crate::t!("gs.expr.sound_upload"), BtnKind::Primary)))
        .child(text(crate::t!("gs.expr.sound_slots", tier = t, n = slots.saturating_sub(items.len()), total = slots), 16., 20., FontWeight::SEMIBOLD, idx::TEXT_STRONG).mt(z(50.)).h(z(24.)))
        .child(div().mt(z(32.)).child(head))
        .child(rows)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotas_follow_boost_tier() {
        assert_eq!((emoji_slots(0), emoji_slots(2), emoji_slots(9)), (50, 150, 250));
        assert_eq!((sticker_slots(1), sticker_slots(3)), (15, 60));
        assert_eq!((sound_slots(0), sound_slots(2)), (8, 36));
    }
}
