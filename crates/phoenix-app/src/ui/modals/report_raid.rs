use gpui::{AnyElement, Context, HighlightStyle, InteractiveText, IntoElement, ParentElement, SharedString, StyledText, Styled, div, prelude::*};

use super::{Modal, card_rich, close_action};
use crate::{
    actions::{close_modal, open_modal, with_modal},
    app::PhoenixApp,
    model::Id,
    theme::{c, generated::idx, tok, z},
    ui::controls::{BtnKind, button, checkbox_option},
};

pub const RAID_TYPES: [&str; 5] = ["MESSAGE_SPAM", "DM_SPAM", "MENTION_SPAM", "SUSPICIOUS_USERS", "SETTINGS_SPAM"];

const RAID_ARTICLE: &str = "10989121220631";

pub struct State {
    pub guild: Id,
    pub selected: Vec<&'static str>,
}

pub fn label(kind: &str) -> SharedString {
    match kind {
        "DM_SPAM" => crate::t!("raid.dm_spam"),
        "MENTION_SPAM" => crate::t!("raid.mention_spam"),
        "MESSAGE_SPAM" => crate::t!("raid.message_spam"),
        "SETTINGS_SPAM" => crate::t!("raid.settings_spam"),
        "SUSPICIOUS_USERS" => crate::t!("raid.suspicious_users"),
        _ => SharedString::default(),
    }
}

pub fn toggle(selected: &mut Vec<&'static str>, kind: &'static str) {
    if let Some(i) = selected.iter().position(|k| *k == kind) {
        selected.remove(i);
    } else {
        selected.push(kind);
    }
}

pub fn article_url() -> String {
    format!("https://support.discord.com/hc/{}/articles/{RAID_ARTICLE}", crate::i18n::locale().to_lowercase())
}

pub fn visible(app: &PhoenixApp, guild: Id) -> bool {
    use crate::actions::{MANAGE_GUILD, MODERATE_MEMBERS};
    use crate::ui::modals::server_settings::{BAN_MEMBERS, KICK_MEMBERS};
    let community = app.guilds.iter().any(|g| g.id == guild && g.community);
    community && crate::ui::inbox::has_perm(app.backend.guild_permissions(guild), BAN_MEMBERS | KICK_MEMBERS | MODERATE_MEMBERS | MANAGE_GUILD)
}

impl PhoenixApp {
    pub fn open_report_raid(&mut self, guild: Id, cx: &mut Context<Self>) {
        open_modal(Modal::ReportRaid(State { guild, selected: Vec::new() }), None);
        cx.notify();
    }
}

fn subtitle() -> AnyElement {
    let full = crate::t!("raid.subtitle").to_string();
    let link = crate::t!("raid.learn_more").to_string();
    let range = full.find(&link).map(|i| i..i + link.len());
    let mut styled = StyledText::new(SharedString::from(full));
    if let Some(r) = range.clone() {
        styled = styled.with_highlights([(r, HighlightStyle { color: Some(c(tok(idx::TEXT_LINK))), ..Default::default() })]);
    }
    let text = InteractiveText::new("raid-subtitle", styled);
    match range {
        Some(r) => text.on_click(vec![r], |_, _, cx| cx.open_url(&article_url())).into_any_element(),
        None => text.into_any_element(),
    }
}

pub fn render(s: &State, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let mut group = div().flex().flex_col().gap(z(16.)).pt(z(8.)).pb(z(4.));
    for (i, kind) in RAID_TYPES.into_iter().enumerate() {
        let on = s.selected.contains(&kind);
        group = group.child(checkbox_option(("raid-type", i), label(kind), on).on_click(cx.listener(move |_, _, _, cx| {
            with_modal(|m| {
                if let Some(Modal::ReportRaid(s)) = m {
                    toggle(&mut s.selected, kind);
                }
            });
            cx.notify();
        })));
    }
    let footer = div()
        .flex()
        .gap(z(8.))
        .child(button("raid-cancel", crate::t!("common.cancel"), BtnKind::Secondary).on_click({
            let close = close_action(cx);
            move |_, w, cx| close(w, cx)
        }))
        .child(button("raid-send", crate::t!("raid.send"), BtnKind::Primary).on_click(cx.listener(|a, _, _, cx| {
            let guild = with_modal(|m| match m {
                Some(Modal::ReportRaid(s)) => Some(s.guild),
                _ => None,
            });
            close_modal();
            if let Some(g) = guild {
                a.blocked_write(crate::actions::guard::GuardedWrite::report_raid(g.0), cx);
            }
            cx.notify();
        })));
    card_rich(480., crate::t!("raid.title"), Some(subtitle()), group.into_any_element(), Some(footer.into_any_element()), close_action(cx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raid_types_follow_the_client_order() {
        assert_eq!(RAID_TYPES, ["MESSAGE_SPAM", "DM_SPAM", "MENTION_SPAM", "SUSPICIOUS_USERS", "SETTINGS_SPAM"]);
        assert!(RAID_TYPES.iter().all(|k| !label(k).is_empty()));
    }

    #[test]
    fn toggling_adds_then_removes() {
        let mut sel = Vec::new();
        toggle(&mut sel, "DM_SPAM");
        toggle(&mut sel, "MESSAGE_SPAM");
        assert_eq!(sel, ["DM_SPAM", "MESSAGE_SPAM"]);
        toggle(&mut sel, "DM_SPAM");
        assert_eq!(sel, ["MESSAGE_SPAM"]);
    }

    #[test]
    fn article_points_at_the_raid_help_page() {
        assert!(article_url().ends_with("/articles/10989121220631"));
    }
}
