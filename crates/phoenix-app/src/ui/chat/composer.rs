//! Message composer (moved out of `chat.rs`): reply bar with the mention
//! toggle, upload tray, upload progress, attach button, paste-image, typing
//! indicator.

use std::time::Duration;

use gpui::{
    Animation, AnimationExt, Context, FontWeight, IntoElement, KeyDownEvent, ParentElement, SharedString,
    Styled, Window, div, img, prelude::*, relative,
};
use gpui_component::input::Input;

use super::{
    icons::{ChatIcon, icon},
    state::EmojiTarget,
};
use crate::{
    app::PhoenixApp,
    assets::Icon,
    theme::{Palette, c, generated::idx, z},
    ui::icon_button,
};

fn human_size(n: u64) -> SharedString {
    if n >= 1 << 20 {
        format!("{:.2} MB", n as f64 / (1 << 20) as f64).into()
    } else {
        format!("{:.2} KB", n as f64 / 1024.).into()
    }
}

fn reply_bar(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<impl IntoElement + use<>> {
    let rid = app.replying_to?;
    let e = app.messages.get(&app.selected_channel?)?;
    let m = e.msgs.iter().find(|m| m.id == rid)?;
    let name = app.users.get(&m.author).map(|u| u.display_name.clone()).unwrap_or_else(|| crate::t!("common.unknown"));
    let own = m.author == app.me.id;
    let mention = app.chat.reply_mention;
    Some(
        // replyBar_: transparent (background-color unset), top radius 8px; text_ margin-inline-start
        // --space-16; closeButton_ / mentionButton_ padding 8px 12px; actions_ margin-end --space-4.
        div()
            .h(z(32.))
            .pl(z(16.))
            .pr(z(4.))
            .rounded_t(z(8.))
            .flex()
            .items_center()
            .text_size(z(14.))
            .text_color(c(Palette::TEXT_MUTED))
            .child(div().child(crate::t!("chat.replying_to")))
            .child(
                div()
                    .ml(z(4.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c(Palette::INTERACTIVE_HOVER))
                    .child(name),
            )
            .child(div().flex_1())
            .when(!own, |d| {
                d.child(
                    div()
                        .id("reply-mention")
                        .px(z(12.))
                        .h(z(32.))
                        .flex()
                        .items_center()
                        .gap(z(2.))
                        .cursor_pointer()
                        .font_weight(FontWeight::BOLD)
                        .text_color(c(if mention { Palette::TEXT_LINK } else { Palette::TEXT_MUTED }))
                        .child("@")
                        .child(if mention { crate::t!("chat.on") } else { crate::t!("chat.off") })
                        .on_click(cx.listener(|a, _, _, cx| {
                            a.chat.reply_mention = !a.chat.reply_mention;
                            cx.notify();
                        })),
                )
                .child(div().w(z(1.)).h(z(20.)).bg(super::tk(idx::BORDER_SUBTLE))) // separator_ 1x20
            })
            .child(
                icon_button("cancel-reply", Icon::Close, super::tk(idx::INTERACTIVE_TEXT_DEFAULT), 16.).px(z(12.)).on_click(cx.listener(
                    |a, _, _, cx| {
                        a.replying_to = None;
                        cx.notify();
                    },
                )),
            ),
    )
}

fn upload_tray(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<impl IntoElement + use<>> {
    if app.chat.uploads.is_empty() {
        return None;
    }
    let mut tray = div()
        .id("upload-tray")
        .p(z(16.))
        .pb(z(4.))
        .flex()
        .gap(z(24.))
        .overflow_x_scroll()
        .border_b_1()
        .border_color(c(Palette::DIVIDER));
    for (ix, f) in app.chat.uploads.iter().enumerate() {
        let preview = match &f.image {
            Some(i) => img(i.clone())
                .size_full()
                .rounded(z(3.))
                .object_fit(gpui::ObjectFit::Contain)
                .into_any_element(),
            None => icon(ChatIcon::Upload, 64., c(Palette::BLURPLE)).into_any_element(),
        };
        tray = tray.child(
            div()
                .relative()
                .flex_none()
                .w(z(216.))
                .h(z(216.))
                .p(z(8.))
                .rounded(z(4.))
                .bg(c(Palette::BG_SECONDARY))
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .when(f.spoiler, |d| d.opacity(0.3))
                        .child(preview),
                )
                .child(
                    div()
                        .mt(z(8.))
                        .text_size(z(14.))
                        .text_color(c(Palette::TEXT_NORMAL))
                        .text_ellipsis()
                        .child(f.name.clone()),
                )
                .child(
                    div()
                        .absolute()
                        .top(z(-12.))
                        .right(z(-12.))
                        .flex()
                        .rounded(z(4.))
                        .bg(c(Palette::BG_PRIMARY))
                        .border_1()
                        .border_color(c(Palette::HEADER_SHADOW))
                        .child(
                            div()
                                .id(("up-spoiler", ix))
                                .size(z(32.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .hover(|s| s.bg(c(0x3D3E45)))
                                .child(icon(
                                    ChatIcon::Eye,
                                    20.,
                                    c(if f.spoiler { Palette::BLURPLE } else { Palette::INTERACTIVE }),
                                ))
                                .on_click(cx.listener(move |a, _, _, cx| a.toggle_upload_spoiler(ix, cx))),
                        )
                        .child(
                            div()
                                .id(("up-remove", ix))
                                .size(z(32.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .hover(|s| s.bg(c(0x3D3E45)))
                                .child(icon(ChatIcon::Trash, 20., c(Palette::RED)))
                                .on_click(cx.listener(move |a, _, _, cx| a.remove_upload(ix, cx))),
                        ),
                ),
        );
    }
    Some(tray)
}

/// In-flight uploads: file name, size, animated progress bar.
fn sending(app: &PhoenixApp) -> impl IntoElement {
    let ch = app.selected_channel;
    let mut col = div().flex().flex_col().gap(z(8.));
    for s in app.chat.sending.iter().filter(|s| Some(s.channel) == ch) {
        col = col.child(
            div()
                .mx(z(16.))
                .p(z(12.))
                .rounded(z(8.))
                .bg(c(Palette::BG_SECONDARY))
                .flex()
                .items_center()
                .gap(z(12.))
                .child(icon(ChatIcon::Upload, 32., c(Palette::BLURPLE)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(z(6.))
                        .child(
                            div()
                                .flex()
                                .gap(z(8.))
                                .text_size(z(14.))
                                .child(div().text_color(c(Palette::TEXT_NORMAL)).text_ellipsis().child(s.label.clone()))
                                .child(div().text_color(c(Palette::TEXT_MUTED)).child(human_size(s.size))),
                        )
                        .child(
                            div().h(z(6.)).rounded(z(3.)).bg(c(Palette::INTERACTIVE_MUTED)).overflow_hidden().child(
                                div().h_full().rounded(z(3.)).bg(c(Palette::BLURPLE)).with_animation(
                                    ("upload-bar", s.key as usize),
                                    Animation::new(Duration::from_millis(1400)).repeat(),
                                    |d, t| d.w(relative(0.15 + 0.8 * t)),
                                ),
                            ),
                        ),
                ),
        );
    }
    col
}

pub fn render(app: &PhoenixApp, _window: &mut Window, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let ch = app.selected_channel.and_then(|c| app.channels.get(&c));
    let placeholder: SharedString = match ch {
        // Group DMs read "Message <name>" (no @), like threads.
        Some(c) if c.guild.is_none() && crate::ui::dm_profile_panel::is_group_dm(app, c.id) => crate::t!("chat.placeholder_thread", name = c.name),
        Some(c) if c.guild.is_none() => crate::t!("chat.placeholder_dm", name = c.name),
        Some(c) => crate::t!("chat.placeholder_channel", name = c.name),
        None => "".into(),
    };
    let ic = c(Palette::INTERACTIVE);
    let placeholder_empty = app.composer.read(cx).value().is_empty();
    // Discord shows the remaining count once you get near the 2000 limit.
    let remaining = 2000 - app.composer.read(cx).value().chars().count() as i64;
    let reply = reply_bar(app, cx);
    let has_banner = reply.is_some();
    let tray = upload_tray(app, cx);
    let typing_active = app.selected_channel.and_then(|c| app.typing.get(&c)).is_some_and(|v| !v.is_empty());
    div()
        .flex_none()
        .relative()
        .flex()
        .flex_col()
        .gap(z(8.))
        .child(sending(app))
        .children(super::slash::render(app, cx))
        .child(
            // form_ padding-inline --space-xs; channelTextArea_ --chat-background-default,
            // 1px --border-muted, radius --radius-sm, margin-bottom --custom-chat-input-margin-bottom.
            div().px(z(crate::theme::space::xs())).mb(z(if typing_active { 24. } else { crate::theme::space::xs() })).child( // #1432 form padding-inline --space-xs; #1435 margin-bottom --custom-chat-input-margin-bottom
                div()
                    .flex()
                    .flex_col()
                    .rounded(z(8.))
                    .border_1()
                    .border_color(super::tk(idx::BORDER_MUTED))
                    .bg(super::tk(idx::CHAT_BACKGROUND_DEFAULT))
                    .children(reply)
                    .children(tray)
                    .child(
                        div()
                            .id("composer")
                            .min_h(z(56.)) // #1437 56px (17px padding + 22px line)
                            .when(has_banner, |d| d.rounded_b(z(8.)))
                            .relative()
                            .when(remaining < 200, |d| {
                                d.child(
                                    div()
                                        .absolute()
                                        .right(z(16.))
                                        .bottom(z(-18.))
                                        .text_size(z(12.))
                                        .text_color(c(if remaining < 0 { Palette::RED } else { Palette::TEXT_MUTED }))
                                        .child(SharedString::from(remaining.to_string())),
                                )
                            })
                            .flex()
                            .items_center()
                            .capture_key_down(cx.listener(|a, ev: &KeyDownEvent, w, cx| {
                                let k = &ev.keystroke;
                                if k.modifiers.number_of_modifiers() == 0 && super::slash::on_key(a, &k.key, w, cx) {
                                    cx.stop_propagation();
                                    return;
                                }
                                let edit = k.key == "up" && k.modifiers.number_of_modifiers() == 0;
                                let paste = k.key == "v" && (k.modifiers.control || k.modifiers.platform);
                                if (edit && a.edit_last(w, cx)) || (paste && a.paste_image(cx)) {
                                    cx.stop_propagation();
                                }
                            }))
                            .child(
                                // inner_ padding-inline-start 15px; attachButton_ margin-inline
                                // -(space-xs + space-6) 10px around a 32px button.
                                // #1438 inner_ padding-inline-start 15px; #1439 attachWrapper_ padding 12px 14px;
                                // #1440 attachButton_ 32px, radius 8px, margin 0 10px 0 -16px, PlusLargeIcon 20px.
                                div().pl(z(15.)).child(div().px(z(14.)).child(
                                    icon_button("attach", Icon::Plus, ic, 20.)
                                        .size(z(32.))
                                        .rounded(z(8.))
                                        .ml(z(-16.))
                                        .mr(z(10.))
                                        .on_click(cx.listener(|a, _, _, cx| a.prompt_attach(cx))),
                                )),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .relative()
                                    .text_size(z(16.))
                                    .text_color(c(Palette::TEXT_NORMAL))
                                    .child(Input::new(&app.composer).appearance(false).bordered(false))
                                    .when(placeholder_empty, |d| {
                                        d.child(
                                            div()
                                                .absolute()
                                                .top_0()
                                                .bottom_0()
                                                .left(z(2.))
                                                .flex()
                                                .items_center()
                                                .text_color(super::tk(idx::CHANNEL_TEXT_AREA_PLACEHOLDER)) // #1445 placeholder_
                                                .child(placeholder),
                                        )
                                    }),
                            )
                            .child(
                                // buttons_ gap --space-8, margin-inline-end 6px; button_ 32px;
                                // icons --chat-input-icon-size (20px).
                                // #1452 buttons_ gap 8px, 12px to the right edge (#1437 ends 12px early);
                                // order: gift, GIF, sticker, emoji, app launcher (#1457–#1480).
                                div()
                                    .mr(z(12.))
                                    .flex()
                                    .gap(z(8.))
                                    .items_center()
                                    .child(icon_button("gift", Icon::Gift, ic, 20.).size(z(32.)).rounded(z(8.)))
                                    .child(icon_button("gif", Icon::Gif, ic, 20.).size(z(32.)).rounded(z(8.)).on_click(cx.listener(|a, _, w, cx| crate::ui::pickers::toggle(a, crate::ui::pickers::Tab::Gif, crate::ui::pickers::EmojiTarget::Composer, w, cx))))
                                    .child(icon_button("sticker", Icon::Sticker, ic, 20.).size(z(32.)).rounded(z(8.)).on_click(cx.listener(|a, _, w, cx| crate::ui::pickers::toggle(a, crate::ui::pickers::Tab::Sticker, crate::ui::pickers::EmojiTarget::Composer, w, cx))))
                                    .child(icon_button("emoji", Icon::Emoji, ic, 20.).size(z(32.)).rounded(z(8.)).on_click(cx.listener(
                                        |a, _, _, cx| a.open_emoji_picker(EmojiTarget::Composer, cx),
                                    )))
                                    .child(
                                        // app-launcher-entrypoint: button_ 32px radius 8px, 20px glyph (#1479).
                                        div()
                                            .id("apps")
                                            .size(z(32.))
                                            .rounded(z(8.))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor_pointer()
                                            .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
                                            .child(gpui::svg().path(crate::icons::path("apps")).size(z(20.)).flex_none().text_color(ic)),
                                    ),
                            ),
                    ),
            ),
        )
        // base_U9Tvvz: absolute, bottom 1px, height 24px, inset-inline --space-16 16px.
        .child(div().absolute().bottom(z(1.)).left(z(16.)).right(z(16.)).child(typing(app)))
}

/// "**Name** is typing…" with bouncing dots, from real TYPING_START events.
fn typing(app: &PhoenixApp) -> impl IntoElement {
    let names: Vec<SharedString> = app
        .selected_channel
        .and_then(|c| app.typing.get(&c))
        .map(|v| v.iter().filter_map(|(u, _)| app.users.get(u).map(|u| u.display_name.clone())).collect())
        .unwrap_or_default();
    let bold = |s: SharedString| div().font_weight(FontWeight::BOLD).child(s);
    let mut line = div().flex().items_center().gap(z(0.)).min_w_0();
    match names.as_slice() {
        [] => {}
        [a] => line = line.child(bold(a.clone())).child(crate::t!("chat.typing_suffix_one")),
        [a, b] => line = line.child(bold(a.clone())).child(crate::t!("chat.typing_and")).child(bold(b.clone())).child(crate::t!("chat.typing_suffix_many")),
        [a, b, c3] => {
            line = line
                .child(bold(a.clone()))
                .child(", ")
                .child(bold(b.clone()))
                .child(crate::t!("chat.typing_comma_and"))
                .child(bold(c3.clone()))
                .child(crate::t!("chat.typing_suffix_many"))
        }
        _ => line = line.child(crate::t!("chat.typing_many_ellipsis")),
    }
    let active = !names.is_empty();
    let dot = |i: usize| {
        div().size(z(7.)).rounded_full().bg(super::tk(idx::TEXT_DEFAULT)).with_animation(
            ("typing-dot", i),
            Animation::new(Duration::from_millis(1200)).repeat(),
            move |d, t| {
                let phase = ((t * 3. - i as f32 * 0.5).rem_euclid(3.) / 3. * std::f32::consts::TAU).sin();
                d.opacity(0.32 + 0.68 * phase.max(0.))
            },
        )
    };
    // base_ (typing): height 24px, 12px / 16px, weight 500, --text-subtle; dots_ gap 2px;
    // text_ margin-inline-start 4px.
    div()
        .h(z(24.))
        .flex()
        .items_center()
        .gap(z(4.))
        .text_size(z(12.))
        .line_height(z(16.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(super::tk(idx::TEXT_SUBTLE))
        .when(active, |d| {
            d.child(div().flex().gap(z(2.)).child(dot(0)).child(dot(1)).child(dot(2))).child(line)
        })
        .when_some(app.chat.error.clone(), |d, err| {
            d.child(div().flex_1()).child(div().text_color(c(Palette::RED)).text_ellipsis().child(err))
        })
}
