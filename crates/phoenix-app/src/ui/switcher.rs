//! Ctrl+K quick switcher over guilds, channels, DMs and users, with fuzzy
//! ranking. Prefixes narrow the search: `@` users/DMs, `#` text channels,
//! `!` voice channels, `*` servers.

use crate::theme::z;
use gpui::{
    Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, Subscription, Window,
    div, prelude::*,
};
use gpui_component::input::{Input, InputEvent, InputState};

use crate::{
    app::PhoenixApp,
    model::{ChannelKind, Id},
    theme::{Palette, c, ca},
    ui::{avatar, icon, sidebar::channel_icon},
};

const MAX: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Channel(Id),
    Dm(Id),
    Guild(Id),
    User(Id),
}

pub struct Switcher {
    pub input: gpui::Entity<InputState>,
    pub results: Vec<Target>,
    pub selected: usize,
    pub _sub: Subscription,
}

/// Fuzzy score of `q` (lowercase) in `hay`; higher is better.
pub fn fuzzy(hay: &str, q: &str) -> Option<i32> {
    if q.is_empty() {
        return Some(0);
    }
    let h = hay.to_lowercase();
    if h == q {
        return Some(1000);
    }
    if h.starts_with(q) {
        return Some(900 - (h.len() - q.len()).min(100) as i32);
    }
    if let Some(pos) = h.find(q) {
        let word_start = pos == 0 || !h.as_bytes()[pos - 1].is_ascii_alphanumeric();
        return Some(if word_start { 800 } else { 700 } - pos.min(100) as i32);
    }
    // Subsequence: reward consecutive runs and word starts, punish gaps.
    let hc: Vec<char> = h.chars().collect();
    let mut score = 300;
    let mut hi = 0;
    let mut prev: Option<usize> = None;
    for qc in q.chars() {
        let found = (hi..hc.len()).find(|&i| hc[i] == qc)?;
        if prev.is_some_and(|p| p + 1 == found) {
            score += 15;
        } else if found == 0 || !hc[found - 1].is_alphanumeric() {
            score += 10;
        } else {
            score -= (found - hi).min(10) as i32;
        }
        prev = Some(found);
        hi = found + 1;
    }
    Some(score)
}

pub fn compute(app: &PhoenixApp, raw: &str) -> Vec<Target> {
    let raw = raw.trim().to_lowercase();
    let (filter, q) = match raw.chars().next() {
        Some(p @ ('@' | '#' | '!' | '*')) => (Some(p), raw[1..].trim().to_string()),
        _ => (None, raw.clone()),
    };
    let recent: Vec<Id> = app.last_channel.values().copied().collect();
    let mut scored: Vec<(i32, Target)> = Vec::new();

    if q.is_empty() && filter.is_none() {
        // Recent channels first, then unread ones.
        for id in &recent {
            if let Some(ch) = app.channels.get(id) {
                let t = if ch.guild.is_some() { Target::Channel(*id) } else { Target::Dm(*id) };
                scored.push((200, t));
            }
        }
        for ch in app.channels.values().filter(|c| c.unread && c.kind != ChannelKind::Category && !recent.contains(&c.id)) {
            scored.push((100 + ch.mentions.min(50) as i32, Target::Channel(ch.id)));
        }
    } else {
        let want_channels = matches!(filter, None | Some('#') | Some('!'));
        if want_channels {
            for ch in app.channels.values().filter(|c| c.guild.is_some() && c.kind != ChannelKind::Category) {
                let voice = matches!(ch.kind, ChannelKind::Voice | ChannelKind::Stage);
                if (filter == Some('#') && voice) || (filter == Some('!') && !voice) {
                    continue;
                }
                if let Some(s) = fuzzy(&ch.name, &q) {
                    let bonus = if recent.contains(&ch.id) { 60 } else { 0 }
                        + if ch.unread { 20 } else { 0 }
                        + if ch.guild == app.selected_guild { 25 } else { 0 };
                    scored.push((s + bonus, Target::Channel(ch.id)));
                }
            }
        }
        if matches!(filter, None | Some('@')) {
            let mut dm_users = std::collections::HashSet::new();
            for id in &app.dms {
                let Some(ch) = app.channels.get(id) else { continue };
                if let Some(s) = fuzzy(&ch.name, &q) {
                    scored.push((s + 40 + if recent.contains(id) { 60 } else { 0 }, Target::Dm(*id)));
                    dm_users.insert(ch.name.to_lowercase());
                }
            }
            if !q.is_empty() {
                for u in app.users.values().filter(|u| u.id != app.me.id && !u.bot) {
                    if dm_users.contains(&u.display_name.to_lowercase()) || dm_users.contains(&u.name.to_lowercase()) {
                        continue;
                    }
                    let s = fuzzy(&u.display_name, &q).max(fuzzy(&u.name, &q));
                    if let Some(s) = s {
                        scored.push((s - 30, Target::User(u.id)));
                    }
                }
            }
        }
        if matches!(filter, None | Some('*')) {
            for g in &app.guilds {
                if let Some(s) = fuzzy(&g.name, &q) {
                    scored.push((s + 10, Target::Guild(g.id)));
                }
            }
        }
    }
    let name = |t: &Target| -> SharedString {
        match t {
            Target::Channel(id) | Target::Dm(id) => app.channels.get(id).map(|c| c.name.clone()).unwrap_or_default(),
            Target::Guild(id) => app.guilds.iter().find(|g| g.id == *id).map(|g| g.name.clone()).unwrap_or_default(),
            Target::User(id) => app.users.get(id).map(|u| u.display_name.clone()).unwrap_or_default(),
        }
    };
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| name(&a.1).cmp(&name(&b.1))));
    let mut out: Vec<Target> = Vec::new();
    for (_, t) in scored {
        if !out.contains(&t) {
            out.push(t);
        }
        if out.len() == MAX {
            break;
        }
    }
    out
}

pub fn open(app: &mut PhoenixApp, window: &mut Window, cx: &mut Context<PhoenixApp>) {
    if app.screen != crate::app::Screen::Main {
        return;
    }
    let input = cx.new(|cx| InputState::new(window, cx).placeholder(crate::t!("switcher.placeholder")));
    let sub = cx.subscribe_in(&input, window, |app, _, ev: &InputEvent, window, cx| match ev {
        InputEvent::Change => update(app, cx),
        InputEvent::PressEnter { .. } => {
            if let Some(t) = app.switcher.as_ref().and_then(|s| s.results.get(s.selected).copied()) {
                activate(app, t, window, cx);
            }
        }
        _ => {}
    });
    input.update(cx, |s, cx| s.focus(window, cx));
    app.switcher = Some(Switcher { input, results: Vec::new(), selected: 0, _sub: sub });
    update(app, cx);
}

pub fn update(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) {
    let Some(sw) = &app.switcher else { return };
    let q = sw.input.read(cx).value().to_string();
    let res = compute(app, &q);
    if let Some(sw) = &mut app.switcher {
        sw.results = res;
        sw.selected = 0;
    }
    cx.notify();
}

pub fn activate(app: &mut PhoenixApp, t: Target, window: &mut Window, cx: &mut Context<PhoenixApp>) {
    app.close_switcher(window, cx);
    match t {
        Target::Channel(id) | Target::Dm(id) => app.select_channel(id, cx),
        Target::Guild(id) => app.select_guild(Some(id), cx),
        Target::User(user) => {
            let fut = app.backend.open_dm(user);
            cx.spawn(async move |this, cx| {
                if let Ok(ch) = fut.await {
                    let _ = this.update(cx, |a, cx| a.select_channel(ch, cx));
                }
            })
            .detach();
        }
    }
}

/// Arrow keys while the switcher is open.
pub fn on_key(app: &mut PhoenixApp, key: &str, _window: &mut Window, cx: &mut Context<PhoenixApp>) -> bool {
    let Some(sw) = app.switcher.as_mut() else { return false };
    let n = sw.results.len();
    if n == 0 {
        return false;
    }
    match key {
        "up" => sw.selected = (sw.selected + n - 1) % n,
        "down" | "tab" => sw.selected = (sw.selected + 1) % n,
        _ => return false,
    }
    cx.notify();
    true
}

pub fn render(app: &PhoenixApp, sw: &Switcher, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let mut results = div().mt(z(8.)).flex().flex_col().max_h(z(400.)).overflow_hidden();
    if sw.results.is_empty() {
        results = results.child(
            div()
                .py(z(24.))
                .flex()
                .justify_center()
                .text_size(z(14.))
                .text_color(c(Palette::TEXT_MUTED))
                .child(crate::t!("switcher.cant_find")),
        );
    }
    for (i, t) in sw.results.iter().copied().enumerate() {
        let (lead, name, hint): (gpui::AnyElement, SharedString, SharedString) = match t {
            Target::Channel(id) => {
                let Some(ch) = app.channels.get(&id) else { continue };
                let guild: SharedString = ch
                    .guild
                    .and_then(|g| app.guilds.iter().find(|x| x.id == g))
                    .map(|g| g.name.to_uppercase().into())
                    .unwrap_or_default();
                (icon(channel_icon(ch.kind), 18., c(Palette::CHANNEL_ICON)).into_any_element(), ch.name.clone(), guild)
            }
            Target::Dm(id) => {
                let Some(ch) = app.channels.get(&id) else { continue };
                let user = app.users.values().find(|u| u.display_name == ch.name || u.name == ch.name);
                (avatar(user, 20.), ch.name.clone(), crate::t!("switcher.dm"))
            }
            Target::User(id) => {
                let u = app.users.get(&id);
                (
                    avatar(u, 20.),
                    u.map(|u| u.display_name.clone()).unwrap_or_default(),
                    u.map(|u| u.name.clone()).unwrap_or_default(),
                )
            }
            Target::Guild(id) => {
                let Some(g) = app.guilds.iter().find(|g| g.id == id) else { continue };
                let lead = match &g.icon {
                    crate::model::ImageRef::Url(u) => gpui::img(u.clone()).size(z(20.)).rounded(z(6.)).into_any_element(),
                    _ => div()
                        .size(z(20.))
                        .rounded(z(6.))
                        .bg(c(Palette::BG_PRIMARY))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(z(8.))
                        .child(g.acronym.clone())
                        .into_any_element(),
                };
                (lead, g.name.clone(), crate::t!("switcher.server"))
            }
        };
        let selected = i == sw.selected;
        results = results.child(
            div()
                .id(("sw", i))
                .h(z(34.))
                .px(z(8.))
                .rounded(z(4.))
                .flex()
                .items_center()
                .gap(z(8.))
                .cursor_pointer()
                .when(selected, |d| d.bg(c(Palette::BG_MOD_SELECTED)))
                .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
                .on_click(cx.listener(move |a, _, w, cx| activate(a, t, w, cx)))
                .child(lead)
                .child(
                    div()
                        .text_size(z(16.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(c(if selected { Palette::TEXT_HEADER } else { Palette::TEXT_NORMAL }))
                        .child(name),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .text_size(z(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(Palette::TEXT_MUTED))
                        .child(hint),
                ),
        );
    }
    div()
        .id("switcher-backdrop")
        .absolute()
        .inset_0()
        .bg(ca(0x000000B3))
        .flex()
        .justify_center()
        .items_start()
        .pt(z(120.))
        .on_click(cx.listener(|a, _, w, cx| a.close_switcher(w, cx)))
        .child(
            div()
                .id("switcher")
                .w(z(570.))
                .h_auto()
                .p(z(16.))
                .rounded(z(8.))
                .bg(c(Palette::BG_SECONDARY))
                .shadow_lg()
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .h(z(70.))
                        .px(z(16.))
                        .rounded(z(4.))
                        .bg(c(Palette::BG_TERTIARY))
                        .flex()
                        .items_center()
                        .text_size(z(20.))
                        .text_color(c(Palette::TEXT_NORMAL))
                        .child(Input::new(&sw.input).appearance(false).bordered(false)),
                )
                .child(results)
                .child(
                    div()
                        .mt(z(12.))
                        .text_size(z(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(Palette::TEXT_MUTED))
                        .child(crate::t!("switcher.protip")),
                ),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_ranking() {
        let exact = fuzzy("general", "general").unwrap();
        let prefix = fuzzy("general-chat", "general").unwrap();
        let inner = fuzzy("off-topic-general", "general").unwrap();
        let sub = fuzzy("gaming-news-release", "gnr").unwrap();
        assert!(exact > prefix && prefix > inner && inner > sub);
        assert!(fuzzy("random", "xyz").is_none());
        assert!(fuzzy("Rust Programming", "rp").is_some());
        assert!(fuzzy("rust-programming", "rp").unwrap() > fuzzy("superb", "rp").unwrap_or(0));
    }
}
