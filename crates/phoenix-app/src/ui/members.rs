//! 240px member list, virtualized.
//!
//! With a real session the rows come from the server-ordered lazy list
//! (GUILD_MEMBER_LIST_UPDATE, `People::lists`): hoisted role groups, then
//! Online / Offline, with placeholders for unsynced ranges. Scrolling asks
//! for the ranges in view (op 37). Without it (mock), `PhoenixApp::member_rows`
//! groups locally.

use crate::theme::z;
use gpui::{
    AnyElement, App, Context, Entity, FontWeight, IntoElement, ParentElement, SharedString, Styled, div,
    list, prelude::*, px,
};

use crate::{
    app::{MemberRow, PhoenixApp},
    backend::AppEvent,
    model::{Id, Member, Presence},
    assets::Icon,
    presence::{Activity, ActivityKind, ListEntry, ranges_for},
    theme::{Palette, c, generated::idx, m, role_color, tok},
    ui::{avatar_with_status, icon},
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
        .w(z(m::MEMBER_LIST_W)) // --custom-member-list-width
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .bg(c(Palette::BG_SECONDARY)) // container_86es0b --custom-channel-members-bg = --background-base-lower
        .border_l_1()
        .border_color(c(Palette::DIVIDER)) // container_86es0b border-inline-start --border-subtle
        .when_some(app.selected_guild, |d, g| d.child(counts_row(app, g)).child(activity_section(app, &entity, g)))
        .child(
            list(app.member_list.clone(), move |ix, _w, cx| row(&entity, ix, cx))
                .flex_1()
                .w_full()
                .pb(z(10.)), // members_86es0b padding-bottom 10px
        )
}

/// (online, total) from the server list groups, or the local mock list.
fn counts(app: &PhoenixApp, g: Id) -> (u32, u32) {
    if let Some(rows) = app.people.lists.get(&g) {
        let mut online = 0;
        let mut total = 0;
        for r in rows {
            if let ListEntry::Group { id, count } = r {
                total += count;
                if id != "offline" {
                    online += count;
                }
            }
        }
        return (online, total);
    }
    let ms = app.members.get(&g).map(Vec::as_slice).unwrap_or_default();
    let online = ms.iter().filter(|m| m.presence != Presence::Offline).count() as u32;
    (online, ms.len() as u32)
}

/// Header counts: online, members, in voice. The bundle for this row isn't
/// in the cached CSS; sizes follow the member row text (16/20, gap --space-16).
fn counts_row(app: &PhoenixApp, g: Id) -> AnyElement {
    let (online, total) = counts(app, g);
    let voice: usize = app
        .guild_channels
        .get(&g)
        .map(|ids| ids.iter().filter_map(|i| app.channels.get(i)).map(|c| c.voice_users.len()).sum())
        .unwrap_or(0);
    let stat = |dot: AnyElement, n: String, color: u32| {
        div().flex().items_center().gap(z(6.)).child(dot).child(div().text_color(c(color)).child(n))
    };
    div()
        .flex_none()
        .h(z(48.))
        .px(z(16.)) // membersGroup_86es0b padding-inline-start --space-md
        .flex()
        .items_center()
        .gap(z(16.)) // --space-16
        .text_size(z(16.))
        .line_height(z(20.))
        .font_weight(FontWeight::MEDIUM)
        .child(stat(
            div().size(z(10.)).rounded_full().bg(c(Palette::GREEN)).into_any_element(),
            online.to_string(),
            Palette::GREEN,
        ))
        .child(stat(
            div().size(z(10.)).rounded_full().border_2().border_color(c(Palette::TEXT_MUTED)).into_any_element(),
            total.to_string(),
            Palette::TEXT_MUTED,
        ))
        .when(voice > 0, |d| {
            d.child(stat(icon(Icon::Mic, 16., c(Palette::TEXT_MUTED)).into_any_element(), voice.to_string(), Palette::TEXT_MUTED))
        })
        .into_any_element()
}

/// "2d ago" style elapsed time since `start_ms`.
pub fn elapsed(start_ms: i64, now_s: i64) -> String {
    let s = (now_s - start_ms / 1000).max(0);
    match s {
        0..60 => "now".into(),
        60..3600 => format!("{}m ago", s / 60),
        3600..86_400 => format!("{}h ago", s / 3600),
        _ => format!("{}d ago", s / 86_400),
    }
}

/// Members of `g` currently playing something, newest first.
fn playing(app: &PhoenixApp, g: Id) -> Vec<(Id, &Activity)> {
    let in_guild = |u: &Id| match app.people.lists.get(&g) {
        Some(rows) => rows.iter().any(|r| matches!(r, ListEntry::Member(m, _) if m.user == *u)),
        None => app.members.get(&g).is_some_and(|ms| ms.iter().any(|m| m.user == *u)),
    };
    let mut v: Vec<(Id, &Activity)> = app
        .people
        .activities
        .iter()
        .filter(|(u, _)| in_guild(u))
        .filter_map(|(u, acts)| Some((*u, acts.iter().find(|a| a.kind == ActivityKind::Playing)?)))
        .collect();
    v.sort_by_key(|(u, a)| (std::cmp::Reverse(a.start.unwrap_or(0)), u.0));
    v
}

/// The "Activity" section at the top of the member list: one card per
/// member playing a game (name, game title, elapsed time, game art).
/// Card values: contentInventory card in 4187692321375e83 (--space-12
/// padding/gap, --radius-sm, --border-subtle) and the 72px post thumbnail.
fn activity_section(app: &PhoenixApp, entity: &Entity<PhoenixApp>, g: Id) -> AnyElement {
    let acts = playing(app, g);
    if acts.is_empty() {
        return div().into_any_element();
    }
    let now = crate::mock::now();
    let mut col = div().flex_none().flex().flex_col().child(
        div()
            .flex()
            .items_center()
            .pr(z(8.))
            .child(div().flex_1().min_w_0().child(header(format!("Activity — {}", acts.len()).into())))
            .child(gpui::svg().path("icons/chevron-right.svg").size(z(16.)).flex_none().mt(z(16.)).text_color(c(Palette::INTERACTIVE))),
    );
    for (uid, a) in acts.into_iter().take(3) {
        let user = app.users.get(&uid);
        let mem = app.members.get(&g).and_then(|ms| ms.iter().find(|m| m.user == uid));
        let color = mem.and_then(|m| app.member_color(g, m));
        let name = user.map(|u| u.display_name.clone()).unwrap_or_default();
        let e = entity.clone();
        let art = match a.image.clone() {
            Some(url) => gpui::img(url).size_full().into_any_element(),
            None => div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_size(z(24.))
                .font_weight(FontWeight::BOLD)
                .text_color(c(Palette::TEXT_MUTED))
                .child(SharedString::from(a.name.chars().next().map(String::from).unwrap_or_default()))
                .into_any_element(),
        };
        col = col.child(
            div()
                .id(("activity-card", uid.0))
                .mx(z(8.)) // member_Iy0c7A margin-inline-start 8px
                .mb(z(8.)) // --space-8
                .p(z(12.)) // container_DKsPd0 padding --space-12
                .rounded(z(8.)) // --radius-sm
                .border_1()
                .border_color(c(Palette::DIVIDER)) // --border-subtle
                .bg(c(Palette::BG_TERTIARY)) // --background-base-lowest
                .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
                .cursor_pointer()
                .flex()
                .items_center()
                .gap(z(12.)) // details_DKsPd0 margin 0 --space-12
                .on_click(move |ev, window, cx| {
                    let at = ev.position();
                    e.update(cx, |a, cx| a.open_profile(uid, Some(g), at, window, cx));
                })
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(z(2.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(z(6.))
                                .child(crate::ui::avatar(user, 16.))
                                .child(
                                    div()
                                        .min_w_0()
                                        .text_size(z(14.))
                                        .line_height(z(18.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(role_color(color))
                                        .text_ellipsis()
                                        .child(name),
                                )
                                .when_some(user.and_then(|u| u.tag.clone()), |d, t| d.child(server_tag(t.0))),
                        )
                        .child(
                            div()
                                .text_size(z(16.))
                                .line_height(z(20.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(c(Palette::TEXT_HEADER)) // --text-strong
                                .text_ellipsis()
                                .child(a.name.clone()),
                        )
                        .when_some(a.start, |d, st| {
                            d.child(
                                div()
                                    .text_size(z(12.))
                                    .line_height(z(16.))
                                    .text_color(c(Palette::TEXT_MUTED))
                                    .child(elapsed(st, now)),
                            )
                        }),
                )
                .child(
                    div()
                        .flex_none()
                        .size(z(72.)) // thumbnailContainer --__post-thumbnail-size 72px
                        .rounded(z(8.)) // bodyMedia --radius-sm
                        .overflow_hidden()
                        .bg(c(Palette::BG_MOD_SELECTED))
                        .child(art),
                ),
        );
    }
    col.into_any_element()
}

/// Server tag chiplet: chipletContainerInner_k8jyN7 (--background-mod-strong,
/// 4px radius, 0 4px padding, 16px line height).
pub fn server_tag(tag: SharedString) -> gpui::Div {
    div()
        .flex_none()
        .px(z(4.))
        .rounded(z(4.))
        .bg(c(tok(idx::BACKGROUND_MOD_STRONG))) // --background-mod-strong
        .text_size(z(12.))
        .line_height(z(16.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(c(Palette::TEXT_NORMAL))
        .child(tag)
}

fn header(t: SharedString) -> AnyElement {
    // membersGroup_86es0b: padding-block --space-lg --space-xxs (20/4),
    // padding-inline --space-md --space-xxs (16/4), height auto. Not
    // uppercased in the 2026 client; 14px/500 (Text sm/medium).
    div()
        .pt(z(20.))
        .pb(z(4.))
        .pl(z(16.))
        .pr(z(4.))
        .text_size(z(14.))
        .line_height(z(18.))
        .font_weight(FontWeight::MEDIUM)
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
    format!("{name} — {count}").into()
}

fn placeholder(ix: usize) -> AnyElement {
    // Deterministic widths so the skeleton doesn't flicker.
    let w = 60. + ((ix * 37) % 80) as f32;
    div()
        .ml(z(16.))
        .h(px(m::MEMBER_ROW_H))
        .flex()
        .items_center()
        .gap(z(12.))
        .child(div().size(z(32.)).rounded_full().bg(c(Palette::BG_MOD_HOVER)))
        .child(div().w(px(w)).h(z(12.)).rounded(z(6.)).bg(c(Palette::BG_MOD_HOVER)))
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
    let owner = app.guilds.iter().find(|x| x.id == g).and_then(|x| x.owner) == Some(uid);
    let tag = user.and_then(|u| u.tag.clone());
    // member_Iy0c7A margin-inline-start 8px, max-width list - 16px;
    // memberInner_Iy0c7A padding-block --space-xxs (4px) around the 42px layout.
    div().py(z(4.)).child(div()
        .id(("member", mem.user.0))
        .ml(z(8.))
        .mr(z(8.))
        .h(px(m::MEMBER_ROW_H)) // layout_TBNqol height 42px (density-default)
        .px(z(8.))
        .rounded(z(4.)) // layout_TBNqol border-radius 4px
        .flex()
        .items_center()
        .gap(z(12.)) // avatar_TBNqol margin-inline-end 12px
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
                        .gap(z(4.))
                        .child(
                            div()
                                .text_size(z(16.))
                                .line_height(z(20.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(role_color(app.member_color(g, mem)))
                                .text_ellipsis()
                                .child(name),
                        )
                        .when_some(tag, |d, t| d.child(server_tag(t.0).my(z(1.)))) // clanTag_Iy0c7A margin 1px 0 0 4px
                        .when(user.is_some_and(|u| u.bot), |d| d.child(app_tag())) // botTag_Iy0c7A margin-inline-start 4px
                        .when(owner, |d| {
                            // ownerIcon_Iy0c7A: 14px (icon_Iy0c7A), --text-feedback-warning
                            d.child(gpui::svg().path("icons/crown.svg").size(z(14.)).flex_none().text_color(c(tok(idx::TEXT_FEEDBACK_WARNING))))
                        }),
                )
                .when_some(mem.activity.clone().filter(|_| !offline), |d, a| {
                    d.child(
                        div()
                            .mt(z(-2.)) // subText_TBNqol margin-top -2px
                            .text_size(z(12.))
                            .line_height(z(16.))
                            .text_color(c(Palette::TEXT_MUTED))
                            .text_ellipsis()
                            .child(a),
                    )
                }),
        ))
        .into_any_element()
}

pub fn app_tag() -> gpui::Div {
    div()
        .px(z(4.))
        .rounded(z(3.))
        .bg(c(Palette::BLURPLE))
        .text_size(z(10.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(gpui::white())
        .child("APP")
}
