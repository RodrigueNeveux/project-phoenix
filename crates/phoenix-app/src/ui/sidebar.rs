//! Channel sidebar (268px, `--custom-guild-sidebar-width`): guild header,
//! channel tree / DM list, and the floating panels card (voice + user).

/// `--radius-sm`: channel and DM rows in the refreshed client.
const ROW_RADIUS: f32 = 8.;

use gpui::{
    AnyElement, App, Context, Entity, FontWeight, IntoElement, ParentElement, SharedString, Styled,
    Transformation, div, list, percentage, prelude::*, px,
};

use crate::{
    app::{PhoenixApp, SidebarRow},
    ui::home::data::HomeView,
    assets::Icon,
    model::{ChannelKind, Id},
    theme::{Palette, c, m, z},
    ui::{avatar_with_status, badge, icon},
};

pub fn channel_icon(kind: ChannelKind) -> Icon {
    match kind {
        ChannelKind::Text | ChannelKind::Category => Icon::Hash,
        ChannelKind::Voice => Icon::Speaker,
        ChannelKind::Announcement => Icon::Megaphone,
        ChannelKind::Forum => Icon::Forum,
        ChannelKind::Stage => Icon::Stage,
    }
}

/// Sidebar rows for the current selection: DMs, or the guild channel tree
/// with collapsed categories (persisted), hidden muted channels, threads
/// under their parent and live voice members.
pub fn build_rows(app: &mut PhoenixApp) {
    app.sidebar_rows.clear();
    match app.selected_guild {
        None => {
            let reqs = &app.nav.message_requests;
            let rows: Vec<SidebarRow> =
                app.dms.iter().filter(|d| !reqs.contains(d)).map(|id| SidebarRow::Channel(*id)).collect();
            app.sidebar_rows = rows;
        }
        Some(g) => {
            let hide_muted = app.nav.hide_muted.contains(&g);
            let ids = app.guild_channels.get(&g).cloned().unwrap_or_default();
            let mut current_cat: Option<Id> = None;
            let mut rows = Vec::with_capacity(ids.len());
            for id in ids {
                let Some(ch) = app.channels.get(&id) else { continue };
                if ch.kind == ChannelKind::Category {
                    current_cat = Some(id);
                    rows.push(SidebarRow::Category(id));
                    continue;
                }
                let selected = Some(id) == app.selected_channel
                    || app.selected_channel.and_then(|s| app.nav.thread_parent(s)) == Some(id);
                let collapsed = current_cat.is_some_and(|c| app.collapsed.contains(&c));
                // Collapsed categories still show the selected / unread channels.
                if collapsed && !selected && !(ch.unread && !ch.muted) && ch.voice_users.is_empty() {
                    continue;
                }
                if hide_muted && ch.muted && !selected {
                    continue;
                }
                rows.push(SidebarRow::Channel(id));
                for t in app.nav.threads.get(&id).into_iter().flatten() {
                    if !collapsed || Some(t.id) == app.selected_channel {
                        rows.push(SidebarRow::Thread(t.id));
                    }
                }
                rows.extend(ch.voice_users.iter().map(|u| SidebarRow::VoiceUser { user: *u }));
            }
            // Drop (expanded) categories left empty by hidden muted channels.
            if hide_muted {
                let keep: Vec<SidebarRow> = rows
                    .iter()
                    .enumerate()
                    .filter(|(i, r)| match r {
                        SidebarRow::Category(cat) => {
                            app.collapsed.contains(cat) || matches!(rows.get(i + 1), Some(SidebarRow::Channel(_)))
                        }
                        _ => true,
                    })
                    .map(|(_, r)| *r)
                    .collect();
                rows = keep;
            }
            app.sidebar_rows = rows;
        }
    }
    app.sidebar_list.reset(app.sidebar_rows.len() + 1);
}

pub fn render(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let entity = cx.entity();
    div()
        .w(z(m::SIDEBAR_W))
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .bg(c(Palette::BG_SECONDARY))
        .rounded_tl(px(8.))
        .overflow_hidden()
        .relative()
        .child(header(app, cx))
        .child(
            div().flex_1().min_h_0().child(
                list(app.sidebar_list.clone(), move |ix, _w, cx| row(&entity, ix, cx)).size_full(),
            ),
        )
        .when_some(app.selected_guild.filter(|_| app.nav.guild_menu), |d, g| {
            d.child(crate::ui::guild_menu::render(app, g, m::HEADER_H + 4., cx))
        })
}

fn header(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> AnyElement {
    match app.selected_guild.and_then(|g| app.guilds.iter().find(|x| x.id == g)) {
        None => div()
            .h(px(m::HEADER_H))
            .flex_none()
            .px(px(10.))
            .flex()
            .items_center()
            .border_b_1()
            .border_color(c(Palette::HEADER_SHADOW))
            .child(
                div()
                    .w_full()
                    .h(px(28.))
                    .rounded(px(4.))
                    .bg(c(Palette::BG_TERTIARY))
                    .px(px(6.))
                    .flex()
                    .items_center()
                    .text_size(px(14.))
                    .text_color(c(Palette::TEXT_MUTED))
                    .child("Find or start a conversation"),
            )
            .id("dm-search")
            .cursor_pointer()
            .on_click(cx.listener(|a, _, w, cx| a.open_switcher(w, cx)))
            .into_any_element(),
        Some(g) => {
            let hash = app.nav.banners.get(&g.id).cloned();
            let banner = g.banner_color.or(hash.as_ref().map(|_| Palette::BG_SECONDARY));
            let menu_open = app.nav.guild_menu;
            let gid = g.id.0;
            div()
                .id("guild-banner")
                .group("guild-banner")
                .flex_none()
                .relative()
                .when_some(banner, |d, col| {
                    let h = if hash.is_some() { 135. } else { 84. };
                    d.h(px(h))
                        .bg(c(col))
                        .overflow_hidden()
                        .when_some(hash.clone(), |d, hash| {
                            let url = |ext: &str| -> SharedString {
                                format!("https://cdn.discordapp.com/banners/{gid}/{hash}.{ext}?size=480").into()
                            };
                            let animated = hash.starts_with("a_");
                            d.child(gpui::img(url("png")).absolute().inset_0().size_full().object_fit(gpui::ObjectFit::Cover))
                                .when(animated, |d| {
                                    d.child(
                                        div()
                                            .absolute()
                                            .inset_0()
                                            .opacity(0.)
                                            .group_hover("guild-banner", |s| s.opacity(1.))
                                            .child(gpui::img(url("gif")).size_full().object_fit(gpui::ObjectFit::Cover)),
                                    )
                                })
                        })
                        .child(
                            div().absolute().top_0().left_0().right_0().h(px(56.)).bg(gpui::linear_gradient(
                                180.,
                                gpui::linear_color_stop(gpui::black().opacity(0.4), 0.),
                                gpui::linear_color_stop(gpui::transparent_black(), 1.),
                            )),
                        )
                        .child(
                            div().absolute().bottom_0().left_0().right_0().h(px(40.)).bg(gpui::linear_gradient(
                                180.,
                                gpui::linear_color_stop(gpui::transparent_black(), 0.),
                                gpui::linear_color_stop(c(Palette::BG_SECONDARY).opacity(0.8), 1.),
                            )),
                        )
                })
                .child(
                    div()
                        .id("guild-header")
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .h(px(m::HEADER_H))
                        .px(px(16.))
                        .flex()
                        .items_center()
                        .justify_between()
                        .cursor_pointer()
                        .on_click(cx.listener(|a, _, _, cx| {
                            a.nav.guild_menu = !a.nav.guild_menu;
                            cx.notify();
                        }))
                        .when(banner.is_none(), |d| {
                            d.border_b_1().border_color(c(Palette::HEADER_SHADOW)).hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
                        })
                        .child(
                            div()
                                .text_size(px(16.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(c(Palette::TEXT_HEADER))
                                .text_ellipsis()
                                .child(g.name.clone()),
                        )
                        .child(if menu_open {
                            icon(Icon::Close, 18., c(Palette::TEXT_HEADER))
                        } else {
                            icon(Icon::Chevron, 18., c(Palette::TEXT_HEADER))
                        }),
                )
                .when(banner.is_none(), |d| d.h(px(m::HEADER_H)))
                .into_any_element()
        }
    }
}

fn row(entity: &Entity<PhoenixApp>, ix: usize, cx: &mut App) -> AnyElement {
    let app = entity.read(cx);
    if ix == 0 {
        // Top spacer / DM header rows.
        return if app.selected_guild.is_none() {
            dm_top(entity, cx).into_any_element()
        } else {
            div().h(px(8.)).into_any_element()
        };
    }
    let Some(r) = app.sidebar_rows.get(ix - 1).copied() else {
        return div().into_any_element();
    };
    match r {
        SidebarRow::Category(id) => {
            let ch = &app.channels[&id];
            let collapsed = app.collapsed.contains(&id);
            let e = entity.clone();
            div()
                .id(("cat", id.0))
                .group("cat")
                .pt(px(16.))
                .pl(px(8.))
                .pr(px(8.))
                .h(px(40.))
                .flex()
                .items_center()
                .gap(px(2.))
                .cursor_pointer()
                .text_color(c(Palette::TEXT_MUTED))
                .hover(|s| s.text_color(c(Palette::INTERACTIVE_HOVER)))
                .on_click(move |_, _, cx| e.update(cx, |a, cx| a.toggle_category_saved(id, cx)))
                .child(
                    icon(Icon::Chevron, 12., c(Palette::TEXT_MUTED))
                        .group_hover("cat", |s| s.text_color(c(Palette::INTERACTIVE_HOVER)))
                        .with_transformation(Transformation::rotate(percentage(if collapsed {
                            -0.25
                        } else {
                            0.
                        }))),
                )
                .child(
                    div()
                        .flex_1()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_ellipsis()
                        .child(SharedString::from(ch.name.to_uppercase())),
                )
                .child(icon(Icon::Plus, 16., c(Palette::TEXT_MUTED)))
                .into_any_element()
        }
        SidebarRow::Channel(id) => channel_row(app, entity, id),
        SidebarRow::Thread(id) => thread_row(app, entity, id),
        SidebarRow::VoiceUser { user } => crate::ui::voice_panel::voice_user_row(app, entity, user),
    }
}

fn dm_top(entity: &Entity<PhoenixApp>, cx: &App) -> impl IntoElement {
    let app = entity.read(cx);
    let home = app.nav.home;
    let requests = app.nav.message_requests.len() as u32;
    let pending = app.nav.count(crate::ui::home::data::RelationshipKind::Incoming) as u32;
    let nav = |id: &'static str, i: Icon, label: &'static str, view: Option<HomeView>, count: u32| {
        let selected = view.is_some_and(|v| v == home);
        let e = entity.clone();
        div()
            .id(id)
            .mx(px(8.))
            .h(z(42.))
            .px(z(8.))
            .rounded(z(ROW_RADIUS))
            .flex()
            .items_center()
            .gap(px(12.))
            .cursor_pointer()
            .text_size(px(16.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(c(if selected { Palette::TEXT_HEADER } else { Palette::TEXT_MUTED }))
            .when(selected, |d| d.bg(c(Palette::BG_MOD_SELECTED)))
            .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)).text_color(c(Palette::INTERACTIVE_HOVER)))
            .on_click(move |_, _, cx| {
                e.update(cx, |a, cx| match view {
                    Some(v) => a.open_home_view(v, cx),
                    None => a.stub(label, cx),
                })
            })
            .child(icon(i, 24., c(Palette::INTERACTIVE)))
            .child(div().flex_1().child(label))
            .when(count > 0, |d| d.child(badge(count).bg(c(Palette::NOTIFICATION))))
    };
    let e = entity.clone();
    div()
        .pt(px(8.))
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(nav("friends", Icon::Friends, "Friends", Some(HomeView::Friends), pending))
        .child(nav("msg-requests", Icon::Inbox, "Message Requests", Some(HomeView::MessageRequests), requests))
        .child(nav("nitro", Icon::Nitro, "Nitro", None, 0))
        .child(nav("shop", Icon::Gift, "Shop", None, 0))
        .child(
            div()
                .pt(px(18.))
                .pb(px(4.))
                .px(px(18.))
                .flex()
                .justify_between()
                .items_center()
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c(Palette::TEXT_MUTED))
                .child("DIRECT MESSAGES")
                .child(
                    div()
                        .id("new-dm")
                        .cursor_pointer()
                        .on_click(move |_, _, cx| e.update(cx, |a, cx| a.open_group_dm_picker(cx)))
                        .child(icon(Icon::Plus, 16., c(Palette::TEXT_MUTED))),
                ),
        )
}

/// Channel glyph with Discord's small corner badges (lock for private
/// channels, "!" for age-restricted ones).
fn decorated_icon(kind: ChannelKind, color: gpui::Hsla, locked: bool, nsfw: bool, bg: gpui::Hsla) -> AnyElement {
    div()
        .relative()
        .size(px(20.))
        .flex_none()
        .child(icon(channel_icon(kind), 20., color))
        .when(locked || nsfw, |d| {
            d.child(
                div()
                    .absolute()
                    .right(px(-3.))
                    .top(px(-3.))
                    .size(px(11.))
                    .rounded_full()
                    .bg(bg)
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(locked, |d| d.child(icon(Icon::Lock, 9., color)))
                    .when(!locked, |d| {
                        d.text_size(px(9.)).font_weight(FontWeight::BOLD).text_color(color).child("!")
                    }),
            )
        })
        .into_any_element()
}

fn channel_row(app: &PhoenixApp, entity: &Entity<PhoenixApp>, id: Id) -> AnyElement {
    let Some(ch) = app.channels.get(&id) else { return div().into_any_element() };
    let selected = app.selected_channel == Some(id);
    let is_dm = ch.guild.is_none();
    let extra = app.nav.extras.get(&id);
    let is_group = extra.is_some_and(|e| e.recipients > 1 || e.group_icon.is_some());
    let unread = ch.unread && !ch.muted;
    // `--channels-default` at rest, `--interactive-text-active` when selected
    // or unread, `--interactive-muted` when muted.
    let fg = if selected {
        Palette::TEXT_HEADER
    } else if ch.muted {
        Palette::INTERACTIVE_MUTED
    } else if ch.unread {
        Palette::TEXT_HEADER
    } else if is_dm {
        Palette::TEXT_MUTED
    } else {
        Palette::CHANNEL_ICON
    };
    let e = entity.clone();
    let lead: AnyElement = if is_dm && is_group {
        let ex = extra.cloned().unwrap_or_default();
        match ex.group_icon {
            Some(crate::model::ImageRef::Url(u)) => {
                gpui::img(u).size(px(32.)).rounded_full().flex_none().into_any_element()
            }
            _ => div()
                .size(px(32.))
                .flex_none()
                .rounded_full()
                .bg(c(crate::theme::DEFAULT_AVATAR[(id.0 % 5) as usize]))
                .flex()
                .items_center()
                .justify_center()
                .child(icon(Icon::Members, 18., gpui::white()))
                .into_any_element(),
        }
    } else if is_dm {
        let u = ch.voice_users.first().and_then(|u| app.users.get(u));
        let p = ch.voice_users.first().map(|u| app.presence_of(*u)).unwrap_or_default();
        avatar_with_status(u, 32., p, c(Palette::BG_SECONDARY)).into_any_element()
    } else {
        let col = c(if ch.muted { Palette::INTERACTIVE_MUTED } else { Palette::CHANNEL_ICON });
        decorated_icon(
            ch.kind,
            col,
            extra.is_some_and(|e| e.locked),
            extra.is_some_and(|e| e.nsfw),
            c(if selected { Palette::BG_MOD_SELECTED } else { Palette::BG_SECONDARY }),
        )
    };
    let name: SharedString = extra.and_then(|e| e.group_name.clone()).filter(|_| is_dm).unwrap_or_else(|| ch.name.clone());
    let members: Option<SharedString> =
        (is_dm && is_group).then(|| format!("{} Members", extra.map(|e| e.recipients + 1).unwrap_or(1)).into());
    let group = SharedString::from(format!("chrow{}", id.0));
    let close = is_dm.then(|| {
        let e = entity.clone();
        div()
            .id(("dm-close", id.0))
            .opacity(0.)
            .group_hover(group.clone(), |s| s.opacity(1.))
            .on_click(move |_, _, cx| {
                cx.stop_propagation();
                e.update(cx, |a, cx| a.close_dm(id, cx));
            })
            .child(icon(Icon::Close, 16., c(Palette::INTERACTIVE)))
    });
    let invite_hover = (!is_dm && !matches!(ch.kind, ChannelKind::Voice | ChannelKind::Stage)).then(|| {
        div()
            .opacity(0.)
            .group_hover(group.clone(), |s| s.opacity(1.))
            .when(selected, |d| d.opacity(1.))
            .child(icon(Icon::Members, 16., c(Palette::INTERACTIVE)))
    });
    div()
        .id(("ch", id.0))
        .relative()
        .pl(px(8.))
        .pr(px(8.))
        .py(px(1.))
        .child(
            // unread pill on the far left
            div()
                .absolute()
                .left_0()
                .top(px(if is_dm { 17. } else { 13. }))
                .w(px(4.))
                .h(px(8.))
                .rounded_r(px(4.))
                .when(unread && !selected, |d| d.bg(c(Palette::TEXT_HEADER))),
        )
        .child(
            div()
                .id(("chrow", id.0))
                .group(group)
                .h(z(if is_dm { 42. } else { 32. }))
                .px(z(8.))
                .rounded(z(ROW_RADIUS))
                .flex()
                .items_center()
                .gap(px(if is_dm { 12. } else { 6. }))
                .cursor_pointer()
                .text_color(c(fg))
                .when(selected, |d| d.bg(c(Palette::BG_MOD_SELECTED)))
                .when(!selected, |d| {
                    d.hover(|s| s.bg(c(Palette::BG_MOD_HOVER)).text_color(c(Palette::INTERACTIVE_HOVER)))
                })
                .on_click(move |_, _, cx| e.update(cx, |a, cx| a.select_channel(id, cx)))
                .on_mouse_down(gpui::MouseButton::Right, move |ev, w, _| {
                    crate::ui::nav_menus::open(crate::ui::nav_menus::Target::Channel(id), ev.position, w)
                })
                .child(lead)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(16.))
                                .line_height(px(20.))
                                .font_weight(if unread { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                                .text_ellipsis()
                                .child(name),
                        )
                        .when_some(members, |d, m| {
                            d.child(
                                div()
                                    .text_size(px(12.))
                                    .line_height(px(14.))
                                    .text_color(c(Palette::TEXT_MUTED))
                                    .child(m),
                            )
                        }),
                )
                .when(ch.mentions > 0, |d| d.child(badge(ch.mentions).bg(c(Palette::NOTIFICATION))))
                .when(ch.mentions == 0, |d| d.children(invite_hover))
                .children(close),
        )
        .into_any_element()
}

/// Thread sub-row: curved connector from the parent channel, then the name.
fn thread_row(app: &PhoenixApp, entity: &Entity<PhoenixApp>, id: Id) -> AnyElement {
    let Some(t) = app.nav.find_thread(id) else { return div().into_any_element() };
    let selected = app.selected_channel == Some(id);
    let live = app.channels.get(&id);
    let unread = live.is_some_and(|c| c.unread);
    let mentions = live.map(|c| c.mentions).unwrap_or(0);
    let e = entity.clone();
    div()
        .id(("thread", id.0))
        .relative()
        .pl(px(8.))
        .pr(px(8.))
        .child(
            div()
                .absolute()
                .left(px(26.))
                .top(px(-12.))
                .w(px(12.))
                .h(px(28.))
                .border_l_1()
                .border_b_1()
                .rounded_bl(px(8.))
                .border_color(c(Palette::INTERACTIVE_MUTED)),
        )
        .child(
            div()
                .id(("threadrow", id.0))
                .ml(px(36.))
                .h(z(32.))
                .px(z(8.))
                .rounded(z(ROW_RADIUS))
                .flex()
                .items_center()
                .cursor_pointer()
                .text_size(px(14.))
                .font_weight(if unread { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                .text_color(c(if selected || unread { Palette::TEXT_HEADER } else { Palette::TEXT_MUTED }))
                .when(selected, |d| d.bg(c(Palette::BG_MOD_SELECTED)))
                .when(!selected, |d| {
                    d.hover(|s| s.bg(c(Palette::BG_MOD_HOVER)).text_color(c(Palette::INTERACTIVE_HOVER)))
                })
                .on_click(move |_, _, cx| e.update(cx, |a, cx| a.select_thread(id, cx)))
                .child(div().flex_1().min_w_0().text_ellipsis().child(t.name.clone()))
                .when(mentions > 0, |d| d.child(badge(mentions).bg(c(Palette::NOTIFICATION)))),
        )
        .into_any_element()
}

/// The bottom-left area under both the guild rail and the channel list (as in
/// the refreshed client): the rail and sidebar backgrounds continue behind a
/// floating panels card (`--background-base-low`, 1px border, 8px radius, 8px
/// from the edges) holding the voice panel (when connected) above the user row.
pub fn panels(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    div()
        .flex_none()
        .relative()
        .child(div().absolute().top_0().bottom_0().left_0().w(z(m::SERVER_BAR_W)).bg(c(Palette::BG_TERTIARY)))
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .right_0()
                .left(z(m::SERVER_BAR_W))
                .bg(c(Palette::BG_SECONDARY)),
        )
        .child(panels_card(app, cx))
}

fn panels_card(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    div()
        .flex_none()
        .m(z(8.))
        .rounded(z(8.))
        .border_1()
        .border_color(c(Palette::PANEL_BORDER))
        .bg(c(Palette::BG_SECONDARY_ALT))
        .overflow_hidden()
        .flex()
        .flex_col()
        .when(app.voice_channel.is_some(), |d| d.child(crate::ui::voice_panel::voice_panel(app, cx)))
        .child(user_panel(app, cx))
}

/// 32px panel button: 8px radius, mod-subtle on hover; red glyph on a
/// critical tint when `danger` (muted / deafened).
fn panel_button(id: &'static str, i: Icon, danger: bool, tip: &'static str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .size(z(32.))
        .flex_none()
        .rounded(z(8.))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .when(danger, |d| d.bg(c(Palette::RED).opacity(0.15)).hover(|s| s.bg(c(Palette::RED).opacity(0.25))))
        .when(!danger, |d| d.hover(|s| s.bg(c(Palette::BG_MOD_HOVER))))
        .child(icon(i, 20. * crate::theme::zoom(), c(if danger { Palette::RED } else { Palette::INTERACTIVE })))
        .tooltip(crate::ui::tooltip::tip(tip))
}

fn user_panel(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let status = match app.presence {
        crate::model::Presence::Online => "Online",
        crate::model::Presence::Idle => "Idle",
        crate::model::Presence::Dnd => "Do Not Disturb",
        crate::model::Presence::Offline => "Invisible",
    };
    let muted = app.mic_muted || app.deafened;
    div()
        .h(z(m::USER_PANEL_H))
        .flex_none()
        .px(z(8.))
        .flex()
        .items_center()
        .gap(z(4.))
        .child(
            div()
                .id("me")
                .flex_1()
                .min_w_0()
                .h(z(40.))
                .flex()
                .items_center()
                .gap(z(8.))
                .pl(z(4.))
                .pr(z(8.))
                .rounded(z(8.))
                .cursor_pointer()
                .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
                .on_click(cx.listener(|a, _, w, cx| a.toggle_status_picker(w, cx)))
                .child(avatar_with_status(Some(&app.me), 32., app.presence, c(Palette::BG_SECONDARY_ALT)))
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(z(14.))
                                .line_height(z(18.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(c(Palette::TEXT_HEADER))
                                .text_ellipsis()
                                .child(app.me.display_name.clone()),
                        )
                        .child(
                            div()
                                .text_size(z(12.))
                                .line_height(z(14.))
                                .text_color(c(Palette::TEXT_MUTED))
                                .text_ellipsis()
                                .child(app.people.custom_status.clone().unwrap_or(status.into())),
                        ),
                ),
        )
        .child(
            panel_button("mute", if muted { Icon::MicOff } else { Icon::Mic }, muted, if muted { "Unmute" } else { "Mute" })
                .on_click(cx.listener(|a, _, _, cx| {
                    a.mic_muted = !a.mic_muted;
                    crate::voice::sync_self_state(a);
                    cx.notify();
                })),
        )
        .child(
            panel_button(
                "deafen",
                if app.deafened { Icon::HeadphonesOff } else { Icon::Headphones },
                app.deafened,
                if app.deafened { "Undeafen" } else { "Deafen" },
            )
            .on_click(cx.listener(|a, _, _, cx| {
                a.deafened = !a.deafened;
                crate::voice::sync_self_state(a);
                cx.notify();
            })),
        )
        .child(
            panel_button("settings", Icon::Gear, false, "User Settings")
                .on_click(cx.listener(|a, _, _, cx| crate::ui::settings::open(a, cx))),
        )
}
