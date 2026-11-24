//! Voice channel member row (channel list) interactions: profile popout anchor and
//! the row bounds it needs.
//!
//! Live client (vanilla, 2026-11-24): clicking `voiceUser_YUoOxm` opens the standard
//! `user-profile-popout` (300px) with its left edge 8px right of the row and its top
//! aligned with the row; while open the row carries `selected_YUoOxm`. Clicking the
//! row never joins the channel.

use std::cell::RefCell;
use std::collections::HashMap;

use gpui::{AnyElement, Bounds, InteractiveElement, IntoElement, ParentElement, Pixels, Point, StatefulInteractiveElement, Styled, div, point, px, svg};

use crate::theme::{c, generated::idx, tok, z};

use crate::model::Id;

thread_local! {
    static ROWS: RefCell<HashMap<u64, Bounds<Pixels>>> = RefCell::new(HashMap::new());
}

/// Remember where the row for `user` was painted.
thread_local! {
    /// Voice channels with "Hide Names" checked (channel context menu, per session).
    static HIDDEN_NAMES: std::cell::RefCell<std::collections::HashSet<u64>> = Default::default();
}

pub fn names_hidden(channel: Id) -> bool {
    HIDDEN_NAMES.with(|h| h.borrow().contains(&channel.0))
}

pub fn set_names_hidden(channel: Id, hidden: bool) {
    HIDDEN_NAMES.with(|h| {
        let mut h = h.borrow_mut();
        if hidden {
            h.insert(channel.0);
        } else {
            h.remove(&channel.0);
        }
    });
}

/// Whether `user`'s row sits in a voice channel with hidden names.
pub fn user_name_hidden(app: &crate::app::PhoenixApp, user: Id) -> bool {
    HIDDEN_NAMES.with(|h| {
        h.borrow().iter().any(|c| app.channels.get(&Id(*c)).is_some_and(|ch| ch.voice_users.contains(&user)))
    })
}

pub fn record(user: Id, b: Bounds<Pixels>) {
    ROWS.with(|r| r.borrow_mut().insert(user.0, b));
}

/// `open_profile` anchor that lands the popout at (row right + 8, row top).
/// profile.rs places the card at (at.x - 300 - 16, at.y - 60).
pub fn popout_anchor(user: Id, fallback: Point<Pixels>) -> Point<Pixels> {
    let b = ROWS.with(|r| r.borrow().get(&user.0).copied());
    match b {
        Some(b) => point(b.right() + px(8. + 300. + 16.), b.top() + px(60.)),
        None => point(fallback.x + px(300. + 16. + 8.), fallback.y + px(60.)),
    }
}

/// optionsButton_YUoOxm: 24x24, radius 4px, hover/active --interactive-background-active.
/// Opens the user's context menu at the button (voice menu when in the call).
pub fn options_button(user: Id, guild: Option<Id>, in_call: bool) -> AnyElement {
    div()
        .id(("voice-user-options", user.0))
        .size(z(24.))
        .flex_none()
        .rounded(z(4.))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|s| s.bg(c(tok(idx::INTERACTIVE_BACKGROUND_ACTIVE))))
        .child(
            svg()
                .path(crate::icons::path("more-horizontal"))
                .size(z(16.))
                .text_color(c(tok(idx::CHANNELS_DEFAULT))), // .icon_YUoOxm --channels-default
        )
        .on_click(move |ev, window, cx| {
            cx.stop_propagation();
            if in_call {
                crate::ui::voice_panel::open_user_menu(user, ev.position(), cx);
            } else {
                crate::ui::user_menu::open(user, guild, ev.position(), window);
            }
        })
        .into_any_element()
}

/// "In a voice channel" card in the profile popout (`clickableContainer_Bu4Al8` in
/// `cardsList_M2WrKU`, live vanilla popout opened from a voice row): same card
/// metrics as the activity cards (padding 8px, --radius-sm, border --border-subtle,
/// header body-xs/medium), body = guild circle + channel name heading-xs/semibold
/// with the speaker icon (`voiceIcon_Bu4Al8`) and the guild name. Clicking opens the
/// channel's text chat; it never joins voice.
pub fn voice_activity_card(app: &crate::app::PhoenixApp, user: Id, entity: gpui::Entity<crate::app::PhoenixApp>) -> Option<AnyElement> {
    use gpui::{FontWeight, prelude::FluentBuilder};
    let ch = app.channels.values().find(|c| c.voice_users.contains(&user))?;
    let (cid, cname) = (ch.id, ch.name.clone());
    let g = ch.guild.and_then(|g| app.guilds.iter().find(|x| x.id == g));
    let gname = g.map(|g| g.name.clone());
    let acronym = g.map(|g| g.acronym.clone()).unwrap_or_default();
    // GuildIcon: the guild's icon when it has one, the acronym otherwise.
    let icon = g.and_then(|g| match &g.icon {
        crate::model::ImageRef::Url(u) => Some(u.clone()),
        _ => None,
    });
    // `.theme-light :where(.overlay_P1YEjQ)` borders the card
    let light = crate::ui::profile_theme::is_light();
    Some(
        div()
            .id(("profile-voice-card", user.0))
            .pb(z(8.)) // container_M2WrKU card spacing
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(z(8.)) // card_Bu4Al8 gap
                    .p(z(8.)) // --custom-activity-card-padding (popout)
                    .rounded(z(8.)) // --radius-sm
                    // :where(.overlay_P1YEjQ): --user-profile-overlay-background, 1px
                    // --border-muted on light cards only; clickableContainer_:hover → -hover.
                    .bg(c(tok(idx::USER_PROFILE_OVERLAY_BACKGROUND)))
                    .hover(|s| s.bg(c(tok(idx::USER_PROFILE_OVERLAY_BACKGROUND_HOVER))))
                    .when(light, |d| d.border_1().border_color(c(tok(idx::BORDER_MUTED))))
                    .cursor_pointer()
                    .child(
                        div()
                            .text_size(z(12.)) // header_Bu4Al8 body-xs/medium
                            .line_height(z(16.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(c(tok(idx::TEXT_DEFAULT)))
                            .child(crate::t!("members.in_voice")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(z(8.)) // content_Bu4Al8 gap
                            .child(
                                // container_vg4rr1: 60px, --radius-sm, 1px --border-subtle,
                                // --card-background-default under a --background-mod-subtle layer
                                div()
                                    .size(z(60.))
                                    .flex_none()
                                    .rounded(z(8.))
                                    .border_1()
                                    .border_color(c(tok(idx::BORDER_SUBTLE)))
                                    .bg(c(tok(idx::CARD_BACKGROUND_DEFAULT)))
                                    .child(div().absolute().inset_0().rounded(z(8.)).bg(c(tok(idx::BACKGROUND_MOD_SUBTLE))))
                                    .relative()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(z(14.))
                                    .text_color(c(tok(idx::TEXT_DEFAULT)))
                                    .overflow_hidden()
                                    .map(|d| match icon {
                                        Some(u) => d.child(gpui::img(u).absolute().inset_0().size_full().rounded(z(8.))),
                                        None => d.child(acronym),
                                    }),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(z(4.)) // details_Bu4Al8 gap
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(z(4.))
                                            .child(svg().path(crate::icons::path("speaker")).size(z(16.)).flex_none().text_color(c(tok(idx::TEXT_DEFAULT))))
                                            .child(
                                                div()
                                                    .min_w_0()
                                                    .text_size(z(14.)) // heading-xs/semibold
                                                    .line_height(z(18.))
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(c(tok(idx::TEXT_STRONG)))
                                                    .text_ellipsis()
                                                    .child(cname),
                                            ),
                                    )
                                    .when_some(gname, |d, n| {
                                        d.child(
                                            div()
                                                .text_size(z(12.)) // body-xs
                                                .line_height(z(16.))
                                                .text_color(c(tok(idx::TEXT_MUTED)))
                                                .text_ellipsis()
                                                .child(n),
                                        )
                                    }),
                            ),
                    ),
            )
            .on_click(move |_, _, cx| {
                entity.update(cx, |a, cx| {
                    a.people.popout = None;
                    a.open_channel_chat(cid, cx);
                })
            })
            .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::size;

    #[test]
    fn anchor_is_right_of_row() {
        record(Id(5), Bounds::new(point(px(118.), px(290.)), size(px(289.), px(32.))));
        let a = popout_anchor(Id(5), point(px(0.), px(0.)));
        assert_eq!(a.x - px(316.), px(415.));
        assert_eq!(a.y - px(60.), px(290.));
    }
}
