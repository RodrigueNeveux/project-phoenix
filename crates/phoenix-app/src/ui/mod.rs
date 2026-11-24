pub mod autocomplete;
pub mod avatar_deco;
pub mod call_view;
pub mod chat;
pub mod components;
pub mod forum;
pub mod group_dm;
pub mod inbox;
pub mod context_menu;
pub mod controls;
pub mod guild_menu;
pub mod home;
pub mod invite;
pub mod image_viewer;
pub mod login;
pub mod settings;
pub mod members;
pub mod activity_traits;
pub mod nameplate;
pub mod profile_fx;
pub mod modal;
pub mod pickers;
pub mod modals;
pub mod profile;
pub mod dm_profile_panel;
pub mod avatar_color;
pub mod profile_note;
pub mod profile_bio;
pub mod search;
pub mod server_bar;
pub mod sidebar;
pub mod sidebar_resize;
pub mod states;
pub mod status_picker;
pub mod switcher;
pub mod nav_menus;
pub mod thread_menu;
pub mod title_bar;
pub mod tooltip;
pub mod channel_unread_bar;
pub mod user_menu;
pub mod voice_panel;
pub mod voice_row;

use crate::theme::z;
use gpui::{
    AnyElement, Div, FontWeight, Hsla, IntoElement, ParentElement, SharedString, Styled, div,
    prelude::*, px, svg,
};

use crate::{
    assets::Icon,
    model::{ImageRef, Presence, User},
    theme::{DEFAULT_AVATAR, Palette, c, presence_color},
};

pub fn icon(i: Icon, size: f32, color: Hsla) -> gpui::Svg {
    svg().path(i.path()).size(px(size)).flex_none().text_color(color)
}

/// Circle avatar. URL images go through GPUI's asset cache, which fetches
/// lazily only when the element is actually painted (rows are virtualized).
pub fn avatar(user: Option<&User>, size: f32) -> AnyElement {
    let img_ref = user.map(|u| u.avatar.clone()).unwrap_or(ImageRef::Default(0));
    match img_ref {
        ImageRef::Url(url) => crate::media::avatar_image(url, size),
        // The client shows the CDN default avatar (embed/avatars/<n>.png), fetched at runtime;
        // the colour shows while it loads.
        ImageRef::Default(ix) => div()
            .size(px(size))
            .flex_none()
            .rounded_full()
            .bg(c(DEFAULT_AVATAR[ix as usize % DEFAULT_AVATAR.len()]))
            .child(crate::media::avatar_image(format!("https://cdn.discordapp.com/embed/avatars/{ix}.png").into(), size))
            .into_any_element(),
    }
}

/// Avatar with the presence dot cut-out (bottom-right) like Discord.
pub fn avatar_with_status(user: Option<&User>, size: f32, p: Presence, ring: Hsla) -> Div {
    let dot = (size * 0.3125).round().max(10.);
    let border = (size / 10.).round().max(3.);
    div()
        .relative()
        .flex_none()
        .size(px(size))
        .child(avatar(user, size))
        .child(
            div()
                .absolute()
                .right(px(-border / 2. - 1.))
                .bottom(px(-border / 2. - 1.))
                .size(px(dot + border))
                .rounded_full()
                .bg(ring)
                .flex()
                .items_center()
                .justify_center()
                .child(status_dot(p, dot, ring)),
        )
}

pub fn status_dot(p: Presence, size: f32, ring: Hsla) -> Div {
    let base = div().size(px(size)).rounded_full().bg(presence_color(p)).relative();
    match p {
        Presence::Offline => base.child(
            div()
                .absolute()
                .top(px(size * 0.25))
                .left(px(size * 0.25))
                .size(px(size * 0.5))
                .rounded_full()
                .bg(ring),
        ),
        Presence::Dnd => base.flex().items_center().justify_center().child(
            div().w(px(size * 0.6)).h(px((size * 0.2).max(2.))).rounded(z(1.)).bg(ring),
        ),
        Presence::Idle => base.child(
            div()
                .absolute()
                .top(px(-size * 0.1))
                .left(px(-size * 0.1))
                .size(px(size * 0.6))
                .rounded_full()
                .bg(ring),
        ),
        Presence::Online => base,
    }
}

/// Red mention badge ("3").
pub fn badge(n: u32) -> Div {
    let label: SharedString = if n > 99 { "99+".into() } else { n.to_string().into() };
    div()
        .h(z(16.))
        .min_w(z(16.))
        .px(z(4.))
        .rounded(z(8.))
        .bg(c(Palette::RED))
        .text_color(gpui::white())
        .text_size(z(12.))
        .font_weight(FontWeight::BOLD)
        .flex()
        .items_center()
        .justify_center()
        .child(label)
}

/// 32px square icon button with hover background (user panel, header).
pub fn icon_button(id: &'static str, i: Icon, color: Hsla, size: f32) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .size(px(size + 8.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(z(4.))
        .cursor_pointer()
        .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
        .child(icon(i, size, color))
}
