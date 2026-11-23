//! 240px member list, virtualized.
//!
//! With a real session the rows come from the server-ordered lazy list
//! (GUILD_MEMBER_LIST_UPDATE, `People::lists`): hoisted role groups, then
//! Online / Offline, with placeholders for unsynced ranges. Scrolling asks
//! for the ranges in view (op 37). Without it (mock), `PhoenixApp::member_rows`
//! groups locally.

use gpui::{
    AnyElement, App, Context, Entity, FontWeight, IntoElement, ParentElement, SharedString, Styled, div,
    list, prelude::*, px,
};

use crate::{
    app::{MemberRow, PhoenixApp},
    backend::AppEvent,
    model::{Id, Member, Presence},
    presence::{ListEntry, ranges_for},
    theme::{Palette, c, m, role_color},
    ui::avatar_with_status,
};

/// `AppEvent::MemberListRows` / `AppEvent::Activities`.
pub fn on_event(app: &mut PhoenixApp, ev: AppEvent, cx: &mut Context<PhoenixApp>) {
    match ev {
        AppEvent::MemberListRows { guild, rows } => {
            app.people.lists.insert(guild, rows);
            if app.selected_guild == Some(guild) {
                sync_server_list(app);
            }
        }
        AppEvent::Activities { user, activities } => app.people.on_activities(user, activities),
        _ => return,
    }
    cx.notify();
}

/// If the open guild has a server list, size the virtual list for it
/// (keeping the scroll position) and return true.
pub fn sync_server_list(app: &mut PhoenixApp) -> bool {
    let Some(len) = app.selected_guild.and_then(|g| app.people.lists.get(&g)).map(Vec::len) else {
        return false;
    };
    app.member_rows.clear();
    let old = app.member_list.item_count();
    if old == len {
        app.member_list.remeasure();
    } else if old < len {
        app.member_list.splice(old..old, len - old);
    } else {
        app.member_list.splice(len..old, 0);
    }
    true
}

/// Ask for the ranges around the scroll position when it enters a new chunk.
fn request_ranges(app: &PhoenixApp, g: Id, cx: &mut Context<PhoenixApp>) {
    let (Some(ch), true) = (app.selected_channel, app.people.lists.contains_key(&g)) else { return };
    let top = app.member_list.logical_scroll_top().item_ix;
    let key = (g, top / 100);
    if app.people.requested.get() == key {
        return;
    }
    app.people.requested.set(key);
    if key.1 == 0 {
        return; // [0, 99] is subscribed when the channel opens.
    }
    let fut = app.backend.subscribe_member_ranges(g, ch, ranges_for(top));
    cx.background_spawn(fut).detach();
}

pub fn render(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let entity = cx.entity();
    if let Some(g) = app.selected_guild {
        request_ranges(app, g, cx);
    }
    div()
        .w(px(m::MEMBER_LIST_W))
        .flex_none()
        .h_full()
        .bg(c(Palette::BG_SECONDARY))
        .child(list(app.member_list.clone(), move |ix, _w, cx| row(&entity, ix, cx)).size_full())
}

fn header(t: SharedString) -> AnyElement {
    div()
        .pt(px(24.))
        .pl(px(16.))
        .pr(px(8.))
        .h(px(40.))
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(c(Palette::TEXT_MUTED))
        .text_ellipsis()
        .child(t)
        .into_any_element()
}

fn group_title(app: &PhoenixApp, g: Id, id: &str, count: u32) -> SharedString {
    let name = match id {
        "online" => "Online".to_owned(),
        "offline" => "Offline".to_owned(),
        _ => id
            .parse::<u64>()
            .ok()
            .and_then(|r| app.roles.get(&g)?.iter().find(|x| x.id == Id(r)).map(|x| x.name.to_string()))
            .unwrap_or_else(|| "Members".to_owned()),
    };
    format!("{} — {count}", name.to_uppercase()).into()
}

fn placeholder(ix: usize) -> AnyElement {
    // Deterministic widths so the skeleton doesn't flicker.
    let w = 60. + ((ix * 37) % 80) as f32;
    div()
        .ml(px(16.))
        .h(px(m::MEMBER_ROW_H))
        .flex()
        .items_center()
        .gap(px(12.))
        .child(div().size(px(32.)).rounded_full().bg(c(Palette::BG_MOD_HOVER)))
        .child(div().w(px(w)).h(px(12.)).rounded(px(6.)).bg(c(Palette::BG_MOD_HOVER)))
        .into_any_element()
}

fn row(entity: &Entity<PhoenixApp>, ix: usize, cx: &mut App) -> AnyElement {
    let app = entity.read(cx);
    let Some(g) = app.selected_guild else { return div().into_any_element() };
    if let Some(rows) = app.people.lists.get(&g) {
        return match rows.get(ix) {
            Some(ListEntry::Group { id, count }) => header(group_title(app, g, id, *count)),
            Some(ListEntry::Member(mem, nick)) => member_row(app, entity, g, mem, nick.as_ref()),
            Some(ListEntry::Loading) => placeholder(ix),
            None => div().into_any_element(),
        };
    }
    match app.member_rows.get(ix) {
        Some(MemberRow::Header(t)) => header(t.clone()),
        Some(MemberRow::Member(i)) => match app.members.get(&g).and_then(|ms| ms.get(*i)) {
            Some(mem) => member_row(app, entity, g, mem, None),
            None => div().into_any_element(),
        },
        None => div().into_any_element(),
    }
}

fn member_row(
    app: &PhoenixApp,
    entity: &Entity<PhoenixApp>,
    g: Id,
    mem: &Member,
    nick: Option<&SharedString>,
) -> AnyElement {
    let user = app.users.get(&mem.user);
    let offline = mem.presence == Presence::Offline;
    let name = nick.cloned().or_else(|| user.map(|u| u.display_name.clone())).unwrap_or_default();
    let uid = mem.user;
    let e = entity.clone();
    div()
        .id(("member", mem.user.0))
        .ml(px(8.))
        .mr(px(8.))
        .h(px(m::MEMBER_ROW_H))
        .px(px(8.))
        .rounded(px(4.))
        .flex()
        .items_center()
        .gap(px(12.))
        .cursor_pointer()
        .when(offline, |d| d.opacity(0.3))
        .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)).opacity(1.))
        .when(app.people.popout.as_ref().is_some_and(|p| p.user == uid), |d| {
            d.bg(c(Palette::BG_MOD_SELECTED)).opacity(1.)
        })
        .on_click(move |ev, window, cx| {
            let at = ev.position();
            e.update(cx, |a, cx| a.open_profile(uid, Some(g), at, window, cx));
        })
        .on_mouse_down(gpui::MouseButton::Right, move |ev, window, cx| {
            cx.stop_propagation();
            crate::ui::user_menu::open(uid, Some(g), ev.position, window);
        })
        .child(avatar_with_status(user, 32., mem.presence, c(Palette::BG_SECONDARY)))
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
                        .gap(px(4.))
                        .child(
                            div()
                                .text_size(px(16.))
                                .line_height(px(20.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(role_color(app.member_color(g, mem)))
                                .text_ellipsis()
                                .child(name),
                        )
                        .when(user.is_some_and(|u| u.bot), |d| d.child(app_tag())),
                )
                .when_some(mem.activity.clone().filter(|_| !offline), |d, a| {
                    d.child(
                        div()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .text_color(c(Palette::TEXT_MUTED))
                            .text_ellipsis()
                            .child(a),
                    )
                }),
        )
        .into_any_element()
}

pub fn app_tag() -> gpui::Div {
    div()
        .px(px(4.))
        .rounded(px(3.))
        .bg(c(Palette::BLURPLE))
        .text_size(px(10.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(gpui::white())
        .child("APP")
}
