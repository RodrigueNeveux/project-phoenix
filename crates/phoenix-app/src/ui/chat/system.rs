//! System message lines (joins, pins, boosts, thread created, calls...):
//! icon in the avatar gutter, muted text with the actor's name in white,
//! timestamp at the end.

use gpui::{AnyElement, Entity, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, prelude::*};

use super::icons::ChatIcon;
use crate::{
    app::PhoenixApp,
    model::{Message, MessageKind},
    richtext::RenderedMessage,
    theme::{Palette, c, z},
};

/// Stable pick among the join phrasings (by message id) so a row never
/// changes text between renders.
fn join_text(id: u64) -> (SharedString, SharedString) {
    let n = (id >> 22) as usize % 6;
    (crate::i18n::t(&format!("sys.join{n}.before"), &[]), crate::i18n::t(&format!("sys.join{n}.after"), &[]))
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
        .unwrap_or_else(|| crate::t!("sys.someone"));
    let (icon, color, before, after): (ChatIcon, u32, SharedString, SharedString) = match msg.kind {
        MessageKind::Join => {
            let (b, a) = join_text(msg.id.0);
            (ChatIcon::Join, Palette::GREEN, b, a)
        }
        MessageKind::Pin => (ChatIcon::Join, Palette::INTERACTIVE, "".into(), crate::t!("sys.pin")),
        MessageKind::Boost(0) => (ChatIcon::Boost, 0xFF73FA, "".into(), crate::t!("sys.boost")),
        MessageKind::Boost(t) => (
            ChatIcon::Boost,
            0xFF73FA,
            "".into(),
            crate::t!("sys.boost_level", t = t),
        ),
        MessageKind::ThreadCreated => (
            ChatIcon::Thread,
            Palette::INTERACTIVE,
            "".into(),
            crate::t!("sys.thread", c = msg.content),
        ),
        MessageKind::RecipientAdd => (ChatIcon::Join, Palette::GREEN, "".into(), crate::t!("sys.recipient_add")),
        MessageKind::RecipientRemove => (ChatIcon::Join, Palette::RED, "".into(), crate::t!("sys.recipient_remove")),
        MessageKind::Call => (ChatIcon::Join, Palette::GREEN, "".into(), crate::t!("sys.call")),
        MessageKind::ChannelNameChange => (
            ChatIcon::Edit,
            Palette::INTERACTIVE,
            "".into(),
            crate::t!("sys.rename", c = msg.content),
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
        // systemMessage_ row: content at x+80; iconContainer_bLMKT5 72x22 at x+8, centred icon
        // (live dump ref-cache message-content-*.json, 2026-11-24).
        .pl(z(80.))
        .pr(z(48.))
        .py(z(2.))
        .mt(z(if r.group_start { 17. } else { 0. }))
        .min_h(z(22.))
        .flex()
        .flex_wrap()
        .items_center()
        .text_size(z(16.))
        .text_color(c(Palette::TEXT_MUTED))
        .hover(|s| s.bg(c(Palette::BG_MSG_HOVER)))
        .child(
            div()
                .absolute()
                .left(z(36.)) // 8 + (72 - 16) / 2
                .top(z(7.)) // container_bLMKT5 padding-top 2 + (22 - 16) / 2 + row py 2
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
                .ml(z(6.))
                .text_size(z(12.))
                .text_color(c(Palette::TEXT_MUTED))
                .child(r.time_full.clone()),
        )
        .when_some(thread, |d, t| {
            d.child(
                div()
                    .id(("sys-thread", msg.id.0))
                    .ml(z(6.))
                    .text_size(z(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c(Palette::TEXT_LINK))
                    .cursor_pointer()
                    .hover(|s| s.underline())
                    .on_click(move |_, w, cx| {
                        let t = t.clone();
                        e.update(cx, |a, cx| a.open_thread(t, w, cx));
                    })
                    .child(crate::t!("sys.see_threads")),
            )
        });
    let cta = (msg.kind == MessageKind::Join).then(|| welcome_cta(app, entity, msg)).flatten();
    let line: AnyElement = match cta {
        Some(b) => div().child(line).child(b).into_any_element(),
        None => line.into_any_element(),
    };
    Some(match r.day_divider.clone() {
        Some(label) => div().child(super::day_divider(label)).child(line).into_any_element(),
        None => line,
    })
}

/// Greeting stickers the client cycles through for "Wave to say hi!"
/// (id, is APNG); the rest are Lottie, which we can't draw yet.
const WAVE_STICKERS: [(u64, bool); 6] = [
    (749054660769218631, false),
    (751606379340365864, false),
    (754108890559283200, false),
    (781291131828699156, true),
    (816087792291282944, false),
    (819128604311027752, false),
];

/// Client pick: (our snowflake timestamp + message timestamp) % 6.
fn wave_sticker(me: u64, msg: u64) -> (u64, bool) {
    WAVE_STICKERS[((me >> 22) + (msg >> 22)) as usize % WAVE_STICKERS.len()]
}

/// welcomeCTA_eZQ1Gc (margin-bottom 2) under a join line: secondary button
/// with the greeting sticker as its icon. Hidden for bots, ourselves and
/// channels where we can't send.
fn welcome_cta(app: &PhoenixApp, entity: &Entity<PhoenixApp>, msg: &Message) -> Option<AnyElement> {
    let bot = app.users.get(&msg.author).is_some_and(|u| u.bot);
    let blocked = app.nav.extras.get(&msg.channel).is_some_and(|x| x.send_blocked);
    if bot || blocked || msg.author == app.me.id {
        return None;
    }
    let (sticker, apng) = wave_sticker(app.me.id.0, msg.id.0);
    let label = crate::t!("sys.wave");
    let button = if apng {
        let url = SharedString::from(format!("https://media.discordapp.net/stickers/{sticker}.png?size=40"));
        crate::ui::controls::button(("wave-cta", msg.id.0), SharedString::default(), crate::ui::controls::BtnKind::Secondary)
            .h(z(32.))
            .px(z(12.))
            .text_size(z(14.))
            .gap(z(4.))
            .child(gpui::img(url).flex_none().size(z(20.)))
            .child(label)
    } else {
        crate::ui::controls::button_icon(("wave-cta", msg.id.0), "sticker", label, crate::ui::controls::BtnKind::Secondary)
            .h(z(32.))
            .px(z(12.))
            .text_size(z(14.))
    };
    let (e, channel) = (entity.clone(), msg.channel);
    Some(
        div()
            .pl(z(80.))
            .pt(z(4.))
            .mb(z(2.)) // welcomeCTA_eZQ1Gc margin-bottom 2px
            .child(button.on_click(move |_, _, cx| {
                e.update(cx, |a, cx| crate::ui::pickers::detach(a.backend.send_sticker(channel, crate::model::Id(sticker)), cx));
            }))
            .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn join_text_is_stable() {
        assert_eq!(join_text(1 << 22), join_text(1 << 22));
        assert!(is_system(MessageKind::Join));
        assert!(!is_system(MessageKind::Reply));
        assert_eq!(wave_sticker(0, 0).0, 749054660769218631);
        assert_eq!(wave_sticker(3 << 22, 0), (781291131828699156, true));
    }
}
