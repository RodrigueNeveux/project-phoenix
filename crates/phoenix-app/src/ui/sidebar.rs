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
    theme::{Palette, c, generated::idx, m, tok, z},
    ui::{avatar_with_status, badge, icon},
};

/// Colour of a generated CSS token.
fn t(i: usize) -> gpui::Hsla {
    c(tok(i))
}

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
            let mut rows = rows;
            crate::actions::pins::apply(&mut rows);
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
            crate::actions::pins::apply(&mut rows);
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
        .border_l_1()
        .border_t_1()
        .border_color(c(Palette::APP_FRAME_BORDER))
        .rounded_tl(z(m::FRAME_RADIUS))
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
            .px(z(10.))
            .flex()
            .items_center()
            .border_b_1()
            .border_color(t(idx::BORDER_SUBTLE))
            .child(
                // dump -channels-me+ #588: `button_F7qvhY sm secondary fullWidth`: ~31px,
                // radius 8, `--control-secondary-background-default`, 1px `--border-muted`,
                // centered `body-sm/medium` (14/18) `--text-strong`.
                div()
                    .w_full()
                    .h(z(31.))
                    .rounded(z(8.))
                    .bg(t(idx::CONTROL_SECONDARY_BACKGROUND_DEFAULT))
                    .border_1()
                    .border_color(t(idx::BORDER_MUTED))
                    .px(z(11.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(z(14.))
                    .line_height(z(18.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(t(idx::TEXT_STRONG))
                    .child(div().min_w_0().text_ellipsis().child(crate::t!("dm.find"))),
            )
            .id("dm-search")
            .cursor_pointer()
            .on_click(cx.listener(|a, _, w, cx| a.open_switcher(w, cx)))
            .into_any_element(),
        Some(g) => {
            let hash = app.nav.banners.get(&g.id).cloned();
            // The refreshed client only shows a banner when the guild has one.
            let banner = hash.as_ref().map(|_| g.banner_color.unwrap_or(Palette::BG_SECONDARY));
            let menu_open = app.nav.guild_menu;
            let gid = g.id.0;
            div()
                .id("guild-banner")
                .group("guild-banner")
                .flex_none()
                .relative()
                .when_some(banner, |d, col| {
                    let h = (m::SIDEBAR_W * 9. / 21.).round();
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
                            div().absolute().top_0().left_0().right_0().h(z(56.)).bg(gpui::linear_gradient(
                                180.,
                                gpui::linear_color_stop(gpui::black().opacity(0.4), 0.),
                                gpui::linear_color_stop(gpui::transparent_black(), 1.),
                            )),
                        )
                        .child(
                            div().absolute().bottom_0().left_0().right_0().h(z(40.)).bg(gpui::linear_gradient(
                                180.,
                                gpui::linear_color_stop(gpui::transparent_black(), 0.),
                                gpui::linear_color_stop(c(Palette::BG_SECONDARY).opacity(0.8), 1.),
                            )),
                        )
                })
                .child({
                    // `header_XH4OpX` (dump -channels-N-N+ #588): `--custom-channel-header-height`,
                    // padding `--space-xs` (10px cozy), 1px `--border-subtle` bottom.
                    // `guildDropdown_XH4OpX` (#590): 32px, padding 0 10, radius 8, gap 2;
                    // name `body-md/semibold` (16/20) + 16px chevron. `inviteButton_XH4OpX`
                    // (#609): 32px, radius 8, 20px glyph.
                    let fg = if banner.is_some() { gpui::white() } else { t(idx::TEXT_STRONG) };
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .h(px(m::HEADER_H))
                        .p(z(10.))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(z(4.))
                        .when(banner.is_none(), |d| d.border_b_1().border_color(t(idx::BORDER_SUBTLE)))
                        .child(
                            div()
                                .id("guild-header")
                                .h(z(32.))
                                .min_w_0()
                                .px(z(10.))
                                .flex()
                                .items_center()
                                .gap(z(2.))
                                .rounded(z(8.))
                                .cursor_pointer()
                                .hover(|s| s.bg(t(idx::BACKGROUND_MOD_NORMAL)))
                                .when(menu_open, |d| d.bg(t(idx::BACKGROUND_MOD_NORMAL)))
                                .on_click(cx.listener(|a, _, _, cx| {
                                    a.nav.guild_menu = !a.nav.guild_menu;
                                    cx.notify();
                                }))
                                .child(
                                    div()
                                        .min_w_0()
                                        .text_size(z(16.))
                                        .line_height(z(20.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(fg)
                                        .text_ellipsis()
                                        .child(g.name.clone()),
                                )
                                .child(icon(if menu_open { Icon::Close } else { Icon::Chevron }, 16. * crate::theme::zoom(), fg)),
                        )
                        .child(
                            div()
                                .id("guild-invite")
                                .size(z(32.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(z(8.))
                                .cursor_pointer()
                                .hover(|s| s.bg(t(idx::BACKGROUND_MOD_NORMAL)))
                                .child(
                                    gpui::svg()
                                        .path(crate::icons::path("invite"))
                                        .size(z(20.))
                                        .flex_none()
                                        .text_color(fg),
                                ),
                        )
                })
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
            guild_top(app).into_any_element()
        };
    }
    let Some(r) = app.sidebar_rows.get(ix - 1).copied() else {
        return div().into_any_element();
    };
    match r {
        SidebarRow::Category(id) => {
            // `containerDefault_yOdhm_` padding-top `--custom-category-channel-space-before-category`;
            // `wrapperCommon_yOdhm_`: 24px, padding-inline `--space-md` / `--space-md - --space-xs`,
            // `--channels-default` (hover `--interactive-text-hover`); name then a 12px
            // chevron (`mainContent` gap 4px, rotated -90deg when collapsed); 16px add
            // button shown on hover only (`addButton_yOdhm_`).
            let ch = &app.channels[&id];
            let collapsed = app.collapsed.contains(&id);
            let e = entity.clone();
            div()
                .id(("cat", id.0))
                .group("cat")
                // dump -channels-N-N+ #655/#656: li padding-top 16, row 24px,
                // `.enlarge-target-size .wrapper_yOdhm_` padding-inline 20 / 6 (cozy).
                .pt(z(16.))
                .pl(z(20.))
                .pr(z(6.))
                .h(z(40.))
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .cursor_pointer()
                .text_color(t(idx::CHANNELS_DEFAULT))
                .hover(|s| s.text_color(t(idx::INTERACTIVE_TEXT_HOVER)))
                .on_click(move |_, _, cx| e.update(cx, |a, cx| a.toggle_category_saved(id, cx)))
                .on_mouse_down(gpui::MouseButton::Right, move |ev, w, _| {
                    crate::ui::nav_menus::open(crate::ui::nav_menus::Target::Channel(id), ev.position, w)
                })
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(z(4.))
                        .child(
                            div()
                                .min_w_0()
                                .text_size(z(14.))
                                .line_height(z(18.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_ellipsis()
                                .child(ch.name.clone()),
                        )
                        .child(
                            icon(Icon::Chevron, 12., t(idx::CHANNELS_DEFAULT))
                                .group_hover("cat", |s| s.text_color(t(idx::INTERACTIVE_TEXT_HOVER)))
                                .with_transformation(Transformation::rotate(percentage(if collapsed {
                                    -0.25
                                } else {
                                    0.
                                }))),
                        ),
                )
                .child(
                    // `addButton_yOdhm_ forceVisible_yOdhm_` (dump #662): 24px box, padding 4,
                    // 16px glyph, always shown.
                    div()
                        .size(z(24.))
                        .flex_none()
                        .p(z(4.))
                        .child(icon(Icon::Plus, 16. * crate::theme::zoom(), t(idx::CHANNELS_DEFAULT))),
                )
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
    let nav = |id: &'static str, i: Icon, label: SharedString, view: Option<HomeView>, count: u32| {
        let selected = view.is_some_and(|v| v == home);
        let e = entity.clone();
        let stub = label.clone();
        // dump -channels-me+ #597..#609: `channel_zln_tS` li 40px (padding 1/0, margin-left 8),
        // `interactive_zln_tS` 38px radius 8, `link_zln_tS` padding 8 16 8 10; 20px glyph
        // + 12px margin; `body-md/medium` label. Idle `--text-muted`, selected
        // `--background-mod-strong` + `--text-default`.
        div().w_full().pl(z(8.)).pr(z(8.)).py(z(1.)).child(
            div()
                .id(id)
                .h(z(38.))
                .pl(z(10.))
                .pr(z(16.))
                .rounded(z(8.))
                .flex()
                .items_center()
                .gap(z(12.))
                .cursor_pointer()
                .text_size(z(16.))
                .line_height(z(20.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(t(if selected { idx::TEXT_DEFAULT } else { idx::TEXT_MUTED }))
                .when(selected, |d| d.bg(t(idx::BACKGROUND_MOD_STRONG)))
                .when(!selected, |d| {
                    d.hover(|s| s.bg(t(idx::INTERACTIVE_BACKGROUND_HOVER)).text_color(t(idx::INTERACTIVE_TEXT_HOVER)))
                })
                .on_click(move |_, _, cx| {
                    e.update(cx, |a, cx| match view {
                        Some(v) => a.open_home_view(v, cx),
                        None => a.stub(&stub, cx),
                    })
                })
                .child(icon(i, 20. * crate::theme::zoom(), t(if selected { idx::TEXT_STRONG } else { idx::TEXT_MUTED })))
                .child(div().flex_1().min_w_0().text_ellipsis().child(label))
                .when(count > 0, |d| d.child(badge(count).bg(c(Palette::NOTIFICATION)))),
        )
    };
    let e = entity.clone();
    div()
        .pt(z(8.)) // dump -channels-me+ #595: 8px spacer
        .flex()
        .flex_col()
        .child(nav("friends", Icon::Friends, crate::t!("friends.title"), Some(HomeView::Friends), pending))
        // vanilla (dump vanilla-friends-online): Friends, Nitro, Shop, Quests.
        // "Message Requests" only shows while requests are pending.
        .when(requests > 0, |d| {
            d.child(nav("msg-requests", Icon::Inbox, crate::t!("friends.message_requests"), Some(HomeView::MessageRequests), requests))
        })
        .child(nav("nitro", Icon::Nitro, crate::t!("dm.nitro"), None, 0))
        .child(nav("shop", Icon::Gift, crate::t!("dm.shop"), None, 0))
        // Quests (`/quest-home`): QuestsIcon (vanilla-friends-online #653).
        .child(nav("quests", Icon::Quests, crate::t!("dm.quests"), None, 0))
        // `sectionDivider_u88IQl` (#648): 1px, margin 12/8, `--border-subtle`.
        .child(div().mx(z(8.)).my(z(12.)).h(z(1.)).bg(t(idx::BORDER_SUBTLE)))
        .child(
            // `privateChannelsHeaderContainer_Bce46W` (#649): 24px, padding 0 6 4 20,
            // `body-sm/medium` 14/18 `--channels-default`; 24px action (padding 4, 16px glyph).
            div()
                .h(z(24.))
                .pl(z(20.))
                .pr(z(6.))
                .pb(z(4.))
                .flex()
                .justify_between()
                .items_center()
                .text_size(z(14.))
                .line_height(z(18.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(t(idx::CHANNELS_DEFAULT))
                .child(div().min_w_0().text_ellipsis().child(crate::t!("dm.header")))
                .child(
                    div()
                        .id("new-dm")
                        .size(z(24.))
                        .p(z(4.))
                        .cursor_pointer()
                        .on_click(move |_, _, cx| e.update(cx, |a, cx| a.open_group_dm_picker(cx)))
                        .child(icon(Icon::Plus, 16. * crate::theme::zoom(), t(idx::TEXT_MUTED))),
                ),
        )
}

/// Channel glyph with Discord's small corner badges (lock for private
/// channels, "!" for age-restricted ones).
fn decorated_icon(kind: ChannelKind, color: gpui::Hsla, locked: bool, nsfw: bool, bg: gpui::Hsla) -> AnyElement {
    div()
        .relative()
        .size(z(20.))
        .flex_none()
        .child(icon(channel_icon(kind), 20., color))
        .when(locked || nsfw, |d| {
            d.child(
                div()
                    .absolute()
                    .right(z(-3.))
                    .top(z(-3.))
                    .size(z(11.))
                    .rounded_full()
                    .bg(bg)
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(locked, |d| d.child(icon(Icon::Lock, 9., color)))
                    .when(!locked, |d| {
                        d.text_size(z(9.)).font_weight(FontWeight::BOLD).text_color(color).child("!")
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
    // `name_TNyK-N`: `--channels-default`; `modeSelected`/`modeUnreadImportant`:
    // `--interactive-text-active`; `modeMuted`: `--text-muted` at 50 % opacity.
    let fg = if selected || (ch.unread && !ch.muted) {
        tok(idx::INTERACTIVE_TEXT_ACTIVE)
    } else if ch.muted {
        tok(idx::TEXT_MUTED)
    } else {
        tok(idx::CHANNELS_DEFAULT)
    };
    let e = entity.clone();
    let lead: AnyElement = if is_dm && is_group {
        let ex = extra.cloned().unwrap_or_default();
        match ex.group_icon {
            Some(crate::model::ImageRef::Url(u)) => {
                gpui::img(u).size(z(32.)).rounded_full().flex_none().into_any_element()
            }
            _ => div()
                .size(z(32.))
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
        // `icon_TNyK-N` `--channel-icon`; muted `--icon-muted`; unread `--icon-subtle`.
        let col = t(if ch.muted {
            idx::ICON_MUTED
        } else if unread && !selected {
            idx::ICON_SUBTLE
        } else {
            idx::CHANNEL_ICON
        });
        decorated_icon(
            ch.kind,
            col,
            extra.is_some_and(|e| e.locked),
            extra.is_some_and(|e| e.nsfw),
            if selected { t(idx::INTERACTIVE_BACKGROUND_SELECTED) } else { c(Palette::BG_SECONDARY) },
        )
    };
    let name: SharedString = extra.and_then(|e| e.group_name.clone()).filter(|_| is_dm).unwrap_or_else(|| ch.name.clone());
    let members: Option<SharedString> =
        (is_dm && is_group).then(|| crate::t!("invite.members", n = extra.map(|e| e.recipients + 1).unwrap_or(1)));
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
            .child(icon(Icon::Close, 16., t(idx::INTERACTIVE_TEXT_DEFAULT)))
    });
    // `children_TNyK-N` (vanilla-guild-text #691): margin -4 -4 -4 0, then 24px
    // `iconItem_Wx6SnQ` boxes (padding 4, 16px `actionIcon`): "Invite" and, with
    // Manage Channels, "Edit Channel". Shown on hover and on the selected row.
    let can_edit = ch.guild.is_some_and(|g| {
        crate::ui::inbox::has_perm(app.backend.guild_permissions(g), 1 << 4) // MANAGE_CHANNELS
    });
    let action = |i: Icon, path: Option<&'static str>| {
        let col = t(idx::INTERACTIVE_TEXT_DEFAULT);
        div().size(z(24.)).p(z(4.)).flex_none().child(match path {
            Some(p) => gpui::svg().path(crate::icons::path(p)).size(z(16.)).text_color(col).into_any_element(),
            None => icon(i, 16., col).into_any_element(),
        })
    };
    let invite_hover = (!is_dm && !matches!(ch.kind, ChannelKind::Voice | ChannelKind::Stage)).then(|| {
        div()
            .flex()
            .flex_none()
            .my(z(-4.))
            .mr(z(-4.))
            .opacity(0.)
            .group_hover(group.clone(), |s| s.opacity(1.))
            .when(selected, |d| d.opacity(1.))
            .child(action(Icon::Members, Some("invite")))
            .when(can_edit, |d| d.child(action(Icon::Gear, None)))
    });
    div()
        .id(("ch", id.0))
        .relative()
        // `wrapper_TNyK-N` margin-inline-start `--space-xs` (10px cozy, dump -channels-N-N+
        // #628); DM `channel_zln_tS` margin 8 (dump -channels-me+ #656). Right: scrollbar gutter.
        .pl(z(if is_dm { 8. } else { 10. }))
        .pr(z(8.))
        .py(z(1.))
        .child(
            // `unread_TNyK-N`: 4x8, `0 --radius-xs --radius-xs 0`, inset
            // `-(--space-xs) + 1px` from the link, vertically centred.
            div()
                .absolute()
                .left(z(1.))
                .top(z(if is_dm { 21. } else { 15. }))
                .w(z(4.))
                .h(z(8.))
                .rounded_r(z(4.))
                .when(unread && !selected, |d| d.bg(t(idx::INTERACTIVE_TEXT_ACTIVE))),
        )
        .child(
            div()
                .id(("chrow", id.0))
                .group(group)
                // `link_TNyK-N` 36px, padding 4/10 (dump #629); DM `link_zln_tS` 48px, pl 10 (me #658).
                .h(z(if is_dm { 48. } else { 36. }))
                .pl(z(10.))
                .pr(z(if is_dm { 4. } else { 10. }))
                .rounded(z(ROW_RADIUS))
                .flex()
                .items_center()
                .gap(z(if is_dm { 12. } else { 10. })) // linkTop_TNyK-N gap --space-xs (10 cozy); DM avatar margin 12
                .cursor_pointer()
                .text_color(c(fg))
                .when(ch.muted && !selected, |d| d.opacity(0.5))
                .when(selected, |d| d.bg(t(idx::INTERACTIVE_BACKGROUND_SELECTED)))
                .when(!selected, |d| {
                    d.hover(|s| {
                        s.bg(t(idx::INTERACTIVE_BACKGROUND_HOVER)).text_color(t(idx::INTERACTIVE_TEXT_HOVER)).opacity(1.)
                    })
                })
                .on_click(move |_, _, cx| e.update(cx, |a, cx| a.select_channel(id, cx)))
                .on_mouse_down(gpui::MouseButton::Right, move |ev, w, _| {
                    crate::ui::nav_menus::open(crate::ui::nav_menus::Target::Channel(id), ev.position, w)
                })
                // DM `interactive_zln_tS` > `container_AFi54G` (vanilla-home-friends #661):
                // nameplate of the 1:1 recipient behind the row, `.channel` opacity.
                .relative()
                .when_some(
                    (is_dm && !is_group)
                        .then(|| ch.voice_users.first().and_then(|u| app.users.get(u)))
                        .flatten()
                        .and_then(|u| crate::ui::nameplate::layer(u, 48., crate::ui::nameplate::Place::Channel, false, selected)),
                    |d, l| d.child(l),
                )
                .child(lead)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(z(16.))
                                .line_height(z(if is_dm { 20. } else { 28. })) // --channels-name-line-height (.density-cozy 28px)
                                .font_weight(FontWeight::MEDIUM) // --font-weight-semibold-1x-light-theme (500)
                                .text_ellipsis()
                                .child(name),
                        )
                        .when_some(members, |d, m| {
                            d.child(
                                div()
                                    .text_size(z(12.))
                                    .line_height(z(14.))
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
    let Some(th) = app.nav.find_thread(id) else { return div().into_any_element() };
    let selected = app.selected_channel == Some(id);
    let live = app.channels.get(&id);
    let unread = live.is_some_and(|c| c.unread);
    let mentions = live.map(|c| c.mentions).unwrap_or(0);
    let e = entity.clone();
    div()
        .id(("thread", id.0))
        .relative()
        .pl(z(10.)) // --space-xs (.density-cozy)
        .pr(z(8.))
        .child(
            div()
                // `spine_reDhww`: `--channels-spine-offset-left`, `--spine-default`.
                .absolute()
                .left(z(28.)) // --channels-spine-offset-left (.density-cozy 28px)
                .top(z(-14.))
                .w(z(12.))
                .h(z(28.))
                .border_l_1()
                .border_b_1()
                .rounded_bl(z(8.))
                .border_color(t(idx::SPINE_DEFAULT)),
        )
        .child(
            div()
                .id(("threadrow", id.0))
                // `typeThread_TNyK-N.wrapper`: margin `28px + --space-xs` from the list
                // (8px of it is the row inset); link padding 4px / `--space-xs`.
                .ml(z(28.))
                .h(z(36.))
                .px(z(10.))
                .rounded(z(ROW_RADIUS))
                .flex()
                .items_center()
                .cursor_pointer()
                .text_size(z(16.))
                .line_height(z(28.)) // --channels-name-line-height (cozy)
                .font_weight(FontWeight::MEDIUM)
                .text_color(t(if selected || unread { idx::INTERACTIVE_TEXT_ACTIVE } else { idx::CHANNELS_DEFAULT }))
                .when(selected, |d| d.bg(t(idx::INTERACTIVE_BACKGROUND_SELECTED)))
                .when(!selected, |d| {
                    d.hover(|s| s.bg(t(idx::INTERACTIVE_BACKGROUND_HOVER)).text_color(t(idx::INTERACTIVE_TEXT_HOVER)))
                })
                .on_click(move |_, _, cx| e.update(cx, |a, cx| a.select_thread(id, cx)))
                .child(div().flex_1().min_w_0().text_ellipsis().child(th.name.clone()))
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
                .border_l_1()
                .border_color(c(Palette::APP_FRAME_BORDER))
                .bg(c(Palette::BG_SECONDARY)),
        )
        .child(panels_card(app, cx))
}

fn panels_card(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    div()
        .flex_none()
        .m(z(10.)) // dump -channels-N-N+ #976 `panels_pog62O` at x=10 / bottom 10 (cozy --space-xs)
        .rounded(z(8.))
        .border_1()
        .border_color(t(idx::BORDER_MUTED)) // panels_pog62O
        .bg(t(idx::BACKGROUND_BASE_LOW))
        .overflow_hidden()
        .flex()
        .flex_col()
        .when(app.voice_channel.is_some(), |d| d.child(crate::ui::voice_panel::voice_panel(app, cx)))
        .child(user_panel(app, cx))
}

fn user_panel(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    use crate::ui::voice_panel::{Audio, audio_button, panel_button, panel_glyph};
    let status = match app.presence {
        crate::model::Presence::Online => crate::t!("status.online"),
        crate::model::Presence::Idle => crate::t!("status.idle"),
        crate::model::Presence::Dnd => crate::t!("status.dnd"),
        crate::model::Presence::Offline => crate::t!("status.invisible"),
    };
    let muted = app.mic_muted || app.deafened;
    // `container_g6VwXp` (dump -channels-N-N+ #979): 56px, padding 14px, gap
    // `--space-xs` (10px cozy).
    div()
        .h(z(m::USER_PANEL_H))
        .flex_none()
        .px(z(14.))
        .flex()
        .items_center()
        .gap(z(10.))
        .child(
            // `accountPopoutButtonWrapper_g6VwXp`: -4px inline-start margin,
            // 5px 4px padding, `--radius-xs`, 8px gap, hover `--background-mod-normal`.
            div()
                .id("me")
                .flex_1()
                .min_w(z(112.))
                .ml(z(-6.)) // dump #980: margin-inline-start -6, padding 5/6
                .py(z(5.))
                .px(z(6.))
                .flex()
                .items_center()
                .gap(z(8.))
                .rounded(z(4.))
                .cursor_pointer()
                .hover(|s| s.bg(t(idx::BACKGROUND_MOD_NORMAL)))
                .on_click(cx.listener(|a, _, w, cx| a.toggle_status_picker(w, cx)))
                .child(avatar_with_status(Some(&app.me), 32., app.presence, c(Palette::BG_SECONDARY_ALT)))
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .mt(z(-1.)) // panelTitleContainer_g6VwXp
                                .mb(z(2.))
                                // dump #985 `body-md/medium` 16/22, `--text-strong`
                                .text_size(z(16.))
                                .line_height(z(22.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(t(idx::TEXT_STRONG))
                                .text_ellipsis()
                                .child(app.me.display_name.clone()),
                        )
                        .child(
                            div()
                                // dump #989 `body-xs/medium` 12/13, `--interactive-text-default`
                                .text_size(z(12.))
                                .line_height(z(13.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(t(idx::INTERACTIVE_TEXT_DEFAULT))
                                .text_ellipsis()
                                .child(
                                    // vanilla-guild-text user panel: the custom status replaces "Online".
                                    // Emoji + text from the own custom-status activity while it matches.
                                    {
                                        let act = app.people.activities.get(&app.me.id).and_then(|v| {
                                            v.iter().find(|a| a.kind == crate::presence::ActivityKind::Custom)
                                        });
                                        match (&app.people.custom_status, act) {
                                            (Some(c), Some(a)) if a.state.as_ref() == Some(c) => a.headline(),
                                            (Some(c), _) => c.clone(),
                                            (None, Some(a)) if app.people.custom_status.is_none() && a.state.is_none() => a.headline(),
                                            _ => status,
                                        }
                                    },
                                ),
                        ),
                ),
        )
        .child(
            // `buttons_g6VwXp`: `--space-8` gap.
            div()
                .flex()
                .flex_none()
                .gap(z(8.))
                .child(audio_button(
                    Audio::Mic,
                    if muted { Icon::MicOff } else { Icon::Mic },
                    muted,
                    if muted { crate::t!("voice.unmute") } else { crate::t!("voice.mute") },
                    |a, cx| {
                        a.mic_muted = !a.mic_muted;
                        crate::voice::sync_self_state(a);
                        cx.notify();
                    },
                    cx,
                ))
                .child(audio_button(
                    Audio::Deafen,
                    if app.deafened { Icon::HeadphonesOff } else { Icon::Headphones },
                    app.deafened,
                    if app.deafened { crate::t!("voice.undeafen") } else { crate::t!("voice.deafen") },
                    |a, cx| {
                        a.deafened = !a.deafened;
                        crate::voice::sync_self_state(a);
                        cx.notify();
                    },
                    cx,
                ))
                .child(
                    panel_button("settings", panel_glyph(Icon::Gear, false), false)
                        .tooltip(crate::ui::tooltip::tip(crate::t!("settings.user_settings")))
                        .on_click(cx.listener(|a, _, _, cx| crate::ui::settings::open(a, cx))),
                ),
        )
}

/// Rows above the first channel of a guild (vanilla-guild-text #663..#680): the
/// same `wrapper_TNyK-N`/`link_TNyK-N` rows as channels (36px, padding 4 10, gap 10,
/// 20px `icon_TNyK-N`, 16/28 medium `--channels-default`) for "Events", "Members"
/// (with moderation rights) and "Server Boosts", then `sectionDivider_ysO7Gy`
/// (1px `--border-subtle`, inset 20 / 11) and a 12px gap.
fn guild_top(app: &PhoenixApp) -> impl IntoElement {
    let perms = app.selected_guild.map(|g| app.backend.guild_permissions(g)).unwrap_or(0);
    let mods = crate::ui::inbox::has_perm(perms, (1 << 1) | (1 << 2) | (1 << 5)); // KICK | BAN | MANAGE_GUILD
    let item = |id: &'static str, glyph: &'static str, label: SharedString| {
        div().pl(z(10.)).pr(z(8.)).py(z(1.)).child(
            div()
                .id(id)
                .h(z(36.))
                .px(z(10.))
                .rounded(z(ROW_RADIUS))
                .flex()
                .items_center()
                .gap(z(10.))
                .cursor_pointer()
                .text_color(t(idx::CHANNELS_DEFAULT))
                .hover(|s| s.bg(t(idx::INTERACTIVE_BACKGROUND_HOVER)).text_color(t(idx::INTERACTIVE_TEXT_HOVER)))
                .child(gpui::svg().path(crate::icons::path(glyph)).size(z(20.)).flex_none().text_color(t(idx::CHANNEL_ICON)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(z(16.))
                        .line_height(z(28.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_ellipsis()
                        .child(label),
                ),
        )
    };
    div()
        .pt(z(8.))
        .flex()
        .flex_col()
        .child(item("g-events", "calendar", crate::t!("guild.top.events")))
        .when(mods, |d| d.child(item("g-members", "members", crate::t!("guild.top.members"))))
        .child(item("g-boosts", "boost", crate::t!("guild.top.boosts")))
        .child(div().mx(z(20.)).ml(z(20.)).mr(z(11.)).h(z(1.)).bg(t(idx::BORDER_SUBTLE)))
        .child(div().h(z(12.)))
}
