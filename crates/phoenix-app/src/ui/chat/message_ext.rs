//! Per-message interactive pieces: hover toolbar, reaction pills, inline
//! edit box, thread chip, "NEW" unread divider, right-click handler.

use gpui::{
    AnyElement, App, Div, Entity, FontWeight, IntoElement, MouseDownEvent, ParentElement,
    SharedString, Styled, Window, div, prelude::*, px,
};
use gpui_component::input::Input;

use super::{
    icons::{ChatIcon, icon},
    state::{EmojiTarget, QUICK_REACTIONS},
};
use crate::{
    app::PhoenixApp,
    assets::Icon,
    model::{Message, ThreadRef},
    theme::{Palette, c, ca},
    ui,
};

fn tool_button(id: &'static str, i: SharedString, size: f32) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .size(px(32.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .cursor_pointer()
        .hover(|s| s.bg(c(0x3D3E45)))
        .child(gpui::svg().path(i).size(px(size)).flex_none().text_color(c(Palette::INTERACTIVE)))
}

/// The floating hover toolbar (top-right of the hovered message).
pub fn toolbar(app: &PhoenixApp, entity: &Entity<PhoenixApp>, msg: &Message, shift: bool) -> impl IntoElement {
    let (ch, id) = (msg.channel, msg.id);
    let own = msg.author == app.me.id;
    let e = entity.clone();
    let mut bar = div()
        .absolute()
        .right(px(16.))
        .top(px(-16.))
        .invisible()
        .group_hover("msg", |s| s.visible())
        .when(app.chat.menu.is_some_and(|m| m.msg == id), |d| d.visible())
        .flex()
        .p(px(2.))
        .bg(c(Palette::BG_PRIMARY))
        .border_1()
        .border_color(c(Palette::HEADER_SHADOW))
        .rounded(px(4.))
        .shadow_sm();
    // Quick reactions (Discord shows frequent emoji first).
    for (i, q) in QUICK_REACTIONS.iter().take(3).enumerate() {
        let e = e.clone();
        bar = bar.child(
            div()
                .id(("quick", i))
                .size(px(32.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .cursor_pointer()
                .text_size(px(18.))
                .hover(|s| s.bg(c(0x3D3E45)))
                .child(*q)
                .on_click(move |_, _, cx| e.update(cx, |a, cx| a.react_with(ch, id, (*q).into(), None, cx))),
        );
    }
    bar = bar.child(div().w(px(1.)).my(px(6.)).mx(px(2.)).bg(c(Palette::DIVIDER)));
    bar = bar.child(tool_button("tb-react", ChatIcon::AddReaction.path(), 20.).on_click({
        let e = e.clone();
        move |_, _, cx| e.update(cx, |a, cx| a.open_emoji_picker(EmojiTarget::Reaction { channel: ch, msg: id }, cx))
    }));
    bar = bar.child(tool_button("tb-reply", Icon::Reply.path(), 20.).on_click({
        let e = e.clone();
        move |_, w, cx| e.update(cx, |a, cx| a.start_reply(id, w, cx))
    }));
    if own {
        bar = bar.child(tool_button("tb-edit", ChatIcon::Edit.path(), 18.).on_click({
            let e = e.clone();
            move |_, w, cx| e.update(cx, |a, cx| a.start_edit(id, w, cx))
        }));
    }
    if shift && own {
        bar = bar.child(
            div()
                .id("tb-delete")
                .size(px(32.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .cursor_pointer()
                .hover(|s| s.bg(c(0x3D3E45)))
                .child(icon(ChatIcon::Trash, 18., c(Palette::RED)))
                .on_click({
                    let e = e.clone();
                    move |ev, _, cx| {
                        let skip = ev.modifiers().shift;
                        e.update(cx, |a, cx| a.request_delete(ch, id, skip, cx))
                    }
                }),
        );
    }
    bar.child(tool_button("tb-more", ChatIcon::More.path(), 20.).on_click(move |ev, _, cx| {
        let pos = ev.position();
        e.update(cx, |a, cx| a.open_message_menu(ch, id, pos, cx))
    }))
}

pub fn on_right_click(entity: &Entity<PhoenixApp>, msg: &Message) -> impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static {
    let e = entity.clone();
    let (ch, id) = (msg.channel, msg.id);
    move |ev, _, cx| {
        cx.stop_propagation();
        let pos = ev.position;
        e.update(cx, |a, cx| a.open_message_menu(ch, id, pos, cx));
    }
}

/// Reaction pills; click toggles ours. Ends with an "add reaction" button
/// shown on hover.
pub fn reactions(app: &PhoenixApp, entity: &Entity<PhoenixApp>, msg: &Message) -> AnyElement {
    let _ = app;
    let (ch, id) = (msg.channel, msg.id);
    let mut row = div().mt(px(4.)).flex().flex_wrap().gap(px(4.));
    for (ix, r) in msg.reactions.iter().enumerate() {
        let e = entity.clone();
        let rc = r.clone();
        let custom = r.emoji.starts_with('<');
        let emoji_el: AnyElement = if custom {
            let id_str = r.emoji_id.map(|i| i.0.to_string()).unwrap_or_default();
            let ext = if r.emoji.starts_with("<a:") { "gif" } else { "webp" };
            gpui::img(SharedString::from(format!("https://cdn.discordapp.com/emojis/{id_str}.{ext}?size=48")))
                .size(px(16.))
                .into_any_element()
        } else {
            div().text_size(px(16.)).child(r.emoji.clone()).into_any_element()
        };
        row = row.child(
            div()
                .id(("reaction", ix))
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
                .child(emoji_el)
                .child(
                    div()
                        .text_size(px(14.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(c(if r.me { 0xDEE0FC } else { Palette::INTERACTIVE }))
                        .child(SharedString::from(r.count.to_string())),
                )
                .on_click(move |_, _, cx| e.update(cx, |a, cx| a.toggle_reaction(ch, id, &rc, cx))),
        );
    }
    let e = entity.clone();
    row.child(
        div()
            .id("reaction-add")
            .h(px(24.))
            .px(px(6.))
            .rounded(px(8.))
            .flex()
            .items_center()
            .bg(c(Palette::BG_SECONDARY))
            .invisible()
            .group_hover("msg", |s| s.visible())
            .cursor_pointer()
            .hover(|s| s.border_1().border_color(c(0x4E5058)))
            .child(icon(ChatIcon::AddReaction, 18., c(Palette::INTERACTIVE)))
            .on_click(move |_, _, cx| {
                e.update(cx, |a, cx| a.open_emoji_picker(EmojiTarget::Reaction { channel: ch, msg: id }, cx))
            }),
    )
    .into_any_element()
}

/// Inline edit box that replaces the message content.
pub fn edit_box(app: &PhoenixApp, entity: &Entity<PhoenixApp>, msg: &Message) -> Option<Div> {
    let ed = app.chat.edit.as_ref().filter(|e| e.id == msg.id)?;
    let e1 = entity.clone();
    let e2 = entity.clone();
    let link = |id: &'static str, label: &'static str| {
        div()
            .id(id)
            .text_color(c(Palette::TEXT_LINK))
            .cursor_pointer()
            .hover(|s| s.underline())
            .child(label)
    };
    Some(
        div()
            .flex()
            .flex_col()
            .min_w_0()
            .child(
                div()
                    .id("edit-box")
                    .mt(px(4.))
                    .min_h(px(44.))
                    .px(px(16.))
                    .rounded(px(8.))
                    .bg(c(Palette::INPUT_BG))
                    .flex()
                    .items_center()
                    .text_size(px(16.))
                    .text_color(c(Palette::TEXT_NORMAL))
                    .capture_key_down({
                        let e = entity.clone();
                        move |ev, w, cx| {
                            if ev.keystroke.key == "escape" {
                                cx.stop_propagation();
                                e.update(cx, |a, cx| a.cancel_edit(w, cx));
                            }
                        }
                    })
                    .child(div().flex_1().min_w_0().child(Input::new(&ed.input).appearance(false).bordered(false)))
                    .child(ui::icon(Icon::Emoji, 22., c(Palette::INTERACTIVE))),
            )
            .child(
                div()
                    .py(px(4.))
                    .flex()
                    .gap(px(4.))
                    .text_size(px(12.))
                    .text_color(c(Palette::TEXT_NORMAL))
                    .child("escape to ")
                    .child(link("edit-cancel", "cancel").on_click(move |_, w, cx| e1.update(cx, |a, cx| a.cancel_edit(w, cx))))
                    .child(" • enter to ")
                    .child(link("edit-save", "save").on_click(move |_, w, cx| e2.update(cx, |a, cx| a.save_edit(w, cx)))),
            ),
    )
}

/// "N Messages ›" chip under a message that started a thread.
pub fn thread_chip(app: &PhoenixApp, entity: &Entity<PhoenixApp>, t: &ThreadRef) -> AnyElement {
    let _ = app;
    let e = entity.clone();
    let t2 = t.clone();
    div()
        .id(("thread-chip", t.id.0))
        .mt(px(4.))
        .max_w(px(480.))
        .p(px(8.))
        .rounded(px(4.))
        .bg(c(Palette::BG_SECONDARY))
        .flex()
        .items_center()
        .gap(px(8.))
        .cursor_pointer()
        .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
        .child(icon(ChatIcon::Thread, 18., c(Palette::INTERACTIVE)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_ellipsis()
                .text_size(px(14.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c(Palette::TEXT_HEADER))
                .child(t.name.clone()),
        )
        .child(
            div()
                .text_size(px(14.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c(Palette::TEXT_LINK))
                .child(SharedString::from(match t.message_count {
                    1 => "1 Message".to_string(),
                    n => format!("{n} Messages"),
                })),
        )
        .child(icon(ChatIcon::ArrowRight, 14., c(Palette::TEXT_LINK)))
        .on_click(move |_, w, cx| {
            let t = t2.clone();
            e.update(cx, |a, cx| a.open_thread(t, w, cx));
        })
        .into_any_element()
}

/// Red "NEW" line drawn above the first unread message.
pub fn new_divider() -> impl IntoElement {
    div()
        .mx(px(16.))
        .mt(px(8.))
        .h(px(1.))
        .bg(c(Palette::RED))
        .relative()
        .child(
            div()
                .absolute()
                .right(px(-16.))
                .top(px(-8.))
                .h(px(16.))
                .px(px(4.))
                .rounded_l(px(4.))
                .bg(c(Palette::RED))
                .text_size(px(10.))
                .font_weight(FontWeight::BOLD)
                .text_color(gpui::white())
                .flex()
                .items_center()
                .child("NEW"),
        )
}

/// Click on an author's avatar or name: open the profile popout there.
pub fn open_author(entity: &Entity<PhoenixApp>, msg: &Message) -> impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static {
    let e = entity.clone();
    let (user, ch) = (msg.author, msg.channel);
    move |ev, w, cx| {
        let pos = ev.position();
        e.update(cx, |a, cx| {
            let guild = a.channels.get(&ch).and_then(|c| c.guild);
            a.open_profile(user, guild, pos, w, cx)
        });
    }
}
