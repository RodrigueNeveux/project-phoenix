//! Chat area: header, virtualized message list, composer, typing indicator.

use gpui::{
    AnyElement, App, Context, Entity, FontWeight, IntoElement, ParentElement,
    SharedString, StyledText, Styled, InteractiveText, Window, div, img, list, prelude::*, px,
};

use crate::{
    app::PhoenixApp,
    assets::Icon,
    media,
    model::{Attachment, AttachmentKind, ChannelKind, Embed, Id, Message},
    richtext::{ClickTarget, RBlock, RenderedMessage, RichText},
    theme::{FONT_MONO, Palette, c, ca, m, role_color},
    ui::{avatar, icon, icon_button, members, sidebar::channel_icon},
};

// Messaging features live in submodules (composer moved to `composer.rs`).
pub mod composer;
pub mod icons;
pub mod message_ext;
pub mod panels;
pub mod state;
pub mod slash;
pub mod system;
pub use state::ChatState;

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
                        .child(
                            div()
                                .flex_1()
                                .min_h_0()
                                .relative()
                                .child(
                                    list(app.message_list.clone(), move |ix, _w, cx| row(&entity, ix, cx))
                                        .size_full(),
                                )
                                .children(panels::new_messages_bar(app, cx))
                                .children(panels::jump_bar(app, cx)),
                        )
                        .child(composer::render(app, window, cx))
                        .child(panels::drop_overlay(app)),
                )
                .when(show_members, |d| d.child(members::render(app, cx)))
                .children(panels::thread_panel(app, cx)),
        )
        .children(panels::pins_popout(app, cx))
}

fn header(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let ch = app.selected_channel.and_then(|c| app.channels.get(&c));
    let is_dm = ch.is_some_and(|c| c.guild.is_none());
    let name = ch.map(|c| c.name.clone()).unwrap_or_default();
    let topic = ch.and_then(|c| c.topic.clone());
    let kind = ch.map(|c| c.kind).unwrap_or(ChannelKind::Text);
    let ic = c(Palette::INTERACTIVE);
    div()
        .h(px(m::HEADER_H))
        .flex_none()
        .px(px(8.))
        .flex()
        .items_center()
        .gap(px(8.))
        .border_b_1()
        .border_color(c(Palette::HEADER_SHADOW))
        .child(
            div()
                .pl(px(8.))
                .child(if is_dm {
                    avatar(ch.and_then(|c| c.voice_users.first()).and_then(|u| app.users.get(u)), 24.)
                } else {
                    icon(channel_icon(kind), 24., c(Palette::CHANNEL_ICON)).into_any_element()
                }),
        )
        .child(
            div()
                .flex_none()
                .text_size(px(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c(Palette::TEXT_HEADER))
                .child(name),
        )
        .when_some(topic, |d, t| {
            let topic_click = crate::ui::home::topic::on_click(app, cx);
            d.child(div().w(px(1.)).h(px(24.)).mx(px(8.)).bg(c(Palette::DIVIDER)))
                .child(
                    div()
                        .id("channel-topic")
                        .flex_1()
                        .min_w_0()
                        .cursor_pointer()
                        .text_size(px(14.))
                        .text_color(c(Palette::TEXT_MUTED))
                        .text_ellipsis()
                        .on_click(topic_click)
                        .child(t),
                )
        })
        .child(div().flex_1())
        .child(icon_button("threads", Icon::Threads, ic, 24.))
        .child(icon_button("bell", Icon::Bell, ic, 24.))
        .child(
            icon_button("pin", Icon::Pin, if app.chat.pins.is_some() { c(Palette::INTERACTIVE_HOVER) } else { ic }, 24.)
                .on_click(cx.listener(|a, _, _, cx| a.toggle_pins(cx))),
        )
        .child(
            icon_button(
                "members",
                Icon::Members,
                if app.show_members { c(Palette::INTERACTIVE_HOVER) } else { ic },
                24.,
            )
            .on_click(cx.listener(|a, _, _, cx| {
                a.show_members = !a.show_members;
                cx.notify();
            })),
        )
        .child(
            div()
                .w(px(144.))
                .h(px(24.))
                .mx(px(4.))
                .rounded(px(4.))
                .bg(c(Palette::BG_TERTIARY))
                .px(px(6.))
                .flex()
                .items_center()
                .justify_between()
                .text_size(px(14.))
                .text_color(c(Palette::TEXT_MUTED))
                .child("Search")
                .child(icon(Icon::Search, 16., c(Palette::TEXT_MUTED))),
        )
        .child(icon_button("inbox", Icon::Inbox, ic, 24.))
        .child(icon_button("help", Icon::Help, ic, 24.))
}

fn row(entity: &Entity<PhoenixApp>, ix: usize, cx: &mut App) -> AnyElement {
    let app = entity.read(cx);
    let Some(ch_id) = app.selected_channel else { return div().into_any_element() };
    let Some(entry) = app.messages.get(&ch_id) else { return div().into_any_element() };
    if ix == 0 {
        return intro(app, ch_id, entry.reached_start, entry.loading).into_any_element();
    }
    let i = ix - 1;
    let (Some(msg), Some(r)) = (entry.msgs.get(i), entry.rendered.get(i)) else {
        return div().into_any_element();
    };
    message(app, entity, msg, r, entry, cx)
}

fn intro(app: &PhoenixApp, ch: Id, reached_start: bool, loading: bool) -> impl IntoElement {
    let channel = app.channels.get(&ch);
    let name = channel.map(|c| c.name.clone()).unwrap_or_default();
    if !reached_start {
        return div()
            .h(px(64.))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(14.))
            .text_color(c(Palette::TEXT_MUTED))
            .child(if loading { "Loading messages…" } else { "" });
    }
    if let Some(chan) = channel.filter(|c| c.guild.is_none()) {
        let peer = chan.voice_users.first().and_then(|u| app.users.get(u));
        return div()
            .m(px(16.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(avatar(peer, 80.))
            .child(
                div()
                    .text_size(px(32.))
                    .line_height(px(40.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(c(Palette::TEXT_HEADER))
                    .child(name.clone()),
            )
            .child(
                div()
                    .text_size(px(16.))
                    .text_color(c(Palette::TEXT_MUTED))
                    .child(SharedString::from(format!(
                        "This is the beginning of your direct message history with {name}."
                    ))),
            );
    }
    div()
        .m(px(16.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            div()
                .size(px(68.))
                .rounded_full()
                .bg(c(0x41434A))
                .flex()
                .items_center()
                .justify_center()
                .child(icon(Icon::Hash, 42., gpui::white())),
        )
        .child(
            div()
                .text_size(px(32.))
                .line_height(px(40.))
                .font_weight(FontWeight::BOLD)
                .text_color(c(Palette::TEXT_HEADER))
                .child(SharedString::from(format!("Welcome to #{name}!"))),
        )
        .child(
            div()
                .text_size(px(16.))
                .text_color(c(Palette::TEXT_MUTED))
                .child(SharedString::from(format!("This is the start of the #{name} channel."))),
        )
}

fn day_divider(label: SharedString) -> impl IntoElement {
    div()
        .mx(px(16.))
        .mt(px(24.))
        .mb(px(8.))
        .h(px(1.))
        .bg(c(Palette::DIVIDER))
        .flex()
        .justify_center()
        .items_center()
        .child(
            div()
                .bg(c(Palette::BG_PRIMARY))
                .px(px(4.))
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c(Palette::TEXT_MUTED))
                .child(label),
        )
}

/// "Monday, November 23, 2026 at 3:04 PM" (timestamp tooltip).
pub fn format_time_tooltip(ts: i64) -> SharedString {
    use chrono::{Local, TimeZone};
    Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|d| d.format("%A, %B %-d, %Y at %-I:%M %p").to_string().into())
        .unwrap_or_default()
}

fn time_tooltip(ts: i64) -> impl Fn(&mut Window, &mut App) -> gpui::AnyView + 'static {
    move |window, cx| gpui_component::tooltip::Tooltip::new(format_time_tooltip(ts)).build(window, cx)
}

/// Discord's "APP" tag next to bot authors.
fn bot_tag() -> impl IntoElement {
    div()
        .flex_none()
        .ml(px(4.))
        .px(px(4.))
        .h(px(15.))
        .rounded(px(3.))
        .bg(c(Palette::BLURPLE))
        .flex()
        .items_center()
        .text_size(px(10.))
        .line_height(px(15.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(gpui::white())
        .child("APP")
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
    let name = author.map(|u| u.display_name.clone()).unwrap_or_else(|| "Unknown".into());
    let color = role_color(app.author_color(msg.author));
    let id = msg.id;
    if let Some(sys) = system::render(app, entity, msg, r) {
        return sys;
    }
    let compact = app.prefs.synced.compact;
    let fs = app.prefs.local.font_size.clamp(12., 24.);
    let scale = fs / 16.;
    let editing = app.chat.edit.as_ref().is_some_and(|e| e.id == id);
    let replying = app.replying_to == Some(id);
    let highlight = r.mentions_me || msg.mention_everyone || replying;
    let ts = msg.timestamp;

    let mut content = div().flex().flex_col().min_w_0().text_size(px(fs)).text_color(c(Palette::TEXT_NORMAL));
    let last_text = r.blocks.iter().rposition(|b| matches!(b, RBlock::Text(_)));
    let n_blocks = r.blocks.len();
    for (bi, b) in r.blocks.iter().enumerate() {
        let edited_here = msg.edited && last_text == Some(bi) && bi + 1 == n_blocks;
        content = content.child(block(entity, id, bi, b, edited_here, fs));
    }
    if msg.edited && !last_text.is_some_and(|l| l + 1 == n_blocks) {
        content = content.child(edited_marker(ts));
    }
    for a in &msg.attachments {
        content = content.child(attachment(a, cx));
    }
    for e in &msg.embeds {
        content = content.child(embed(e));
    }
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
        let found = entry.msgs.iter().position(|m| m.id == rid);
        let (ru, preview) = match found {
            Some(p) => (app.users.get(&entry.msgs[p].author), entry.rendered[p].preview.clone()),
            None => (None, SharedString::from("Original message was deleted")),
        };
        div()
            .relative()
            .h(px(18.))
            .mb(px(4.))
            .flex()
            .items_center()
            .gap(px(4.))
            .text_size(px(14. * scale))
            .line_height(px(18.))
            .child(
                // Reply spine from the avatar column up to the preview.
                div()
                    .absolute()
                    .left(px(if compact { -20. } else { -36. }))
                    .top(px(8.))
                    .w(px(if compact { 18. } else { 33. }))
                    .h(px(13.))
                    .border_l_2()
                    .border_t_2()
                    .rounded_tl(px(6.))
                    .border_color(c(Palette::INTERACTIVE_MUTED)),
            )
            .when(!compact, |d| d.child(avatar(ru, 16.)))
            .child(
                div()
                    .flex_none()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(role_color(ru.and_then(|u| app.author_color(u.id))))
                    .opacity(0.64)
                    .child(SharedString::from(format!(
                        "@{}",
                        ru.map(|u| u.display_name.as_ref()).unwrap_or("Unknown")
                    ))),
            )
            .when(ru.is_some_and(|u| u.bot), |d| d.child(bot_tag()))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_color(c(Palette::INTERACTIVE))
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .when(found.is_none(), |d| d.italic())
                    .child(preview),
            )
    });

    let author_name = div()
        .flex_none()
        .text_size(px(fs))
        .line_height(px(22. * scale))
        .font_weight(FontWeight::MEDIUM)
        .text_color(color)
        .id(("msg-author", id.0))
        .cursor_pointer()
        .hover(|s| s.underline())
        .on_click(message_ext::open_author(entity, msg))
        .child(name);
    let is_bot = author.is_some_and(|a| a.bot);

    let base = div()
        .id(("msg", id.0))
        .group("msg")
        .on_mouse_down(gpui::MouseButton::Right, message_ext::on_right_click(entity, msg))
        .when(app.chat.menu.is_some_and(|m| m.msg == id), |d| d.bg(c(Palette::BG_MSG_HOVER)))
        .relative()
        .pr(px(48.))
        .when(highlight, |d| {
            d.bg(ca(Palette::MENTIONED_MSG_BG))
                .border_l_2()
                .border_color(c(Palette::MENTIONED_MSG_BAR))
        })
        .when(editing, |d| d.bg(c(Palette::BG_MSG_HOVER)))
        .hover(|s| s.bg(c(Palette::BG_MSG_HOVER)));

    let body = if compact {
        // Compact (IRC-like): [time] name: text, no avatars.
        base.pl(px(16.))
            .py(px(1.))
            .when(r.group_start, |d| d.mt(px(4.)))
            .when_some(reply, |d, reply| d.child(div().pl(px(68.)).child(reply)))
            .child(
                div()
                    .flex()
                    .items_start()
                    .child(
                        div()
                            .id(("msg-time", id.0))
                            .flex_none()
                            .w(px(56.))
                            .mr(px(4.))
                            .h(px(22. * scale))
                            .flex()
                            .items_center()
                            .justify_end()
                            .text_size(px(11.))
                            .text_color(c(Palette::TEXT_MUTED))
                            .when(!r.group_start, |d| {
                                d.text_color(gpui::transparent_black())
                                    .group_hover("msg", |s| s.text_color(c(Palette::TEXT_MUTED)))
                            })
                            .tooltip(time_tooltip(ts))
                            .child(r.time_short.clone()),
                    )
                    .when(r.group_start, |d| {
                        d.child(
                            div()
                                .flex_none()
                                .flex()
                                .items_center()
                                .mr(px(4.))
                                .child(author_name)
                                .when(is_bot, |d| d.child(bot_tag())),
                        )
                    })
                    .child(div().flex_1().min_w_0().child(content)),
            )
    } else {
        base.pl(px(m::MSG_GUTTER))
            .py(px(2.))
            .when(r.group_start, |d| d.mt(px(17.)))
            .when_some(reply, |d, reply| d.child(reply))
            .when(r.group_start, |d| {
                d.child(
                    div()
                        .absolute()
                        .left(px(16.))
                        .top(px(if msg.reply_to.is_some() { 24. } else { 2. }))
                        .id(("msg-avatar", id.0))
                        .cursor_pointer()
                        .on_click(message_ext::open_author(entity, msg))
                        .child(avatar(author, m::AVATAR)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .h(px(22. * scale))
                        .child(author_name)
                        .when(is_bot, |d| d.child(bot_tag()))
                        .child(
                            div()
                                .id(("msg-time", id.0))
                                .ml(px(8.))
                                .text_size(px(12.))
                                .text_color(c(Palette::TEXT_MUTED))
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
                        .top(px(2.))
                        .w(px(64.))
                        .h(px(22. * scale))
                        .flex()
                        .items_center()
                        .justify_end()
                        .text_size(px(11.))
                        .text_color(gpui::transparent_black())
                        .group_hover("msg", |s| s.text_color(c(Palette::TEXT_MUTED)))
                        .tooltip(time_tooltip(ts))
                        .child(r.time_short.clone()),
                )
            })
            .child(content)
    }
    .child(message_ext::toolbar(app, entity, msg, app.chat.shift_held));
    let new_marker = app.chat.unread_marker.is_some_and(|(_, m)| m == id);
    if r.day_divider.is_some() || new_marker {
        div()
            .when_some(r.day_divider.clone(), |d, label| d.child(day_divider(label)))
            .when(new_marker, |d| d.child(message_ext::new_divider()))
            .child(body)
            .into_any_element()
    } else {
        body.into_any_element()
    }
}

fn edited_marker(ts: i64) -> impl IntoElement {
    div()
        .id(("edited", ts as u64))
        .text_size(px(10.))
        .line_height(px(16.))
        .text_color(c(Palette::TEXT_MUTED))
        .child("(edited)")
}

/// Append Discord's small muted " (edited)" after the last line of text.
fn with_edited(t: &RichText) -> (SharedString, Vec<gpui::TextRun>) {
    let mut text = t.text.to_string();
    text.push_str(" (edited)");
    let mut runs = t.runs.clone();
    let mut f = crate::richtext::ui_font();
    f.weight = FontWeight::NORMAL;
    runs.push(gpui::TextRun {
        len: " (edited)".len(),
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
            return div().flex().flex_wrap().items_end().child(el).child(div().ml(px(4.)).child(edited_marker(msg.0 as i64))).into_any_element();
        }
        return el;
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

fn block(entity: &Entity<PhoenixApp>, msg: Id, bi: usize, b: &RBlock, edited: bool, fs: f32) -> AnyElement {
    let scale = fs / 16.;
    match b {
        RBlock::Text(t) => rich(entity, msg, bi, t, edited),
        RBlock::Quote(t) => div()
            .flex()
            .child(div().w(px(4.)).rounded(px(4.)).bg(c(Palette::INTERACTIVE_MUTED)).flex_none())
            .child(div().flex_1().min_w_0().pl(px(12.)).pr(px(8.)).child(rich(entity, msg, bi, t, false)))
            .into_any_element(),
        RBlock::Heading(level, t) => {
            let size = match level {
                1 => 24.,
                2 => 20.,
                _ => 16.,
            } * scale;
            div()
                .mt(px(if bi == 0 { 8. } else { 16. }))
                .mb(px(8.))
                .text_size(px(size))
                .child(rich(entity, msg, bi, t, edited))
                .into_any_element()
        }
        RBlock::Subtext(t) => div()
            .text_size(px(13. * scale))
            .text_color(c(Palette::TEXT_MUTED))
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
                .ml(px(16. * (*depth as f32 + 1.) - 16.))
                .mb(px(4.))
                .child(
                    div()
                        .flex_none()
                        .w(px(if ordered.is_some() { 24. } else { 16. }))
                        .pr(px(4.))
                        .flex()
                        .justify_end()
                        .line_height(gpui::relative(1.375))
                        .child(marker),
                )
                .child(div().flex_1().min_w_0().child(rich(entity, msg, bi, text, edited)))
                .into_any_element()
        }
        RBlock::Code { code, runs, .. } => div()
            .my(px(4.))
            .max_w(px(900.))
            .p(px(8.))
            .rounded(px(4.))
            .bg(c(Palette::CODE_BG))
            .border_1()
            .border_color(c(Palette::CODE_BORDER))
            .font_family(FONT_MONO)
            .text_size(px(14. * scale))
            .line_height(px(18. * scale))
            .text_color(c(0xB9BBBE))
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
    img(src.clone())
        .id(gpui::ElementId::Name(src))
        .w(px(w))
        .h(px(h))
        .rounded(px(8.))
        .object_fit(gpui::ObjectFit::Cover)
        .cursor_pointer()
        .with_loading(|| div().size_full().rounded(px(8.)).bg(c(Palette::BG_SECONDARY)).into_any_element())
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
            .w(px(w))
            .h(px(h))
            .rounded(px(8.))
            .overflow_hidden()
            .cursor_pointer()
            .bg(c(Palette::BG_SECONDARY))
            .child(img(tiny).absolute().inset_0().size_full().object_fit(gpui::ObjectFit::Cover))
            .child(
                div().absolute().inset_0().bg(ca(0x00000099)).flex().items_center().justify_center().child(
                    div()
                        .px(px(12.))
                        .py(px(6.))
                        .rounded_full()
                        .bg(ca(0x000000CC))
                        .text_size(px(15.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(gpui::white())
                        .child("SPOILER"),
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
                    .mt(px(4.))
                    .size(px(160.))
                    .object_fit(gpui::ObjectFit::Contain)
                    .with_fallback(move || sticker_name(name.clone()))
                    .into_any_element(),
                None => div().mt(px(4.)).child(sticker_name(name)).into_any_element(),
            };
        }
        AttachmentKind::Image | AttachmentKind::Video if remote => {
            let size = media::fit(a.width, a.height, media::MAX_W, media::MAX_H);
            if spoiler && let Some(cover) = spoiler_cover(a.url.clone(), &a.url, size, cx) {
                return div().mt(px(4.)).child(cover).into_any_element();
            }
            if a.kind == AttachmentKind::Video {
                let poster = media::video_poster(&a.url, (size.0 * 2.) as u32, (size.1 * 2.) as u32);
                let original = if a.original.is_empty() { a.url.clone() } else { a.original.clone() };
                return div()
                    .id(gpui::ElementId::Name(format!("video-{}", a.url).into()))
                    .mt(px(4.))
                    .relative()
                    .w(px(size.0))
                    .h(px(size.1))
                    .rounded(px(8.))
                    .overflow_hidden()
                    .bg(gpui::black())
                    .cursor_pointer()
                    .child(img(poster).size_full().object_fit(gpui::ObjectFit::Cover))
                    .child(
                        div().absolute().inset_0().flex().items_center().justify_center().child(
                            div()
                                .size(px(48.))
                                .rounded_full()
                                .bg(ca(0x000000B3))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(gpui::white())
                                .text_size(px(20.))
                                .child("\u{25B6}"),
                        ),
                    )
                    .on_click(move |_, _, cx| cx.open_url(&original))
                    .into_any_element();
            }
            return div()
                .mt(px(4.))
                .child(media_image(&a.url, a.original.clone(), a.filename.clone(), (a.width, a.height), size))
                .into_any_element();
        }
        _ => {}
    }
    if a.is_image {
        let (w, h) = media::fit(a.width, a.height, media::MAX_W, media::MAX_H);
        return div()
            .mt(px(4.))
            .w(px(w))
            .h(px(h))
            .rounded(px(8.))
            .bg(c(Palette::BG_SECONDARY))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(4.))
            .child(icon(Icon::Image, 48., c(Palette::INTERACTIVE_MUTED)))
            .child(div().text_size(px(12.)).text_color(c(Palette::TEXT_MUTED)).child(a.filename.clone()))
            .into_any_element();
    }
    let url = a.url.clone();
    div()
        .id(gpui::ElementId::Name(format!("file-{}", a.url).into()))
        .mt(px(4.))
        .w(px(432.))
        .p(px(16.))
        .rounded(px(8.))
        .bg(c(Palette::BG_SECONDARY))
        .border_1()
        .border_color(c(Palette::BG_TERTIARY))
        .flex()
        .items_center()
        .gap(px(8.))
        .child(icon(Icon::File, 40., c(Palette::BLURPLE)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(div().text_size(px(16.)).text_color(c(Palette::TEXT_LINK)).text_ellipsis().child(a.filename.clone()))
                .child(div().text_size(px(12.)).text_color(c(Palette::TEXT_MUTED)).child(human_size(a.size))),
        )
        .child(icon(Icon::Download, 24., c(Palette::INTERACTIVE)))
        .when(remote, |d| d.cursor_pointer().on_click(move |_, _, cx| cx.open_url(&url)))
        .into_any_element()
}

fn sticker_name(name: SharedString) -> AnyElement {
    div()
        .px(px(8.))
        .py(px(4.))
        .rounded(px(4.))
        .bg(c(Palette::BG_SECONDARY))
        .text_size(px(14.))
        .text_color(c(Palette::TEXT_MUTED))
        .child(SharedString::from(format!("Sticker: {name}")))
        .into_any_element()
}

/// Embed description / field values: Discord renders them as markdown.
fn embed_markdown(src: &str, size: f32) -> AnyElement {
    let parsed = crate::markdown::parse(src, &crate::markdown::NoResolve);
    let blocks = crate::richtext::render_blocks(&parsed, &|_| true, c(Palette::TEXT_NORMAL));
    let mut d = div().flex().flex_col().text_size(px(size)).line_height(px(18.)).min_w_0();
    for b in blocks {
        d = d.child(match b {
            RBlock::Code { code, runs, .. } => div()
                .my(px(4.))
                .p(px(8.))
                .rounded(px(4.))
                .bg(c(Palette::BG_TERTIARY))
                .font_family(FONT_MONO)
                .text_size(px(12.))
                .line_height(px(16.))
                .child(StyledText::new(code).with_runs(runs))
                .into_any_element(),
            RBlock::Quote(t) => div()
                .flex()
                .child(div().w(px(4.)).rounded(px(4.)).bg(c(Palette::INTERACTIVE_MUTED)).flex_none())
                .child(div().pl(px(12.)).child(StyledText::new(t.text).with_runs(t.runs)))
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
                    RBlock::Heading(..) => el.font_weight(FontWeight::BOLD).text_size(px(size + 2.)),
                    RBlock::Subtext(_) => el.text_size(px(12.)).text_color(c(Palette::TEXT_MUTED)),
                    RBlock::ListItem { .. } => div().flex().child(div().w(px(16.)).child("\u{2022}")).child(el),
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
            .mt(px(4.))
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
        let mut grid = div().flex().flex_col().gap(px(8.));
        for r in rows {
            let mut row = div().flex().gap(px(8.));
            for f in r {
                row = row.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .mb(px(2.))
                                .text_size(px(14.))
                                .line_height(px(18.))
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
            .gap(px(8.))
            .when_some(e.footer.as_ref().and_then(|f| f.icon.clone()), |d, u| {
                d.child(img(media::resized(&u, 40, 40)).size(px(20.)).rounded_full().flex_none())
            })
            .child(
                div()
                    .text_size(px(12.))
                    .line_height(px(16.))
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
            .gap(px(8.))
            .when_some(a.icon.clone(), |d, u| {
                d.child(img(media::resized(&u, 48, 48)).size(px(24.)).rounded_full().flex_none())
            })
            .child(
                div()
                    .text_size(px(14.))
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
            .text_size(px(16.))
            .line_height(px(22.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(c(if url.is_some() { Palette::TEXT_LINK } else { Palette::TEXT_HEADER }))
            .when(url.is_some(), |d| d.hover(|s| s.underline()))
            .child(t)
            .when_some(url, |d, u| d.cursor_pointer().on_click(move |_, _, cx| cx.open_url(&u)))
    });

    div()
        .mt(px(4.))
        .max_w(px(516.))
        .flex()
        .rounded(px(4.))
        .overflow_hidden()
        .bg(c(Palette::EMBED_BG))
        .child(div().w(px(4.)).flex_none().bg(c(e.color.unwrap_or(0x1E1F22))))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .pt(px(8.))
                .pr(px(16.))
                .pb(px(16.))
                .pl(px(12.))
                .flex()
                .gap(px(16.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .pt(px(8.))
                        .when_some(e.provider.clone(), |d, p| {
                            d.child(div().text_size(px(12.)).line_height(px(16.)).text_color(c(Palette::TEXT_NORMAL)).child(p))
                        })
                        .children(author)
                        .children(title)
                        .when_some(e.description.clone(), |d, t| d.child(embed_markdown(&t, 14.)))
                        .children(fields)
                        .when_some(big, |d, m| {
                            let size = media::fit(m.width, m.height, 400., 300.);
                            let original = m.url.clone();
                            d.child(media_image(&m.url, original, "".into(), (m.width, m.height), size).rounded(px(4.)))
                        })
                        .children(footer),
                )
                .when_some(thumb, |d, m| {
                    let size = media::fit(m.width, m.height, 80., 80.);
                    let original = m.url.clone();
                    d.child(
                        div()
                            .flex_none()
                            .pt(px(8.))
                            .child(media_image(&m.url, original, "".into(), (m.width, m.height), size).rounded(px(4.))),
                    )
                }),
        )
        .into_any_element()
}
