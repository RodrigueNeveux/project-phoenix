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
    presence::{Activity, ActivityKind, ListEntry, ranges_for},
    theme::{Palette, c, generated::idx, m, role_color, tok},
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
    // +1: item 0 is the Activity section, which scrolls with
    // the list like the client's (they sit inside `members_86es0b`).
    let Some(len) = app.selected_guild.and_then(|g| app.people.lists.get(&g)).map(|l| l.len() + 1) else {
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
    let top = app.member_list.logical_scroll_top().item_ix.saturating_sub(1);
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
        .child(
            list(app.member_list.clone(), move |ix, _w, cx| row(&entity, ix, cx))
                .flex_1()
                .w_full()
                .pb(z(10.)), // members_86es0b padding-bottom 10px
        )
}

/// "2d ago" style elapsed time since `start_ms`.
pub fn elapsed(start_ms: i64, now_s: i64) -> String {
    let s = (now_s - start_ms / 1000).max(0);
    match s {
        0..60 => crate::t!("time.now").to_string(),
        60..3600 => crate::t!("time.minutes_ago_short", n = s / 60).to_string(),
        3600..86_400 => crate::t!("time.hours_ago_short", n = s / 3600).to_string(),
        _ => crate::t!("time.days_ago_short", n = s / 86_400).to_string(),
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
/// member playing a game. Dump `-channels-N-N+` nodes 1502 (header) and
/// 1512 (card `container_g7esM5 usesCardRows_g7esM5`).
fn activity_section(app: &PhoenixApp, entity: &Entity<PhoenixApp>, g: Id) -> AnyElement {
    let acts = playing(app, g);
    if acts.is_empty() {
        return div().into_any_element();
    }
    let now = crate::mock::now();
    // headerContainer_HxSvbL: flex, margin -3px 0; label, 24px settingsButton
    // (padding 4, 12px gear), spacer, 24x18 toggleExpandIcon.
    let muted = c(Palette::TEXT_MUTED);
    let mut col = div().flex_none().flex().flex_col().child(
        div()
            .pt(z(24.)) // membersGroup_86es0b padding 24px 4px 4px 20px
            .pb(z(4.))
            .pl(z(20.))
            .pr(z(4.))
            .child(
                div()
                    .h(z(24.))
                    .my(z(-3.))
                    .flex()
                    .items_center()
                    .text_size(z(14.))
                    .line_height(z(18.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(muted)
                    .child(crate::t!("members.activity", n = acts.len()))
                    .child(
                        div()
                            .size(z(24.))
                            .p(z(4.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(gpui::svg().path(crate::icons::path("gear")).size(z(12.)).text_color(muted)),
                    )
                    .child(div().flex_1())
                    .child(gpui::svg().path(crate::icons::path("chevron")).w(z(24.)).h(z(18.)).text_color(c(Palette::INTERACTIVE))),
            ),
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
                .text_size(z(16.))
                .font_weight(FontWeight::BOLD)
                .text_color(muted)
                .child(SharedString::from(a.name.chars().next().map(String::from).unwrap_or_default()))
                .into_any_element(),
        };
        let badge = |ic: &'static str, t: SharedString| {
            // badgeContainer_CTWncy: gap 4, 12px icon, body-xs/normal 12/16
            div()
                .flex()
                .items_center()
                .gap(z(4.))
                .child(gpui::svg().path(ic).size(z(12.)).flex_none().text_color(c(Palette::TEXT_MUTED)))
                .child(div().text_size(z(12.)).line_height(z(16.)).text_color(c(Palette::TEXT_MUTED)).text_ellipsis().child(t))
        };
        col = col.child(
            div()
                .id(("activity-card", uid.0))
                .mt(z(8.)) // container_g7esM5 margin 8px 0 0 8px
                .ml(z(8.))
                .mr(z(8.))
                .p(z(8.)) // padding 8px
                .rounded(z(12.)) // border-radius 12px
                .border_1()
                .border_color(c(Palette::DIVIDER)) // --border-subtle (#97979f @16%)
                .bg(c(Palette::BG_TERTIARY)) // --background-base-lowest (#0a0a0c midnight)
                .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
                .cursor_pointer()
                .flex()
                .gap(z(8.)) // cardRow_g7esM5 gap 8px
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
                        .mb(z(-2.)) // infoSection_g7esM5 margin-bottom -2px
                        .child(
                            // userSection_g7esM5: 18px, margin-bottom 2px; 16px facePile, margin-right 6
                            div()
                                .h(z(18.))
                                .mb(z(2.))
                                .flex()
                                .items_center()
                                .child(div().mr(z(6.)).child(crate::ui::avatar(user, 16.)))
                                .child(
                                    div()
                                        .min_w_0()
                                        .text_size(z(14.))
                                        .line_height(z(18.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(role_color(color))
                                        .text_ellipsis()
                                        .child(name),
                                ),
                        )
                        .child(
                            // heading-xs/medium lineClamp1: 14/18, 500, --text-default #d4d5d8
                            div()
                                .text_size(z(14.))
                                .line_height(z(18.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(c(Palette::TEXT_NORMAL))
                                .text_ellipsis()
                                .child(a.name.clone()),
                        )
                        .child(
                            // badgesContainer_CTWncy: gap 0 8px
                            div()
                                .h(z(16.))
                                .flex()
                                .gap(z(8.))
                                .overflow_hidden()
                                .when_some(a.start, |d, st| d.child(badge("icons/controller.svg", elapsed(st, now).into())))
                                .when_some(a.state.clone().or(a.details.clone()), |d, t| d.child(badge("icons/refresh.svg", t))),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .size(z(48.)) // thumbnailButton_NJVMoP 48x48, radius 8; img radius 4
                        .rounded(z(4.))
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
    // Dump node 1598: padding 24px 4px 4px 20px, 46px tall.
    div()
        .pt(z(24.))
        .pb(z(4.))
        .pl(z(20.))
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
        "online" => crate::t!("status.online").to_string(),
        "offline" => crate::t!("status.offline").to_string(),
        _ => id
            .parse::<u64>()
            .ok()
            .and_then(|r| app.roles.get(&g)?.iter().find(|x| x.id == Id(r)).map(|x| x.name.to_string()))
            .unwrap_or_else(|| crate::t!("members.members").to_string()),
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
    // Item 0: the Activity section, scrolled with the members. (The online /
    // total / voice counts row was Vencord's MemberCount plugin, not vanilla.)
    let Some(ix) = ix.checked_sub(1) else {
        let entity = entity.clone();
        return div().flex().flex_col().child(activity_section(app, &entity, g)).into_any_element();
    };
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
    let in_voice = app
        .guild_channels
        .get(&g)
        .is_some_and(|ids| ids.iter().filter_map(|i| app.channels.get(i)).any(|ch| ch.voice_users.contains(&uid)));
    // Dump `-channels-N-N+` node 1643: member_Iy0c7A margin-left 8px,
    // padding 1px 0, radius 8 (44px); childContainer_J1Adu_ padding 0 8px 0 10px;
    // memberInner_Iy0c7A padding 4px 0 (42px); avatar 32 + margin-right 12.
    div().py(z(1.)).child(div()
        .id(("member", mem.user.0))
        .ml(z(8.))
        .h(px(m::MEMBER_ROW_H))
        .pl(z(10.))
        .pr(z(8.))
        .py(z(4.))
        .rounded(z(8.))
        .flex()
        .items_center()
        .gap(z(12.))
        .cursor_pointer()
        .when(offline, |d| d.opacity(0.3))
        .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)).opacity(1.))
        // `nameplated_J1Adu_`: nameplate layer behind the row content.
        .relative()
        .when_some(
            user.and_then(|u| {
                let sel = app.people.popout.as_ref().is_some_and(|p| p.user == uid);
                crate::ui::nameplate::layer(u, m::MEMBER_ROW_H - 2., crate::ui::nameplate::Place::Member, false, sel)
            }),
            |d, l| d.child(l),
        )
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
                        .when_some(tag, |d, t| d.child(server_tag(t.0).my(z(1.)))) // chipletContainerInner_k8jyN7 margin 1px 0 1px 4px
                        .when(user.is_some_and(|u| u.bot), |d| d.child(app_tag())) // botTag_Iy0c7A margin-inline-start 4px
                        .when(owner, |d| {
                            // ownerIcon_Iy0c7A: 14px (icon_Iy0c7A), --text-feedback-warning
                            d.child(gpui::svg().path(crate::icons::path("crown")).size(z(14.)).flex_none().text_color(c(tok(idx::TEXT_FEEDBACK_WARNING))))
                        }),
                )
                .when(!offline && (in_voice || mem.activity.is_some()), |d| {
                    // subText_J1Adu_ margin-top -2px; container_HigqCD flex gap 4:
                    // 12px voice icon, "•" dot (--text-default), body-xs/medium 12/16 status.
                    d.child(
                        div()
                            .mt(z(-2.))
                            .h(z(16.))
                            .flex()
                            .items_center()
                            .gap(z(4.))
                            .text_size(z(12.))
                            .line_height(z(16.))
                            .text_color(c(Palette::TEXT_MUTED))
                            .when(in_voice, |d| d.child(gpui::svg().path(crate::icons::path("speaker")).size(z(12.)).flex_none().text_color(c(Palette::TEXT_MUTED))))
                            .when(in_voice && mem.activity.is_some(), |d| d.child(div().flex_none().text_color(c(Palette::TEXT_NORMAL)).child("•")))
                            .when_some(mem.activity.clone(), |d, a| {
                                d.child(div().min_w_0().font_weight(FontWeight::MEDIUM).text_ellipsis().child(a))
                            }),
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
        .child(crate::t!("common.app"))
}
