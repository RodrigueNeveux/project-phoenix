//! Chat area: header, virtualized message list, composer, typing indicator.

use gpui::{
    AnyElement, App, Context, Entity, FontWeight, IntoElement, ParentElement,
    SharedString, StyledText, Styled, InteractiveText, Window, div, img, list, prelude::*,
};

use crate::{
    app::PhoenixApp,
    assets::Icon,
    media,
    model::{Attachment, AttachmentKind, ChannelKind, Embed, Id, Message},
    richtext::{ClickTarget, RBlock, RenderedMessage, RichText},
    theme::{FONT_MONO, Palette, c, ca, fz, generated::idx, m, space, tok, z},
    ui::{avatar, icon, icon_button, members, sidebar::channel_icon},
};

// Messaging features live in submodules (composer moved to `composer.rs`).
pub mod composer;
pub mod icons;
pub mod forward;
pub mod message_ext;
pub mod mosaic;
pub mod panels;
pub mod state;
pub mod slash;
pub mod system;
pub use state::ChatState;

/// Colour of a generated CSS token (`--name` → `idx::NAME`) for the active theme.
pub(crate) fn tk(i: usize) -> gpui::Hsla {
    c(tok(i))
}

pub fn render(app: &PhoenixApp, window: &mut Window, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let entity = cx.entity();
    let show_members = app.show_members && app.selected_guild.is_some();
    let show_members = show_members && app.chat.thread.is_none();
    div()
        .flex_1()
        .min_w_0()
        .h_full()
        .relative()
        .flex()
        .flex_col()
        .bg(c(Palette::BG_PRIMARY))
        .on_modifiers_changed(cx.listener(|a, ev: &gpui::ModifiersChangedEvent, _, cx| {
            if a.chat.shift_held != ev.modifiers.shift {
                a.chat.shift_held = ev.modifiers.shift;
                cx.notify();
            }
        }))
        .child(header(app, cx))
        .child(
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_row()
                .child(
                    div()
                        .id("chat-column")
                        .group("chat-drop")
                        .relative()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .on_drop(cx.listener(|a, paths: &gpui::ExternalPaths, _, cx| {
                            a.add_paths(paths.paths().to_vec(), cx)
                        }))
                        .children(discovery_notice(app, cx))
                        .child(
                            div()
                                .flex_1()
                                .min_h_0()
                                .relative()
                                // scroller_ scrollbarGutterStable_: messageListItem_ is 579px in a
                                // 595px chat (vanilla-guild-text-groups), 16px reserved for the bar.
                                .pr(z(16.))
                                .child(
                                    list(app.message_list.clone(), move |ix, _w, cx| row(&entity, ix, cx))
                                        .size_full(),
                                )
                                .children(panels::new_messages_bar(app, cx))
                                .children(panels::jump_bar(app, cx)),
                        )
                        .child(composer::render(app, window, cx))
                        .child(panels::drop_overlay(app))
                        .children(crate::ui::forum::render(app, cx)),
                )
                .when(show_members, |d| d.child(members::render(app, cx)))
                .children(crate::ui::dm_profile_panel::render(app, cx))
                .children(panels::thread_panel(app, cx)),
        )
        .children(panels::pins_popout(app, cx))
}

fn guild_discoverable(app: &PhoenixApp) -> bool {
    app.selected_guild.and_then(|g| app.guilds.iter().find(|x| x.id == g)).is_some_and(|g| g.discoverable)
}

/// Guilds whose Discovery notice was closed this session (Discord keeps it client-side).
static NOTICE_CLOSED: std::sync::Mutex<Vec<u64>> = std::sync::Mutex::new(Vec::new());

/// Discovery channel notice. vanilla-guild-text #1101 `bar_8PBDGr`: --background-feedback-mod-info,
/// border-bottom 1px --inlinenotice-border-info, margin-top -1, padding 4 8 4 16, gap 8;
/// #1103 text_ body-sm/normal 14/18 --text-default with an underlined "En savoir plus" (#1104);
/// #1105 close_ 32×32, --icon-muted (hover --icon-default), 18px glyph.
/// Window x of the "Mode Découverte" label centre (f32 bits), set while painting the header.
static LABEL_CX: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

fn discovery_label_probe() -> impl IntoElement {
    gpui::canvas(
        |b, _, _| {
            let cx = f32::from(b.origin.x) + f32::from(b.size.width) / 2.;
            LABEL_CX.store(cx.to_bits(), std::sync::atomic::Ordering::Relaxed);
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// vanilla-guild-text #1102 `arrow_8PBDGr`: 16×10, fixed, centred under the header's
/// Discovery label (#1061, centre 666 vs arrow 657+8), top = bar top − 9 so its base
/// sits on the bar's 1px top border. Filled --background-base-lower then
/// --background-feedback-mod-info, edges stroked --inlinenotice-border-info.
/// Our own path (rounded apex), not the client's SVG.
fn discovery_arrow() -> impl IntoElement {
    gpui::canvas(
        |_, _, _| {},
        |b, _, window, _| {
            let lcx = f32::from_bits(LABEL_CX.load(std::sync::atomic::Ordering::Relaxed));
            if lcx <= 0. {
                return;
            }
            let k = crate::theme::zoom();
            let (x0, y0) = (lcx - 8. * k, f32::from(b.origin.y) - 9. * k);
            let p = |x: f32, y: f32| gpui::point(gpui::px(x0 + x * k), gpui::px(y0 + y * k));
            let shape = |mut pb: gpui::PathBuilder, close: bool| {
                pb.move_to(p(0., 10.));
                pb.line_to(p(6., 2.9));
                pb.curve_to(p(10., 2.9), p(8., 0.7));
                pb.line_to(p(16., 10.));
                if close {
                    pb.close();
                }
                pb.build().ok()
            };
            for col in [tk(idx::BACKGROUND_BASE_LOWER), tk(idx::BACKGROUND_FEEDBACK_MOD_INFO)] {
                if let Some(path) = shape(gpui::PathBuilder::fill(), true) {
                    window.paint_path(path, col);
                }
            }
            if let Some(path) = shape(gpui::PathBuilder::stroke(gpui::px(k)), false) {
                window.paint_path(path, tk(idx::INLINENOTICE_BORDER_INFO));
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

fn discovery_notice(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<gpui::AnyElement> {
    let gid = app.selected_guild?;
    let ch = app.selected_channel.and_then(|c| app.channels.get(&c))?;
    if ch.guild.is_none() || !guild_discoverable(app) || NOTICE_CLOSED.lock().ok()?.contains(&gid.0) {
        return None;
    }
    Some(
        div()
            .flex_none()
            .mt(z(-1.))
            .pt(z(4.))
            .pb(z(4.))
            .pl(z(16.))
            .pr(z(8.))
            .flex()
            .items_center()
            .gap(z(8.))
            .bg(tk(idx::BACKGROUND_FEEDBACK_MOD_INFO))
            .border_t_1()
            .border_b_1()
            .border_color(tk(idx::INLINENOTICE_BORDER_INFO))
            .relative()
            .child(discovery_arrow())
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .flex()
                    .flex_wrap()
                    .text_size(z(14.))
                    .line_height(z(18.))
                    .text_color(tk(idx::TEXT_DEFAULT))
                    .child(crate::t!("chat.discovery_notice"))
                    .child(div().underline().child(crate::t!("chat.discovery_more"))),
            )
            .child(
                div()
                    .id("discovery-notice-close")
                    .group("discovery-notice-close")
                    .flex_none()
                    .size(z(32.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .on_click(cx.listener(move |_, _, _, cx| {
                        if let Ok(mut v) = NOTICE_CLOSED.lock() {
                            v.push(gid.0);
                        }
                        cx.notify();
                    }))
                    .child(
                        gpui::svg()
                            .path(crate::icons::path("close"))
                            .size(z(18.))
                            .text_color(tk(idx::ICON_MUTED))
                            .group_hover("discovery-notice-close", |s| s.text_color(tk(idx::ICON_DEFAULT))),
                    ),
            )
            .into_any_element(),
    )
}

/// `--chat-input-icon-size` glyph in a 32px toolbar button.
const HDR_ICON: f32 = 20.;

/// --custom-message-margin-left-content-cozy = avatar + 2 × --space-md
/// (density-aware; cozy: dump -channels-N-N+ #1184 padding-left 80px).
pub(crate) fn msg_left() -> f32 {
    m::AVATAR + 2. * space::md()
}

fn hdr_button(id: &'static str, i: Icon, color: gpui::Hsla, size: f32) -> gpui::Stateful<gpui::Div> {
    icon_button(id, i, color, size * crate::theme::zoom()).size(z(32.)).rounded(z(8.))
}

/// Header toolbar button whose glyph comes from the runtime icon provider by name
/// (iconWrapper_ .clickable_: 32px box, 20px svg).
fn hdr_named(id: &'static str, name: &str, color: gpui::Hsla) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .size(z(32.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(z(8.))
        .cursor_pointer()
        .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
        .child(gpui::svg().path(crate::icons::path(name)).size(z(HDR_ICON)).flex_none().text_color(color))
}

/// Channel / DM header. Dump -channels-N-N+ #1040 (guild) and -channels-me-N+ #1088 (DM):
/// section title_ padding 8px 8px 8px 20px, height 49px, border-bottom --border-subtle;
/// upperContainer_ → children_ (icon/avatar, title, dot, topic) + toolbar_ (gap 10px, padding-left 10px).
fn header(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let ch = app.selected_channel.and_then(|c| app.channels.get(&c));
    let is_dm = ch.is_some_and(|c| c.guild.is_none());
    let name = ch.map(|c| c.name.clone()).unwrap_or_default();
    let topic = ch.and_then(|c| c.topic.clone());
    let kind = ch.map(|c| c.kind).unwrap_or(ChannelKind::Text);
    let discoverable = !is_dm && guild_discoverable(app);
    // container_ .clickable_ .icon_ color: --icon-muted; .selected_: --icon-strong.
    let ic = tk(idx::ICON_MUTED);
    let on = tk(idx::ICON_STRONG);
    let peer = ch.and_then(|c| c.voice_users.first()).and_then(|u| app.users.get(u));
    let scope: SharedString = if is_dm {
        peer.map(|u| u.name.clone()).unwrap_or_else(|| name.clone())
    } else {
        app.selected_guild.and_then(|g| app.guilds.iter().find(|x| x.id == g)).map(|g| g.name.clone()).unwrap_or_default()
    };
    let search_ph = SharedString::from(format!("{} {}", crate::t!("search.placeholder"), scope));
    div()
        .h(z(m::HEADER_H + 1.))
        .flex_none()
        .pl(z(20.))
        .pr(z(8.))
        .py(z(8.))
        .flex()
        .items_center()
        .bg(c(Palette::BG_PRIMARY))
        .border_t_1()
        .border_b_1()
        .border_color(tk(idx::BORDER_SUBTLE))
        .child(
            // children_: flex 1, items centred.
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .child(if is_dm {
                    // avatar_ 20px, margin-inline-end 8px (#1092).
                    div().mr(z(8.)).child(avatar(peer, 20. * crate::theme::zoom())).into_any_element()
                } else {
                    // channelIcon_ 20px glyph in a 20×32 box, margin 0 11px 0 -1px (#1043); --channel-icon.
                    div()
                        .ml(z(-1.))
                        .mr(z(11.))
                        .w(z(20.))
                        .h(z(32.))
                        .flex()
                        .items_center()
                        .child(icon(channel_icon(kind), 20. * crate::theme::zoom(), tk(idx::CHANNEL_ICON)))
                        .into_any_element()
                })
                .child(
                    // titleWrapper_ margin-inline-end 8px; h1 body-md/medium 16px/20px --text-strong (#1047).
                    div()
                        .flex_none()
                        .mr(z(8.))
                        .text_size(z(16.))
                        .line_height(z(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(tk(idx::TEXT_STRONG))
                        .child(name),
                )
                .when(discoverable, |d| {
                    // vanilla-guild-text #1060 dot_GxzgQj: 4px, --background-mod-strong, margin-inline 4 12;
                    // #1061/#1062 label_GxzgQj > body-sm/medium 14px/18px --text-muted "Mode Découverte".
                    d.child(div().flex_none().ml(z(4.)).mr(z(12.)).size(z(4.)).rounded_full().bg(tk(idx::BACKGROUND_MOD_STRONG)))
                        .child(
                            div()
                                .flex_none()
                                .text_size(z(14.))
                                .line_height(z(18.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(tk(idx::TEXT_MUTED))
                                .relative()
                                .child(crate::t!("chat.discovery_mode"))
                                .child(discovery_label_probe()),
                        )
                })
                .when_some(topic.filter(|_| !is_dm), |d, t| {
                    let topic_click = crate::ui::home::topic::on_click(app, cx);
                    // #1068 leadingBar_pVbL3l after the Discovery label: 1×16, --border-subtle, ml 8.
                    // Otherwise dot_ 4px, margin 0 4px, --text-muted 24% (#1049); topic_ margin-inline-start 8px,
                    // body-sm/medium 14px/18px --text-muted (#1050).
                    d.child(if discoverable {
                        div().flex_none().ml(z(8.)).w(z(1.)).h(z(16.)).bg(tk(idx::BORDER_SUBTLE))
                    } else {
                        div().flex_none().mx(z(4.)).size(z(4.)).rounded_full().bg(tk(idx::BACKGROUND_MOD_STRONG))
                    })
                        .child(
                            div()
                                .id("channel-topic")
                                .ml(z(8.))
                                .flex_1()
                                .min_w_0()
                                .cursor_pointer()
                                .text_size(z(14.))
                                .line_height(z(18.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(tk(idx::TEXT_MUTED))
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .text_ellipsis()
                                .on_click(topic_click)
                                .child(t),
                        )
                }),
        )
        .child(
            div()
                .flex_none()
                .pl(z(10.))
                .flex()
                .items_center()
                .gap(z(10.))
                .when(is_dm, |d| {
                    // DM toolbar (#1101–#1109): voice call, video call, pins, add to DM, user profile.
                    d.child(hdr_named("dm-call", "phone-call", ic))
                        .child(hdr_named("dm-video", "video-call", ic))
                        .child(
                            hdr_button("pin", Icon::Pin, if app.chat.pins.is_some() { on } else { ic }, HDR_ICON)
                                .on_click(cx.listener(|a, _, _, cx| a.toggle_pins(cx))),
                        )
                        .child(hdr_named("dm-add", "add-to-dm", ic))
                        .child(hdr_named("dm-profile", "user-profile", if app.show_members { on } else { ic }).on_click(cx.listener(|a, _, _, cx| {
                            a.show_members = !a.show_members;
                            cx.notify();
                        })))
                })
                .when(!is_dm, |d| {
                    // Guild toolbar (#1055–#1061): threads, notifications, pins, member list.
                    d.child(hdr_button("threads", Icon::Threads, ic, HDR_ICON).on_click(cx.listener(|a, _, _, cx| a.toggle_threads_popout(cx))))
                        .child(hdr_button("bell", Icon::Bell, ic, HDR_ICON))
                        .child(
                            hdr_button("pin", Icon::Pin, if app.chat.pins.is_some() { on } else { ic }, HDR_ICON)
                                .on_click(cx.listener(|a, _, _, cx| a.toggle_pins(cx))),
                        )
                        .child(
                            hdr_button("members", Icon::Members, if app.show_members { on } else { ic }, HDR_ICON).on_click(
                                cx.listener(|a, _, _, cx| {
                                    a.show_members = !a.show_members;
                                    cx.notify();
                                }),
                            ),
                        )
                })
                .child(
                    // search_ margin-inline-start 4px; searchBar_ 244×32, radius 8px,
                    // --input-background-default, 1px --input-border-default (#1063–#1073);
                    // editor_ padding 0 6px, 14px/22px medium; icon_ 24px box, padding 4px, 16px glyph.
                    div()
                        .ml(z(4.))
                        .w(z(244.))
                        .h(z(32.))
                        .rounded(z(8.))
                        .bg(tk(idx::INPUT_BACKGROUND_DEFAULT))
                        .border_1()
                        .border_color(tk(idx::INPUT_BORDER_DEFAULT))
                        .flex()
                        .items_center()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .px(z(6.))
                                .text_size(z(14.))
                                .line_height(z(22.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(tk(idx::INPUT_PLACEHOLDER_TEXT_DEFAULT))
                                .whitespace_nowrap()
                                .overflow_hidden()
                                .text_ellipsis()
                                .child(search_ph),
                        )
                        .child(div().size(z(24.)).p(z(4.)).flex().items_center().child(icon(Icon::Search, 16. * crate::theme::zoom(), tk(idx::ICON_DEFAULT)))),
                ),
        )
}

fn row(entity: &Entity<PhoenixApp>, ix: usize, cx: &mut App) -> AnyElement {
    let app = entity.read(cx);
    let Some(ch_id) = app.selected_channel else { return div().into_any_element() };
    let Some(entry) = app.messages.get(&ch_id) else { return div().into_any_element() };
    if ix == 0 {
        return row_wrapper().child(intro(app, ch_id, entry.reached_start, entry.loading)).into_any_element();
    }
    let i = ix - 1;
    let (Some(msg), Some(r)) = (entry.msgs.get(i), entry.rendered.get(i)) else {
        return div().into_any_element();
    };
    let el = message(app, entity, msg, r, entry, cx);
    if i + 1 == entry.msgs.len() {
        // scrollerSpacer_8AE1Qq height 40px under the last message, overlapped 16px by
        // the form_ (vanilla-guild-text-groups #2777 at y609, form #2779 at y633): 24px
        // visible. Our list already ends 9px above the composer (real compare
        // 20261123-153201: last line 15px lower than vanilla), so pad the remaining 15px.
        return row_wrapper().pb(z(15.)).child(el).into_any_element();
    }
    el
}

fn intro(app: &PhoenixApp, ch: Id, reached_start: bool, loading: bool) -> impl IntoElement {
    let channel = app.channels.get(&ch);
    let name = channel.map(|c| c.name.clone()).unwrap_or_default();
    if !reached_start {
        return div()
            .h(z(64.))
            .flex()
            .items_center()
            .justify_center()
            .text_size(z(14.))
            .text_color(c(Palette::TEXT_MUTED))
            .child(if loading { crate::t!("chat.loading_messages") } else { "".into() });
    }
    if let Some(chan) = channel.filter(|c| c.guild.is_none()) {
        let peer = chan.voice_users.first().and_then(|u| app.users.get(u));
        return div()
            .m(z(16.))
            .flex()
            .flex_col()
            .gap(z(8.))
            .child(avatar(peer, 80.))
            .child(
                div()
                    .text_size(z(32.))
                    .line_height(z(40.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(c(Palette::TEXT_HEADER))
                    .child(name.clone()),
            )
            .child(
                div()
                    .text_size(z(16.))
                    .text_color(tk(idx::TEXT_DEFAULT)) // container_ .description_ color
                    .child(crate::t!("chat.dm_start", name = name)),
            );
    }
    // container_ (welcome) margin: --custom-message-margin-horizontal; header_ margin 8px 0.
    div()
        .m(z(16.))
        .flex()
        .flex_col()
        .gap(z(8.))
        .child(
            div()
                .mt(z(16.)) // emptyChannelIcon_ margin-top
                .size(z(68.)) // emptyChannelIcon_ 68px
                .rounded_full()
                .bg(tk(idx::BACKGROUND_MOD_MUTED)) // emptyChannelIcon_ background-color
                .flex()
                .items_center()
                .justify_center()
                .child(icon(Icon::Hash, 42., gpui::white())),
        )
        .child(
            div()
                .text_size(z(32.))
                .line_height(z(40.))
                .font_weight(FontWeight::BOLD)
                .text_color(c(Palette::TEXT_HEADER))
                .child(crate::t!("chat.welcome", name = name)),
        )
        .child(
            div()
                .text_size(z(16.))
                .text_color(tk(idx::TEXT_DEFAULT)) // description_ color
                .child(crate::t!("chat.channel_start", name = name)),
        )
}

fn day_divider(label: SharedString) -> impl IntoElement {
    // divider_ (message list) margin-inline 1rem .875rem; .group-spacing-16
    // .divider_.hasContent_ margin 1.5rem top / .5rem bottom; divider_ border-top
    // --border-subtle; content_ 12px semibold, line-height 13px, padding 2px 4px,
    // radius 8px, --text-muted on --background-base-lower.
    div()
        .ml(z(16.))
        .mr(z(14.))
        .mt(z(24.))
        .mb(z(8.))
        .h(z(1.))
        .bg(tk(idx::BORDER_SUBTLE))
        .flex()
        .justify_center()
        .items_center()
        .child(
            div()
                .bg(tk(idx::BACKGROUND_BASE_LOWER))
                .rounded(z(8.))
                .px(z(4.))
                .py(z(2.))
                .text_size(z(12.))
                .line_height(z(13.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tk(idx::TEXT_MUTED))
                .child(label),
        )
}

/// "Monday, November 23, 2026 at 3:04 PM" (timestamp tooltip).
pub fn format_time_tooltip(ts: i64) -> SharedString {
    use chrono::{Local, TimeZone};
    Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|d| crate::i18n::full_datetime(d).into())
        .unwrap_or_default()
}

fn time_tooltip(ts: i64) -> impl Fn(&mut Window, &mut App) -> gpui::AnyView + 'static {
    crate::ui::tooltip::tip(format_time_tooltip(ts))
}

/// Server tag chiplet after a username: clanTagChiplet_ (--background-mod-strong, radius 4px,
/// padding 0 4px, .72rem, margin-inline-start 6px; 2px in reply lines) holding
/// chipletContainerInner_ (line-height 16px) with the guild badge and tag text.
fn server_tag_chip(tag: &(SharedString, Option<SharedString>), in_reply: bool) -> gpui::Div {
    div()
        .flex_none()
        // vanilla-dm #1270 (message): margin -2 0 0 6, box h 15; #1424 (reply): margin -2 4 0 2, opacity .64.
        .ml(z(if in_reply { 2. } else { 6. }))
        .when(in_reply, |d| d.mr(z(4.)))
        .h(z(15.))
        .px(z(4.))
        .rounded(z(4.))
        .bg(tk(idx::BACKGROUND_MOD_STRONG))
        .flex()
        .items_center()
        .gap(z(2.))
        .text_size(fz(11.52))
        .line_height(z(16.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(tk(idx::TEXT_DEFAULT))
        .when_some(tag.1.clone(), |d, u| d.child(img(u).size(z(if in_reply { 12. } else { 14. })).flex_none())) // #1192 12px / #1208 14px
        .child(tag.0.clone())
}

/// Discord's "APP" tag next to bot authors (vanilla-guild-text-groups botTagCozy_ /
/// botTagVerified_ / botText_: 15px tall, 46px wide for a verified "APP").
fn bot_tag(verified: bool) -> gpui::Div {
    div()
        .flex_none()
        .ml(z(4.)) // botTagCozy_ margin-inline-start .25rem
        .mt(z(1.6)) // botTagCozy_ position relative, top .1rem
        .px(z(4.4)) // .rem_.botTag_ padding 0 .275rem
        .h(z(15.)) // .rem_.botTag_ height .9375rem
        .rounded(z(4.)) // .rem_.botTag_ border-radius 4px
        .bg(tk(idx::BACKGROUND_BRAND)) // botTagRegular_
        .flex()
        .items_center()
        .text_size(z(12.8)) // botText_ font-size .8rem
        .line_height(z(15.))
        .font_weight(FontWeight::SEMIBOLD) // botText_ 600
        .text_color(gpui::white())
        .when(verified, |d| {
            // botTagVerified_ 16px, margin-inline-start -.2rem, fill --white
            d.child(gpui::svg().path(crate::icons::path("check")).flex_none().size(z(16.)).ml(z(-3.2)).text_color(gpui::white()))
        })
        .child(crate::t!("common.app"))
}

fn message(
    app: &PhoenixApp,
    entity: &Entity<PhoenixApp>,
    msg: &Message,
    r: &RenderedMessage,
    entry: &crate::app::ChannelMessages,
    cx: &App,
) -> AnyElement {
    let author = app.users.get(&msg.author);
    let name = msg.author_nick.clone().or_else(|| app.member_nick(msg.author)).or_else(|| author.map(|u| u.display_name.clone())).unwrap_or_else(|| crate::t!("common.unknown"));
    // username_ color: role colour, else --text-strong.
    let color = app.author_color(msg.author).map(c).unwrap_or_else(|| tk(idx::TEXT_STRONG));
    let id = msg.id;
    if let Some(sys) = system::render(app, entity, msg, r) {
        return row_wrapper().child(sys).into_any_element();
    }
    let compact = app.prefs.synced.compact;
    let editing = app.chat.edit.as_ref().is_some_and(|e| e.id == id);
    let replying = app.replying_to == Some(id);
    // mentioned_: content mentions, @everyone, or a pinging reply to one of my messages.
    let reply_pings_me = msg.reply_mentions_author
        && msg.reply_to.and_then(|rid| entry.index_of(rid)).is_some_and(|p| entry.msgs[p].author == app.me.id);
    let mentioned = r.mentions_me || msg.mention_everyone || reply_pings_me;
    let ts = msg.timestamp;

    let mut content = div().flex().flex_col().min_w_0().text_size(fz(16.)).text_color(c(Palette::TEXT_NORMAL));
    let last_text = r.blocks.iter().rposition(|b| matches!(b, RBlock::Text(_)));
    let n_blocks = r.blocks.len();
    // Compact: the absolute "[time] Name" header overlaps the first line by
    // `overflow` px; pad the first text line with thin spaces to clear it.
    let is_bot = author.is_some_and(|a| a.bot);
    let overflow = if compact { compact_header_overflow(&r.time_short, &name, is_bot, author.and_then(|u| u.tag.as_ref()).map(|t| t.0.as_ref()), cx) } else { 0. };
    if compact && !matches!(r.blocks.first(), Some(RBlock::Text(_))) {
        content = content.child(div().flex_none().h(fz(22.)));
    }
    for (bi, b) in r.blocks.iter().enumerate() {
        let edited_here = msg.edited && last_text == Some(bi) && bi + 1 == n_blocks;
        match b {
            RBlock::Text(t) if compact && bi == 0 => {
                content = content.child(rich(entity, id, bi, &indent_rich(t, overflow, cx), edited_here));
            }
            _ => content = content.child(block(entity, id, bi, b, edited_here)),
        }
    }
    if msg.edited && !last_text.is_some_and(|l| l + 1 == n_blocks) {
        content = content.child(edited_marker(ts));
    }
    // Two or more images/videos share one mosaic; files follow (nonVisualMediaItem_ margin-top 8px).
    let visual: Vec<&Attachment> = msg.attachments.iter().filter(|a| mosaic::is_visual(a)).collect();
    if visual.len() >= 2 {
        content = content.child(mosaic::render(&visual, cx));
        for a in msg.attachments.iter().filter(|a| !mosaic::is_visual(a)) {
            content = content.child(div().mt(z(4.)).child(attachment(a, cx)));
        }
    } else {
        for a in &msg.attachments {
            content = content.child(attachment(a, cx));
        }
    }
    for e in &msg.embeds {
        content = content.child(embed(e));
    }
    // Forward: the snapshot sits inside the quote frame; reactions/threads stay outside.
    if let Some(f) = &msg.forward {
        content = div().flex().flex_col().min_w_0().text_size(fz(16.)).text_color(c(Palette::TEXT_NORMAL)).child(forward::wrap(app, f, content));
    }
    content = content.children(crate::ui::components::render(app, entity, msg)).children(crate::ui::components::render_poll(app, entity, msg));
    if !msg.reactions.is_empty() {
        content = content.child(message_ext::reactions(app, entity, msg));
    }
    if let Some(t) = &msg.thread {
        content = content.child(message_ext::thread_chip(app, entity, t));
    }
    let content = match message_ext::edit_box(app, entity, msg) {
        Some(b) => b,
        None => content,
    };

    let reply = msg.reply_to.map(|rid| {
        let found = entry.index_of(rid);
        let (ru, preview, rnick) = match found {
            Some(p) => (app.users.get(&entry.msgs[p].author), entry.rendered[p].preview.clone(), entry.msgs[p].author_nick.clone().or_else(|| app.member_nick(entry.msgs[p].author)).or_else(|| {
                // referenced_message has no `member`: reuse the nick from any loaded message by that author.
                let a = entry.msgs[p].author;
                entry.msgs.iter().rev().filter(|m| m.author == a).find_map(|m| m.author_nick.clone())
            })),
            None => (None, crate::t!("chat.reply_deleted"), None),
        };
        // repliedMessage_: 14px, line-height --custom-message-reply-message-preview-line-height
        // (1.125rem), cozy margin-bottom --reply-spacing (4px). Compact: margin-inline-start
        // timestamp-width + meta-space - (compact-indent - 1rem) = -24px, padding-inline-start
        // --custom-message-reply-indent (.625rem).
        div()
            .relative()
            .h(z(if compact { 20. } else { 18. })) // cozy: vanilla-guild-text-groups #2624 repliedMessage_ 18px (line-height 1.125rem); compact #1185 20px
            .when(!compact, |d| d.mb(z(4.)))
            .when(compact, |d| d.ml(z(-24.)).pl(z(10.)))
            .flex()
            .items_center()
            .gap(z(4.)) // replyAvatar_ margin-inline-end --custom-message-meta-space
            .text_size(fz(14.))
            .line_height(z(18.))
            .text_color(tk(idx::TEXT_DEFAULT))
            .child(
                // messageSpine_:before — 2px --spine-default, radius 6px, top 50%, bottom 0,
                // from the avatar centre (avatar/2 + gutter, minus half the spine) to 4px
                // (--reply-spacing) before the reply. Compact: avatar = 2.25rem timestamp,
                // gutter = meta-space, ends at the reply indent.
                div()
                    .absolute()
                    .left(z(if compact { -23. } else { -41. })) // #1186 left -40px, margin-left -1px
                    .top(z(9.)) // #1186 top 10px, margin-top -1px
                    .w(z(if compact { 29. } else { 37. })) // #1186 37px
                    .h(z(if compact { 11. } else { 13. }))
                    .border_l_2()
                    .border_t_2()
                    .rounded_tl(z(6.))
                    .border_color(tk(idx::SPINE_DEFAULT)),
            )
            .when(!compact, |d| d.child(avatar(ru, 16.)))
            // vanilla-guild-text #1283: `botTagCompact_` sits before the replied username.
            .when(ru.is_some_and(|u| u.bot), |d| d.child(bot_tag(ru.is_some_and(|u| u.verified_bot)).ml_0()))
            .child(
                div()
                    .flex_none()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(ru.and_then(|u| app.author_color(u.id)).map(c).unwrap_or_else(|| tk(idx::TEXT_STRONG)))
                    .opacity(0.64)
                    .child(SharedString::from(format!(
                        "{}{}",
                        if msg.reply_mentions_author { "@" } else { "" }, // repliedMessage_ username: "@" only when the reply pinged
                        rnick.map(|n| n.to_string()).or_else(|| ru.map(|u| u.display_name.to_string())).unwrap_or_else(|| crate::t!("common.unknown").to_string())
                    ))),
            )
            .when_some(ru.and_then(|u| u.tag.clone()), |d, tg| d.child(server_tag_chip(&tg, true).opacity(0.64)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_color(tk(idx::TEXT_DEFAULT)) // repliedTextContent_ color: inherit
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    // #1291 `repliedTextPlaceholder_`: no text content (embeds, attachments only).
                    .when(found.is_none() || preview.is_empty(), |d| d.italic())
                    .child(if found.is_some() && preview.is_empty() { crate::t!("chat.reply_click_to_see") } else { preview }),
            )
    });

    let author_name = crate::fonts::styled_name(div(), author, false)
        .flex_none()
        .text_size(fz(16.))
        .line_height(fz(22.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(color)
        .id(("msg-author", id.0))
        .cursor_pointer()
        .hover(|s| s.underline())
        .on_click(message_ext::open_author(entity, msg))
        .child(name);

    let base = div()
        .id(("msg", id.0))
        .group("msg")
        .on_mouse_down(gpui::MouseButton::Right, message_ext::on_right_click(entity, msg))
        .relative()
        .pr(z(space::xl())) // message_ padding-inline-end --space-xl (#1184)
        .map(|d| {
            // message_:hover / .selected_ --message-background-hover; .mentioned_ and
            // .replying_ have their own backgrounds, hover shades and a 2px bar
            // (--icon-feedback-warning / --brand-500) drawn over the row.
            let (bg, hover, bar) = if replying {
                (idx::MESSAGE_HIGHLIGHT_BACKGROUND_DEFAULT, idx::BRAND_10A, Some(idx::BRAND_500))
            } else if mentioned {
                (idx::MESSAGE_MENTIONED_BACKGROUND_DEFAULT, idx::MESSAGE_MENTIONED_BACKGROUND_HOVER, Some(idx::ICON_FEEDBACK_WARNING))
            } else {
                (usize::MAX, idx::MESSAGE_BACKGROUND_HOVER, None)
            };
            let selected = editing || app.chat.menu.is_some_and(|m| m.msg == id);
            let d = match (bar, selected) {
                (_, true) => d.bg(tk(hover)),
                (Some(_), false) => d.bg(tk(bg)),
                (None, false) => d,
            };
            d.hover(move |s| s.bg(tk(hover))).when_some(bar, |d, b| {
                d.child(div().absolute().left_0().top_0().bottom_0().w(z(2.)).bg(tk(b)))
            })
        });

    let body = if compact {
        // Compact (Discord's IRC-like layout): "[time] Name" sits inline at the
        // start of the first line and wrapped lines are indented to 80 px,
        // like the web client's negative text-indent.
        base.pl(z(80.))
            .py(z(2.))
            .when_some(reply, |d, reply| d.child(reply))
            .child(
                div()
                    .absolute()
                    .left(z(16.))
                    .top(z(if msg.reply_to.is_some() { 24. } else { 2. }))
                    .h(fz(22.))
                    .flex()
                    .items_center()
                    .child(
                        // .compact_ .timestamp_: .6875rem, --text-muted, margin-inline-end
                        // --custom-message-meta-space.
                        div()
                            .id(("msg-time", id.0))
                            .flex_none()
                            .mr(z(4.))
                            .text_size(z(11.))
                            .text_color(tk(idx::TEXT_MUTED))
                            .tooltip(time_tooltip(ts))
                            .child(r.time_short.clone()),
                    )
                    .child(author_name)
                    .when_some(author.and_then(|u| u.tag.clone()), |d, tg| d.child(server_tag_chip(&tg, false)))
                    .when(is_bot, |d| d.child(bot_tag(author.is_some_and(|u| u.verified_bot)))),
            )
            .child(content)
    } else {
        base.pl(z(msg_left())) // --custom-message-margin-left-content-cozy (#1184 80px)
            .py(z(2.))
            .when_some(reply, |d, reply| d.child(reply))
            .when(r.group_start, |d| {
                d.child(
                    div()
                        .absolute()
                        // avatar_: inset-inline-start --custom-message-margin-horizontal,
                        // margin-top 4px - cozy vertical padding (so 4px below the row top,
                        // after the reply line when there is one).
                        .left(z(space::md())) // #1200 left 20px
                        // #1200: 2px row padding + 18px reply + 4px --reply-spacing + 2px margin-top.
                        .top(z(if msg.reply_to.is_some() { 26. } else { 4. })) // #2640: 2 + 18 reply + 4 + 2
                        .id(("msg-avatar", id.0))
                        .cursor_pointer()
                        .on_click(message_ext::open_author(entity, msg))
                        .child(avatar(author, m::AVATAR)),
                )
                .when_some(author.and_then(|u| u.decoration.clone()), |d, deco| {
                    // avatarDecoration_2i8Bfm: --custom-message-avatar-decoration-size = avatar × 1.2
                    // (--decoration-to-avatar-ratio), inset-inline-start margin-horizontal − (deco−avatar)/2,
                    // margin-top 4px − 2px − (deco−avatar)/2, translateY(1px), z-index 1, no pointer events.
                    let s = m::AVATAR * 1.2;
                    let off = (s - m::AVATAR) / 2.;
                    d.child(
                        gpui::img(deco)
                            .absolute()
                            .size(z(s))
                            .left(z(space::md() - off))
                            .top(z(if msg.reply_to.is_some() { 26. } else { 4. } - off)), // #2641 avatarDecoration_ 4px above the avatar
                    )
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .h(fz(22.))
                        .child(author_name)
                        .when_some(author.and_then(|u| u.tag.clone()), |d, tg| d.child(server_tag_chip(&tg, false)))
                        .when(is_bot, |d| d.child(bot_tag(author.is_some_and(|u| u.verified_bot))))
                        .child(
                            // .cozy_ .timestamp_: .75rem, --chat-text-muted, medium;
                            // timestampInline_ margin-inline-start .25rem.
                            div()
                                .id(("msg-time", id.0))
                                .ml(z(4.))
                                .text_size(z(12.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(tk(idx::CHAT_TEXT_MUTED))
                                .tooltip(time_tooltip(ts))
                                .child(r.time_full.clone()),
                        ),
                )
            })
            .when(!r.group_start, |d| {
                d.child(
                    div()
                        .id(("msg-time", id.0))
                        .absolute()
                        .left_0()
                        .top(z(2.))
                        .w(z(56.))
                        .h(fz(22.))
                        .flex()
                        .items_center()
                        .justify_end()
                        // .cozy_ .timestamp_.alt_: .6875rem, --text-muted, width 56px.
                        .text_size(z(11.))
                        .text_color(gpui::transparent_black())
                        .group_hover("msg", |s| s.text_color(tk(idx::TEXT_MUTED)))
                        .tooltip(time_tooltip(ts))
                        .child(r.time_short.clone()),
                )
            })
            .child(content)
    }
    .child(message_ext::toolbar(app, entity, msg, app.chat.shift_held));
    let new_marker = app.chat.unread_marker.is_some_and(|(_, m)| m == id);
    // The list measures each row with `layout_as_root`, which ignores the
    // root's own margins: group spacing must be padding on a wrapper or rows
    // get painted over each other.
    let gap = if !r.group_start { 0. } else if compact { 4. } else { 17. };
    row_wrapper()
        .when_some(r.day_divider.clone(), |d, label| d.child(day_divider(label)))
        .when(new_marker, |d| d.child(message_ext::new_divider()))
        .child(div().pt(z(gap)).child(body))
        .into_any_element()
}

/// Root of a list row: a flex column so children's margins stay inside the
/// measured box (no margin collapsing through the root).
fn row_wrapper() -> gpui::Div {
    div().w_full().flex().flex_col()
}

/// Width in px of `s` drawn in `font` at `size` (no kerning; close enough
/// for layout offsets).
fn text_width(cx: &App, font: &gpui::Font, size: gpui::Pixels, s: &str) -> f32 {
    let ts = cx.text_system();
    let id = ts.resolve_font(font);
    s.chars().map(|ch| ts.advance(id, size, ch).map(|a| f32::from(a.width)).unwrap_or(0.)).sum()
}

/// How far the compact "[time] Name " header runs past the 80 px content
/// indent, i.e. the first-line text indent.
fn compact_header_overflow(time: &str, name: &str, bot: bool, tag: Option<&str>, cx: &App) -> f32 {
    let mut name_font = crate::richtext::ui_font();
    name_font.weight = FontWeight::MEDIUM;
    let mut w = text_width(cx, &crate::richtext::ui_font(), z(11.), time) + f32::from(z(4.));
    w += text_width(cx, &name_font, fz(16.), name);
    if bot {
        let mut f = name_font.clone();
        f.weight = FontWeight::SEMIBOLD;
        w += text_width(cx, &f, z(12.8), "APP") + f32::from(z(12.8));
    }
    if let Some(tg) = tag {
        let mut f = name_font.clone();
        f.weight = FontWeight::SEMIBOLD;
        w += text_width(cx, &f, fz(11.52), tg) + f32::from(z(14.)); // ml 6 + px 4*2
    }
    w += f32::from(z(4.));
    (f32::from(z(16.)) + w - f32::from(z(80.))).max(0.)
}

/// Prefix `t` with transparent thin spaces at least `width` px wide.
fn indent_rich(t: &RichText, width: f32, cx: &App) -> RichText {
    if width <= 0. {
        return t.clone();
    }
    const SP: char = '\u{2009}';
    let font = crate::richtext::ui_font();
    let adv = text_width(cx, &font, fz(16.), "\u{2009}").max(1.);
    let n = (width / adv).ceil() as usize;
    let prefix: String = std::iter::repeat_n(SP, n).collect();
    let shift = prefix.len();
    let mut out = t.clone();
    out.text = format!("{prefix}{}", t.text).into();
    out.runs.insert(
        0,
        gpui::TextRun { len: shift, font, color: gpui::transparent_black(), background_color: None, underline: None, strikethrough: None },
    );
    for r in &mut out.click_ranges {
        *r = r.start + shift..r.end + shift;
    }
    for (r, _) in &mut out.emoji {
        *r = r.start + shift..r.end + shift;
    }
    out
}

fn edited_marker(ts: i64) -> impl IntoElement {
    div()
        .id(("edited", ts as u64))
        .text_size(z(10.)) // edited_ font-size .625rem
        .line_height(z(10.)) // edited_ line-height 1
        .font_weight(FontWeight::NORMAL) // edited_ --font-weight-normal
        .text_color(c(Palette::TEXT_MUTED))
        .child(crate::t!("chat.edited"))
}

/// Append Discord's small muted " (edited)" after the last line of text.
fn with_edited(t: &RichText) -> (SharedString, Vec<gpui::TextRun>) {
    let mut text = t.text.to_string();
    text.push_str(&crate::t!("chat.edited_suffix"));
    let mut runs = t.runs.clone();
    let mut f = crate::richtext::ui_font();
    f.weight = FontWeight::NORMAL;
    runs.push(gpui::TextRun {
        len: crate::t!("chat.edited_suffix").len(),
        font: f,
        color: c(Palette::TEXT_MUTED),
        background_color: None,
        underline: None,
        strikethrough: None,
    });
    (text.into(), runs)
}

fn rich(entity: &Entity<PhoenixApp>, msg: Id, bi: usize, t: &RichText, edited: bool) -> AnyElement {
    if !t.emoji.is_empty() {
        let el = message_ext::rich_with_emoji(entity, msg, bi, t);
        if edited {
            return div().flex().flex_wrap().items_end().child(el).child(div().ml(z(4.)).mb(z(5.)).child(edited_marker(msg.0 as i64))).into_any_element();
        }
        return el;
    }
    // StyledText runs cannot change size, so a short one-line message gets the real 10px
    // `edited_` span next to it (baseline-aligned: 22px line, 16px text); longer text keeps the
    // inline suffix so it follows the last wrapped line.
    if edited && t.click_ranges.is_empty() && !t.text.contains('\n') && t.text.chars().count() <= 60 {
        let plain = div().line_height(gpui::relative(1.375)).child(StyledText::new(t.text.clone()).with_runs(t.runs.clone()));
        return div()
            .flex()
            .flex_wrap()
            .items_end()
            .child(plain)
            .child(div().ml(z(4.)).mb(z(5.)).child(edited_marker(msg.0 as i64))) // " " text node + baseline
            .into_any_element();
    }
    let (text, runs) = if edited { with_edited(t) } else { (t.text.clone(), t.runs.clone()) };
    let styled = StyledText::new(text).with_runs(runs);
    let lh = gpui::relative(1.375);
    if t.click_ranges.is_empty() {
        return div().line_height(lh).child(styled).into_any_element();
    }
    let targets = t.click_targets.clone();
    let e = entity.clone();
    div()
        .line_height(lh)
        .child(
            InteractiveText::new(("rt", (msg.0 << 8) | bi as u64), styled).on_click(
                t.click_ranges.clone(),
                move |ix, _w, cx| match &targets[ix] {
                    ClickTarget::Link(url) => cx.open_url(url),
                    ClickTarget::Spoiler(sp) => {
                        let sp = *sp;
                        e.update(cx, |a, cx| a.reveal_spoiler(msg, sp, cx));
                    }
                    ClickTarget::Mention(_) => {}
                },
            ),
        )
        .into_any_element()
}

fn block(entity: &Entity<PhoenixApp>, msg: Id, bi: usize, b: &RBlock, edited: bool) -> AnyElement {
    match b {
        RBlock::Text(t) => rich(entity, msg, bi, t, edited),
        // blockquoteContainer_: margin-block --space-4; blockquoteDivider_ 4px, radius 4px,
        // --spine-default; blockquote padding-inline 12px 8px, max-width 90%.
        RBlock::Quote(t) => div()
            .flex()
            .my(z(4.))
            .max_w(gpui::relative(0.9))
            .child(div().w(z(4.)).rounded(z(4.)).bg(tk(idx::SPINE_DEFAULT)).flex_none())
            .child(div().flex_1().min_w_0().pl(z(12.)).pr(z(8.)).child(rich(entity, msg, bi, t, false)))
            .into_any_element(),
        RBlock::Heading(level, t) => {
            let size = match level {
                1 => 24.,
                2 => 20.,
                _ => 16.,
            };
            // markup_ h1 1.5rem / h2 1.25rem / h3 1rem, margin 16px 0 8px; first child
            // margin-top 8px (h1, h2) or 4px (h3).
            let first = if *level <= 2 { 8. } else { 4. };
            div()
                .mt(z(if bi == 0 { first } else { 16. }))
                .mb(z(8.))
                .text_size(fz(size))
                .child(rich(entity, msg, bi, t, edited))
                .into_any_element()
        }
        // markup_ small: .875rem, line-height 1.20313rem, --text-subtle.
        RBlock::Subtext(t) => div()
            .text_size(fz(14.))
            .line_height(fz(19.25))
            .text_color(tk(idx::TEXT_SUBTLE))
            .child(rich(entity, msg, bi, t, edited))
            .into_any_element(),
        RBlock::ListItem { ordered, depth, text } => {
            let marker: SharedString = match ordered {
                Some(n) => format!("{n}.").into(),
                None if *depth == 0 => "\u{2022}".into(),
                None => "\u{25E6}".into(),
            };
            div()
                .flex()
                .ml(z(16. * (*depth as f32 + 1.) - 16.))
                .mb(z(4.))
                .child(
                    div()
                        .flex_none()
                        .w(z(if ordered.is_some() { 24. } else { 16. }))
                        .pr(z(4.))
                        .flex()
                        .justify_end()
                        .line_height(gpui::relative(1.375))
                        .child(marker),
                )
                .child(div().flex_1().min_w_0().child(rich(entity, msg, bi, text, edited)))
                .into_any_element()
        }
        // markup_ pre: margin-top 6px, radius 4px, max-width 90%; markup_ code: .875rem /
        // 1.125rem, 1px --border-normal; .hljs: padding .5em, --background-base-lower,
        // --text-code.
        RBlock::Code { code, runs, .. } => div()
            .mt(z(6.))
            .max_w(gpui::relative(0.9))
            .p(fz(7.))
            .rounded(z(4.))
            .bg(tk(idx::BACKGROUND_BASE_LOWER))
            .border_1()
            .border_color(tk(idx::BORDER_NORMAL))
            .font_family(FONT_MONO())
            .text_size(fz(14.))
            .line_height(fz(18.))
            .text_color(tk(idx::TEXT_CODE))
            .child(StyledText::new(code.clone()).with_runs(runs.clone()))
            .into_any_element(),
    }
}

fn human_size(n: u64) -> SharedString {
    if n > 1 << 20 {
        format!("{:.2} MB", n as f64 / (1 << 20) as f64).into()
    } else {
        format!("{:.2} KB", n as f64 / 1024.).into()
    }
}

/// Clickable image (opens the viewer). `w x h` is the drawn size; the proxy
/// is asked for 2x for HiDPI.
fn media_image(url: &str, original: SharedString, filename: SharedString, (ow, oh): (u32, u32), (w, h): (f32, f32)) -> gpui::Stateful<gpui::Img> {
    let src = media::resized(url, (w * 2.) as u32, (h * 2.) as u32);
    let viewer = media::ViewerImage {
        url: media::resized(url, ow.min(2048), oh.min(2048)),
        original: if original.is_empty() { url.to_owned().into() } else { original },
        filename,
        width: ow,
        height: oh,
    };
    let mut im = img(src.clone());
    // Bounded LRU instead of the window's keep-forever asset cache.
    if let Some(cache) = media::lru::chat() {
        im = im.image_cache(&cache);
    }
    im.id(gpui::ElementId::Name(src))
        .w(z(w))
        .h(z(h))
        .rounded(z(8.))
        .object_fit(gpui::ObjectFit::Cover)
        .cursor_pointer()
        .with_loading(|| div().size_full().rounded(z(8.)).bg(c(Palette::BG_SECONDARY)).into_any_element())
        .on_click(move |_, _, cx| media::open_viewer(viewer.clone(), cx))
}

/// Spoiler cover: blurred (tiny upscaled proxy image) + dark veil + pill.
fn spoiler_cover(key: SharedString, url: &str, (w, h): (f32, f32), cx: &App) -> Option<AnyElement> {
    if media::is_revealed(&key, cx) {
        return None;
    }
    let tiny = media::resized(url, ((w / 24.) as u32).max(2), ((h / 24.) as u32).max(2));
    Some(
        div()
            .id(gpui::ElementId::Name(format!("spoiler-{key}").into()))
            .relative()
            .w(z(w))
            .h(z(h))
            .rounded(z(8.))
            .overflow_hidden()
            .cursor_pointer()
            .bg(c(Palette::BG_SECONDARY))
            .child(img(tiny).absolute().inset_0().size_full().object_fit(gpui::ObjectFit::Cover))
            .child(
                div().absolute().inset_0().bg(ca(0x00000099)).flex().items_center().justify_center().child(
                    div()
                        .px(z(12.))
                        .py(z(6.))
                        .rounded_full()
                        .bg(ca(0x000000CC))
                        .text_size(z(15.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(gpui::white())
                        .child(crate::t!("chat.spoiler")),
                ),
            )
            .on_click(move |_, _, cx| media::reveal(key.clone(), cx))
            .into_any_element(),
    )
}

fn attachment(a: &Attachment, cx: &App) -> AnyElement {
    let remote = a.url.starts_with("https://");
    let spoiler = a.filename.starts_with("SPOILER_");
    match a.kind {
        AttachmentKind::Sticker(fmt) => {
            let name = a.filename.clone();
            return match media::sticker_url(0, fmt).and(Some(a.url.clone())).filter(|u| !u.is_empty()) {
                Some(u) => img(u)
                    .mt(z(4.))
                    .size(z(160.))
                    .object_fit(gpui::ObjectFit::Contain)
                    .with_fallback(move || sticker_name(name.clone()))
                    .into_any_element(),
                None => div().mt(z(4.)).child(sticker_name(name)).into_any_element(),
            };
        }
        AttachmentKind::Image | AttachmentKind::Video if remote => {
            let size = media::fit(a.width, a.height, media::MAX_W, media::MAX_H);
            if spoiler && let Some(cover) = spoiler_cover(a.url.clone(), &a.url, size, cx) {
                return div().mt(z(4.)).child(cover).into_any_element();
            }
            if a.kind == AttachmentKind::Video {
                let poster = media::video_poster(&a.url, (size.0 * 2.) as u32, (size.1 * 2.) as u32);
                let original = if a.original.is_empty() { a.url.clone() } else { a.original.clone() };
                return div()
                    .id(gpui::ElementId::Name(format!("video-{}", a.url).into()))
                    .mt(z(4.))
                    .relative()
                    .w(z(size.0))
                    .h(z(size.1))
                    .rounded(z(8.))
                    .overflow_hidden()
                    .bg(gpui::black())
                    .cursor_pointer()
                    .child(img(poster).size_full().object_fit(gpui::ObjectFit::Cover))
                    .child(
                        div().absolute().inset_0().flex().items_center().justify_center().child(
                            div()
                                .size(z(48.))
                                .rounded_full()
                                .bg(ca(0x000000B3))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(gpui::white())
                                .text_size(z(20.))
                                .child("\u{25B6}"),
                        ),
                    )
                    .on_click(move |_, _, cx| cx.open_url(&original))
                    .into_any_element();
            }
            return div()
                .mt(z(4.))
                .child(media_image(&a.url, a.original.clone(), a.filename.clone(), (a.width, a.height), size))
                .into_any_element();
        }
        _ => {}
    }
    if a.is_image {
        let (w, h) = media::fit(a.width, a.height, media::MAX_W, media::MAX_H);
        return div()
            .mt(z(4.))
            .w(z(w))
            .h(z(h))
            .rounded(z(8.))
            .bg(c(Palette::BG_SECONDARY))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(z(4.))
            .child(icon(Icon::Image, 48., c(Palette::INTERACTIVE_MUTED)))
            .child(div().text_size(z(12.)).text_color(c(Palette::TEXT_MUTED)).child(a.filename.clone()))
            .into_any_element();
    }
    let url = a.url.clone();
    div()
        .id(gpui::ElementId::Name(format!("file-{}", a.url).into()))
        .mt(z(4.))
        .w(z(432.))
        .p(z(16.))
        .rounded(z(8.))
        .bg(c(Palette::BG_SECONDARY))
        .border_1()
        .border_color(c(Palette::BG_TERTIARY))
        .flex()
        .items_center()
        .gap(z(8.))
        .child(icon(Icon::File, 40., c(Palette::BLURPLE)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(div().text_size(z(16.)).text_color(c(Palette::TEXT_LINK)).text_ellipsis().child(a.filename.clone()))
                .child(div().text_size(z(12.)).text_color(c(Palette::TEXT_MUTED)).child(human_size(a.size))),
        )
        .child(icon(Icon::Download, 24., c(Palette::INTERACTIVE)))
        .when(remote, |d| d.cursor_pointer().on_click(move |_, _, cx| cx.open_url(&url)))
        .into_any_element()
}

fn sticker_name(name: SharedString) -> AnyElement {
    div()
        .px(z(8.))
        .py(z(4.))
        .rounded(z(4.))
        .bg(c(Palette::BG_SECONDARY))
        .text_size(z(14.))
        .text_color(c(Palette::TEXT_MUTED))
        .child(crate::t!("chat.sticker", name = name))
        .into_any_element()
}

/// Embed description / field values: Discord renders them as markdown.
/// Parsed embed markdown, keyed by source text and style generation, so
/// visible embeds are not re-parsed on every frame. Cleared when full.
fn embed_blocks(src: &str) -> std::rc::Rc<[RBlock]> {
    use std::{cell::RefCell, collections::HashMap, rc::Rc};
    type EmbedCache = (u32, HashMap<SharedString, Rc<[RBlock]>>);
    thread_local! {
        static CACHE: RefCell<EmbedCache> = RefCell::default();
    }
    CACHE.with_borrow_mut(|(generation, map)| {
        let g = crate::theme::style_generation();
        if *generation != g || map.len() >= 512 {
            *generation = g;
            map.clear();
        }
        if let Some(b) = map.get(src) {
            return b.clone();
        }
        let parsed = crate::markdown::parse(src, &crate::markdown::NoResolve);
        let blocks: Rc<[RBlock]> = crate::richtext::render_blocks(&parsed, &|_| true, c(Palette::TEXT_NORMAL)).into();
        map.insert(SharedString::from(src.to_owned()), blocks.clone());
        blocks
    })
}

fn embed_markdown(src: &str, size: f32) -> AnyElement {
    let blocks = embed_blocks(src);
    let mut d = div().flex().flex_col().text_size(fz(size)).line_height(z(18.)).min_w_0();
    for b in blocks.iter().cloned() {
        d = d.child(match b {
            RBlock::Code { code, runs, .. } => div()
                .my(z(4.))
                .p(z(8.))
                .rounded(z(4.))
                .bg(c(Palette::BG_TERTIARY))
                .font_family(FONT_MONO())
                .text_size(z(12.))
                .line_height(z(16.))
                .child(StyledText::new(code).with_runs(runs))
                .into_any_element(),
            RBlock::Quote(t) => div()
                .flex()
                .child(div().w(z(4.)).rounded(z(4.)).bg(c(Palette::INTERACTIVE_MUTED)).flex_none())
                .child(div().pl(z(12.)).child(StyledText::new(t.text).with_runs(t.runs)))
                .into_any_element(),
            b => {
                let t = b.text().cloned().unwrap_or_else(|| RichText {
                    text: "".into(),
                    runs: vec![],
                    click_ranges: vec![],
                    click_targets: vec![],
                    emoji: vec![],
                });
                let el = div().child(StyledText::new(t.text).with_runs(t.runs));
                match b {
                    RBlock::Heading(..) => el.font_weight(FontWeight::BOLD).text_size(z(size + 2.)),
                    RBlock::Subtext(_) => el.text_size(z(12.)).text_color(c(Palette::TEXT_MUTED)),
                    RBlock::ListItem { .. } => div().flex().child(div().w(z(16.)).child("\u{2022}")).child(el),
                    _ => el,
                }
                .into_any_element()
            }
        });
    }
    d.into_any_element()
}

fn embed(e: &Embed) -> AnyElement {
    let has_text = e.title.is_some() || e.description.is_some() || e.author.is_some() || !e.fields.is_empty();
    let media_only = matches!(e.kind.as_deref(), Some("image" | "gifv")) && !has_text;
    // Image-only embeds (links to pictures/gifs) render bare, like attachments.
    if media_only && let Some(m) = e.thumbnail.as_ref().or(e.image.as_ref()) {
        let size = media::fit(m.width, m.height, media::MAX_W, media::MAX_H);
        let original = e.url.clone().unwrap_or_else(|| m.url.clone());
        return div()
            .mt(z(4.))
            .child(media_image(&m.url, original, "".into(), (m.width, m.height), size))
            .into_any_element();
    }
    // Small thumbnail on the right for article/link embeds, big image below otherwise.
    let side_thumb = e.image.is_none() && e.kind.as_deref() != Some("video");
    let thumb = e.thumbnail.clone().filter(|_| side_thumb);
    let big = e.image.clone().or_else(|| e.thumbnail.clone().filter(|_| !side_thumb));

    // Fields: inline ones share a row (up to 3, or 2 with a side thumbnail).
    let per_row = if thumb.is_some() { 2 } else { 3 };
    let mut rows: Vec<Vec<&crate::model::EmbedField>> = Vec::new();
    for f in &e.fields {
        match rows.last_mut() {
            Some(r) if f.inline && r.len() < per_row && r.iter().all(|x| x.inline) => r.push(f),
            _ => rows.push(vec![f]),
        }
    }
    let fields = (!rows.is_empty()).then(|| {
        let mut grid = div().flex().flex_col().gap(z(8.));
        for r in rows {
            let mut row = div().flex().gap(z(8.));
            for f in r {
                row = row.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .mb(z(2.))
                                .text_size(z(14.))
                                .line_height(z(18.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(c(Palette::TEXT_HEADER))
                                .child(f.name.clone()),
                        )
                        .child(embed_markdown(&f.value, 14.)),
                );
            }
            grid = grid.child(row);
        }
        grid
    });

    let footer = (e.footer.is_some() || e.timestamp.is_some()).then(|| {
        let mut parts: Vec<String> = Vec::new();
        if let Some(f) = &e.footer
            && !f.text.is_empty()
        {
            parts.push(f.text.to_string());
        }
        if let Some(ts) = e.timestamp {
            parts.push(crate::app::format_time_full(ts).to_string());
        }
        div()
            .flex()
            .items_center()
            .gap(z(8.))
            .when_some(e.footer.as_ref().and_then(|f| f.icon.clone()), |d, u| {
                d.child(img(media::resized(&u, 40, 40)).size(z(20.)).rounded_full().flex_none())
            })
            .child(
                div()
                    .text_size(z(12.))
                    .line_height(z(16.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c(Palette::TEXT_NORMAL))
                    .child(SharedString::from(parts.join(" \u{2022} "))),
            )
    });

    let author = e.author.clone().map(|a| {
        let url = a.url.clone();
        div()
            .id(gpui::ElementId::Name(format!("embed-author-{}", a.name).into()))
            .flex()
            .items_center()
            .gap(z(8.))
            .when_some(a.icon.clone(), |d, u| {
                d.child(img(media::resized(&u, 48, 48)).size(z(24.)).rounded_full().flex_none())
            })
            .child(
                div()
                    .text_size(z(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c(Palette::TEXT_HEADER))
                    .when(url.is_some(), |d| d.hover(|s| s.underline()))
                    .child(a.name.clone()),
            )
            .when_some(url, |d, u| d.cursor_pointer().on_click(move |_, _, cx| cx.open_url(&u)))
    });

    let title = e.title.clone().map(|t| {
        let url = e.url.clone();
        div()
            .id(gpui::ElementId::Name(format!("embed-title-{t}").into()))
            .text_size(z(16.))
            .line_height(z(22.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(c(if url.is_some() { Palette::TEXT_LINK } else { Palette::TEXT_HEADER }))
            .when(url.is_some(), |d| d.hover(|s| s.underline()))
            .child(t)
            .when_some(url, |d, u| d.cursor_pointer().on_click(move |_, _, cx| cx.open_url(&u)))
    });

    // embed_ radius 4px; gridContainer_ max-width 516px; embedFull_ --background-surface-high,
    // 1px --border-subtle, 4px start border (embed colour or --border-normal); grid_ padding
    // .125rem 1rem .5rem .75rem; children embedMargin_ margin-top 8px.
    div()
        .mt(z(4.))
        .max_w(z(516.))
        .flex()
        .rounded(z(4.))
        .overflow_hidden()
        .bg(tk(idx::BACKGROUND_SURFACE_HIGH))
        .border_1()
        .border_color(tk(idx::BORDER_SUBTLE))
        .child(div().w(z(4.)).flex_none().bg(e.color.map(c).unwrap_or_else(|| tk(idx::BORDER_NORMAL))))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .pt(z(2.))
                .pr(z(16.))
                .pb(z(16.))
                .pl(z(12.))
                .flex()
                .gap(z(16.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(z(8.))
                        .pt(z(8.))
                        .when_some(e.provider.clone(), |d, p| {
                            // embedProvider_ .embedLink_ --interactive-text-default
                            d.child(div().text_size(z(12.)).line_height(z(16.)).text_color(tk(idx::INTERACTIVE_TEXT_DEFAULT)).child(p))
                        })
                        .children(author)
                        .children(title)
                        .when_some(e.description.clone(), |d, t| d.child(embed_markdown(&t, 14.)))
                        .children(fields)
                        .when_some(big, |d, m| {
                            let size = media::fit(m.width, m.height, 400., 300.);
                            let original = m.url.clone();
                            // embedFull_ .embedMedia_ margin-top 16px, radius 4px.
                            d.child(div().mt(z(8.)).child(media_image(&m.url, original, "".into(), (m.width, m.height), size).rounded(z(4.))))
                        })
                        .children(footer),
                )
                .when_some(thumb, |d, m| {
                    let size = media::fit(m.width, m.height, 80., 80.);
                    let original = m.url.clone();
                    d.child(
                        div()
                            .flex_none()
                            .pt(z(8.))
                            .child(media_image(&m.url, original, "".into(), (m.width, m.height), size).rounded(z(4.))),
                    )
                }),
        )
        .into_any_element()
}
