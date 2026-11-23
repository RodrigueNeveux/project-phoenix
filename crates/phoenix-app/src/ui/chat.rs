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
    div()
        .px(px(16.))
        .pt(px(32.))
        .pb(px(8.))
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
    let editing = app.chat.edit.as_ref().is_some_and(|e| e.id == id);
    let replying = app.replying_to == Some(id);
    let highlight = r.mentions_me || msg.mention_everyone || replying;

    let mut content = div().flex().flex_col().min_w_0();
    for (bi, b) in r.blocks.iter().enumerate() {
        content = content.child(block(entity, id, bi, b));
    }
    if msg.edited {
        content = content.child(
            div()
                .text_size(px(10.))
                .text_color(c(Palette::TEXT_MUTED))
                .child("(edited)"),
        );
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
            .h(px(22.))
            .flex()
            .items_center()
            .gap(px(4.))
            .text_size(px(14.))
            .child(
                // reply spine
                div()
                    .absolute()
                    .left(px(-36.))
                    .top(px(10.))
                    .w(px(33.))
                    .h(px(12.))
                    .border_l_2()
                    .border_t_2()
                    .rounded_tl(px(6.))
                    .border_color(c(0x4E5058)),
            )
            .child(avatar(ru, 16.))
            .child(
                div()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(role_color(ru.and_then(|u| app.author_color(u.id))))
                    .child(SharedString::from(format!(
                        "@{}",
                        ru.map(|u| u.display_name.as_ref()).unwrap_or("Unknown")
                    ))),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_color(c(Palette::INTERACTIVE))
                    .text_ellipsis()
                    .child(preview),
            )
    });

    let body = div()
        .id(("msg", id.0))
        .group("msg")
        .on_mouse_down(gpui::MouseButton::Right, message_ext::on_right_click(entity, msg))
        .when(app.chat.menu.is_some_and(|m| m.msg == id), |d| d.bg(c(Palette::BG_MSG_HOVER)))
        .relative()
        .pl(px(m::MSG_GUTTER))
        .pr(px(48.))
        .py(px(2.))
        .when(r.group_start, |d| d.mt(px(17.)))
        .when(highlight, |d| {
            d.bg(ca(Palette::MENTIONED_MSG_BG))
                .border_l_2()
                .border_color(c(Palette::MENTIONED_MSG_BAR))
        })
        .when(editing, |d| d.bg(c(Palette::BG_MSG_HOVER)))
        .hover(|s| s.bg(c(Palette::BG_MSG_HOVER)))
        .when_some(reply, |d, reply| d.child(reply))
        .when(r.group_start, |d| {
            d.child(
                div()
                    .absolute()
                    .left(px(16.))
                    .top(px(if msg.reply_to.is_some() { 24. } else { 4. }))
                    .id(("msg-avatar", id.0))
                    .cursor_pointer()
                    .on_click(message_ext::open_author(entity, msg))
                    .child(avatar(author, m::AVATAR)),
            )
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(px(4.))
                    .h(px(22.))
                    .child(
                        div()
                            .text_size(px(16.))
                            .line_height(px(22.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(color)
                            .id(("msg-author", id.0))
                            .cursor_pointer()
                            .hover(|s| s.underline())
                            .on_click(message_ext::open_author(entity, msg))
                            .child(name),
                    )
                    .when(author.is_some_and(|a| a.bot), |d| {
                        d.child(
                            div()
                                .px(px(4.))
                                .h(px(15.))
                                .rounded(px(3.))
                                .bg(c(Palette::BLURPLE))
                                .text_size(px(10.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(gpui::white())
                                .child("APP"),
                        )
                    })
                    .child(
                        div()
                            .ml(px(4.))
                            .text_size(px(12.))
                            .text_color(c(Palette::TEXT_MUTED))
                            .child(r.time_full.clone()),
                    ),
            )
        })
        .when(!r.group_start, |d| {
            d.child(
                div()
                    .absolute()
                    .left_0()
                    .top(px(4.))
                    .w(px(m::MSG_GUTTER - 4.))
                    .flex()
                    .justify_center()
                    .text_size(px(11.))
                    .text_color(gpui::transparent_black())
                    .group_hover("msg", |s| s.text_color(c(Palette::TEXT_MUTED)))
                    .child(r.time_short.clone()),
            )
        })
        .child(content)
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

fn rich(entity: &Entity<PhoenixApp>, msg: Id, bi: usize, t: &RichText) -> AnyElement {
    if !t.emoji.is_empty() {
        return message_ext::rich_with_emoji(entity, msg, bi, t);
    }
    let styled = StyledText::new(t.text.clone()).with_runs(t.runs.clone());
    if t.click_ranges.is_empty() {
        return div().text_size(px(16.)).line_height(px(22.)).child(styled).into_any_element();
    }
    let targets = t.click_targets.clone();
    let e = entity.clone();
    div()
        .text_size(px(16.))
        .line_height(px(22.))
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

fn block(entity: &Entity<PhoenixApp>, msg: Id, bi: usize, b: &RBlock) -> AnyElement {
    match b {
        RBlock::Text(t) => rich(entity, msg, bi, t),
        RBlock::Quote(t) => div()
            .flex()
            .child(div().w(px(4.)).rounded(px(4.)).bg(c(0x4E5058)).flex_none())
            .child(div().pl(px(12.)).pr(px(8.)).child(rich(entity, msg, bi, t)))
            .into_any_element(),
        RBlock::Code { code, .. } => div()
            .my(px(4.))
            .max_w(px(900.))
            .p(px(8.))
            .rounded(px(4.))
            .bg(c(Palette::CODE_BG))
            .border_1()
            .border_color(c(Palette::CODE_BORDER))
            .font_family(FONT_MONO)
            .text_size(px(14.))
            .line_height(px(18.))
            .text_color(c(Palette::TEXT_NORMAL))
            .child(code.clone())
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

fn embed(e: &Embed) -> AnyElement {
    let has_text = e.title.is_some() || e.description.is_some();
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
                .p(px(12.))
                .pr(px(16.))
                .flex()
                .gap(px(16.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .when_some(e.provider.clone(), |d, p| {
                            d.child(div().text_size(px(12.)).text_color(c(Palette::TEXT_NORMAL)).child(p))
                        })
                        .when_some(e.title.clone(), |d, t| {
                            d.child(
                                div()
                                    .text_size(px(16.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(c(if e.url.is_some() { Palette::TEXT_LINK } else { Palette::TEXT_HEADER }))
                                    .child(t),
                            )
                        })
                        .when_some(e.description.clone(), |d, t| {
                            d.child(div().text_size(px(14.)).line_height(px(18.)).child(t))
                        })
                        .when_some(big, |d, m| {
                            let size = media::fit(m.width, m.height, 400., 300.);
                            let original = m.url.clone();
                            d.child(media_image(&m.url, original, "".into(), (m.width, m.height), size).rounded(px(4.)))
                        }),
                )
                .when_some(thumb, |d, m| {
                    let size = media::fit(m.width, m.height, 80., 80.);
                    let original = m.url.clone();
                    d.child(
                        div()
                            .flex_none()
                            .child(media_image(&m.url, original, "".into(), (m.width, m.height), size).rounded(px(4.))),
                    )
                }),
        )
        .into_any_element()
}
