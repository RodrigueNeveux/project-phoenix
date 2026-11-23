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
//! - effect/frame: preview = the full profile card (`profile_card`), tiles
//!   composed over a skeleton mini profile (`sample_profile`).
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

/// Skeleton mini profile drawn under effect thumbnails and frames (the
/// client's static `presetEffectBackground_` image, which we may not ship):
/// grey banner, avatar with ring and status cut-out, four text bars. Geometry
/// and colours measured on the 83x80 tiles of `vanilla-profile-picker-effect`
/// (#4845), scaled to `(w, h)`.
fn sample_profile(w: f32, h: f32) -> Div {
    let (sx, sy) = (w / 83., h / 80.);
    let bar = |x: f32, y: f32, bw: f32| div().absolute().left(z(x * sx)).top(z(y * sy)).w(z(bw * sx)).h(z(3.5 * sy)).rounded_full().bg(c(0x232326));
    div()
        .absolute()
        .inset_0()
        .rounded(z(4.))
        .overflow_hidden()
        .bg(c(0x0B0B0C))
        .child(div().absolute().top_0().left_0().right_0().h(z(29. * sy)).bg(c(0x242428)))
        .child(div().absolute().left(z(4. * sx)).top(z(15. * sy)).size(z(26. * sx)).rounded_full().bg(c(0x0B0B0C)))
        .child(div().absolute().left(z(6. * sx)).top(z(17. * sy)).size(z(22. * sx)).rounded_full().bg(c(0x3A3A3E)))
        .child(div().absolute().left(z(22. * sx)).top(z(33. * sy)).size(z(7. * sx)).rounded_full().bg(c(0x0B0B0C)))
        .child(bar(6., 48., 26.))
        .child(bar(6., 55.5, 47.))
        .child(bar(6., 68., 16.))
        .child(bar(6., 75., 61.))
}

/// Picker preview card (previewContainer_-JS7Po, dumps
/// `vanilla-profile-picker-{effect,frame}` #4980-#5021): 304x333 at +10 in the
/// preview column. outer_P1YEjQ: padding 2, r8, linear-gradient(primary 105px,
/// secondary), shadow 0 0 0 1 white/.08 + 0 12 24 black/.24; inner_ r6 with
/// the --profile-gradient-overlay-color (black/.6), padding-bottom 4, gap 8.
/// header_ 140: banner_ 105 + avatar_ 80 at (14, 59); body_NOlkhE padding
/// 4 8 8 16 gap 12: name heading-lg/semibold 20/24, username body-sm 14/18 +
/// tag chiplet 24 (padding 0 8, body-xs) + badges, bio body-sm 3 lines;
/// footer_ padding 0 16 12: sm primary "Example button" 268x31.
fn profile_card(app: &PhoenixApp, e: &super::profile_editor::EditorState, overlay: Option<SharedString>) -> Div {
    use super::profile_editor::{badges_row, bio_markup, gradient_button_color, hex, mix, white};
    let me = &app.me;
    let theme = e.theme;
    let banner_col = theme.map(|t| t.0).or(e.profile.as_ref().and_then(|p| p.accent)).unwrap_or(0x5865F2);
    let banner: AnyElement = match (&e.banner, e.profile.as_ref().and_then(|p| p.banner.clone())) {
        (Some(p), _) => img(p.clone()).w_full().h(z(105.)).object_fit(gpui::ObjectFit::Cover).into_any_element(),
        (None, Some(url)) => img(url).w_full().h(z(105.)).object_fit(gpui::ObjectFit::Cover).into_any_element(),
        _ => div().w_full().h(z(105.)).bg(hex(banner_col)).into_any_element(),
    };
    let ring = match theme {
        Some(t) => hex(mix(t.0, 0, 0.6)),
        None => c(tok(idx::BACKGROUND_SURFACE_HIGH)),
    };
    let (strong, outer): (gpui::Hsla, gpui::Background) = match theme {
        Some(t) => (white(0.96), gpui::linear_gradient(180., gpui::linear_color_stop(hex(t.0), 105. / 333.), gpui::linear_color_stop(hex(t.1), 1.))),
        None => (c(tok(idx::TEXT_STRONG)), c(tok(idx::BACKGROUND_SURFACE_HIGH)).into()),
    };
    let bio: SharedString = e.original.bio.clone().into();
    let tag = me.tag.clone().map(|(t, badge)| {
        // chipletContainerInner_k8jyN7: 24 high, padding 0 8, r8, 1px border;
        // badge_ 14, tagText_ body-xs 12/16, gap 2.
        div()
            .flex_none()
            .h(z(24.))
            .px(z(8.))
            .rounded(z(8.))
            .border_1()
            .border_color(c(tok(idx::BORDER_SUBTLE)))
            .bg(c(tok(idx::BACKGROUND_MOD_NORMAL)))
            .flex()
            .items_center()
            .gap(z(2.))
            .text_size(z(12.))
            .line_height(z(16.))
            .text_color(strong)
            .when_some(badge, |d, b| d.child(img(b).size(z(14.)).flex_none()))
            .child(t)
    });
    let body = div()
        .flex()
        .flex_col()
        .gap(z(12.))
        .pt(z(4.))
        .pr(z(8.))
        .pb(z(8.))
        .pl(z(16.))
        .child(
            div()
                .flex()
                .flex_col()
                .child(crate::fonts::styled_name(div().h(z(24.)).text_size(z(20.)).line_height(z(24.)).font_weight(FontWeight::SEMIBOLD).text_color(strong), Some(me), true).child(crate::fonts::name_text(me.display_name.clone(), Some(me), 20.)))
                .child(
                    div()
                        .h(z(24.))
                        .flex()
                        .items_center()
                        .gap(z(9.))
                        .text_size(z(14.))
                        .line_height(z(18.))
                        .text_color(strong)
                        .child(me.name.clone())
                        .children(tag)
                        .children(badges_row(e.profile.as_ref())),
                ),
        )
        .child(div().h(z(54.)).overflow_hidden().text_size(z(14.)).line_height(z(18.)).text_color(strong).child(bio_markup(bio)));
    let button_bg = theme.map(|t| hex(gradient_button_color(t.0)));
    div()
        .relative()
        .mt(z(6.)) // previewContainer_w__gu5 at content + 14 (padding-top 8 already applied)
        .ml(z(10.))
        .w(z(304.))
        .h(z(333.))
        .flex_none()
        .p(z(2.))
        .rounded(z(8.))
        .bg(outer)
        .shadow(vec![
            gpui::BoxShadow { color: gpui::hsla(0., 0., 1., 0.08), offset: gpui::point(z(0.), z(0.)), blur_radius: z(0.), spread_radius: z(1.), inset: false },
            gpui::BoxShadow { color: gpui::hsla(0., 0., 0., 0.24), offset: gpui::point(z(0.), z(12.)), blur_radius: z(24.), spread_radius: z(0.), inset: false },
        ])
        .child(
            div()
                .relative()
                .size_full()
                .rounded(z(6.))
                .overflow_hidden()
                .bg(gpui::hsla(0., 0., 0., if theme.is_some() { 0.6 } else { 0. }))
                .flex()
                .flex_col()
                .gap(z(8.))
                .pb(z(4.))
                .child(
                    div()
                        .relative()
                        .h(z(140.))
                        .flex_none()
                        .child(banner)
                        .child(
                            div()
                                .absolute()
                                .left(z(8.))
                                .top(z(53.))
                                .size(z(92.))
                                .rounded_full()
                                .bg(ring)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(crate::ui::avatar_with_status(Some(me), 80., app.presence_of(me.id), ring)),
                        ),
                )
                .child(div().flex_1().min_h_0().child(body))
                .child(
                    div().px(z(16.)).pb(z(12.)).child(
                        controls::button("pk-card-btn", crate::t!("pk.example_button"), BtnKind::Primary)
                            .w_full()
                            .h(z(31.))
                            .text_size(z(14.))
                            .line_height(z(18.))
                            .when_some(button_bg, |b, bg| b.bg(bg)),
                    ),
                )
                .children(overlay.map(|u| img(u).absolute().inset_0().size_full().object_fit(gpui::ObjectFit::Cover))),
        )
}

fn preview(app: &PhoenixApp, e: &super::profile_editor::EditorState, kind: Kind, item: Option<&Collectible>) -> Div {
    let av = crate::ui::avatar(Some(&app.me), 120.);
    let overlay = item.and_then(|i| i.preview.clone());
    match kind {
        Kind::Effect | Kind::Frame => profile_card(app, e, item.filter(|_| kind == Kind::Effect).and_then(|i| i.full.clone().or(i.preview.clone()))),
        Kind::Decoration => div()
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
    // Nitro members can use the "Nitro exclusive" items (no lock badge,
    // client `canUsePremiumCollectibles`).
    let premium = app.backend.has_premium();
    let owns_sel = p.sel.is_none_or(|s| owned.iter().any(|o| o.sku == s) || (premium && nitro.iter().any(|o| o.sku == s)));
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
                    // Effect tiles (#4844): presetEffectBackground_ + presetEffectImg_,
                    // both absolute 83x80 cover r4. Frame tiles (#4847,
                    // framePresetItem_ padding 6 14): sampleProfile_ small 52x56
                    // (1px --border-strong) under the frame layer (55 wide).
                    .when(kind == Kind::Effect, |d| d.child(sample_profile(sz.0 - 4., sz.1 - 4.)))
                    // With the frame's layers known, draw the whole layered frame
                    // (back layers, sample profile, front layers) the way the
                    // profile card does; else the single static layer below.
                    .when(kind == Kind::Frame, |d| {
                        let sample = div().absolute().left(z(13.)).top(z(10.)).w(z(52.)).h(z(56.)).rounded(z(4.)).border_1().border_color(c(tok(idx::BORDER_STRONG)));
                        match &it.frame {
                            Some(f) => d.child(
                                sample
                                    .children(crate::ui::profile_fx::frame(f, 50., false))
                                    .child(sample_profile(50., 54.))
                                    .children(crate::ui::profile_fx::frame(f, 50., true)),
                            ),
                            None => d.child(sample.child(sample_profile(50., 54.))),
                        }
                    })
                    .children(it.preview.clone().filter(|_| !(kind == Kind::Frame && it.frame.is_some())).map(|u| match kind {
                        // The 100x195 thumbnail fills the tile width from the top
                        // (what the dump shows for presetEffectImg_).
                        Kind::Effect => {
                            let (w, h) = (sz.0 - 4., (sz.0 - 4.) * 1.95);
                            div()
                                .absolute()
                                .inset_0()
                                .rounded(z(4.))
                                .overflow_hidden()
                                .child(img(u).absolute().left_0().top_0().w(z(w)).h(z(h)))
                                .into_any_element()
                        }
                        Kind::Frame => img(u).absolute().left(z(12.)).top(z(10.)).w(z(55.)).h(z(56.)).object_fit(gpui::ObjectFit::Contain).into_any_element(),
                        _ if full => img(u).size_full().rounded(z(4.)).into_any_element(),
                        _ => img(u).size(z(64.)).into_any_element(),
                    }))
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
    let nitro_tiles = item_tiles(&nitro, !premium, cx);
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
        .child(div().flex_1().min_h_0().pt(z(8.)).px(z(24.)).flex().gap(z(12.)).child(list).child(if matches!(kind, Kind::Effect | Kind::Frame) { preview(app, e, kind, chosen) } else { div().mt(z(24.)).child(preview(app, e, kind, chosen)) }))
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
