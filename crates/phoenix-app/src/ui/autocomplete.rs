//! Composer autocomplete: `:emoji`, `@mention` and `#channel` popups.
//! Up/Down move, Tab/Enter accept, Escape closes (routed by
//! `ui::pickers::route_key`).

use gpui::{
    AnyElement, App, Context, Focusable, FontWeight, ParentElement, SharedString, Styled, Window,
    div, prelude::*, px,
};

use crate::{
    app::PhoenixApp,
    model::{ChannelKind, Id},
    theme::{Palette, c, m as layout},
    ui::{
        avatar, icon,
        pickers::{
            Pickers,
            emoji::{self, name_score},
            grid::{PickItem, item_element},
            types::GuildEmoji,
        },
        sidebar::channel_icon,
    },
};

const MAX: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Emoji,
    Mention,
    Channel,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Emoji(PickItem),
    User(Id),
    Role { id: Id, name: SharedString, color: Option<u32> },
    Everyone(&'static str),
    Channel(Id),
}

#[derive(Default)]
pub struct Autocomplete {
    pub active: Option<Active>,
}

pub struct Active {
    pub kind: Kind,
    pub query: String,
    /// Byte range of the trigger token in the composer.
    pub start: usize,
    pub end: usize,
    pub items: Vec<Item>,
    pub selected: usize,
}

impl gpui::Global for Autocomplete {}

/// Token being typed right before the cursor: (kind, query, start byte).
pub fn token_at(text: &str, cursor: usize) -> Option<(Kind, String, usize)> {
    let cursor = cursor.min(text.len());
    if !text.is_char_boundary(cursor) {
        return None;
    }
    let before = &text[..cursor];
    let start = before.rfind(char::is_whitespace).map(|i| i + before[i..].chars().next().map_or(1, char::len_utf8)).unwrap_or(0);
    let tok = &before[start..];
    let mut chars = tok.chars();
    let kind = match chars.next()? {
        ':' => Kind::Emoji,
        '@' => Kind::Mention,
        '#' => Kind::Channel,
        _ => return None,
    };
    let q: String = chars.collect();
    match kind {
        // Discord waits for two characters and stops at the closing colon.
        Kind::Emoji if q.chars().count() < 2 || q.contains(':') => None,
        Kind::Mention | Kind::Channel if q.contains(['<', '>']) => None,
        _ => Some((kind, q, start)),
    }
}

pub fn dismiss(cx: &mut App) {
    if let Some(a) = cx.try_global::<Autocomplete>()
        && a.active.is_some()
    {
        cx.global_mut::<Autocomplete>().active = None;
    }
}

fn custom_emojis(app: &PhoenixApp) -> Vec<GuildEmoji> {
    let mut guilds: Vec<Id> = app.guilds.iter().map(|g| g.id).collect();
    if let Some(sel) = app.selected_guild {
        guilds.sort_by_key(|g| *g != sel);
    }
    guilds.into_iter().flat_map(|g| app.backend.guild_emojis(g)).collect()
}

pub fn suggestions(app: &PhoenixApp, kind: Kind, q: &str) -> Vec<Item> {
    let q = q.to_lowercase();
    match kind {
        Kind::Emoji => {
            let mut custom: Vec<(u8, GuildEmoji)> = custom_emojis(app)
                .into_iter()
                .filter_map(|e| name_score(&e.name.to_lowercase(), &q).map(|s| (s, e)))
                .collect();
            custom.sort_by_key(|(s, _)| *s);
            let mut out: Vec<Item> = custom.into_iter().take(MAX).map(|(_, e)| Item::Emoji(PickItem::Custom(e))).collect();
            out.extend(emoji::search(&q, MAX).into_iter().map(|i| Item::Emoji(PickItem::Unicode(i))));
            out.truncate(MAX);
            out
        }
        Kind::Mention => {
            let mut scored: Vec<(u8, SharedString, Item)> = Vec::new();
            let mut seen = std::collections::HashSet::new();
            let mut consider = |id: Id, scored: &mut Vec<(u8, SharedString, Item)>| {
                if !seen.insert(id) {
                    return;
                }
                let Some(u) = app.users.get(&id) else { return };
                let s = [u.display_name.to_lowercase(), u.name.to_lowercase()]
                    .iter()
                    .filter_map(|n| if q.is_empty() { Some(4) } else { name_score(n, &q) })
                    .min();
                if let Some(s) = s {
                    scored.push((s, u.display_name.clone(), Item::User(id)));
                }
            };
            if let Some(g) = app.selected_guild {
                for m in app.members.get(&g).into_iter().flatten() {
                    consider(m.user, &mut scored);
                }
                // Recent authors of the open channel.
                if let Some(msgs) = app.selected_channel.and_then(|c| app.messages.get(&c)) {
                    for m in msgs.msgs.iter().rev().take(200) {
                        consider(m.author, &mut scored);
                    }
                }
                for r in app.roles.get(&g).into_iter().flatten() {
                    if r.name == "@everyone" {
                        continue;
                    }
                    if let Some(s) = if q.is_empty() { Some(5) } else { name_score(&r.name.to_lowercase(), &q) } {
                        scored.push((s, r.name.clone(), Item::Role { id: r.id, name: r.name.clone(), color: r.color }));
                    }
                }
                for e in ["everyone", "here"] {
                    if e.starts_with(&q) {
                        scored.push((if q.is_empty() { 6 } else { 1 }, e.into(), Item::Everyone(e)));
                    }
                }
            } else if let Some(msgs) = app.selected_channel.and_then(|c| app.messages.get(&c)) {
                for m in msgs.msgs.iter().rev().take(200) {
                    consider(m.author, &mut scored);
                }
                consider(app.me.id, &mut scored);
            }
            scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
            scored.into_iter().take(MAX).map(|(_, _, i)| i).collect()
        }
        Kind::Channel => {
            let Some(g) = app.selected_guild else { return Vec::new() };
            let mut scored: Vec<(u8, i32, Item)> = app
                .guild_channels
                .get(&g)
                .into_iter()
                .flatten()
                .filter_map(|id| app.channels.get(id))
                .filter(|ch| ch.kind != ChannelKind::Category)
                .filter_map(|ch| {
                    let s = if q.is_empty() { Some(4) } else { name_score(&ch.name.to_lowercase(), &q) }?;
                    Some((s, ch.position, Item::Channel(ch.id)))
                })
                .collect();
            scored.sort_by_key(|(s, p, _)| (*s, *p));
            scored.into_iter().take(MAX).map(|(_, _, i)| i).collect()
        }
    }
}

/// Recompute after a composer edit.
pub fn refresh(app: &mut PhoenixApp, _window: &mut Window, cx: &mut Context<PhoenixApp>) {
    if !cx.has_global::<Autocomplete>() {
        cx.set_global(Autocomplete::default());
    }
    let (text, cursor) = {
        let s = app.composer.read(cx);
        (s.value().to_string(), s.cursor())
    };
    let next = token_at(&text, cursor).and_then(|(kind, query, start)| {
        let items = suggestions(app, kind, &query);
        (!items.is_empty()).then_some(Active { kind, query, start, end: cursor, items, selected: 0 })
    });
    let had = cx.global::<Autocomplete>().active.is_some();
    if had || next.is_some() {
        cx.global_mut::<Autocomplete>().active = next;
        cx.notify();
    }
}

pub fn insertion(app: &PhoenixApp, item: &Item, tone: u8) -> String {
    match item {
        Item::Emoji(PickItem::Unicode(ix)) => emoji::toned(*ix, tone),
        Item::Emoji(PickItem::Custom(e)) => e.token(),
        Item::User(id) => format!("<@{}>", id.0),
        Item::Role { id, .. } => format!("<@&{}>", id.0),
        Item::Everyone(e) => format!("@{e}"),
        Item::Channel(id) => {
            let _ = app;
            format!("<#{}>", id.0)
        }
    }
}

pub fn accept(app: &mut PhoenixApp, index: usize, window: &mut Window, cx: &mut Context<PhoenixApp>) {
    let Some(a) = cx.try_global::<Autocomplete>().and_then(|a| a.active.as_ref()) else { return };
    let Some(item) = a.items.get(index).cloned() else { return };
    let range = a.start..a.end;
    let tone = cx.try_global::<Pickers>().map(|p| p.prefs.tone).unwrap_or(0);
    if let (Item::Emoji(pi), true) = (&item, cx.has_global::<Pickers>()) {
        cx.global_mut::<Pickers>().prefs.record(pi);
    }
    let text = format!("{} ", insertion(app, &item, tone));
    cx.global_mut::<Autocomplete>().active = None;
    app.composer.update(cx, |s, cx| {
        s.set_selected_range(range, cx);
        s.replace(text, window, cx);
    });
    cx.notify();
}

/// Keys while the popup is shown and the composer has focus.
pub fn on_key(app: &mut PhoenixApp, key: &str, _shift: bool, window: &mut Window, cx: &mut Context<PhoenixApp>) -> bool {
    let Some(a) = cx.try_global::<Autocomplete>().and_then(|a| a.active.as_ref()) else { return false };
    if !app.composer.focus_handle(cx).is_focused(window) {
        return false;
    }
    let (n, sel) = (a.items.len(), a.selected);
    match key {
        "up" | "down" => {
            let next = if key == "up" { (sel + n - 1) % n } else { (sel + 1) % n };
            if let Some(a) = cx.global_mut::<Autocomplete>().active.as_mut() {
                a.selected = next;
            }
            cx.notify();
            true
        }
        "tab" | "enter" => {
            accept(app, sel, window, cx);
            true
        }
        "escape" => {
            dismiss(cx);
            cx.notify();
            true
        }
        _ => false,
    }
}

pub fn render(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    let a = cx.try_global::<Autocomplete>()?.active.as_ref()?;
    let tone = cx.try_global::<Pickers>().map(|p| p.prefs.tone).unwrap_or(0);
    let title: SharedString = match a.kind {
        Kind::Emoji => crate::t!("ac.emoji", q = a.query),
        Kind::Mention => if a.query.is_empty() { crate::t!("members.members") } else { crate::t!("ac.members", q = a.query) },
        Kind::Channel => if a.query.is_empty() { crate::t!("ac.text_channels") } else { crate::t!("ac.channels", q = a.query) },
    };
    let mut rows = div().flex().flex_col().p(px(8.)).pt(px(0.));
    for (i, item) in a.items.iter().enumerate() {
        let selected = i == a.selected;
        let (lead, main, hint): (AnyElement, SharedString, Option<SharedString>) = match item {
            Item::Emoji(pi) => (item_element(pi, tone, 20.), format!(":{}:", pi.name()).into(), None),
            Item::User(id) => {
                let u = app.users.get(id);
                (
                    avatar(u, 24.),
                    u.map(|u| u.display_name.clone()).unwrap_or_else(|| crate::t!("common.unknown")),
                    u.map(|u| u.name.clone()),
                )
            }
            Item::Role { name, color, .. } => (
                div().w(px(24.)).text_color(c(color.unwrap_or(Palette::TEXT_MUTED))).child("@").into_any_element(),
                format!("@{name}").into(),
                Some(crate::t!("ac.role")),
            ),
            Item::Everyone(e) => (
                div().w(px(24.)).child("@").into_any_element(),
                format!("@{e}").into(),
                Some(if *e == "here" { crate::t!("ac.here") } else { crate::t!("ac.everyone") }),
            ),
            Item::Channel(id) => {
                let ch = app.channels.get(id);
                let parent = ch.and_then(|c| c.parent).and_then(|p| app.channels.get(&p)).map(|p| p.name.to_uppercase().into());
                (
                    icon(channel_icon(ch.map_or(ChannelKind::Text, |c| c.kind)), 20., c(Palette::CHANNEL_ICON)).into_any_element(),
                    ch.map(|c| c.name.clone()).unwrap_or_default(),
                    parent,
                )
            }
        };
        rows = rows.child(
            div()
                .id(("ac", i))
                .h(px(36.))
                .px(px(8.))
                .rounded(px(4.))
                .flex()
                .items_center()
                .gap(px(10.))
                .cursor_pointer()
                .when(selected, |d| d.bg(c(Palette::BG_MOD_SELECTED)))
                .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
                .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(cx.listener(move |a, _, w, cx| accept(a, i, w, cx)))
                .child(lead)
                .child(
                    div()
                        .text_size(px(15.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(c(if selected { Palette::TEXT_HEADER } else { Palette::TEXT_NORMAL }))
                        .child(main),
                )
                .child(div().flex_1())
                .when_some(hint, |d, h| d.child(div().text_size(px(12.)).text_color(c(Palette::TEXT_MUTED)).child(h))),
        );
    }
    let left = layout::SERVER_BAR_W + layout::SIDEBAR_W + 16.;
    let right = 16. + if app.show_members && app.selected_guild.is_some() { layout::MEMBER_LIST_W } else { 0. };
    let bottom = 84. + if app.replying_to.is_some() || app.editing.is_some() { 34. } else { 0. };
    Some(
        div()
            .absolute()
            .left(px(left))
            .right(px(right))
            .bottom(px(bottom))
            .rounded(px(8.))
            .bg(c(Palette::BG_SECONDARY))
            .border_1()
            .border_color(c(Palette::BG_TERTIARY))
            .shadow_lg()
            .occlude()
            .child(
                div()
                    .h(px(32.))
                    .px(px(16.))
                    .flex()
                    .items_center()
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c(Palette::TEXT_MUTED))
                    .child(title.to_uppercase()),
            )
            .child(rows)
            .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens() {
        assert_eq!(token_at("hi :th", 6), Some((Kind::Emoji, "th".into(), 3)));
        assert_eq!(token_at("hi :t", 5), None);
        assert_eq!(token_at("hi :thumbsup: x", 13), None);
        assert_eq!(token_at("@", 1), Some((Kind::Mention, "".into(), 0)));
        assert_eq!(token_at("go #gen", 7), Some((Kind::Channel, "gen".into(), 3)));
        assert_eq!(token_at("go #gen now", 11), None);
        assert_eq!(token_at("é @bo", 6), Some((Kind::Mention, "bo".into(), 3)));
        assert_eq!(token_at("mail a@b", 8), None);
    }
}
