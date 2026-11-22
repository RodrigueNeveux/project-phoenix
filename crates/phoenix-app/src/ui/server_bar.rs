//! 72px guild rail.

use gpui::{
    AnyElement, App, Context, Entity, FontWeight, ImageSource, IntoElement, ParentElement,
    SharedString, Styled, div, img, list, prelude::*, px,
};

use crate::{
    app::PhoenixApp,
    assets::Icon,
    model::{Guild, ImageRef},
    theme::{DEFAULT_AVATAR, Palette, c, m},
    ui::{badge, icon},
};

const GROUP: &str = "guild-item";

pub fn render(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let entity = cx.entity();
    div()
        .w(px(m::SERVER_BAR_W))
        .flex_none()
        .h_full()
        .bg(c(Palette::BG_TERTIARY))
        .pt(px(12.))
        .child(
            list(app.guild_list.clone(), move |ix, _w, cx| item(&entity, ix, cx))
                .size_full(),
        )
}

fn item(entity: &Entity<PhoenixApp>, ix: usize, cx: &mut App) -> AnyElement {
    let app = entity.read(cx);
    let n = app.guilds.len();
    match ix {
        0 => slot(
            entity,
            "home",
            app.selected_guild.is_none(),
            false,
            0,
            Content::Home,
            "Direct Messages".into(),
            None,
        ),
        1 => separator(),
        i if i < n + 2 => {
            let g: &Guild = &app.guilds[i - 2];
            slot(
                entity,
                "guild",
                app.selected_guild == Some(g.id),
                g.unread,
                g.mentions,
                Content::Guild(g.icon.clone(), g.acronym.clone()),
                g.name.clone(),
                Some(g.id),
            )
        }
        i if i == n + 2 => separator(),
        i if i == n + 3 => action("add-server", Icon::Plus),
        _ => action("discover", Icon::Compass),
    }
}

enum Content {
    Home,
    Guild(ImageRef, SharedString),
}

#[allow(clippy::too_many_arguments)]
fn slot(
    entity: &Entity<PhoenixApp>,
    id: &'static str,
    selected: bool,
    unread: bool,
    mentions: u32,
    content: Content,
    _tooltip: SharedString,
    guild: Option<crate::model::Id>,
) -> AnyElement {
    let pill_h = if selected { 40. } else if unread { 8. } else { 0. };
    let is_home = matches!(content, Content::Home);
    let (base_bg, hover_bg) = match &content {
        Content::Home => (c(Palette::BG_PRIMARY), c(Palette::BLURPLE)),
        Content::Guild(ImageRef::Default(i), _) => {
            let _ = DEFAULT_AVATAR[*i as usize % 5];
            (c(Palette::BG_PRIMARY), c(Palette::BLURPLE))
        }
        Content::Guild(ImageRef::Url(_), _) => (c(Palette::BG_PRIMARY), c(Palette::BG_PRIMARY)),
    };
    let inner: AnyElement = match content {
        Content::Home => img(ImageSource::from("icon.png"))
            .size(px(30.))
            .into_any_element(),
        Content::Guild(ImageRef::Url(u), _) => img(u).size_full().into_any_element(),
        Content::Guild(_, acr) => div()
            .text_size(px(if acr.len() > 2 { 14. } else { 18. }))
            .font_weight(FontWeight::MEDIUM)
            .child(acr)
            .into_any_element(),
    };
    let e = entity.clone();
    let icon_box = div()
        .size(px(m::GUILD_ICON))
        .flex()
        .items_center()
        .justify_center()
        .overflow_hidden()
        .text_color(c(Palette::TEXT_NORMAL))
        .when(selected, |d| d.rounded(px(16.)).bg(hover_bg).text_color(gpui::white()))
        .when(!selected, |d| {
            d.rounded(px(24.))
                .bg(base_bg)
                .group_hover(GROUP, move |s| s.rounded(px(16.)).bg(hover_bg).text_color(gpui::white()))
        })
        .child(inner);
    div()
        .id((id, guild.map(|g| g.0).unwrap_or(0)))
        .group(GROUP)
        .w_full()
        .h(px(m::GUILD_ICON + 8.))
        .flex()
        .justify_center()
        .relative()
        .cursor_pointer()
        .on_click(move |_, _, cx| {
            e.update(cx, |app, cx| app.select_guild(guild, cx));
        })
        // Pill indicator
        .child(
            div()
                .absolute()
                .left_0()
                .top_0()
                .h(px(m::GUILD_ICON))
                .w(px(4.))
                .flex()
                .items_center()
                .child(
                    div()
                        .w(px(4.))
                        .h(px(pill_h))
                        .bg(gpui::white())
                        .rounded_r(px(4.))
                        .when(!selected, |d| d.group_hover(GROUP, |s| s.h(px(20.)))),
                ),
        )
        .child(
            div()
                .relative()
                .size(px(m::GUILD_ICON))
                .child(icon_box)
                .when(mentions > 0, |d| {
                    d.child(
                        div()
                            .absolute()
                            .right(px(-4.))
                            .bottom(px(-4.))
                            .p(px(3.))
                            .rounded(px(12.))
                            .bg(c(Palette::BG_TERTIARY))
                            .child(badge(mentions)),
                    )
                })
                .when(is_home && unread, |d| d),
        )
        .into_any_element()
}

fn separator() -> AnyElement {
    div()
        .w_full()
        .h(px(10.))
        .flex()
        .justify_center()
        .items_start()
        .child(div().w(px(32.)).h(px(2.)).rounded(px(1.)).bg(c(0x35363C)))
        .into_any_element()
}

fn action(id: &'static str, i: Icon) -> AnyElement {
    div()
        .id(id)
        .group(GROUP)
        .w_full()
        .h(px(m::GUILD_ICON + 8.))
        .flex()
        .justify_center()
        .cursor_pointer()
        .child(
            div()
                .size(px(m::GUILD_ICON))
                .rounded(px(24.))
                .bg(c(Palette::BG_PRIMARY))
                .text_color(c(Palette::GREEN))
                .flex()
                .items_center()
                .justify_center()
                .group_hover(GROUP, |s| s.rounded(px(16.)).bg(c(Palette::GREEN)).text_color(gpui::white()))
                .child(
                    icon(i, 24., c(Palette::GREEN))
                        .group_hover(GROUP, |s| s.text_color(gpui::white())),
                ),
        )
        .into_any_element()
}
