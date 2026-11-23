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
//! - sections "Exclusivité Nitro" (catalogue `premium_type` 2) and "Va jeter
//!   un œil à la Boutique" (other products of the kind), 16px apart, from
//!   `GET /collectibles-categories/v2`. Shop items preview only; Apply stays
//!   disabled for items you don't own (never buys).
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

/// Max tiles per shop section.
const SHOP_CAP: usize = 60;

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

/// Open the picker for `kind`, loading owned collectibles and the shop
/// catalogue the first time.
pub fn open(app: &mut PhoenixApp, kind: Kind, cx: &mut Cx) {
    let Some(e) = app.settings.as_mut().and_then(|s| s.profile.as_mut()) else { return };
    let (need, need_shop) = (e.collectibles.is_none(), e.shop.is_none());
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
    if need_shop {
        if let Some(e) = app.settings.as_mut().and_then(|s| s.profile.as_mut()) {
            e.shop = Some(Default::default());
        }
        let fut = app.backend.collectibles_shop();
        cx.spawn(async move |this, cx| {
            let Ok(v) = fut.await else { return };
            let _ = this.update(cx, |this, cx| {
                if let Some(e) = this.settings.as_mut().and_then(|s| s.profile.as_mut()) {
                    e.shop = Some(v);
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
    let shop = e.shop.as_ref();
    // The client virtualises these grids; we cap them to keep image loads bounded.
    fn of_kind<'a>(v: Option<&'a Vec<Collectible>>, kind: Kind, owned: &[&Collectible]) -> Vec<&'a Collectible> {
        v.into_iter().flatten().filter(|c| c.kind == kind && !owned.iter().any(|o| o.sku == c.sku)).take(SHOP_CAP).collect()
    }
    let nitro = of_kind(shop.map(|s| &s.nitro), kind, &owned);
    let store = of_kind(shop.map(|s| &s.shop), kind, &owned);
    let chosen = owned.iter().chain(&nitro).chain(&store).copied().find(|c| Some(c.sku) == p.sel);
    let owns_sel = p.sel.is_none_or(|s| owned.iter().any(|o| o.sku == s));
    let changed = e.equipped.get(kind).flatten() != p.sel && owns_sel;
    let pick = |sel: Option<u64>| move |a: &mut PhoenixApp, _: &gpui::ClickEvent, _: &mut gpui::Window, cx: &mut Cx| {
        with_picker(a, |e| {
            if let Some(p) = e.picker.as_mut() {
                p.sel = sel;
            }
        });
        cx.notify();
    };
    let row = kind == Kind::Nameplate;
    let item_tiles = |items: &[&Collectible], locked: bool, cx: &mut Cx| -> Vec<AnyElement> {
        items
            .iter()
            .map(|it| {
                let sz = if row { (224., 48.) } else { size };
                // Effect/frame thumbnails fill the 83x80 tile; decorations are 64px.
                let full = row || matches!(kind, Kind::Effect | Kind::Frame);
                tile(SharedString::from(format!("pk-item-{}", it.sku)), sz, p.sel == Some(it.sku))
                    .relative()
                    .children(it.preview.clone().map(|u| if full { img(u).size_full().rounded(z(4.)).into_any_element() } else { img(u).size(z(64.)).into_any_element() }))
                    .when(locked, |d| {
                        // iconBadge_: 20x20 circle, black, 12px white lock, top -6 right -4.
                        d.child(
                            div()
                                .absolute()
                                .top(z(-6.))
                                .right(z(-4.))
                                .size(z(20.))
                                .rounded_full()
                                .bg(gpui::black())
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(gpui::svg().path(crate::icons::path("lock")).size(z(12.)).text_color(gpui::white())),
                        )
                    })
                    .on_click(cx.listener(pick(Some(it.sku))))
                    .into_any_element()
            })
            .collect()
    };
    let mut grid: Vec<AnyElement> = vec![
        icon_tile("none", size, "deny", crate::t!("pk.none"), p.sel.is_none()).on_click(cx.listener(pick(None))).into_any_element(),
        icon_tile("shop", size, "shop", crate::t!("pk.shop"), false).into_any_element(),
    ];
    let owned_tiles = item_tiles(&owned, false, cx);
    let rows = if row { owned_tiles } else {
        grid.extend(owned_tiles);
        Vec::new()
    };
    let nitro_tiles = item_tiles(&nitro, true, cx);
    let store_tiles = item_tiles(&store, true, cx);
    // Section heading: h2 heading-sm/semibold 16/20 (nameplate: body-md/medium),
    // grid 4px below (effect/frame 8px: tiles at top 28 in section_cR4ic2), 12px gaps.
    let gap_below = if matches!(kind, Kind::Effect | Kind::Frame) { 8. } else { 4. };
    let heading = |key: &'static str| {
        div()
            .text_size(z(16.))
            .line_height(z(20.))
            .font_weight(if row { FontWeight::MEDIUM } else { FontWeight::SEMIBOLD })
            .text_color(c(tok(idx::TEXT_STRONG)))
            .mb(z(gap_below))
            .child(crate::t!(key))
    };
    let section = |key: &'static str, tiles: Vec<AnyElement>| {
        div()
            .mt(z(16.))
            .flex()
            .flex_col()
            .child(heading(key))
            .child(if row { div().flex().flex_col().gap(z(12.)) } else { div().flex().flex_wrap().gap(z(12.)) }.children(tiles))
    };
    let list = div()
        .id("pk-list")
        .w(z(if matches!(kind, Kind::Effect | Kind::Frame) { 298. } else { 262. }))
        .h_full()
        .overflow_y_scroll()
        .pb(z(16.))
        .flex()
        .flex_col()
        .child(heading(yours))
        .child(div().flex().flex_wrap().gap(z(12.)).children(grid))
        .when(row, |d| d.child(div().mt(z(12.)).flex().flex_col().gap(z(12.)).children(rows)))
        .when(!nitro_tiles.is_empty(), |d| d.child(section("pk.nitro", nitro_tiles)))
        .when(!store_tiles.is_empty(), |d| d.child(section("pk.browse", store_tiles)));
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
                .child(controls::button("pk-apply", crate::t!("pk.apply"), BtnKind::Primary).when(!changed, |b| b.opacity(0.5)).when(changed, |b| b.on_click(cx.listener(apply))))
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
