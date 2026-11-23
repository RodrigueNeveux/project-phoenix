//! Message composer (moved out of `chat.rs`): reply bar with the mention
//! toggle, upload tray, upload progress, attach button, paste-image, typing
//! indicator.

use std::time::Duration;

use gpui::{
    Animation, AnimationExt, Context, FontWeight, IntoElement, KeyDownEvent, ParentElement, SharedString,
    Styled, Window, div, img, prelude::*, px, relative,
};
use gpui_component::input::Input;

use super::{
    icons::{ChatIcon, icon},
    state::EmojiTarget,
};
use crate::{
    app::PhoenixApp,
    assets::Icon,
    theme::{Palette, c},
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
    let name = app.users.get(&m.author).map(|u| u.display_name.clone()).unwrap_or_else(|| "Unknown".into());
    let own = m.author == app.me.id;
    let mention = app.chat.reply_mention;
    Some(
        div()
            .h(px(34.))
            .pl(px(16.))
            .pr(px(8.))
            .rounded_t(px(8.))
            .bg(c(Palette::BG_SECONDARY))
            .flex()
            .items_center()
            .text_size(px(14.))
            .text_color(c(Palette::TEXT_MUTED))
            .child(div().child("Replying to "))
            .child(
                div()
                    .ml(px(4.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c(Palette::INTERACTIVE_HOVER))
                    .child(name),
            )
            .child(div().flex_1())
            .when(!own, |d| {
                d.child(
                    div()
                        .id("reply-mention")
                        .px(px(8.))
                        .h(px(24.))
                        .flex()
                        .items_center()
                        .gap(px(2.))
                        .cursor_pointer()
                        .font_weight(FontWeight::BOLD)
                        .text_color(c(if mention { Palette::TEXT_LINK } else { Palette::TEXT_MUTED }))
                        .child("@")
                        .child(if mention { "ON" } else { "OFF" })
                        .on_click(cx.listener(|a, _, _, cx| {
                            a.chat.reply_mention = !a.chat.reply_mention;
                            cx.notify();
                        })),
                )
                .child(div().w(px(1.)).h(px(16.)).mx(px(4.)).bg(c(Palette::DIVIDER)))
            })
            .child(
                icon_button("cancel-reply", Icon::Close, c(Palette::INTERACTIVE), 16.).on_click(cx.listener(
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
        .p(px(16.))
        .pb(px(4.))
        .flex()
        .gap(px(24.))
        .overflow_x_scroll()
        .border_b_1()
        .border_color(c(Palette::DIVIDER));
    for (ix, f) in app.chat.uploads.iter().enumerate() {
        let preview = match &f.image {
            Some(i) => img(i.clone())
                .size_full()
                .rounded(px(3.))
                .object_fit(gpui::ObjectFit::Contain)
                .into_any_element(),
            None => icon(ChatIcon::Upload, 64., c(Palette::BLURPLE)).into_any_element(),
        };
        tray = tray.child(
            div()
                .relative()
                .flex_none()
                .w(px(216.))
                .h(px(216.))
                .p(px(8.))
                .rounded(px(4.))
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
                        .mt(px(8.))
                        .text_size(px(14.))
                        .text_color(c(Palette::TEXT_NORMAL))
                        .text_ellipsis()
                        .child(f.name.clone()),
                )
                .child(
                    div()
                        .absolute()
                        .top(px(-12.))
                        .right(px(-12.))
                        .flex()
                        .rounded(px(4.))
                        .bg(c(Palette::BG_PRIMARY))
                        .border_1()
                        .border_color(c(Palette::HEADER_SHADOW))
                        .child(
                            div()
                                .id(("up-spoiler", ix))
                                .size(px(32.))
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
                                .size(px(32.))
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
    let mut col = div().flex().flex_col().gap(px(8.));
    for s in app.chat.sending.iter().filter(|s| Some(s.channel) == ch) {
        col = col.child(
            div()
                .mx(px(16.))
                .p(px(12.))
                .rounded(px(8.))
                .bg(c(Palette::BG_SECONDARY))
                .flex()
                .items_center()
                .gap(px(12.))
                .child(icon(ChatIcon::Upload, 32., c(Palette::BLURPLE)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .child(
                            div()
                                .flex()
                                .gap(px(8.))
                                .text_size(px(14.))
                                .child(div().text_color(c(Palette::TEXT_NORMAL)).text_ellipsis().child(s.label.clone()))
                                .child(div().text_color(c(Palette::TEXT_MUTED)).child(human_size(s.size))),
                        )
                        .child(
                            div().h(px(6.)).rounded(px(3.)).bg(c(Palette::INTERACTIVE_MUTED)).overflow_hidden().child(
                                div().h_full().rounded(px(3.)).bg(c(Palette::BLURPLE)).with_animation(
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
        Some(c) if c.guild.is_none() => format!("Message @{}", c.name).into(),
        Some(c) => format!("Message #{}", c.name).into(),
        None => "".into(),
    };
    let ic = c(Palette::INTERACTIVE);
    let placeholder_empty = app.composer.read(cx).value().is_empty();
    // Discord shows the remaining count once you get near the 2000 limit.
    let remaining = 2000 - app.composer.read(cx).value().chars().count() as i64;
    let reply = reply_bar(app, cx);
    let has_banner = reply.is_some();
    let tray = upload_tray(app, cx);
    div()
        .flex_none()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(sending(app))
        .children(super::slash::render(app, cx))
        .child(
            div().px(px(16.)).child(
                div()
                    .flex()
                    .flex_col()
                    .rounded(px(8.))
                    .bg(c(Palette::INPUT_BG))
                    .children(reply)
                    .children(tray)
                    .child(
                        div()
                            .id("composer")
                            .min_h(px(44.))
                            .when(has_banner, |d| d.rounded_b(px(8.)))
                            .relative()
                            .when(remaining < 200, |d| {
                                d.child(
                                    div()
                                        .absolute()
                                        .right(px(16.))
                                        .bottom(px(-18.))
                                        .text_size(px(12.))
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
                                div().pl(px(16.)).pr(px(16.)).child(
                                    icon_button("attach", Icon::PlusCircle, ic, 24.)
                                        .mx(px(-4.))
                                        .on_click(cx.listener(|a, _, _, cx| a.prompt_attach(cx))),
                                ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .relative()
                                    .text_size(px(16.))
                                    .text_color(c(Palette::TEXT_NORMAL))
                                    .child(Input::new(&app.composer).appearance(false).bordered(false))
                                    .when(placeholder_empty, |d| {
                                        d.child(
                                            div()
                                                .absolute()
                                                .top_0()
                                                .bottom_0()
                                                .left(px(2.))
                                                .flex()
                                                .items_center()
                                                .text_color(c(0x6D6F78))
                                                .child(placeholder),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .pr(px(8.))
                                    .flex()
                                    .items_center()
                                    .child(icon_button("gift", Icon::Gift, ic, 24.))
                                    .child(icon_button("gif", Icon::Gif, ic, 24.).on_click(cx.listener(|a, _, w, cx| crate::ui::pickers::toggle(a, crate::ui::pickers::Tab::Gif, crate::ui::pickers::EmojiTarget::Composer, w, cx))))
                                    .child(icon_button("sticker", Icon::Sticker, ic, 24.).on_click(cx.listener(|a, _, w, cx| crate::ui::pickers::toggle(a, crate::ui::pickers::Tab::Sticker, crate::ui::pickers::EmojiTarget::Composer, w, cx))))
                                    .child(icon_button("emoji", Icon::Emoji, ic, 24.).on_click(cx.listener(
                                        |a, _, _, cx| a.open_emoji_picker(EmojiTarget::Composer, cx),
                                    ))),
                            ),
                    ),
            ),
        )
        .child(div().px(px(16.)).child(typing(app)))
}

/// "**Name** is typing…" with bouncing dots, from real TYPING_START events.
fn typing(app: &PhoenixApp) -> impl IntoElement {
    let names: Vec<SharedString> = app
        .selected_channel
        .and_then(|c| app.typing.get(&c))
        .map(|v| v.iter().filter_map(|(u, _)| app.users.get(u).map(|u| u.display_name.clone())).collect())
        .unwrap_or_default();
    let bold = |s: SharedString| div().font_weight(FontWeight::BOLD).child(s);
    let mut line = div().flex().items_center().gap(px(0.)).min_w_0();
    match names.as_slice() {
        [] => {}
        [a] => line = line.child(bold(a.clone())).child(" is typing…"),
        [a, b] => line = line.child(bold(a.clone())).child(" and ").child(bold(b.clone())).child(" are typing…"),
        [a, b, c3] => {
            line = line
                .child(bold(a.clone()))
                .child(", ")
                .child(bold(b.clone()))
                .child(", and ")
                .child(bold(c3.clone()))
                .child(" are typing…")
        }
        _ => line = line.child("Several people are typing…"),
    }
    let active = !names.is_empty();
    let dot = |i: usize| {
        div().size(px(6.)).rounded_full().bg(c(Palette::INTERACTIVE_HOVER)).with_animation(
            ("typing-dot", i),
            Animation::new(Duration::from_millis(1200)).repeat(),
            move |d, t| {
                let phase = ((t * 3. - i as f32 * 0.5).rem_euclid(3.) / 3. * std::f32::consts::TAU).sin();
                d.opacity(0.4 + 0.6 * phase.max(0.))
            },
        )
    };
    div()
        .h(px(24.))
        .flex()
        .items_center()
        .gap(px(6.))
        .text_size(px(12.))
        .text_color(c(Palette::TEXT_NORMAL))
        .when(active, |d| {
            d.child(div().flex().gap(px(3.)).child(dot(0)).child(dot(1)).child(dot(2))).child(line)
        })
        .when_some(app.chat.error.clone(), |d, err| {
            d.child(div().flex_1()).child(div().text_color(c(Palette::RED)).text_ellipsis().child(err))
        })
}
