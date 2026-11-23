//! 240px channel sidebar: guild header, channel tree / DM list, user panel.

use gpui::{
    AnyElement, App, Context, Entity, FontWeight, IntoElement, ParentElement, SharedString, Styled,
    Transformation, div, list, percentage, prelude::*, px,
};

use crate::{
    app::{PhoenixApp, SidebarRow},
    assets::Icon,
    model::{ChannelKind, Id},
    theme::{Palette, c, m},
    ui::{avatar, avatar_with_status, badge, icon, icon_button},
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

pub fn render(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let entity = cx.entity();
    div()
        .w(px(m::SIDEBAR_W))
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .bg(c(Palette::BG_SECONDARY))
        .rounded_tl(px(8.))
        .overflow_hidden()
        .child(header(app))
        .child(
            div().flex_1().min_h_0().child(
                list(app.sidebar_list.clone(), move |ix, _w, cx| row(&entity, ix, cx)).size_full(),
            ),
        )
        .when(app.voice_channel.is_some(), |d| d.child(voice_panel(app, cx)))
        .child(user_panel(app, cx))
}

fn header(app: &PhoenixApp) -> AnyElement {
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
            .into_any_element(),
        Some(g) => {
            let banner = g.banner_color;
            div()
                .flex_none()
                .relative()
                .when_some(banner, |d, col| {
                    d.h(px(84.)).bg(c(col)).child(
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
                        .child(icon(Icon::Chevron, 18., c(Palette::TEXT_HEADER))),
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
            dm_top(entity).into_any_element()
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
                .on_click(move |_, _, cx| e.update(cx, |a, cx| a.toggle_category(id, cx)))
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
        SidebarRow::VoiceUser { user } => {
            let u = app.users.get(&user);
            let speaking = user.0 % 3 == 0;
            div()
                .ml(px(36.))
                .mr(px(8.))
                .h(px(32.))
                .px(px(8.))
                .rounded(px(4.))
                .flex()
                .items_center()
                .gap(px(8.))
                .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
                .child(
                    div()
                        .rounded_full()
                        .border_2()
                        .border_color(if speaking { c(Palette::GREEN) } else { gpui::transparent_black() })
                        .child(avatar(u, 20.)),
                )
                .child(
                    div()
                        .flex_1()
                        .text_size(px(14.))
                        .text_color(c(Palette::TEXT_MUTED))
                        .text_ellipsis()
                        .child(u.map(|u| u.display_name.clone()).unwrap_or_default()),
                )
                .when(user == app.me.id && app.mic_muted, |d| d.child(icon(Icon::MicOff, 16., c(Palette::TEXT_MUTED))))
                .into_any_element()
        }
    }
}

fn dm_top(entity: &Entity<PhoenixApp>) -> impl IntoElement {
    let _ = entity;
    let nav = |id: &'static str, i: Icon, label: &'static str, selected: bool| {
        div()
            .id(id)
            .mx(px(8.))
            .h(px(42.))
            .px(px(8.))
            .rounded(px(4.))
            .flex()
            .items_center()
            .gap(px(12.))
            .cursor_pointer()
            .text_size(px(16.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(c(if selected { Palette::TEXT_HEADER } else { Palette::TEXT_MUTED }))
            .when(selected, |d| d.bg(c(Palette::BG_MOD_SELECTED)))
            .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)).text_color(c(Palette::INTERACTIVE_HOVER)))
            .child(icon(i, 24., c(Palette::INTERACTIVE)))
            .child(label)
    };
    div()
        .pt(px(8.))
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(nav("friends", Icon::Friends, "Friends", false))
        .child(nav("nitro", Icon::Nitro, "Nitro", false))
        .child(nav("shop", Icon::Gift, "Shop", false))
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
                .child(icon(Icon::Plus, 16., c(Palette::TEXT_MUTED))),
        )
}

fn channel_row(app: &PhoenixApp, entity: &Entity<PhoenixApp>, id: Id) -> AnyElement {
    let ch = &app.channels[&id];
    let selected = app.selected_channel == Some(id);
    let is_dm = ch.guild.is_none();
    let fg = if selected {
        Palette::TEXT_HEADER
    } else if ch.muted {
        Palette::INTERACTIVE_MUTED
    } else if ch.unread {
        Palette::TEXT_HEADER
    } else {
        Palette::TEXT_MUTED
    };
    let e = entity.clone();
    let lead: AnyElement = if is_dm {
        let u = ch.voice_users.first().and_then(|u| app.users.get(u));
        let p = ch.voice_users.first().map(|u| app.presence_of(*u)).unwrap_or_default();
        avatar_with_status(u, 32., p, c(Palette::BG_SECONDARY)).into_any_element()
    } else {
        icon(channel_icon(ch.kind), 20., c(if ch.muted { Palette::INTERACTIVE_MUTED } else { Palette::CHANNEL_ICON }))
            .into_any_element()
    };
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
                .when(ch.unread && !selected && !ch.muted, |d| d.bg(gpui::white())),
        )
        .child(
            div()
                .id(("chrow", id.0))
                .h(px(if is_dm { 42. } else { 32. }))
                .px(px(8.))
                .rounded(px(4.))
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
                .child(lead)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(16.))
                        .line_height(px(20.))
                        .font_weight(if ch.unread && !ch.muted { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                        .text_ellipsis()
                        .child(ch.name.clone()),
                )
                .when(ch.mentions > 0, |d| d.child(badge(ch.mentions))),
        )
        .into_any_element()
}

fn voice_panel(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let ch = app.voice_channel.and_then(|v| app.channels.get(&v));
    let guild = ch
        .and_then(|c| c.guild)
        .and_then(|g| app.guilds.iter().find(|x| x.id == g))
        .map(|g| g.name.clone())
        .unwrap_or_default();
    let label: SharedString = format!("{} / {}", ch.map(|c| c.name.as_ref()).unwrap_or(""), guild).into();
    div()
        .flex_none()
        .bg(c(Palette::BG_SECONDARY_ALT))
        .border_b_1()
        .border_color(c(Palette::DIVIDER))
        .px(px(8.))
        .py(px(8.))
        .flex()
        .items_center()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .text_size(px(14.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(Palette::GREEN))
                        .child("Voice Connected"),
                )
                .child(div().text_size(px(12.)).text_color(c(Palette::TEXT_MUTED)).text_ellipsis().child(label)),
        )
        .child(
            icon_button("hangup", Icon::PhoneHangup, c(Palette::INTERACTIVE), 20.)
                .on_click(cx.listener(|a, _, _, cx| a.join_voice(None, cx))),
        )
}

fn user_panel(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let status = match app.presence {
        crate::model::Presence::Online => "Online",
        crate::model::Presence::Idle => "Idle",
        crate::model::Presence::Dnd => "Do Not Disturb",
        crate::model::Presence::Offline => "Invisible",
    };
    let red = c(Palette::RED);
    let normal = c(Palette::INTERACTIVE);
    div()
        .h(px(m::USER_PANEL_H))
        .flex_none()
        .bg(c(Palette::BG_SECONDARY_ALT))
        .px(px(8.))
        .flex()
        .items_center()
        .gap(px(2.))
        .child(
            div()
                .id("me")
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap(px(8.))
                .pl(px(2.))
                .py(px(4.))
                .rounded(px(4.))
                .cursor_pointer()
                .hover(|s| s.bg(c(0x3D3E45)))
                .on_click(cx.listener(|a, _, w, cx| a.toggle_status_picker(w, cx)))
                .child(avatar_with_status(Some(&app.me), 32., app.presence, c(Palette::BG_SECONDARY_ALT)))
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(14.))
                                .line_height(px(18.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(c(Palette::TEXT_HEADER))
                                .text_ellipsis()
                                .child(app.me.display_name.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(13.))
                                .text_color(c(Palette::TEXT_MUTED))
                                .text_ellipsis()
                                .child(app.people.custom_status.clone().unwrap_or(status.into())),
                        ),
                ),
        )
        .child(
            icon_button(
                "mute",
                if app.mic_muted || app.deafened { Icon::MicOff } else { Icon::Mic },
                if app.mic_muted || app.deafened { red } else { normal },
                20.,
            )
            .on_click(cx.listener(|a, _, _, cx| {
                a.mic_muted = !a.mic_muted;
                cx.notify();
            })),
        )
        .child(
            icon_button(
                "deafen",
                if app.deafened { Icon::HeadphonesOff } else { Icon::Headphones },
                if app.deafened { red } else { normal },
                20.,
            )
            .on_click(cx.listener(|a, _, _, cx| {
                a.deafened = !a.deafened;
                cx.notify();
            })),
        )
        .child(
            // No settings screen yet: the gear logs out (and forgets the stored token).
            icon_button("settings", Icon::Gear, normal, 20.)
                .tooltip(|window, cx| gpui_component::tooltip::Tooltip::new("Log out").build(window, cx))
                .on_click(cx.listener(|a, _, _, cx| crate::ui::settings::open(a, cx))),
        )
}
