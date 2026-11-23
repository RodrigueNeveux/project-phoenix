//! Chat overlays and side panels: message context menu, delete confirm,
//! pinned-messages popout, thread panel, "new messages" / "jump to present"
//! bars, drag & drop target.

use std::rc::Rc;

use gpui::{
    AnyElement, Context, Entity, FontWeight, IntoElement, ParentElement, Styled, Window, div,
    prelude::*,
};
use gpui_component::input::Input;

use super::{
    icons::{ChatIcon, icon},
    state::{EmojiTarget, QUICK_REACTIONS},
};
use crate::{
    app::{PhoenixApp, format_time_full},
    assets::Icon,
    model::Message,
    theme::{Palette, c, ca, z},
    ui::{avatar, context_menu, icon_button, modal},
};

/// Everything drawn above the whole window (root `children` hook).
pub fn overlays(app: &PhoenixApp, _window: &mut Window, cx: &mut Context<PhoenixApp>) -> Vec<AnyElement> {
    let mut out = Vec::new();
    if let Some(m) = app.chat.menu {
        out.push(message_menu(app, m, cx));
    }
    if let Some((ch, id)) = app.chat.confirm_delete {
        out.push(delete_modal(app, ch, id, cx));
    }
    out
}

fn message_menu(app: &PhoenixApp, m: super::state::MenuState, cx: &mut Context<PhoenixApp>) -> AnyElement {
    use context_menu::MenuItem;
    let e = cx.entity();
    let (ch, id) = (m.channel, m.msg);
    let msg = app.messages.get(&ch).and_then(|x| x.msgs.iter().find(|x| x.id == id));
    let own = msg.is_some_and(|x| x.author == app.me.id);
    let pinned = msg.is_some_and(|x| x.pinned);
    let has_text = msg.is_some_and(|x| !x.content.is_empty());
    let ic = |i: ChatIcon| Some(i.path());

    // Vanilla order (vanilla-menu-message dump, 2026-11-23): 4 quick reactions
    // as 44px tiles, Add Reaction, View Reactions | Edit (own), Reply, Forward,
    // Create Thread | Copy Text, Pin, Bookmark, Remind Me >, Apps >, Mark
    // Unread, Copy Message Link, Speak Message | Delete, Report (others) |
    // Copy Message ID. Every row has a left icon.
    let has_reactions = msg.is_some_and(|x| !x.reactions.is_empty());
    let in_guild = app.channels.get(&ch).is_some_and(|c| c.guild.is_some());
    let stub = |key: &'static str, icon: &str| {
        let ee = e.clone();
        MenuItem::action(crate::t!(key), Some(crate::icons::path(icon)), move |_, _, cx| {
            ee.update(cx, |a, cx| a.stub(&crate::t!(key), cx))
        })
    };
    let mut strip = div().flex().gap(z(8.)).px(z(2.)).py(z(2.));
    for (i, q) in QUICK_REACTIONS.iter().take(4).enumerate() {
        let e = e.clone();
        strip = strip.child(
            div()
                .id(("menu-quick", i))
                .size(z(44.))
                .rounded(z(8.))
                .bg(crate::ui::chat::tk(crate::theme::generated::idx::BACKGROUND_MOD_SUBTLE))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(|s| s.bg(crate::ui::chat::tk(crate::theme::generated::idx::BACKGROUND_MOD_NORMAL)))
                .child(crate::media::emoji::emoji_element(q, 24.))
                .on_click(move |_, _, cx| {
                    e.update(cx, |a, cx| {
                        a.chat.menu = None;
                        a.react_with(ch, id, (*q).into(), None, cx);
                    })
                }),
        );
    }
    let mut items = vec![MenuItem::Custom(strip.into_any_element())];
    let ee = e.clone();
    items.push(MenuItem::action(crate::t!("msg.add_reaction"), ic(ChatIcon::AddReaction), move |_, _, cx| {
        ee.update(cx, |a, cx| a.open_emoji_picker(EmojiTarget::Reaction { channel: ch, msg: id }, cx))
    }));
    if has_reactions {
        let ee = e.clone();
        items.push(MenuItem::action(crate::t!("msg.view_reactions"), Some(crate::icons::path("chat/eye")), move |_, _, cx| {
            ee.update(cx, |a, cx| {
                a.chat.menu = None;
                a.open_reactions(ch, id, 0, cx)
            })
        }));
    }
    items.push(MenuItem::Separator);
    if own {
        let ee = e.clone();
        items.push(MenuItem::action(crate::t!("msg.edit"), ic(ChatIcon::Edit), move |_, w, cx| {
            ee.update(cx, |a, cx| a.start_edit(id, w, cx))
        }));
    }
    let ee = e.clone();
    items.push(MenuItem::action(crate::t!("msg.reply"), Some(Icon::Reply.path()), move |_, w, cx| {
        ee.update(cx, |a, cx| a.start_reply(id, w, cx))
    }));
    let ee = e.clone();
    items.push(MenuItem::action(crate::t!("msg.forward"), Some(crate::icons::path("nav/forward")), move |_, _, cx| {
        ee.update(cx, |a, cx| a.open_forward(ch, id, cx))
    }));
    if in_guild && msg.is_some_and(|m| m.thread.is_none()) {
        let ee = e.clone();
        items.push(MenuItem::action(crate::t!("msg.create_thread"), Some(crate::icons::path("chat/thread")), move |_, _, cx| {
            ee.update(cx, |a, cx| a.open_thread_form(ch, id, cx))
        }));
    }
    items.push(MenuItem::Separator);
    if has_text {
        let ee = e.clone();
        items.push(MenuItem::action(crate::t!("msg.copy_text"), ic(ChatIcon::Copy), move |_, _, cx| {
            ee.update(cx, |a, cx| a.copy_message_text(ch, id, cx))
        }));
    }
    let ee = e.clone();
    items.push(MenuItem::action(
        if pinned { crate::t!("msg.unpin") } else { crate::t!("msg.pin") },
        Some(Icon::Pin.path()),
        move |_, _, cx| ee.update(cx, |a, cx| a.toggle_pin(ch, id, cx)),
    ));
    let ee = e.clone();
    items.push(MenuItem::action(crate::t!("msg.bookmark"), Some(crate::icons::path("inbox/bookmark")), move |_, _, cx| {
        ee.update(cx, |a, cx| {
            a.chat.menu = None;
            a.save_message(ch, id, cx)
        })
    }));
    let mut reminders: Vec<MenuItem> = crate::actions::message::REMINDERS
        .iter()
        .map(|&(k, secs)| {
            let ee = e.clone();
            MenuItem::action(crate::t!(k), None, move |_, _, cx| {
                ee.update(cx, |a, cx| a.remind_message(ch, id, crate::actions::message::due_for(secs), cx))
            })
        })
        .collect();
    let ee = e.clone();
    reminders.push(MenuItem::action(crate::t!("msg.remind.custom"), None, move |_, _, cx| {
        ee.update(cx, |a, cx| a.open_custom_reminder(ch, id, cx))
    }));
    items.push(MenuItem::submenu_with_icon(crate::t!("msg.remind"), "inbox/clock", reminders));
    items.push(MenuItem::submenu_with_icon(crate::t!("menu.apps"), "apps", vec![stub("menu.apps_none", "apps")]));
    let ee = e.clone();
    items.push(MenuItem::action(crate::t!("msg.mark_unread"), ic(ChatIcon::Unread), move |_, _, cx| {
        ee.update(cx, |a, cx| a.mark_unread_from(ch, id, cx))
    }));
    let ee = e.clone();
    items.push(MenuItem::action(crate::t!("msg.copy_link"), ic(ChatIcon::Link), move |_, _, cx| {
        ee.update(cx, |a, cx| a.copy_message_link(ch, id, cx))
    }));
    items.push(stub("msg.speak", "speaker"));
    items.push(MenuItem::Separator);
    let guild = app.channels.get(&ch).and_then(|c| c.guild);
    if has_reactions && crate::actions::has(app, guild, crate::actions::MANAGE_MESSAGES | crate::actions::ADMINISTRATOR) {
        let emojis: Vec<MenuItem> = msg
            .map(|m| m.reactions.clone())
            .unwrap_or_default()
            .into_iter()
            .map(|r| {
                let ee = e.clone();
                let label: gpui::SharedString = format!("{} {}", r.emoji, r.count).into();
                MenuItem::action(label, None, move |_, _, cx| ee.update(cx, |a, cx| a.remove_reactions(ch, id, Some(r.clone()), cx)))
            })
            .collect();
        items.push(MenuItem::submenu(crate::t!("msg.remove_emoji_reactions"), emojis));
        let ee = e.clone();
        items.push(MenuItem::danger(crate::t!("msg.remove_all_reactions"), None, move |_, _, cx| {
            ee.update(cx, |a, cx| a.remove_reactions(ch, id, None, cx))
        }));
    }
    if own {
        let ee = e.clone();
        items.push(MenuItem::danger(crate::t!("msg.delete"), ic(ChatIcon::Trash), move |ev, _, cx| {
            let skip = ev.modifiers().shift;
            ee.update(cx, |a, cx| a.request_delete(ch, id, skip, cx))
        }));
    } else {
        let ee = e.clone();
        items.push(MenuItem::danger(crate::t!("msg.report"), Some(crate::icons::path("flag")), move |_, _, cx| {
            ee.update(cx, |a, cx| a.report_message(ch, id, cx))
        }));
    }
    if app.prefs.synced.developer_mode {
    items.push(MenuItem::Separator);
    items.push(MenuItem::action(crate::t!("msg.copy_id"), ic(ChatIcon::IdBadge), move |_, _, cx| {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(id.0.to_string()))
    }));
    }
    let ed = e.clone();
    let dismiss: context_menu::Dismiss = Rc::new(move |_, cx| {
        ed.update(cx, |a, cx| {
            a.chat.menu = None;
            cx.notify();
        })
    });
    context_menu::render(m.pos, items, dismiss)
}

/// Small message card used in the delete confirm and the pins popout.
fn message_card(app: &PhoenixApp, m: &Message) -> gpui::Div {
    let u = app.users.get(&m.author);
    div()
        .p(z(12.))
        .rounded(z(4.))
        .bg(c(Palette::BG_SECONDARY))
        .flex()
        .gap(z(12.))
        .child(avatar(u, 40.))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_baseline()
                        .gap(z(8.))
                        .child(
                            div()
                                .text_size(z(16.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(crate::theme::role_color(app.author_color(m.author)))
                                .child(u.map(|u| u.display_name.clone()).unwrap_or_else(|| crate::t!("common.unknown"))),
                        )
                        .child(
                            div()
                                .text_size(z(12.))
                                .text_color(c(Palette::TEXT_MUTED))
                                .child(format_time_full(m.timestamp)),
                        ),
                )
                .child(
                    div()
                        .text_size(z(16.))
                        .line_height(z(22.))
                        .text_color(c(Palette::TEXT_NORMAL))
                        .child(if m.content.is_empty() && !m.attachments.is_empty() {
                            crate::t!("msg.attachments", n = m.attachments.len())
                        } else {
                            m.content.clone()
                        }),
                ),
        )
}

fn delete_modal(app: &PhoenixApp, ch: crate::model::Id, id: crate::model::Id, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let e = cx.entity();
    let e2 = e.clone();
    let msg = app.messages.get(&ch).and_then(|x| x.msgs.iter().find(|x| x.id == id));
    let mut body: Vec<AnyElement> = vec![
        div()
            .text_size(z(16.))
            .child(crate::t!("msg.delete_confirm"))
            .into_any_element(),
    ];
    if let Some(m) = msg {
        body.push(message_card(app, m).shadow_md().into_any_element());
    }
    body.push(
        div()
            .flex()
            .flex_col()
            .gap(z(4.))
            .child(div().text_size(z(12.)).font_weight(FontWeight::BOLD).text_color(c(Palette::GREEN)).child(crate::t!("msg.protip")))
            .child(
                div()
                    .text_size(z(14.))
                    .text_color(c(Palette::TEXT_NORMAL))
                    .child(crate::t!("msg.protip_delete")),
            )
            .into_any_element(),
    );
    modal::confirm(modal::Confirm {
        title: crate::t!("msg.delete"),
        body,
        confirm_label: crate::t!("common.delete"),
        danger: true,
        on_confirm: Rc::new(move |_, cx| e.update(cx, |a, cx| a.confirm_delete(cx))),
        on_cancel: Rc::new(move |_, cx| {
            e2.update(cx, |a, cx| {
                a.chat.confirm_delete = None;
                cx.notify();
            })
        }),
    })
}

/// Pinned messages popout under the header pin icon (lives in `ui::inbox`).
pub fn pins_popout(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    crate::ui::inbox::pins_popout(app, cx)
}

/// Thread side panel (replaces the member list while open).
pub fn thread_panel(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    let t = app.chat.thread.as_ref()?;
    let entity: Entity<PhoenixApp> = cx.entity();
    let id = t.thread.id;
    let mut body = div().id("thread-msgs").flex_1().min_h_0().overflow_y_scroll().flex().flex_col().pb(z(8.));
    match app.messages.get(&id) {
        Some(entry) if entry.loaded_once => {
            for (m, r) in entry.msgs.iter().zip(entry.rendered.iter()) {
                body = body.child(super::message(app, &entity, m, r, entry, cx));
            }
        }
        _ => {
            body = body.child(div().p(z(16.)).text_color(c(Palette::TEXT_MUTED)).child(crate::t!("common.loading")));
        }
    }
    let empty = t.input.read(cx).value().is_empty();
    Some(
        div()
            .w(z(420.))
            .flex_none()
            .h_full()
            .border_l_1()
            .border_color(c(Palette::HEADER_SHADOW))
            .bg(c(Palette::BG_PRIMARY))
            .flex()
            .flex_col()
            .child(
                div()
                    .h(z(48.))
                    .flex_none()
                    .px(z(16.))
                    .flex()
                    .items_center()
                    .gap(z(8.))
                    .border_b_1()
                    .border_color(c(Palette::HEADER_SHADOW))
                    .child(icon(ChatIcon::Thread, 22., c(Palette::CHANNEL_ICON)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_ellipsis()
                            .text_size(z(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(c(Palette::TEXT_HEADER))
                            .child(t.thread.name.clone()),
                    )
                    .child(
                        icon_button("close-thread", Icon::Close, c(Palette::INTERACTIVE), 20.)
                            .on_click(cx.listener(|a, _, _, cx| a.close_thread(cx))),
                    ),
            )
            .child(body)
            .child(
                div().p(z(16.)).pt(z(0.)).child(
                    div()
                        .min_h(z(44.))
                        .px(z(16.))
                        .rounded(z(8.))
                        .bg(c(Palette::INPUT_BG))
                        .relative()
                        .flex()
                        .items_center()
                        .text_size(z(16.))
                        .child(div().flex_1().min_w_0().child(Input::new(&t.input).appearance(false).bordered(false)))
                        .when(empty, |d| {
                            d.child(
                                div()
                                    .absolute()
                                    .left(z(18.))
                                    .top_0()
                                    .bottom_0()
                                    .flex()
                                    .items_center()
                                    .text_color(c(0x6D6F78))
                                    .child(crate::t!("chat.placeholder_thread", name = t.thread.name)),
                            )
                        }),
                ),
            )
            .into_any_element(),
    )
}

/// Blurple "N new messages since ..." bar at the top of the list.
/// newMessagesBar_ barBase_: absolute, top 0, inset-inline --space-8 --space-16, height 32px,
/// padding-inline --space-16, --brand-500, radius 0 0 8px 8px, --elevation-low, white,
/// 1px --border-subtle border. barButtonMain_ (body-sm/medium 14px, ellipsis, flex 1) jumps
/// to the first unread; barButtonAlt_ (semibold, padding --space-4 --space-8, --radius-sm,
/// hover --control-secondary-background-hover) "Mark As Read" + ChatCheckIcon (barButtonIcon_
/// 16px, margin-inline-start 8px, top -1px).
pub fn new_messages_bar(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    use crate::{theme::generated::idx, ui::chat::tk};
    use gpui::px;
    let (n, ts) = app.chat.new_messages?;
    if app.chat.at_bottom {
        return None;
    }
    let since = format_time_full(ts);
    let text = if n == 1 { crate::t!("chat.new_since_one", since = since) } else { crate::t!("chat.new_since_many", n = n, since = since) };
    Some(
        div()
            .id("new-messages-bar")
            .absolute()
            .top_0()
            .left(z(8.))
            .right(z(16.))
            .h(z(32.))
            .px(z(16.))
            .rounded_b(z(8.))
            .border_1()
            .border_color(tk(idx::BORDER_SUBTLE))
            .bg(tk(idx::BRAND_500))
            .shadow(vec![gpui::BoxShadow {
                // --elevation-low: 0 1px 0 --opacity-black-20 (+ two 4% layers, too faint to draw).
                color: gpui::hsla(0., 0., 0., 0.2), // --opacity-black-20 (dark)
                offset: gpui::point(px(0.), px(1.)),
                blur_radius: px(0.),
                spread_radius: px(0.),
                inset: false,
            }])
            .flex()
            .items_center()
            .text_size(z(14.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(tk(idx::WHITE))
            .child(
                div()
                    .id("new-messages-jump")
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .items_center()
                    .pl(z(4.))
                    .pr(z(12.))
                    .cursor_pointer()
                    .on_click(cx.listener(|a, _, _, cx| a.jump_to_present(cx)))
                    .child(div().truncate().child(text)),
            )
            .child(
                div()
                    .id("mark-read")
                    .flex_none()
                    .flex()
                    .items_center()
                    .py(z(4.))
                    .pl(z(4.))
                    .pr(z(8.))
                    .rounded(z(8.)) // --radius-sm (:root 8px)
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .hover(|s| s.bg(tk(idx::CONTROL_SECONDARY_BACKGROUND_HOVER)))
                    .on_click(cx.listener(|a, _, _, cx| {
                        cx.stop_propagation();
                        a.mark_channel_read(cx)
                    }))
                    .child(crate::t!("menu.mark_read"))
                    .child(gpui::svg().path(crate::icons::path("chat-check")).flex_none().size(z(16.)).ml(z(8.)).mt(z(-1.)).text_color(tk(idx::WHITE))),
            )
            .into_any_element(),
    )
}

/// "You're viewing older messages — Jump To Present" above the composer.
pub fn jump_bar(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    let scrolled_up = !app.chat.at_bottom && app.message_list.is_scrolled_to_end() == Some(false);
    if !scrolled_up {
        return None;
    }
    Some(
        div()
            .id("jump-present")
            .absolute()
            .bottom(z(0.))
            .left(z(16.))
            .right(z(16.))
            .h(z(32.))
            .px(z(12.))
            .rounded_t(z(8.))
            .bg(c(Palette::BG_SECONDARY_ALT))
            .flex()
            .items_center()
            .justify_between()
            .text_size(z(14.))
            .text_color(c(Palette::TEXT_NORMAL))
            .child(crate::t!("chat.older"))
            .child(
                div()
                    .id("jump-present-btn")
                    .px(z(8.))
                    .h(z(24.))
                    .rounded(z(4.))
                    .bg(c(Palette::BLURPLE))
                    .flex()
                    .items_center()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(gpui::white())
                    .cursor_pointer()
                    .hover(|s| s.bg(c(0x4752C4)))
                    .on_click(cx.listener(|a, _, _, cx| a.jump_to_present(cx)))
                    .child(crate::t!("chat.jump_present")),
            )
            .into_any_element(),
    )
}

/// Full-area overlay shown while files are dragged over the chat.
pub fn drop_overlay(app: &PhoenixApp) -> impl IntoElement {
    let name = app.selected_channel.and_then(|c| app.channels.get(&c)).map(|c| c.name.clone()).unwrap_or_default();
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .invisible()
        .group_drag_over::<gpui::ExternalPaths>("chat-drop", |s| s.visible())
        .bg(ca(0x000000B3))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .w(z(310.))
                .p(z(8.))
                .rounded(z(10.))
                .bg(c(Palette::BLURPLE))
                .child(
                    div()
                        .p(z(16.))
                        .pt(z(48.))
                        .rounded(z(8.))
                        .border_2()
                        .border_dashed()
                        .border_color(ca(0xFFFFFF80))
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(z(8.))
                        .child(icon(ChatIcon::Upload, 48., gpui::white()))
                        .child(
                            div()
                                .text_size(z(20.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(gpui::white())
                                .child(crate::t!("chat.upload_to", name = name)),
                        )
                        .child(
                            div()
                                .text_size(z(14.))
                                .text_color(gpui::white())
                                .child(crate::t!("chat.upload_hint")),
                        ),
                ),
        )
}
