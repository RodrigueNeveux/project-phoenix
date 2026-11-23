//! Friends page (Online / All / Pending / Blocked / Add Friend) with the
//! "Active Now" column, and the Message Requests page.

use crate::theme::z;
use gpui::{
    AnyElement, Context, FontWeight, Hsla, IntoElement, ParentElement, SharedString, Styled, Window, div,
    prelude::*, px,
};
use gpui_component::input::{Input, InputEvent, InputState};

use crate::{
    app::PhoenixApp,
    assets::Icon,
    model::{Id, Presence},
    theme::{Palette, c, m},
    ui::{
        avatar_with_status, badge, icon,
        home::data::{FriendsTab, HomeView, NavRequest, RelAction, Relationship, RelationshipKind},
    },
};

pub fn render(app: &mut PhoenixApp, window: &mut Window, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    if app.nav.home == HomeView::MessageRequests {
        return message_requests(app, cx).into_any_element();
    }
    if app.nav.friends_tab == FriendsTab::AddFriend && app.nav.add_friend_input.is_none() {
        ensure_add_input(app, window, cx);
    }
    let body: AnyElement = match app.nav.friends_tab {
        FriendsTab::AddFriend => add_friend(app, cx).into_any_element(),
        tab => list(app, tab, cx).into_any_element(),
    };
    div()
        .flex_1()
        .min_w_0()
        .h_full()
        .flex()
        .flex_col()
        .bg(c(Palette::BG_PRIMARY))
        .child(header(app, cx))
        .child(
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_row()
                .child(div().flex_1().min_w_0().h_full().child(body))
                .child(active_now(app, cx)),
        )
        .into_any_element()
}

fn ensure_add_input(app: &mut PhoenixApp, window: &mut Window, cx: &mut Context<PhoenixApp>) {
    let input = cx.new(|cx| InputState::new(window, cx).placeholder("You can add friends with their Discord username."));
    let sub = cx.subscribe_in(&input, window, |this: &mut PhoenixApp, _, ev: &InputEvent, _w, cx| {
        if let InputEvent::PressEnter { .. } = ev {
            send_request(this, cx);
        }
    });
    app.nav.subs.push(sub);
    app.nav.add_friend_input = Some(input);
}

fn send_request(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) {
    let Some(input) = app.nav.add_friend_input.clone() else { return };
    let name = input.read(cx).value().trim().to_string();
    if name.is_empty() {
        return;
    }
    app.nav_request(
        NavRequest::Relationship { user: Id(0), action: RelAction::AddByName(name.clone()) },
        cx,
        move |this, res, _| {
            this.nav.add_friend_result = Some(match res {
                Ok(_) => Ok(format!("Success! Your friend request to {name} was sent.").into()),
                Err(e) => Err(if e.0.is_empty() {
                    "Hm, didn't work. Double check that the username is correct.".into()
                } else {
                    e.0.into()
                }),
            });
        },
    );
}

fn header(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let pending = app.nav.count(RelationshipKind::Incoming) as u32;
    let tab = |id: &'static str, label: &'static str, t: FriendsTab, cx: &mut Context<PhoenixApp>| {
        let sel = app.nav.friends_tab == t;
        div()
            .id(id)
            .mx(z(8.))
            .px(z(8.))
            .h(z(24.))
            .rounded(z(4.))
            .flex()
            .items_center()
            .gap(z(6.))
            .cursor_pointer()
            .text_size(z(16.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(c(if sel { Palette::TEXT_HEADER } else { Palette::INTERACTIVE }))
            .when(sel, |d| d.bg(c(Palette::BG_MOD_SELECTED)))
            .when(!sel, |d| d.hover(|s| s.bg(c(Palette::BG_MOD_HOVER)).text_color(c(Palette::INTERACTIVE_HOVER))))
            .on_click(cx.listener(move |a, _, _, cx| {
                a.nav.friends_tab = t;
                cx.notify();
            }))
            .child(label)
            .when(t == FriendsTab::Pending && pending > 0, |d| d.child(badge(pending)))
    };
    let add_sel = app.nav.friends_tab == FriendsTab::AddFriend;
    div()
        .h(px(m::HEADER_H))
        .flex_none()
        .px(z(16.))
        .flex()
        .items_center()
        .border_b_1()
        .border_color(c(Palette::HEADER_SHADOW))
        .child(icon(Icon::Friends, 24., c(Palette::CHANNEL_ICON)))
        .child(
            div()
                .ml(z(8.))
                .text_size(z(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c(Palette::TEXT_HEADER))
                .child("Friends"),
        )
        .child(div().mx(z(8.)).w(z(1.)).h(z(24.)).bg(c(Palette::DIVIDER)))
        .child(tab("tab-online", "Online", FriendsTab::Online, cx))
        .child(tab("tab-all", "All", FriendsTab::All, cx))
        .child(tab("tab-pending", "Pending", FriendsTab::Pending, cx))
        .child(tab("tab-blocked", "Blocked", FriendsTab::Blocked, cx))
        .child(
            div()
                .id("tab-add")
                .mx(z(8.))
                .px(z(8.))
                .h(z(24.))
                .rounded(z(4.))
                .flex()
                .items_center()
                .cursor_pointer()
                .text_size(z(14.))
                .font_weight(FontWeight::MEDIUM)
                .when(add_sel, |d| d.text_color(c(Palette::GREEN)))
                .when(!add_sel, |d| d.bg(c(Palette::GREEN)).text_color(gpui::white()))
                .on_click(cx.listener(|a, _, _, cx| {
                    a.nav.friends_tab = FriendsTab::AddFriend;
                    a.nav.add_friend_result = None;
                    cx.notify();
                }))
                .child("Add Friend"),
        )
}

fn visible(app: &PhoenixApp, tab: FriendsTab) -> Vec<Relationship> {
    let mut v: Vec<Relationship> = app
        .nav
        .relationships
        .iter()
        .filter(|r| match tab {
            FriendsTab::Online => r.kind == RelationshipKind::Friend && app.presence_of(r.user) != Presence::Offline,
            FriendsTab::All => r.kind == RelationshipKind::Friend,
            FriendsTab::Pending => matches!(r.kind, RelationshipKind::Incoming | RelationshipKind::Outgoing),
            FriendsTab::Blocked => r.kind == RelationshipKind::Blocked,
            FriendsTab::AddFriend => false,
        })
        .cloned()
        .collect();
    let name = |r: &Relationship| {
        app.users.get(&r.user).map(|u| u.display_name.to_lowercase()).unwrap_or_default()
    };
    v.sort_by_key(|r| (r.kind == RelationshipKind::Outgoing, name(r)));
    v
}

fn list(app: &PhoenixApp, tab: FriendsTab, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let rows = visible(app, tab);
    let label = match tab {
        FriendsTab::Online => "ONLINE",
        FriendsTab::All => "ALL FRIENDS",
        FriendsTab::Pending => "PENDING",
        _ => "BLOCKED",
    };
    let mut col = div()
        .id("friends-list")
        .size_full()
        .overflow_y_scroll()
        .pt(z(16.))
        .px(z(20.))
        .flex()
        .flex_col();
    if rows.is_empty() {
        let msg = match tab {
            FriendsTab::Online => "No one's around to play with Wumpus.",
            FriendsTab::Pending => "There are no pending friend requests. Here's Wumpus for now.",
            FriendsTab::Blocked => "You can't unblock the Wumpus.",
            _ => "Wumpus is waiting on friends. You don't have to though!",
        };
        return col
            .child(
                div()
                    .flex_1()
                    .pt(z(160.))
                    .flex()
                    .justify_center()
                    .text_size(z(16.))
                    .text_color(c(Palette::TEXT_MUTED))
                    .child(msg),
            )
            .into_any_element();
    }
    col = col.child(
        div()
            .pb(z(16.))
            .text_size(z(12.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(c(Palette::TEXT_MUTED))
            .child(SharedString::from(format!("{label} — {}", rows.len()))),
    );
    for r in rows {
        col = col.child(friend_row(app, &r, cx));
    }
    col.into_any_element()
}

fn round_button(id: (&'static str, u64), glyph: AnyElement, hover: Hsla) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .size(z(36.))
        .rounded_full()
        .bg(c(Palette::BG_SECONDARY))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .text_color(c(Palette::INTERACTIVE))
        .hover(move |s| s.text_color(hover))
        .child(glyph)
}

fn friend_row(app: &PhoenixApp, r: &Relationship, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let user = r.user;
    let u = app.users.get(&user);
    let p = app.presence_of(user);
    let name = r
        .nickname
        .clone()
        .or_else(|| u.map(|u| u.display_name.clone()))
        .unwrap_or_else(|| SharedString::from(user.0.to_string()));
    let sub: SharedString = match r.kind {
        RelationshipKind::Incoming => "Incoming Friend Request".into(),
        RelationshipKind::Outgoing => "Outgoing Friend Request".into(),
        RelationshipKind::Blocked => "Blocked".into(),
        RelationshipKind::Friend => app.nav.activities.get(&user).cloned().unwrap_or_else(|| {
            match p {
                Presence::Online => "Online",
                Presence::Idle => "Idle",
                Presence::Dnd => "Do Not Disturb",
                Presence::Offline => "Offline",
            }
            .into()
        }),
    };
    let username = u.map(|u| u.name.clone()).unwrap_or_default();
    let mut actions = div().flex().gap(z(10.));
    let act = |action: RelAction| {
        cx.listener(move |a: &mut PhoenixApp, _: &gpui::ClickEvent, _, cx| {
            // Optimistic: the gateway confirms with RELATIONSHIP_ADD/REMOVE.
            match &action {
                RelAction::Remove => a.nav.relationships.retain(|x| x.user != user),
                RelAction::Accept => {
                    if let Some(x) = a.nav.relationships.iter_mut().find(|x| x.user == user) {
                        x.kind = RelationshipKind::Friend;
                    }
                }
                _ => {}
            }
            a.nav_request(NavRequest::Relationship { user, action: action.clone() }, cx, |_, _, _| {});
            cx.notify();
        })
    };
    let check = || div().text_size(z(18.)).child("✓").into_any_element();
    let cross = || icon(Icon::Close, 20., c(Palette::INTERACTIVE)).into_any_element();
    match r.kind {
        RelationshipKind::Friend => {
            actions = actions
                .child(
                    round_button(("fr-msg", user.0), icon(Icon::Forum, 20., c(Palette::INTERACTIVE)).into_any_element(), c(Palette::INTERACTIVE_HOVER))
                        .on_click(cx.listener(move |a, _, _, cx| a.open_dm_with(user, cx))),
                )
                .child(round_button(("fr-rm", user.0), cross(), c(Palette::RED)).on_click(act(RelAction::Remove)));
        }
        RelationshipKind::Incoming => {
            actions = actions
                .child(round_button(("fr-ok", user.0), check(), c(Palette::GREEN)).on_click(act(RelAction::Accept)))
                .child(round_button(("fr-no", user.0), cross(), c(Palette::RED)).on_click(act(RelAction::Remove)));
        }
        RelationshipKind::Outgoing => {
            actions = actions.child(round_button(("fr-cancel", user.0), cross(), c(Palette::RED)).on_click(act(RelAction::Remove)));
        }
        RelationshipKind::Blocked => {
            actions = actions.child(
                round_button(("fr-unblock", user.0), icon(Icon::Members, 20., c(Palette::INTERACTIVE)).into_any_element(), c(Palette::RED))
                    .on_click(act(RelAction::Remove)),
            );
        }
    }
    div()
        .id(("friend", user.0))
        .group("friend-row")
        .h(z(62.))
        .flex_none()
        .border_t_1()
        .border_color(c(Palette::DIVIDER))
        .px(z(10.))
        .mx(z(-10.))
        .rounded(z(8.))
        .flex()
        .items_center()
        .gap(z(12.))
        .cursor_pointer()
        .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
        .on_click(cx.listener(move |a, _, _, cx| a.open_dm_with(user, cx)))
        .child(avatar_with_status(u, 32., p, c(Palette::BG_PRIMARY)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .gap(z(6.))
                        .items_baseline()
                        .child(
                            div()
                                .text_size(z(16.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(c(Palette::TEXT_HEADER))
                                .child(name),
                        )
                        .child(
                            div()
                                .text_size(z(14.))
                                .text_color(c(Palette::TEXT_MUTED))
                                .opacity(0.)
                                .group_hover("friend-row", |s| s.opacity(1.))
                                .child(username),
                        ),
                )
                .child(div().text_size(z(14.)).text_color(c(Palette::TEXT_MUTED)).text_ellipsis().child(sub)),
        )
        .child(actions)
}

fn add_friend(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let input = app.nav.add_friend_input.clone();
    let result = app.nav.add_friend_result.clone();
    let border = match &result {
        Some(Ok(_)) => Palette::GREEN,
        Some(Err(_)) => Palette::RED,
        None => Palette::BG_TERTIARY,
    };
    div()
        .px(z(30.))
        .py(z(20.))
        .flex()
        .flex_col()
        .border_b_1()
        .border_color(c(Palette::DIVIDER))
        .child(
            div()
                .text_size(z(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c(Palette::TEXT_HEADER))
                .child("ADD FRIEND"),
        )
        .child(
            div()
                .mt(z(8.))
                .text_size(z(14.))
                .text_color(c(Palette::TEXT_MUTED))
                .child("You can add friends with their Discord username."),
        )
        .child(
            div()
                .mt(z(16.))
                .h(z(52.))
                .px(z(12.))
                .rounded(z(8.))
                .bg(c(Palette::BG_TERTIARY))
                .border_1()
                .border_color(c(border))
                .flex()
                .items_center()
                .gap(z(12.))
                .child(
                    div()
                        .flex_1()
                        .text_size(z(16.))
                        .when_some(input, |d, i| d.child(Input::new(&i).appearance(false).bordered(false))),
                )
                .child(
                    div()
                        .id("send-fr")
                        .h(z(32.))
                        .px(z(16.))
                        .rounded(z(4.))
                        .bg(c(Palette::BLURPLE))
                        .text_color(gpui::white())
                        .text_size(z(14.))
                        .font_weight(FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .on_click(cx.listener(|a, _, _, cx| send_request(a, cx)))
                        .child("Send Friend Request"),
                ),
        )
        .when_some(result, |d, r| {
            let (col, text) = match r {
                Ok(t) => (Palette::GREEN, t),
                Err(t) => (Palette::RED, t),
            };
            d.child(div().mt(z(8.)).text_size(z(14.)).text_color(c(col)).child(text))
        })
}

fn active_now(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let mut active: Vec<(Id, SharedString)> = app
        .nav
        .relationships
        .iter()
        .filter(|r| r.kind == RelationshipKind::Friend)
        .filter_map(|r| app.nav.activities.get(&r.user).map(|a| (r.user, a.clone())))
        .filter(|(u, _)| app.presence_of(*u) != Presence::Offline)
        .collect();
    active.sort_by_key(|(u, _)| u.0);
    let mut col = div()
        .id("active-now")
        .w(z(360.))
        .flex_none()
        .h_full()
        .border_l_1()
        .border_color(c(Palette::DIVIDER))
        .px(z(16.))
        .py(z(16.))
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .gap(z(16.))
        .child(
            div()
                .text_size(z(20.))
                .font_weight(FontWeight::BOLD)
                .text_color(c(Palette::TEXT_HEADER))
                .child("Active Now"),
        );
    if active.is_empty() {
        return col.child(
            div()
                .p(z(16.))
                .flex()
                .flex_col()
                .items_center()
                .gap(z(4.))
                .child(
                    div()
                        .text_size(z(16.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(Palette::TEXT_HEADER))
                        .child("It's quiet for now..."),
                )
                .child(
                    div()
                        .text_size(z(14.))
                        .text_color(c(Palette::TEXT_MUTED))
                        .text_center()
                        .child("When a friend starts an activity—like playing a game or hanging out on voice—we'll show it here!"),
                ),
        );
    }
    for (user, activity) in active {
        let u = app.users.get(&user);
        col = col.child(
            div()
                .id(("active", user.0))
                .p(z(16.))
                .rounded(z(8.))
                .bg(c(Palette::BG_SECONDARY))
                .border_1()
                .border_color(c(Palette::DIVIDER))
                .flex()
                .items_center()
                .gap(z(12.))
                .cursor_pointer()
                .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
                .on_click(cx.listener(move |a, _, _, cx| a.open_dm_with(user, cx)))
                .child(avatar_with_status(u, 32., app.presence_of(user), c(Palette::BG_SECONDARY)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .text_size(z(16.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(c(Palette::TEXT_HEADER))
                                .child(u.map(|u| u.display_name.clone()).unwrap_or_default()),
                        )
                        .child(div().text_size(z(12.)).text_color(c(Palette::TEXT_MUTED)).text_ellipsis().child(activity)),
                ),
        );
    }
    col
}

fn message_requests(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let reqs: Vec<Id> = app.nav.message_requests.iter().copied().filter(|c| app.channels.contains_key(c)).collect();
    let mut body = div().id("msg-requests").flex_1().overflow_y_scroll().p(z(20.)).flex().flex_col();
    if reqs.is_empty() {
        body = body.child(
            div()
                .pt(z(160.))
                .flex()
                .justify_center()
                .text_size(z(16.))
                .text_color(c(Palette::TEXT_MUTED))
                .child("There are no pending message requests."),
        );
    }
    for id in reqs {
        let ch = &app.channels[&id];
        let u = ch.voice_users.first().and_then(|u| app.users.get(u));
        body = body.child(
            div()
                .id(("req", id.0))
                .h(z(62.))
                .flex()
                .items_center()
                .gap(z(12.))
                .border_t_1()
                .border_color(c(Palette::DIVIDER))
                .cursor_pointer()
                .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
                .on_click(cx.listener(move |a, _, _, cx| a.select_channel(id, cx)))
                .child(avatar_with_status(u, 32., Presence::Offline, c(Palette::BG_PRIMARY)))
                .child(div().text_size(z(16.)).text_color(c(Palette::TEXT_HEADER)).child(ch.name.clone())),
        );
    }
    div()
        .flex_1()
        .min_w_0()
        .h_full()
        .flex()
        .flex_col()
        .bg(c(Palette::BG_PRIMARY))
        .child(
            div()
                .h(px(m::HEADER_H))
                .flex_none()
                .px(z(16.))
                .flex()
                .items_center()
                .gap(z(8.))
                .border_b_1()
                .border_color(c(Palette::HEADER_SHADOW))
                .child(icon(Icon::Inbox, 24., c(Palette::CHANNEL_ICON)))
                .child(
                    div()
                        .text_size(z(16.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(Palette::TEXT_HEADER))
                        .child("Message Requests"),
                ),
        )
        .child(body)
}
