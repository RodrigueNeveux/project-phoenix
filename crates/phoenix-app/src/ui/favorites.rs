use gpui::{AnyElement, Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, prelude::*};
use phoenix_api::rest::favorites::{Favorites, Known, Row, snowflake_at};

use crate::{
    app::{PhoenixApp, SidebarRow},
    model::{Channel, ChannelKind, Id},
    ui::settings::SettingsPatch,
};

pub const GUILD: Id = Id(373);

const EMPTY_ART: &str = "https://cdn.discordapp.com/assets/content/88fc1694d1f36c93bf35b9cdc7dda25f1064d8205bfa103d62fd6cbf7fe636f7.png";

pub fn empty_page(cx: &mut Context<PhoenixApp>) -> AnyElement {
    use crate::theme::{c, generated::idx, tok, z};
    let button = div()
        .id("fav-empty-add")
        .h(z(40.))
        .px(z(16.))
        .rounded(z(8.))
        .flex()
        .items_center()
        .justify_center()
        .gap(z(8.))
        .cursor_pointer()
        .bg(c(tok(idx::CONTROL_PRIMARY_BACKGROUND_DEFAULT)))
        .hover(|s| s.bg(c(tok(idx::CONTROL_PRIMARY_BACKGROUND_HOVER))))
        .active(|s| s.bg(c(tok(idx::CONTROL_PRIMARY_BACKGROUND_ACTIVE))))
        .text_size(z(16.))
        .line_height(z(20.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(c(tok(idx::CONTROL_PRIMARY_TEXT_DEFAULT)))
        .on_click(cx.listener(|_, _, _, cx| {
            open_add_modal();
            cx.notify();
        }))
        .child(gpui::svg().path(crate::icons::path("plus-medium")).size(z(18.)).flex_none().text_color(c(tok(idx::CONTROL_PRIMARY_TEXT_DEFAULT))))
        .child(crate::t!("fav.empty_add"));
    div()
        .flex_1()
        .min_w_0()
        .h_full()
        .relative()
        .bg(c(tok(idx::BACKGROUND_BASE_LOWER)))
        .border_t_1()
        .border_color(c(tok(idx::APP_FRAME_BORDER)))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .child(
            div()
                .w_full()
                .max_w(z(440.))
                .flex()
                .flex_col()
                .items_center()
                .gap(z(16.))
                .child(gpui::img(EMPTY_ART).w(z(197.)).h(z(107.)).flex_none())
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .child(
                            div()
                                .max_w(z(321.))
                                .text_center()
                                .text_size(z(24.))
                                .line_height(z(32.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(c(tok(idx::TEXT_DEFAULT)))
                                .child(crate::t!("fav.empty_title")),
                        )
                        .child(
                            div()
                                .mt(z(8.))
                                .max_w(z(321.))
                                .text_center()
                                .text_size(z(16.))
                                .line_height(z(20.))
                                .text_color(c(tok(idx::TEXT_MUTED)))
                                .child(crate::t!("fav.empty_note")),
                        ),
                )
                .child(button),
        )
        .into_any_element()
}

pub fn data(app: &PhoenixApp) -> &Favorites {
    &app.prefs.synced.favorites
}

pub fn enabled(app: &PhoenixApp) -> bool {
    data(app).guild_enabled()
}

pub fn is_favorite(app: &PhoenixApp, channel: Id) -> bool {
    data(app).is_favorite(channel.0)
}

pub fn nickname(app: &PhoenixApp, channel: Id) -> Option<SharedString> {
    if !app.favorites_view {
        return None;
    }
    app.channels.get(&channel)?.guild?;
    data(app).nickname(channel.0).map(|n| SharedString::from(n.to_owned()))
}

fn category(id: Id) -> Channel {
    Channel {
        id,
        guild: Some(GUILD),
        parent: None,
        name: SharedString::default(),
        topic: None,
        kind: ChannelKind::Category,
        position: 0,
        unread: false,
        muted: false,
        mentions: 0,
        voice_users: Vec::new(),
    }
}

pub fn wire_type(ch: &Channel, group_dm: bool) -> u32 {
    match (ch.guild, ch.kind) {
        (None, _) if group_dm => 3,
        (None, _) => 1,
        (_, ChannelKind::Voice) => 2,
        (_, ChannelKind::Category) => 4,
        (_, ChannelKind::Announcement) => 5,
        (_, ChannelKind::Stage) => 13,
        (_, ChannelKind::Forum) => 15,
        _ => 0,
    }
}

fn channel_type(app: &PhoenixApp, id: Id) -> Option<u32> {
    if let Some(ch) = app.channels.get(&id) {
        return Some(wire_type(ch, crate::ui::dm_profile_panel::is_group_dm(app, id)));
    }
    app.nav.find_thread(id).map(|_| 11)
}

fn known(app: &PhoenixApp, id: u64) -> Known {
    match channel_type(app, Id(id)) {
        Some(t) => Known::Visible { channel_type: t },
        None => Known::Missing,
    }
}

pub fn voice_users(app: &PhoenixApp) -> Vec<Id> {
    let mut out = Vec::new();
    for id in data(app).favorite_channels.keys() {
        if let Some(ch) = app.channels.get(&Id(*id)).filter(|c| c.kind == ChannelKind::Voice) {
            for u in &ch.voice_users {
                if !out.contains(u) {
                    out.push(*u);
                }
            }
        }
    }
    out
}

pub fn badge(app: &PhoenixApp) -> (bool, u32) {
    let mut unread = false;
    let mut mentions = 0;
    for id in data(app).favorite_channels.keys() {
        if let Some(ch) = app.channels.get(&Id(*id)) {
            unread |= ch.unread && !ch.muted;
            mentions += ch.mentions;
        }
    }
    (unread && !data(app).muted, mentions)
}

pub fn rows(app: &mut PhoenixApp) -> Vec<SidebarRow> {
    let fav = data(app).clone();
    for (id, name) in fav.categories() {
        let cid = Id(id);
        let entry = app.channels.entry(cid).or_insert_with(|| category(cid));
        entry.name = SharedString::from(name.to_owned());
        if fav.get(id).is_some_and(|f| f.collapsed) && !app.favorites_seeded.contains(&cid) {
            app.collapsed.insert(cid);
        }
        app.favorites_seeded.insert(cid);
    }
    let order = fav.rows(|id| channel_type(app, Id(id)));
    let mut out = Vec::with_capacity(order.len());
    let mut collapsed = false;
    for r in order {
        match r {
            Row::Category(id) => {
                collapsed = app.collapsed.contains(&Id(id));
                out.push(SidebarRow::Category(Id(id)));
            }
            Row::Channel(id) => {
                let id = Id(id);
                let ch = app.channels.get(&id);
                let selected = app.selected_channel == Some(id);
                let keep = !collapsed || selected || ch.is_some_and(|c| (c.unread && !c.muted) || !c.voice_users.is_empty());
                if keep {
                    out.push(SidebarRow::Channel(id));
                }
            }
        }
    }
    out
}

pub fn open(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) {
    app.favorites_view = true;
    let fav = data(app).clone();
    let first = fav.rows(|id| channel_type(app, Id(id))).into_iter().find_map(|r| match r {
        Row::Channel(id) => app.channels.get(&Id(id)).filter(|c| !matches!(c.kind, ChannelKind::Voice | ChannelKind::Stage)).map(|c| c.id),
        Row::Category(_) => None,
    });
    let target = app.favorites_last.filter(|c| fav.is_favorite(c.0)).or(first);
    app.rebuild_sidebar();
    match target {
        Some(t) => app.open_channel_chat(t, cx),
        None => {
            app.selected_guild = None;
            app.selected_channel = None;
            app.rebuild_sidebar();
            app.rebuild_members();
        }
    }
    cx.notify();
}

pub fn on_channel_opened(app: &mut PhoenixApp, channel: Id) {
    if !app.favorites_view {
        return;
    }
    let parent = app.nav.thread_parent(channel);
    if is_favorite(app, channel) || parent.is_some_and(|p| is_favorite(app, p)) {
        app.favorites_last = Some(channel);
    } else {
        app.favorites_view = false;
    }
}

fn save(app: &mut PhoenixApp, fav: Favorites, cx: &mut Context<PhoenixApp>) {
    crate::ui::settings::save(app, SettingsPatch::Favorites(Box::new(fav)), cx);
    app.rebuild_sidebar();
}

pub fn add(app: &mut PhoenixApp, channel: Id, parent: Option<u64>, cx: &mut Context<PhoenixApp>) {
    let mut fav = data(app).clone();
    fav.prune(|id| known(app, id));
    let ty = channel_type(app, channel);
    let nitro = app.backend.premium_tier2();
    match fav.add(&[(channel.0, ty)], parent, nitro) {
        Ok(true) => {
            save(app, fav, cx);
            app.parity_toast(crate::t!("fav.added"), cx);
        }
        Ok(false) => {}
        Err(limit) => app.parity_toast(crate::t!("fav.limit", count = limit), cx),
    }
}

pub fn add_many(app: &mut PhoenixApp, channels: &[Id], cx: &mut Context<PhoenixApp>) {
    let mut fav = data(app).clone();
    fav.prune(|id| known(app, id));
    let list: Vec<(u64, Option<u32>)> = channels.iter().map(|c| (c.0, channel_type(app, *c))).collect();
    match fav.add(&list, None, app.backend.premium_tier2()) {
        Ok(true) => {
            save(app, fav, cx);
            app.parity_toast(crate::t!("fav.added"), cx);
        }
        Ok(false) => {}
        Err(limit) => app.parity_toast(crate::t!("fav.limit", count = limit), cx),
    }
}

pub fn open_add_modal() {
    crate::actions::open_modal(crate::ui::modals::Modal::Forward(crate::ui::modals::forward::Forward::favorites()), None);
}

pub fn create_category(app: &mut PhoenixApp, name: &str, cx: &mut Context<PhoenixApp>) {
    if name.trim().is_empty() {
        return;
    }
    let mut fav = data(app).clone();
    fav.prune(|id| known(app, id));
    if let Some(limit) = fav.limit_reached(phoenix_api::rest::favorites::CATEGORY, app.backend.premium_tier2()) {
        app.parity_toast(crate::t!("fav.limit", count = limit), cx);
        return;
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    fav.create_category(name.trim(), snowflake_at(now));
    save(app, fav, cx);
}

pub fn hide(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) {
    let mut fav = data(app).clone();
    fav.set_guild_visible(false);
    save(app, fav, cx);
    if app.favorites_view {
        app.select_guild(None, cx);
    }
}

pub fn open_create_category(cx: &mut Context<PhoenixApp>) {
    let form = crate::ui::modals::Form {
        title: crate::t!("fav.create_category"),
        subtitle: None,
        fields: vec![crate::ui::modals::Field::new(crate::t!("fav.category_name"), crate::t!("fav.category_ph"), "").max_len(100)],
        choices: Vec::new(),
        choice: None,
        check: None,
        submit: crate::t!("fav.create_category"),
        danger: false,
        preview: None,
        width: 480.,
        on_submit: std::rc::Rc::new(|a, v, _, cx| create_category(a, v.first().map(String::as_str).unwrap_or(""), cx)),
    };
    crate::actions::open_modal(crate::ui::modals::Modal::Form(form), None);
    cx.notify();
}

pub fn add_to_new_category(app: &mut PhoenixApp, channel: Id, name: &str, cx: &mut Context<PhoenixApp>) {
    if name.trim().is_empty() {
        return;
    }
    let mut fav = data(app).clone();
    fav.prune(|id| known(app, id));
    let ty = channel_type(app, channel);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    match fav.add_to_category(&[(channel.0, ty)], name.trim(), snowflake_at(now), app.backend.premium_tier2()) {
        Ok(true) => {
            save(app, fav, cx);
            app.parity_toast(crate::t!("fav.added"), cx);
        }
        Ok(false) => {}
        Err(limit) => app.parity_toast(crate::t!("fav.limit", count = limit), cx),
    }
}

pub fn remove(app: &mut PhoenixApp, channel: Id, cx: &mut Context<PhoenixApp>) {
    let mut fav = data(app).clone();
    if !fav.remove(channel.0) {
        return;
    }
    fav.prune(|id| known(app, id));
    if app.favorites_view && app.selected_channel == Some(channel) {
        app.favorites_last = None;
    }
    save(app, fav, cx);
}

pub fn move_to(app: &mut PhoenixApp, channel: Id, parent: Option<u64>, cx: &mut Context<PhoenixApp>) {
    let mut fav = data(app).clone();
    if !fav.is_favorite(channel.0) {
        return;
    }
    let position = fav.next_position();
    fav.apply_moves(&[phoenix_api::rest::favorites::Move { id: channel.0, position: Some(position), parent: Some(parent) }]);
    save(app, fav, cx);
}

pub fn toggle(app: &mut PhoenixApp, channel: Id, cx: &mut Context<PhoenixApp>) {
    if is_favorite(app, channel) {
        remove(app, channel, cx);
    } else {
        add(app, channel, None, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_types_follow_the_client_enum() {
        let mut ch = category(Id(1));
        ch.guild = Some(Id(2));
        ch.kind = ChannelKind::Text;
        assert_eq!(wire_type(&ch, false), 0);
        ch.kind = ChannelKind::Voice;
        assert_eq!(wire_type(&ch, false), 2);
        ch.kind = ChannelKind::Stage;
        assert_eq!(wire_type(&ch, false), 13);
        ch.kind = ChannelKind::Forum;
        assert_eq!(wire_type(&ch, false), 15);
        ch.guild = None;
        assert_eq!(wire_type(&ch, false), 1);
        assert_eq!(wire_type(&ch, true), 3);
    }
}
