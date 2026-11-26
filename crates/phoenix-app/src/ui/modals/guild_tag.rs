use gpui::{AnyElement, Context, FontWeight, Hsla, IntoElement, ParentElement, SharedString, Styled, div, img, prelude::*};

use super::{Modal, close_action};
use crate::{
    actions::{close_modal, open_modal},
    app::PhoenixApp,
    model::Id,
    theme::{c, generated::idx, tok, z},
    ui::{
        controls::{BtnKind, button},
        tooltip::Tipped as _,
    },
};

pub const FALLBACK_TAG: &str = "WUMP";

pub fn badge_url(guild: Id, badge: &str, size: u32) -> SharedString {
    format!("{}/clan-badges/{}/{badge}.png?size={size}", phoenix_api::models::CDN_BASE, guild.0).into()
}

pub fn tag_of(app: &PhoenixApp, guild: Id) -> Option<(String, Option<String>)> {
    let has_tags = app.backend.guild_features(guild).iter().any(|f| f == "GUILD_TAGS");
    has_tags.then(|| app.backend.guild_tag(guild)).flatten()
}

pub fn adopted(app: &PhoenixApp, guild: Id) -> bool {
    app.me.tag_guild == Some(guild) && app.me.tag.is_some()
}

pub struct ChipStyle {
    pub text: f32,
    pub line: f32,
    pub badge: f32,
    pub color: usize,
}

pub const MENU_CHIP: ChipStyle = ChipStyle { text: 12., line: 16., badge: 14., color: idx::TEXT_DEFAULT };
pub const ROW_CHIP: ChipStyle = ChipStyle { text: 14., line: 18., badge: 16., color: idx::INTERACTIVE_TEXT_DEFAULT };
pub const PREVIEW_CHIP: ChipStyle = ChipStyle { text: 12., line: 16., badge: 12., color: idx::TEXT_DEFAULT };

pub fn chip(guild: Id, tag: &str, badge: Option<&str>, s: &ChipStyle) -> gpui::Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .px(z(4.))
        .rounded(z(4.))
        .bg(c(tok(idx::BACKGROUND_MOD_STRONG)))
        .text_size(z(s.text))
        .line_height(z(s.line.max(16.)))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(c(tok(s.color)))
        .when_some(badge, |d, b| d.child(img(badge_url(guild, b, 32)).size(z(s.badge)).mr(z(2.)).flex_none()))
        .child(div().max_w(z(70.)).overflow_hidden().whitespace_nowrap().child(SharedString::from(tag.to_owned())))
}

impl PhoenixApp {
    pub fn open_guild_tag_adopt(&mut self, guild: Id, cx: &mut Context<Self>) {
        if tag_of(self, guild).is_some() {
            open_modal(Modal::GuildTagAdopt(guild), None);
            cx.notify();
        }
    }

    pub fn open_profile_settings(&mut self, cx: &mut Context<Self>) {
        crate::ui::settings::open(self, cx);
        if let Some(s) = self.settings.as_mut() {
            s.page = crate::ui::settings::Page::Profiles;
        }
        cx.notify();
    }
}

fn color(i: usize) -> Hsla {
    c(tok(i))
}

fn preview_message(avatar: Option<&std::path::PathBuf>, name: &'static str, name_color: usize, message: SharedString, deco: Option<AnyElement>, focused: bool) -> gpui::Div {
    let face = match avatar {
        Some(p) => img(p.clone()).size(z(40.)).rounded_full().into_any_element(),
        None => div().size(z(40.)).rounded_full().bg(color(idx::BACKGROUND_MOD_STRONG)).into_any_element(),
    };
    div()
        .flex()
        .when(!focused, |d| d.opacity(0.2))
        .child(div().size(z(40.)).mr(z(12.)).flex_none().rounded_full().overflow_hidden().child(face))
        .child(
            div()
                .flex()
                .flex_col()
                .justify_between()
                .min_w_0()
                .child(
                    div()
                        .flex()
                        .child(div().text_size(z(16.)).line_height(z(20.)).font_weight(FontWeight::SEMIBOLD).text_color(color(name_color)).child(name))
                        .children(deco.map(|d| div().ml(z(4.)).flex().items_center().child(d))),
                )
                .child(div().text_size(z(16.)).line_height(z(20.)).font_weight(FontWeight::MEDIUM).text_color(color(idx::TEXT_DEFAULT)).child(message)),
        )
}

pub fn preview(guild: Id, tag: &str, badge: Option<&str>) -> gpui::Div {
    let av = crate::icons::art::tag_adopt_avatars();
    let a = |i: usize| av.map(|a| &a[i]);
    let tag = if tag.is_empty() { FALLBACK_TAG } else { tag };
    div()
        .w(z(288.))
        .p(z(12.))
        .flex()
        .flex_col()
        .gap(z(12.))
        .rounded(z(4.))
        .bg(color(idx::BACKGROUND_BASE_LOWEST))
        .child(preview_message(a(0), "Roka", idx::BRAND_400, crate::t!("guild_tag.preview.roka"), None, false))
        .child(preview_message(a(1), "hongo", idx::GREEN_230, crate::t!("guild_tag.preview.hongo"), None, false))
        .child(preview_message(a(2), "Lily", idx::ORANGE_300, crate::t!("guild_tag.preview.lily"), Some(chip(guild, tag, badge, &PREVIEW_CHIP).into_any_element()), true))
        .child(preview_message(a(3), "sharon", idx::BRAND_400, crate::t!("guild_tag.preview.sharon"), None, false))
        .child(preview_message(a(2), "Lily", idx::ORANGE_300, crate::t!("guild_tag.preview.lily_again"), None, false))
}

pub fn render(app: &PhoenixApp, guild: Id, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let close = close_action(cx);
    let Some((tag, badge)) = tag_of(app, guild) else {
        return div().into_any_element();
    };
    let g = app.guilds.iter().find(|g| g.id == guild);
    let name: SharedString = g.map(|g| g.name.clone()).unwrap_or_default();
    let icon = crate::ui::invite::guild_icon(g.map(|g| &g.icon), &name, 32.);
    let done = adopted(app, guild);
    let row = div()
        .flex_1()
        .p(z(8.))
        .rounded(z(8.))
        .border_1()
        .border_color(color(idx::BORDER_SUBTLE))
        .bg(color(idx::BACKGROUND_BASE_LOWEST))
        .flex()
        .items_center()
        .justify_between()
        .min_w_0()
        .child(
            div().flex().items_center().min_w_0().child(div().flex_none().child(icon)).child(
                div().pl(z(8.)).min_w_0().flex_1().child(
                    div()
                        .mr(z(4.))
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .text_size(z(16.))
                        .line_height(z(20.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(color(idx::TEXT_STRONG))
                        .child(name.clone()),
                ),
            ),
        )
        .child(
            chip(guild, &tag, badge.as_deref(), &ROW_CHIP)
                .py(z(4.))
                .px(z(8.))
                .bg(color(idx::BACKGROUND_SURFACE_HIGH))
                .border_1()
                .border_color(color(idx::BORDER_STRONG)),
        );
    let adopt = button("tag-adopt", crate::t!("guild_tag.adopt"), BtnKind::Primary).w_full();
    let adopt = if done {
        div().id("tag-adopt-wrap").w_full().child(adopt.opacity(0.5).cursor_not_allowed()).tip(crate::t!("guild_tag.already")).into_any_element()
    } else {
        adopt
            .on_click(cx.listener(move |a, _, _, cx| {
                close_modal();
                a.blocked_write(crate::actions::guard::GuardedWrite::set_guild_identity(guild.0, true), cx);
                cx.notify();
            }))
            .into_any_element()
    };
    let edit = button("tag-edit-profile", crate::t!("guild_tag.edit_profile"), BtnKind::Secondary).w_full().on_click(cx.listener(|a, _, _, cx| {
        close_modal();
        a.open_profile_settings(cx);
    }));
    let left = div()
        .flex_1()
        .p(z(24.))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .child(
            div()
                .max_w(z(320.))
                .flex()
                .flex_col()
                .gap(z(24.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(z(4.))
                        .child(div().text_size(z(20.)).line_height(z(24.)).font_weight(FontWeight::SEMIBOLD).text_color(color(idx::TEXT_STRONG)).child(crate::t!("guild_tag.title")))
                        .child(div().text_size(z(16.)).line_height(z(20.)).text_color(color(idx::TEXT_DEFAULT)).child(crate::t!("guild_tag.body"))),
                )
                .child(div().flex().child(row))
                .child(div().flex().flex_col().gap(z(8.)).child(adopt).child(edit)),
        );
    let right = div()
        .flex_1()
        .p(z(24.))
        .pr(z(0.))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .bg(color(idx::BACKGROUND_BASE_LOWER))
        .child(preview(guild, &tag, badge.as_deref()));
    let close2 = close.clone();
    let card = div()
        .relative()
        .flex()
        .overflow_hidden()
        .rounded(z(12.))
        .bg(color(idx::BACKGROUND_SURFACE_HIGH))
        .border_1()
        .border_color(color(idx::BORDER_SUBTLE))
        .shadow(vec![gpui::BoxShadow {
            color: gpui::hsla(0., 0., 0., 0.24),
            offset: gpui::point(z(0.), z(12.)),
            blur_radius: z(24.),
            spread_radius: z(0.),
            inset: false,
        }])
        .child(left)
        .child(right)
        .child(
            div()
                .id("tag-modal-close")
                .absolute()
                .top(z(16.))
                .right(z(16.))
                .size(z(24.))
                .rounded(z(4.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(|s| s.bg(color(idx::CONTROL_ICON_ONLY_BACKGROUND_HOVER)))
                .on_click(move |_, w, cx| close(w, cx))
                .child(gpui::svg().path(crate::icons::path("close")).size(z(24.)).text_color(color(idx::CONTROL_ICON_ONLY_ICON_DEFAULT))),
        );
    crate::ui::modal::frame("tag-adopt-backdrop", close2, card)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_comes_from_the_clan_badge_cdn_path() {
        assert!(badge_url(Id(5), "4f56", 32).ends_with("/clan-badges/5/4f56.png?size=32"));
    }

    #[test]
    fn preview_strings_exist() {
        for k in ["guild_tag.preview.roka", "guild_tag.preview.hongo", "guild_tag.preview.lily", "guild_tag.preview.sharon", "guild_tag.preview.lily_again", "guild_tag.title", "guild_tag.body", "guild_tag.adopt", "guild_tag.already", "guild_tag.edit_profile", "guild_tag.menu"] {
            assert_ne!(crate::t!(k).as_ref(), k, "{k}");
        }
    }
}
