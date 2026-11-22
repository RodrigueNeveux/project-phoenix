//! Chat area: header, virtualized message list, composer, typing indicator.

use gpui::{
    AnyElement, App, Context, Entity, FontWeight, IntoElement, KeyDownEvent, ParentElement,
    SharedString, StyledText, Styled, InteractiveText, Window, div, list, prelude::*, px,
};
use gpui_component::input::Input;

use crate::{
    app::PhoenixApp,
    assets::Icon,
    model::{Attachment, ChannelKind, Embed, Id, Message, Reaction},
    richtext::{ClickTarget, RBlock, RenderedMessage, RichText},
    theme::{FONT_MONO, Palette, c, ca, m, role_color},
    ui::{avatar, icon, icon_button, members, sidebar::channel_icon},
};

pub fn render(app: &PhoenixApp, window: &mut Window, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let entity = cx.entity();
    let show_members = app.show_members && app.selected_guild.is_some();
    div()
        .flex_1()
        .min_w_0()
        .h_full()
        .flex()
        .flex_col()
        .bg(c(Palette::BG_PRIMARY))
        .child(header(app, cx))
        .child(
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_row()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div().flex_1().min_h_0().child(
                                list(app.message_list.clone(), move |ix, _w, cx| row(&entity, ix, cx))
                                    .size_full(),
                            ),
                        )
                        .child(composer(app, window, cx)),
                )
                .when(show_members, |d| d.child(members::render(app, cx))),
        )
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
            d.child(div().w(px(1.)).h(px(24.)).mx(px(8.)).bg(c(Palette::DIVIDER)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(14.))
                        .text_color(c(Palette::TEXT_MUTED))
                        .text_ellipsis()
                        .child(t),
                )
        })
        .child(div().flex_1())
        .child(icon_button("threads", Icon::Threads, ic, 24.))
        .child(icon_button("bell", Icon::Bell, ic, 24.))
        .child(icon_button("pin", Icon::Pin, ic, 24.))
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
    message(app, entity, msg, r, entry)
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
) -> AnyElement {
    let author = app.users.get(&msg.author);
    let name = author.map(|u| u.display_name.clone()).unwrap_or_else(|| "Unknown".into());
    let color = role_color(app.author_color(msg.author));
    let id = msg.id;
    let editing = app.editing == Some(id);
    let replying = app.replying_to == Some(id);
    let highlight = r.mentions_me || replying;

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
        content = content.child(attachment(a));
    }
    for e in &msg.embeds {
        content = content.child(embed(e));
    }
    if !msg.reactions.is_empty() {
        content = content.child(reactions(&msg.reactions));
    }

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

    let e = entity.clone();
    let body = div()
        .id(("msg", id.0))
        .group("msg")
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
                    .cursor_pointer()
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
                            .cursor_pointer()
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
        // Hover toolbar
        .child(
            div()
                .absolute()
                .right(px(16.))
                .top(px(-16.))
                .invisible()
                .group_hover("msg", |s| s.visible())
                .flex()
                .bg(c(Palette::BG_PRIMARY))
                .border_1()
                .border_color(c(Palette::HEADER_SHADOW))
                .rounded(px(4.))
                .child(icon_button("react", Icon::Emoji, c(Palette::INTERACTIVE), 20.))
                .child(
                    icon_button("reply", Icon::Reply, c(Palette::INTERACTIVE), 20.).on_click(
                        move |_, w, cx| {
                            e.update(cx, |a, cx| a.start_reply(id, w, cx));
                        },
                    ),
                ),
        );
    if let Some(label) = r.day_divider.clone() {
        div().child(day_divider(label)).child(body).into_any_element()
    } else {
        body.into_any_element()
    }
}

fn rich(entity: &Entity<PhoenixApp>, msg: Id, bi: usize, t: &RichText) -> AnyElement {
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

fn attachment(a: &Attachment) -> AnyElement {
    if a.is_image {
        // Placeholder box sized to the image aspect ratio (max 400x300).
        let scale = (400. / a.width.max(1) as f32).min(300. / a.height.max(1) as f32).min(1.);
        return div()
            .mt(px(4.))
            .w(px(a.width as f32 * scale))
            .h(px(a.height as f32 * scale))
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
    div()
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
        .into_any_element()
}

fn embed(e: &Embed) -> AnyElement {
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
                .p(px(12.))
                .pr(px(16.))
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
                }),
        )
        .into_any_element()
}

fn reactions(rs: &[Reaction]) -> AnyElement {
    let mut row = div().mt(px(4.)).flex().flex_wrap().gap(px(4.));
    for r in rs {
        row = row.child(
            div()
                .h(px(24.))
                .px(px(6.))
                .rounded(px(8.))
                .flex()
                .items_center()
                .gap(px(6.))
                .border_1()
                .when(r.me, |d| d.bg(ca(0x5865F23D)).border_color(c(Palette::BLURPLE)))
                .when(!r.me, |d| d.bg(c(Palette::BG_SECONDARY)).border_color(gpui::transparent_black()))
                .hover(|s| s.border_color(c(0x4E5058)))
                .cursor_pointer()
                .child(div().text_size(px(16.)).child(r.emoji.clone()))
                .child(
                    div()
                        .text_size(px(14.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(c(if r.me { 0xDEE0FC } else { Palette::INTERACTIVE }))
                        .child(SharedString::from(r.count.to_string())),
                ),
        );
    }
    row.into_any_element()
}

fn composer(app: &PhoenixApp, _window: &mut Window, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let ch = app.selected_channel.and_then(|c| app.channels.get(&c));
    let placeholder: SharedString = match ch {
        Some(c) if c.guild.is_none() => format!("Message @{}", c.name).into(),
        Some(c) => format!("Message #{}", c.name).into(),
        None => "".into(),
    };
    let ic = c(Palette::INTERACTIVE);
    let banner: Option<(SharedString, bool)> = if app.editing.is_some() {
        Some(("Editing message — escape to cancel • enter to save".into(), true))
    } else {
        app.replying_to.and_then(|rid| {
            let e = app.messages.get(&app.selected_channel?)?;
            let m = e.msgs.iter().find(|m| m.id == rid)?;
            let u = app.users.get(&m.author)?;
            Some((format!("Replying to {}", u.display_name).into(), false))
        })
    };
    let placeholder_empty = app.composer.read(cx).value().is_empty();
    div()
        .flex_none()
        .px(px(16.))
        .child(
            div()
                .flex()
                .flex_col()
                .rounded(px(8.))
                .bg(c(Palette::INPUT_BG))
                .when_some(banner, |d, (label, edit)| {
                    d.child(
                        div()
                            .h(px(34.))
                            .px(px(16.))
                            .rounded_t(px(8.))
                            .bg(c(Palette::BG_SECONDARY))
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(14.))
                            .text_color(c(Palette::TEXT_MUTED))
                            .child(label)
                            .when(!edit, |d| {
                                d.child(
                                    icon_button("cancel-reply", Icon::Close, ic, 16.).on_click(cx.listener(
                                        |a, _, _, cx| {
                                            a.replying_to = None;
                                            cx.notify();
                                        },
                                    )),
                                )
                            }),
                    )
                })
                .child(
                    div()
                        .id("composer")
                        .min_h(px(44.))
                        .flex()
                        .items_center()
                        .capture_key_down(cx.listener(|a, ev: &KeyDownEvent, w, cx| {
                            if ev.keystroke.key == "up" && ev.keystroke.modifiers.number_of_modifiers() == 0
                                && a.edit_last(w, cx)
                            {
                                cx.stop_propagation();
                            }
                        }))
                        .child(div().pl(px(16.)).pr(px(16.)).child(icon_button("attach", Icon::PlusCircle, ic, 24.).mx(px(-4.))))
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
                                .child(icon_button("gif", Icon::Gif, ic, 24.))
                                .child(icon_button("sticker", Icon::Sticker, ic, 24.))
                                .child(icon_button("emoji", Icon::Emoji, ic, 24.)),
                        ),
                ),
        )
        .child(typing(app))
}

fn typing(app: &PhoenixApp) -> impl IntoElement {
    div()
        .h(px(24.))
        .flex()
        .items_center()
        .gap(px(6.))
        .text_size(px(12.))
        .text_color(c(Palette::TEXT_NORMAL))
        .when_some(app.typing_label.clone(), |d, label| {
            let dot = || div().size(px(6.)).rounded_full().bg(c(Palette::INTERACTIVE_HOVER));
            d.child(div().flex().gap(px(2.)).child(dot()).child(dot()).child(dot()))
                .child(div().font_weight(FontWeight::MEDIUM).child(label))
        })
}
