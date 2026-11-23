//! 72px guild rail: Home, guilds and guild folders (from `guild_folders`),
//! Add a Server (join by invite) and Discover. Guilds can be dragged to
//! reorder them locally (not synced to Discord yet).

use gpui::{
    AnyElement, App, Context, Entity, FontWeight, Hsla, ImageSource, IntoElement, ParentElement, Render,
    SharedString, Styled, Window, div, img, prelude::*, px,
};

use crate::{
    app::PhoenixApp,
    assets::Icon,
    model::{Guild, Id, ImageRef},
    theme::{Palette, c, m},
    ui::{badge, icon},
};

/// Folder colour when none is set (Discord's blurple-ish default).
const FOLDER_DEFAULT: u32 = 0x5865F2;

enum RailItem<'a> {
    Guild(&'a Guild, bool),
    Folder { id: u64, name: SharedString, color: u32, guilds: Vec<&'a Guild>, open: bool },
}

/// Guilds grouped by the user's folders. Guilds not in any folder (joined
/// since the settings were saved) come first, like the official client.
fn rail(app: &PhoenixApp) -> Vec<RailItem<'_>> {
    let by_id = |g: Id| app.guilds.iter().find(|x| x.id == g);
    let in_folders: std::collections::HashSet<Id> =
        app.nav.folders.iter().flat_map(|f| f.guilds.iter().copied()).collect();
    let mut out: Vec<RailItem> =
        app.guilds.iter().filter(|g| !in_folders.contains(&g.id)).map(|g| RailItem::Guild(g, false)).collect();
    for f in &app.nav.folders {
        let guilds: Vec<&Guild> = f.guilds.iter().filter_map(|g| by_id(*g)).collect();
        match f.id {
            Some(id) if !guilds.is_empty() => {
                let name = f.name.clone().unwrap_or_else(|| {
                    guilds.iter().map(|g| g.name.as_ref()).collect::<Vec<_>>().join(", ").into()
                });
                out.push(RailItem::Folder {
                    id,
                    name,
                    color: f.color.filter(|c| *c != 0).unwrap_or(FOLDER_DEFAULT),
                    open: app.nav.expanded_folders.contains(&id),
                    guilds,
                });
            }
            _ => out.extend(guilds.into_iter().map(|g| RailItem::Guild(g, false))),
        }
    }
    out
}

pub fn render(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let entity = cx.entity();
    let mut col = div()
        .id("guild-rail")
        .size_full()
        .overflow_y_scroll()
        .pt(px(12.))
        .pb(px(12.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(home(app, &entity))
        .child(separator());
    for item in rail(app) {
        col = match item {
            RailItem::Guild(g, nested) => col.child(guild(app, &entity, g, nested)),
            RailItem::Folder { id, name, color, guilds, open } => {
                col.child(folder(app, &entity, id, name, color, &guilds, open))
            }
        };
    }
    let e = entity.clone();
    col = col
        .child(separator())
        .child(action("add-server", Icon::Plus).on_click(move |_, w, cx| e.update(cx, |a, cx| a.open_join_server(w, cx))))
        .child({
            let e = entity.clone();
            action("discover", Icon::Compass).on_click(move |_, _, cx| e.update(cx, |a, cx| a.stub("Discover", cx)))
        });
    div().w(px(m::SERVER_BAR_W)).flex_none().h_full().bg(c(Palette::BG_TERTIARY)).child(col)
}

/// White pill on the left edge: 40px selected, 8px unread, 20px on hover.
fn pill(group: SharedString, selected: bool, unread: bool) -> impl IntoElement {
    let h = if selected {
        40.
    } else if unread {
        8.
    } else {
        0.
    };
    div().absolute().left_0().top_0().h(px(m::GUILD_ICON)).w(px(4.)).flex().items_center().child(
        div()
            .w(px(4.))
            .h(px(h))
            .bg(gpui::white())
            .rounded_r(px(4.))
            .when(!selected, |d| d.group_hover(group, |s| s.h(px(20.)))),
    )
}

fn mention_badge(n: u32) -> impl IntoElement {
    div().absolute().right(px(-4.)).bottom(px(-4.)).p(px(3.)).rounded(px(12.)).bg(c(Palette::BG_TERTIARY)).child(badge(n))
}

fn home(app: &PhoenixApp, entity: &Entity<PhoenixApp>) -> AnyElement {
    let selected = app.selected_guild.is_none();
    let unread_dms: u32 = app.dms.iter().filter_map(|d| app.channels.get(d)).map(|c| c.mentions).sum();
    let pending = app.nav.count(crate::ui::home::data::RelationshipKind::Incoming) as u32;
    let e = entity.clone();
    let group: SharedString = "rail-home".into();
    div()
        .id("home")
        .group(group.clone())
        .w_full()
        .h(px(m::GUILD_ICON))
        .flex()
        .justify_center()
        .relative()
        .cursor_pointer()
        .on_click(move |_, _, cx| e.update(cx, |a, cx| a.select_guild(None, cx)))
        .child(pill(group.clone(), selected, false))
        .child(
            div()
                .relative()
                .size(px(m::GUILD_ICON))
                .child(
                    div()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .when(selected, |d| d.rounded(px(16.)).bg(c(Palette::BLURPLE)))
                        .when(!selected, |d| {
                            d.rounded(px(24.))
                                .bg(c(Palette::BG_PRIMARY))
                                .group_hover(group, |s| s.rounded(px(16.)).bg(c(Palette::BLURPLE)))
                        })
                        .child(img(ImageSource::from("icon.png")).size(px(30.))),
                )
                .when(unread_dms + pending > 0, |d| d.child(mention_badge(unread_dms + pending))),
        )
        .into_any_element()
}

fn guild_inner(g: &Guild, size: f32) -> AnyElement {
    match &g.icon {
        ImageRef::Url(u) => img(u.clone()).size(px(size)).into_any_element(),
        _ => div()
            .text_size(px(if size < 30. {
                7.
            } else if g.acronym.len() > 2 {
                14.
            } else {
                18.
            }))
            .font_weight(FontWeight::MEDIUM)
            .child(g.acronym.clone())
            .into_any_element(),
    }
}

/// Dragged guild + its preview.
#[derive(Clone)]
pub struct DraggedGuild {
    pub id: Id,
    icon: ImageRef,
    acronym: SharedString,
}

impl Render for DraggedGuild {
    fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let inner: AnyElement = match &self.icon {
            ImageRef::Url(u) => img(u.clone()).size_full().into_any_element(),
            _ => div().text_size(px(16.)).child(self.acronym.clone()).into_any_element(),
        };
        div()
            .size(px(m::GUILD_ICON))
            .rounded(px(16.))
            .overflow_hidden()
            .bg(c(Palette::BLURPLE))
            .text_color(gpui::white())
            .flex()
            .items_center()
            .justify_center()
            .opacity(0.8)
            .child(inner)
    }
}

impl PhoenixApp {
    /// Local reorder stub: move `dragged` just before `target` (and into the
    /// target's folder). Not yet persisted to Discord's settings.
    pub fn move_guild(&mut self, dragged: Id, target: Id, cx: &mut Context<Self>) {
        if dragged == target {
            return;
        }
        if let Some(from) = self.guilds.iter().position(|g| g.id == dragged) {
            let g = self.guilds.remove(from);
            let to = self.guilds.iter().position(|g| g.id == target).unwrap_or(self.guilds.len());
            self.guilds.insert(to, g);
        }
        for f in &mut self.nav.folders {
            f.guilds.retain(|g| *g != dragged);
        }
        if let Some(f) = self.nav.folders.iter_mut().find(|f| f.guilds.contains(&target)) {
            let at = f.guilds.iter().position(|g| *g == target).unwrap_or(0);
            f.guilds.insert(at, dragged);
        }
        self.nav.folders.retain(|f| !f.guilds.is_empty());
        cx.notify();
    }
}

fn guild(app: &PhoenixApp, entity: &Entity<PhoenixApp>, g: &Guild, _nested: bool) -> AnyElement {
    let id = g.id;
    let selected = app.selected_guild == Some(id);
    let group: SharedString = format!("rail-{}", id.0).into();
    let (base_bg, hover_bg): (Hsla, Hsla) = match g.icon {
        ImageRef::Url(_) => (c(Palette::BG_PRIMARY), c(Palette::BG_PRIMARY)),
        _ => (c(Palette::BG_PRIMARY), c(Palette::BLURPLE)),
    };
    let e = entity.clone();
    let drop_e = entity.clone();
    let drag = DraggedGuild { id, icon: g.icon.clone(), acronym: g.acronym.clone() };
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
                .group_hover(group.clone(), move |s| s.rounded(px(16.)).bg(hover_bg).text_color(gpui::white()))
        })
        .child(guild_inner(g, m::GUILD_ICON));
    div()
        .id(("guild", id.0))
        .group(group.clone())
        .w_full()
        .h(px(m::GUILD_ICON))
        .flex()
        .justify_center()
        .relative()
        .cursor_pointer()
        .tooltip({
            let name = g.name.clone();
            move |window, cx| gpui_component::tooltip::Tooltip::new(name.clone()).build(window, cx)
        })
        .on_click(move |_, _, cx| e.update(cx, |app, cx| app.select_guild(Some(id), cx)))
        .on_mouse_down(gpui::MouseButton::Right, move |ev, w, _| {
            crate::ui::nav_menus::open(crate::ui::nav_menus::Target::Guild(id), ev.position, w)
        })
        .on_drag(drag, |d: &DraggedGuild, _, _, cx: &mut App| cx.new(|_| d.clone()))
        .drag_over::<DraggedGuild>(|s, _, _, _| s.border_t_2().border_color(c(Palette::GREEN)))
        .on_drop(move |d: &DraggedGuild, _, cx| drop_e.update(cx, |a, cx| a.move_guild(d.id, id, cx)))
        .child(pill(group, selected, g.unread))
        .child(div().relative().size(px(m::GUILD_ICON)).child(icon_box).when(g.mentions > 0, |d| d.child(mention_badge(g.mentions))))
        .into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn folder(
    app: &PhoenixApp,
    entity: &Entity<PhoenixApp>,
    id: u64,
    name: SharedString,
    color: u32,
    guilds: &[&Guild],
    open: bool,
) -> AnyElement {
    let tint = c(color).opacity(0.4);
    let unread = guilds.iter().any(|g| g.unread);
    let mentions: u32 = guilds.iter().map(|g| g.mentions).sum();
    let has_selected = guilds.iter().any(|g| Some(g.id) == app.selected_guild);
    let group: SharedString = format!("folder-{id}").into();
    let e = entity.clone();
    let head_inner: AnyElement = if open {
        // Open folder: folder glyph in the folder colour.
        div()
            .w(px(24.))
            .h(px(18.))
            .rounded(px(3.))
            .bg(c(color))
            .relative()
            .child(div().absolute().top(px(-3.)).left_0().w(px(10.)).h(px(5.)).rounded_t(px(2.)).bg(c(color)))
            .into_any_element()
    } else {
        // Closed folder: 2x2 grid of mini guild icons.
        let mut grid = div().size(px(36.)).flex().flex_wrap().gap(px(4.));
        for g in guilds.iter().take(4) {
            grid = grid.child(
                div()
                    .size(px(16.))
                    .rounded_full()
                    .overflow_hidden()
                    .bg(c(Palette::BG_PRIMARY))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(c(Palette::TEXT_NORMAL))
                    .child(guild_inner(g, 16.)),
            );
        }
        grid.into_any_element()
    };
    let header = div()
        .id(("folder", id))
        .group(group.clone())
        .w_full()
        .h(px(m::GUILD_ICON))
        .flex()
        .justify_center()
        .relative()
        .cursor_pointer()
        .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(name.clone()).build(window, cx))
        .on_click(move |_, _, cx| e.update(cx, |a, cx| a.toggle_folder(id, cx)))
        .child(pill(group.clone(), !open && has_selected, !open && unread))
        .child(
            div()
                .relative()
                .size(px(m::GUILD_ICON))
                .child(
                    div()
                        .size_full()
                        .rounded(px(16.))
                        .bg(if open { gpui::transparent_black() } else { tint })
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(head_inner),
                )
                .when(!open && mentions > 0, |d| d.child(mention_badge(mentions))),
        );
    if !open {
        return header.into_any_element();
    }
    let mut body = div().w_full().flex().flex_col().gap(px(8.)).pb(px(8.));
    for g in guilds {
        body = body.child(guild(app, entity, g, true));
    }
    div()
        .w_full()
        .relative()
        .child(
            // Tinted column behind the open folder.
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(px(12.))
                .w(px(m::GUILD_ICON))
                .rounded(px(16.))
                .bg(c(color).opacity(0.15)),
        )
        .child(div().flex().flex_col().gap(px(8.)).child(header).child(body))
        .into_any_element()
}

fn separator() -> AnyElement {
    div()
        .w_full()
        .h(px(2.))
        .flex_none()
        .flex()
        .justify_center()
        .child(div().w(px(32.)).h(px(2.)).rounded(px(1.)).bg(c(0x35363C)))
        .into_any_element()
}

fn action(id: &'static str, i: Icon) -> gpui::Stateful<gpui::Div> {
    let group: SharedString = format!("rail-{id}").into();
    div()
        .id(id)
        .group(group.clone())
        .w_full()
        .h(px(m::GUILD_ICON))
        .flex()
        .justify_center()
        .cursor_pointer()
        .child(
            div()
                .size(px(m::GUILD_ICON))
                .rounded(px(24.))
                .bg(c(Palette::BG_PRIMARY))
                .flex()
                .items_center()
                .justify_center()
                .group_hover(group.clone(), |s| s.rounded(px(16.)).bg(c(Palette::GREEN)))
                .child(icon(i, 24., c(Palette::GREEN)).group_hover(group, |s| s.text_color(gpui::white()))),
        )
}
