//! System message lines (joins, pins, boosts, thread created, calls...):
//! icon in the avatar gutter, muted text with the actor's name in white,
//! timestamp at the end.

use gpui::{AnyElement, Entity, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, prelude::*, px};

use super::icons::ChatIcon;
use crate::{
    app::PhoenixApp,
    model::{Message, MessageKind},
    richtext::RenderedMessage,
    theme::{Palette, c},
};

/// Stable pick among the join phrasings (by message id) so a row never
/// changes text between renders.
fn join_text(id: u64) -> (&'static str, &'static str) {
    const JOIN: [(&str, &str); 6] = [
        ("", " joined the server."),
        ("Welcome, ", ". Say hi!"),
        ("", " just showed up."),
        ("Everyone welcome ", "!"),
        ("", " hopped into the server."),
        ("Good to see you, ", "."),
    ];
    JOIN[(id >> 22) as usize % JOIN.len()]
}

pub fn is_system(kind: MessageKind) -> bool {
    !matches!(kind, MessageKind::Default | MessageKind::Reply)
}

pub fn render(app: &PhoenixApp, entity: &Entity<PhoenixApp>, msg: &Message, r: &RenderedMessage) -> Option<AnyElement> {
    if !is_system(msg.kind) {
        return None;
    }
    let name: SharedString = app
        .users
        .get(&msg.author)
        .map(|u| u.display_name.clone())
        .unwrap_or_else(|| "Someone".into());
    let (icon, color, before, after): (ChatIcon, u32, SharedString, SharedString) = match msg.kind {
        MessageKind::Join => {
            let (b, a) = join_text(msg.id.0);
            (ChatIcon::Join, Palette::GREEN, b.into(), a.into())
        }
        MessageKind::Pin => (ChatIcon::Join, Palette::INTERACTIVE, "".into(), " pinned a message to this channel.".into()),
        MessageKind::Boost(0) => (ChatIcon::Boost, 0xFF73FA, "".into(), " just boosted the server!".into()),
        MessageKind::Boost(t) => (
            ChatIcon::Boost,
            0xFF73FA,
            "".into(),
            format!(" just boosted the server! The server has reached Level {t}!").into(),
        ),
        MessageKind::ThreadCreated => (
            ChatIcon::Thread,
            Palette::INTERACTIVE,
            "".into(),
            format!(" started a thread: {}", msg.content).into(),
        ),
        MessageKind::RecipientAdd => (ChatIcon::Join, Palette::GREEN, "".into(), " added someone to the group.".into()),
        MessageKind::RecipientRemove => (ChatIcon::Join, Palette::RED, "".into(), " left the group.".into()),
        MessageKind::Call => (ChatIcon::Join, Palette::GREEN, "".into(), " started a call.".into()),
        MessageKind::ChannelNameChange => (
            ChatIcon::Edit,
            Palette::INTERACTIVE,
            "".into(),
            format!(" changed the channel name: {}", msg.content).into(),
        ),
        _ => (
            ChatIcon::More,
            Palette::INTERACTIVE,
            "".into(),
            if msg.content.is_empty() { "".into() } else { format!(": {}", msg.content).into() },
        ),
    };
    let thread = msg.thread.clone().filter(|_| msg.kind == MessageKind::ThreadCreated);
    let e = entity.clone();
    let line = div()
        .id(("sysmsg", msg.id.0))
        .relative()
        .pl(px(72.))
        .pr(px(48.))
        .py(px(2.))
        .mt(px(if r.group_start { 17. } else { 0. }))
        .min_h(px(22.))
        .flex()
        .flex_wrap()
        .items_center()
        .text_size(px(16.))
        .text_color(c(Palette::TEXT_MUTED))
        .hover(|s| s.bg(c(Palette::BG_MSG_HOVER)))
        .child(
            div()
                .absolute()
                .left(px(28.))
                .top(px(5.))
                .child(super::icons::icon(icon, 16., c(color))),
        )
        .child(before)
        .child(
            div()
                .font_weight(FontWeight::MEDIUM)
                .text_color(c(Palette::TEXT_HEADER))
                .cursor_pointer()
                .hover(|s| s.underline())
                .child(name),
        )
        .child(after)
        .child(
            div()
                .ml(px(6.))
                .text_size(px(12.))
                .text_color(c(Palette::TEXT_MUTED))
                .child(r.time_full.clone()),
        )
        .when_some(thread, |d, t| {
            d.child(
                div()
                    .id(("sys-thread", msg.id.0))
                    .ml(px(6.))
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c(Palette::TEXT_LINK))
                    .cursor_pointer()
                    .hover(|s| s.underline())
                    .on_click(move |_, w, cx| {
                        let t = t.clone();
                        e.update(cx, |a, cx| a.open_thread(t, w, cx));
                    })
                    .child("See All Threads"),
            )
        });
    Some(match r.day_divider.clone() {
        Some(label) => div().child(super::day_divider(label)).child(line).into_any_element(),
        None => line.into_any_element(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn join_text_is_stable() {
        assert_eq!(join_text(1 << 22), join_text(1 << 22));
        assert!(is_system(MessageKind::Join));
        assert!(!is_system(MessageKind::Reply));
    }
}
