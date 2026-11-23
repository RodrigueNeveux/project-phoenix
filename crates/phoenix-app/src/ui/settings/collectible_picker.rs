//! Profile editor collectible pickers: avatar decoration, profile effect,
//! profile frame and nameplate. Opening one loads the owned items
//! (`owned_collectibles`); "Apply" only changes the editor preview — nothing
//! is sent (equipping collectibles is not part of the profile save).
//!
//! Dumps `vanilla-profile-picker-{decoration,effect,frame,nameplate}`
//! (2026-11-23, vanilla client):
//! - modal_UlS6S9 root__KoJTP medium: 603 wide (effect/frame 666),
//!   radius 12, 1px --border-subtle, --background-base-lowest, shadow 0 4 8 .16;
//!   header padding 16 24 0, h1 heading-lg/semibold 20/24, 24px close.
//! - content padding 8 24 0: list (sections h2 heading-sm/semibold 16/20,
//!   gap 4 to the grid) + modalPreview_ (margin-top 24, gap 12).
//! - decorationGridItem_: 75x80 (effect/frame 83x80), radius 4, 12 apart,
//!   selected = 2px inset --brand-500 ring; "Aucun"/"Boutique" = 24px icon
//!   and a body-xs 12/16 label; preset images 64px. Nameplate: 2 tiles 105x80,
//!   then 224x48 rows.
//! - decorationPreview_ 208x208 r4 with the 120px avatar centred; small
//!   previews column 56 wide (40/32/40/32 avatars, 16 apart).
//! - footer padding 16 24, gap 8, reverse: Apply (md primary, .5 opacity
//!   until something changes) + Cancel (md secondary).

use gpui::{AnyElement, Context, Div, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, img, prelude::*};

use super::collectibles::{Collectible, Kind};
use crate::{
    app::PhoenixApp,
    theme::{c, generated::idx, tok, z},
    ui::controls::{self, BtnKind},
};

type Cx<'a> = Context<'a, PhoenixApp>;

/// Open picker state (inside the profile editor).
#[derive(Clone, Debug)]
pub struct PickerState {
    pub kind: Kind,
    /// Highlighted item: None = "None" tile.
    pub sel: Option<u64>,
}

/// Collectibles chosen with "Apply" (preview only).
#[derive(Clone, Debug, Default)]
pub struct Equipped {
    pub decoration: Option<Option<u64>>,
    pub effect: Option<Option<u64>>,
    pub frame: Option<Option<u64>>,
    pub nameplate: Option<Option<u64>>,
}

impl Equipped {
    pub fn get_mut(&mut self, k: Kind) -> &mut Option<Option<u64>> {
        match k {
            Kind::Decoration => &mut self.decoration,
            Kind::Effect => &mut self.effect,
            Kind::Frame => &mut self.frame,
            Kind::Nameplate => &mut self.nameplate,
        }
    }
    pub fn get(&self, k: Kind) -> Option<Option<u64>> {
        match k {
            Kind::Decoration => self.decoration,
            Kind::Effect => self.effect,
            Kind::Frame => self.frame,
            Kind::Nameplate => self.nameplate,
        }
    }
}

fn keys(k: Kind) -> (&'static str, &'static str) {
    match k {
        Kind::Decoration => ("pk.deco.title", "pk.deco.yours"),
        Kind::Effect => ("pk.effect.title", "pk.effect.yours"),
        Kind::Frame => ("pk.frame.title", "pk.frame.yours"),
        Kind::Nameplate => ("pk.plate.title", "pk.plate.yours"),
    }
}

/// Tile box (w, h) per kind (decorationGridItem_ in each dump).
fn tile_size(k: Kind) -> (f32, f32) {
    match k {
        Kind::Decoration => (75., 80.),
        Kind::Effect | Kind::Frame => (83., 80.),
        Kind::Nameplate => (105., 80.),
    }
}

/// Open the picker for `kind`, loading owned collectibles the first time.
pub fn open(app: &mut PhoenixApp, kind: Kind, cx: &mut Cx) {
    let need = app.settings.as_ref().and_then(|s| s.profile.as_ref()).is_some_and(|e| e.collectibles.is_none());
    let Some(e) = app.settings.as_mut().and_then(|s| s.profile.as_mut()) else { return };
    let sel = e.equipped.get(kind).flatten();
    e.picker = Some(PickerState { kind, sel });
    if need {
        e.collectibles = Some(Vec::new());
        let fut = app.backend.owned_collectibles();
        cx.spawn(async move |this, cx| {
            let Ok(v) = fut.await else { return };
            let _ = this.update(cx, |this, cx| {
                if let Some(e) = this.settings.as_mut().and_then(|s| s.profile.as_mut()) {
                    e.collectibles = Some(v);
                }
                cx.notify();
            });
        })
        .detach();
    }
    cx.notify();
}

fn with_picker(a: &mut PhoenixApp, f: impl FnOnce(&mut super::profile_editor::EditorState)) {
    if let Some(e) = a.settings.as_mut().and_then(|s| s.profile.as_mut()) {
        f(e);
    }
}

fn tile(id: SharedString, (w, h): (f32, f32), selected: bool) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .w(z(w))
        .h(z(h))
        .flex_none()
        .rounded(z(4.))
        .bg(c(tok(idx::BACKGROUND_MOD_SUBTLE)))
        .border_2()
        .border_color(if selected { c(tok(idx::BRAND_500)) } else { gpui::transparent_black() })
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .overflow_hidden()
}

fn icon_tile(id: &str, size: (f32, f32), icon: &str, label: SharedString, selected: bool) -> gpui::Stateful<Div> {
    tile(SharedString::from(format!("pk-{id}")), size, selected)
        .child(gpui::svg().path(crate::icons::path(icon)).size(z(24.)).mb(z(4.)).text_color(c(tok(idx::TEXT_STRONG))))
        .child(div().text_size(z(12.)).line_height(z(16.)).text_color(c(tok(idx::TEXT_STRONG))).child(label))
}

fn preview(app: &PhoenixApp, kind: Kind, item: Option<&Collectible>) -> Div {
    let av = crate::ui::avatar(Some(&app.me), 120.);
    let overlay = item.and_then(|i| i.preview.clone());
    match kind {
        Kind::Decoration | Kind::Frame => div()
            .flex()
            .gap(z(12.))
            .child(
                div()
                    .size(z(208.))
                    .rounded(z(4.))
                    .bg(c(tok(idx::BACKGROUND_BASE_LOWEST)))
                    .relative()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(av)
                    // Decoration art is 1.2x the avatar (avatarDecoration_ size).
                    .children(overlay.clone().map(|u| div().absolute().top(z(32.)).left(z(32.)).size(z(144.)).child(img(u).size(z(144.))))),
            )
            .when(kind == Kind::Decoration, |d| {
                d.child(
                    div()
                        .w(z(56.))
                        .h(z(208.))
                        .p(z(8.))
                        .rounded(z(4.))
                        .bg(c(tok(idx::BACKGROUND_BASE_LOWEST)))
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(z(16.))
                        .children([40., 32., 40., 32.].into_iter().map(|s| {
                            div()
                                .relative()
                                .size(z(s))
                                .child(crate::ui::avatar(Some(&app.me), s))
                                .children(overlay.clone().map(|u| div().absolute().top(z(-s * 0.1)).left(z(-s * 0.1)).size(z(s * 1.2)).child(img(u).size(z(s * 1.2)))))
                        })),
                )
            }),
        Kind::Effect => div()
            .w(z(300.))
            .h(z(330.))
            .rounded(z(8.))
            .bg(c(tok(idx::BACKGROUND_SURFACE_HIGH)))
            .overflow_hidden()
            .relative()
            .children(overlay.map(|u| img(u).size_full())),
        Kind::Nameplate => div()
            .w(z(276.))
            .h(z(190.))
            .rounded(z(8.))
            .bg(c(tok(idx::BACKGROUND_BASE_LOWEST)))
            .flex()
            .flex_col()
            .justify_center()
            .px(z(32.))
            .child(
                div()
                    .relative()
                    .h(z(42.))
                    .rounded(z(4.))
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .gap(z(8.))
                    .px(z(8.))
                    .children(overlay.map(|u| div().absolute().inset_0().child(img(u).size_full())))
                    .child(crate::ui::avatar(Some(&app.me), 32.))
                    .child(div().text_size(z(16.)).font_weight(FontWeight::MEDIUM).text_color(c(tok(idx::TEXT_STRONG))).child(app.me.display_name.clone())),
            ),
    }
}

pub fn render(app: &PhoenixApp, e: &super::profile_editor::EditorState, cx: &mut Cx) -> Option<AnyElement> {
    let p = e.picker.as_ref()?;
    let kind = p.kind;
    let (title, yours) = keys(kind);
    let size = tile_size(kind);
    let owned: Vec<&Collectible> = e.collectibles.iter().flatten().filter(|c| c.kind == kind).collect();
    let chosen = owned.iter().copied().find(|c| Some(c.sku) == p.sel);
    let changed = e.equipped.get(kind).flatten() != p.sel;
    let pick = |sel: Option<u64>| move |a: &mut PhoenixApp, _: &gpui::ClickEvent, _: &mut gpui::Window, cx: &mut Cx| {
        with_picker(a, |e| {
            if let Some(p) = e.picker.as_mut() {
                p.sel = sel;
            }
        });
        cx.notify();
    };
    let mut grid: Vec<AnyElement> = vec![
        icon_tile("none", size, "deny", crate::t!("pk.none"), p.sel.is_none()).on_click(cx.listener(pick(None))).into_any_element(),
        icon_tile("shop", size, "shop", crate::t!("pk.shop"), false).into_any_element(),
    ];
    let row = kind == Kind::Nameplate;
    let items: Vec<AnyElement> = owned
        .iter()
        .map(|it| {
            let sz = if row { (224., 48.) } else { size };
            let img_sz = if row { 0. } else { 64. };
            tile(SharedString::from(format!("pk-item-{}", it.sku)), sz, p.sel == Some(it.sku))
                .children(it.preview.clone().map(|u| if row { img(u).size_full().into_any_element() } else { img(u).size(z(img_sz)).into_any_element() }))
                .on_click(cx.listener(pick(Some(it.sku))))
                .into_any_element()
        })
        .collect();
    let rows = if row { items } else {
        grid.extend(items);
        Vec::new()
    };
    let list = div()
        .id("pk-list")
        .w(z(if matches!(kind, Kind::Effect | Kind::Frame) { 298. } else { 262. }))
        .h_full()
        .overflow_y_scroll()
        .pb(z(16.))
        .flex()
        .flex_col()
        .gap(z(4.))
        .child(div().text_size(z(16.)).line_height(z(20.)).font_weight(FontWeight::SEMIBOLD).text_color(c(tok(idx::TEXT_STRONG))).child(crate::t!(yours)))
        .child(div().flex().flex_wrap().gap(z(12.)).children(grid))
        .when(row, |d| d.child(div().mt(z(12.)).flex().flex_col().gap(z(12.)).children(rows)));
    let close = |a: &mut PhoenixApp, _: &gpui::ClickEvent, _: &mut gpui::Window, cx: &mut Cx| {
        with_picker(a, |e| e.picker = None);
        cx.notify();
    };
    let apply = move |a: &mut PhoenixApp, _: &gpui::ClickEvent, _: &mut gpui::Window, cx: &mut Cx| {
        with_picker(a, |e| {
            if let Some(p) = e.picker.take() {
                *e.equipped.get_mut(p.kind) = Some(p.sel);
            }
        });
        cx.notify();
    };
    let w = if matches!(kind, Kind::Effect | Kind::Frame) { 666. } else { 603. };
    let modal = div()
        .id("pk-modal")
        .occlude()
        .w(z(w))
        .h(z(if matches!(kind, Kind::Effect | Kind::Frame) { 652. } else { 541. }))
        .rounded(z(12.))
        .border_1()
        .border_color(c(tok(idx::BORDER_SUBTLE)))
        .bg(c(tok(idx::BACKGROUND_BASE_LOWEST)))
        .shadow_md()
        .flex()
        .flex_col()
        .child(
            div()
                .pt(z(16.))
                .px(z(24.))
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_size(z(20.)).line_height(z(24.)).font_weight(FontWeight::SEMIBOLD).text_color(c(tok(idx::TEXT_STRONG))).child(crate::t!(title)))
                .child(
                    div()
                        .id("pk-close")
                        .size(z(24.))
                        .cursor_pointer()
                        .child(gpui::svg().path(crate::icons::path("close")).size(z(24.)).text_color(c(tok(idx::TEXT_STRONG))))
                        .on_click(cx.listener(close)),
                ),
        )
        .child(div().flex_1().min_h_0().pt(z(8.)).px(z(24.)).flex().gap(z(12.)).child(list).child(div().mt(z(24.)).child(preview(app, kind, chosen))))
        .child(
            div()
                .p(z(16.))
                .px(z(24.))
                .flex()
                .flex_row_reverse()
                .gap(z(8.))
                .child(controls::button("pk-apply", crate::t!("pk.apply"), BtnKind::Primary).when(!changed, |b| b.opacity(0.5)).on_click(cx.listener(apply)))
                .child(controls::button("pk-cancel", crate::t!("pk.cancel"), BtnKind::Secondary).on_click(cx.listener(close))),
        );
    Some(
        div()
            .id("pk-layer")
            .absolute()
            .inset_0()
            .bg(c(tok(idx::BACKGROUND_SCRIM)))
            .flex()
            .items_center()
            .justify_center()
            .on_click(cx.listener(close))
            .child(modal)
            .into_any_element(),
    )
}
