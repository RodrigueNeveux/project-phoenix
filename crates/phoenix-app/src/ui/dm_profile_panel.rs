//! User profile side panel on the right of a 1:1 DM (`aside_HbUF2J` >
//! `outer_P1YEjQ user-profile-sidebar`). Structure and values from the CDP
//! dump of a DM with the panel open (see docs/parity/popouts.md, "DM profile
//! panel"): 320px aside with 8px padding, 304px card (2px padding, radius 8),
//! 105px banner, 80px avatar, body margin 0 16 / gap 20, footer button.

use std::cell::RefCell;
use std::collections::HashSet;

use gpui::{AnyElement, Context, FontWeight, IntoElement, ParentElement, Styled, div, prelude::*};

use crate::{
    app::PhoenixApp,
    model::Id,
    presence::format_date,
    theme::{Palette, c, generated::idx, tok, z},
    ui::profile::{avatar_block, banner, info, mutuals_row, name_block, section_title},
};

thread_local! {
    static REQUESTED: RefCell<HashSet<Id>> = RefCell::new(HashSet::new());
}

/// The DM peer whose panel should show, if the open channel is a 1:1 DM.
pub fn peer(app: &PhoenixApp) -> Option<Id> {
    let ch = app.selected_channel.and_then(|c| app.channels.get(&c))?;
    if ch.guild.is_some() || ch.voice_users.len() != 1 {
        return None;
    }
    ch.voice_users.first().copied()
}

fn ensure_profile(app: &PhoenixApp, uid: Id, cx: &mut Context<PhoenixApp>) {
    if app.people.profiles.contains_key(&uid) || !REQUESTED.with(|r| r.borrow_mut().insert(uid)) {
        return;
    }
    let fut = app.backend.user_profile(uid, None);
    cx.spawn(async move |this, cx| {
        let Ok(p) = fut.await else { return };
        let _ = this.update(cx, |a, cx| {
            a.people.profiles.insert(uid, p);
            cx.notify();
        });
    })
    .detach();
}

/// The panel, or None outside a 1:1 DM / when toggled off.
pub fn render(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    if !app.show_members {
        return None;
    }
    let uid = peer(app)?;
    ensure_profile(app, uid, cx);
    let i = info(app, uid, None);
    // `:where(.outer_P1YEjQ).user-profile-sidebar-redesign:not(.custom-user-profile-theme)`
    // background --chat-background-default (dump vanilla-dm-profile-panel #1548).
    // With a Nitro theme the outer card is the primary -> secondary gradient and
    // `inner_P1YEjQ` lays --profile-gradient-overlay (black / .6) over it.
    let theme = i.prof.and_then(|p| p.theme);
    let card_bg = tok(idx::CHAT_BACKGROUND_DEFAULT);
    let mix = |rgb: u32| {
        // color-mix(in oklab, overlay 100%, theme colour): approximated as 60% black over it.
        let ch = |sh: u32| (((rgb >> sh) & 0xFF) as f32 * 0.4).round() as u32;
        (ch(16) << 16) | (ch(8) << 8) | ch(0)
    };
    let ring_bg = theme.map(|(p, _)| mix(p)).unwrap_or(card_bg);
    let e = cx.entity();
    // bannerButton_C4gRrZ: 32px circle, rgba(0,0,0,.56), 1px rgba(255,255,255,.08), 16px glyph.
    let banner_btn = |id: &'static str, icon: &'static str| {
        div()
            .id(id)
            .size(z(32.))
            .rounded_full()
            .bg(gpui::hsla(0., 0., 0., 0.56))
            .border_1()
            .border_color(gpui::hsla(0., 0., 1., 0.08))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .child(gpui::svg().path(icon).size(z(16.)).text_color(gpui::white()))
    };
    let bio = i.prof.map(|p| p.bio.clone()).filter(|b| !b.is_empty());
    let created = i.prof.map(|p| p.created);
    let header = div()
        .relative()
        .h(z(144.)) // header_S_xUx6 144px, margin-bottom 8
        .mb(z(8.))
        .child(banner(&i, 105., 6.)) // banner_vNEdYY 300x105
        // wrapper_xpwejc: top/right 8px, gap 8
        .child(
            div()
                .absolute()
                .top(z(8.))
                .right(z(8.))
                .flex()
                .gap(z(8.))
                .child(banner_btn("dm-panel-friend", "icons/friends.svg"))
                .child(banner_btn("dm-panel-more", "icons/chat/more.svg")),
        )
        // avatar_WRLIGC: 80px at (14, 55) incl. the ring
        .child(div().absolute().left(z(8.)).top(z(49.)).child(avatar_block(&i, 80., 6., ring_bg)));
    let mut body = div()
        .mx(z(16.)) // body_A7qlid margin 0 16, gap 20
        .flex()
        .flex_col()
        .gap(z(20.))
        .child(
            // info_A7qlid gap 8: name block, then mutuals_PXYMZ1
            div()
                .flex()
                .flex_col()
                .gap(z(8.))
                .child(name_block(&i, false))
                // mutuals_PXYMZ1: 16px face pile + "N mutual friends" • "N mutual servers"
                .children(i.prof.and_then(mutuals_row)),
        );
    if let Some(b) = bio {
        // section_NcvRRg gap 8; descriptionClamp_BcddaK body-sm 14/18, 3 lines;
        // viewFullBio_BcddaK margin-top 8, 12/16 text button.
        let e2 = e.clone();
        body = body.child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(z(14.))
                        .line_height(z(18.))
                        .text_color(c(tok(idx::TEXT_DEFAULT)))
                        .line_clamp(3)
                        .child(b),
                )
                .child(
                    div()
                        .id("dm-panel-full-bio")
                        .mt(z(8.))
                        .text_size(z(12.))
                        .line_height(z(16.))
                        .text_color(c(tok(idx::TEXT_DEFAULT)))
                        .cursor_pointer()
                        .hover(|s| s.underline())
                        .on_click(move |_, _, cx| e2.update(cx, |a, cx| a.open_profile_modal(uid, cx)))
                        .child(crate::t!("profile.view_full_bio")),
                ),
        );
    }
    if let Some(t) = created {
        // headings_NcvRRg: body-xs/semibold 12/16; value 14/18
        body = body.child(
            div()
                .flex()
                .flex_col()
                .gap(z(8.))
                .child(section_title(crate::t!("profile.member_since")).mb(z(0.)))
                .child(div().text_size(z(14.)).line_height(z(18.)).text_color(c(tok(idx::TEXT_DEFAULT))).child(format_date(t))),
        );
    }
    let e3 = e.clone();
    let footer = div().flex_none().p(z(16.)).child(
        // redesignFooter_YBm90d padding 16; button_F7qvhY radius 8, 7px 15px, 16/20 500
        div()
            .id("dm-panel-view-full")
            .w_full()
            .h(z(39.))
            .rounded(z(8.))
            .bg(c(tok(idx::BACKGROUND_MOD_SUBTLE))) // secondary_F7qvhY rgba(151,151,159,.12)
            .border_1()
            .border_color(c(Palette::DIVIDER))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|s| s.bg(c(Palette::BG_MOD_SELECTED)))
            .text_size(z(16.))
            .line_height(z(20.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(c(Palette::TEXT_HEADER))
            .on_click(move |_, _, cx| e3.update(cx, |a, cx| a.open_profile_modal(uid, cx)))
            .child(crate::t!("profile.view_full")),
    );
    Some(
        div()
            .w(z(320.)) // aside_HbUF2J 320px, padding 8
            .flex_none()
            .h_full()
            .p(z(8.))
            .bg(c(Palette::BG_PRIMARY))
            .child(
                div()
                    .size_full()
                    .p(z(2.)) // outer_P1YEjQ padding --custom-user-profile-theme-padding, radius --radius-sm
                    .rounded(z(8.))
                    .map(|d| match theme {
                        Some((p, s2)) => d.bg(gpui::linear_gradient(
                            180.,
                            gpui::linear_color_stop(c(p), 0.18), // primary until the banner height
                            gpui::linear_color_stop(c(s2), 0.88), // secondary at 100% - footer height
                        )),
                        None => d.bg(c(card_bg)),
                    })
                    .child(
                        div()
                            .size_full()
                            .rounded(z(6.)) // inner_P1YEjQ radius 6
                            .overflow_hidden()
                            .when_some(theme, |d, (p, s2)| {
                                d.bg(gpui::linear_gradient(
                                    180.,
                                    gpui::linear_color_stop(c(mix(p)), 0.18),
                                    gpui::linear_color_stop(c(mix(s2)), 1.),
                                ))
                            })
                            .flex()
                            .flex_col()
                            .child(div().id("dm-panel-scroll").flex_1().min_h_0().overflow_y_scroll().child(header).child(body))
                            .child(footer),
                    ),
            )
            .into_any_element(),
    )
}
