//! "Inventaire des cadeaux" settings page (dump vanilla-settings-16-3):
//! gift card promo, Redeem codes, the gifting badge ladder and "Gifts I
//! purchased". Phoenix handles no payments: buttons are inert.

use gpui::{AnyElement, Context, Div, ElementId, FontWeight, IntoElement, ParentElement, SharedString, Styled, div};

use crate::{
    app::PhoenixApp,
    theme::{c, generated::idx, tok, z},
    ui::controls::{self, BtnKind},
};

fn tr(k: &str) -> SharedString {
    crate::t!(k)
}

fn eid(s: impl Into<String>) -> ElementId {
    ElementId::Name(SharedString::from(s.into()))
}

fn text(size: f32, line: f32, weight: FontWeight, color: usize, s: SharedString) -> Div {
    div().text_size(z(size)).line_height(z(line)).font_weight(weight).text_color(c(tok(color))).child(s)
}

/// `container_J9WkLo`: 90deg --expressive-gradient-blue-end → -start over
/// --background-surface-high, --radius-lg, min-height 180, padding 16 16 16 28;
/// stack gap 24: heading-lg/semibold 20/24, then body-xs bullet items
/// (12px glyph, gap 6, 8 apart) and `overlay-primary` sm buttons 12 apart.
pub(super) fn promo(_: &PhoenixApp, _: &mut Context<'_, PhoenixApp>) -> AnyElement {
    let item = |ic: &'static str, k: &'static str| {
        div()
            .flex()
            .items_start()
            .gap(z(6.))
            .pl(z(2.))
            .child(gpui::svg().path(crate::icons::path(ic)).flex_none().size(z(12.)).mt(z(2.)).text_color(c(tok(idx::TEXT_DEFAULT))))
            .child(text(12., 16., FontWeight::NORMAL, idx::TEXT_DEFAULT, tr(k)))
    };
    let store = |k: &'static str| {
        // overlay-primary_F7qvhY: white fill, black text, 1px --border-normal
        controls::button_sm(eid(format!("gift-store-{k}")), tr(k), BtnKind::Secondary)
            .bg(gpui::white())
            .text_color(gpui::black())
    };
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(z(16.))
        .min_h(z(180.))
        .pt(z(16.))
        .pb(z(16.))
        .pr(z(16.))
        .pl(z(28.))
        .rounded(z(16.)) // --radius-lg
        .bg(gpui::linear_gradient(
            90.,
            gpui::linear_color_stop(c(tok(idx::EXPRESSIVE_GRADIENT_BLUE_END)), 0.),
            gpui::linear_color_stop(c(tok(idx::EXPRESSIVE_GRADIENT_BLUE_START)), 1.),
        ))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(z(24.))
                .w(z(350.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(z(12.))
                        .child(text(20., 24., FontWeight::SEMIBOLD, idx::TEXT_STRONG, tr("sp.gi.buy")))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(z(8.))
                                .child(item("shop", "sp.gi.b1"))
                                .child(item("nitro", "sp.gi.b2"))
                                .child(item("file", "sp.gi.b3"))
                                .child(item("info", "sp.gi.b4")),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .gap(z(12.))
                        .child(store("sp.gi.s1"))
                        .child(store("sp.gi.s2"))
                        .child(store("sp.gi.s3"))
                        .child(store("sp.gi.s4")),
                ),
        )
        .child(
            // animatedGraphic_J9WkLo (282x188 lottie): our own glyph stands in.
            div()
                .w(z(282.))
                .h(z(188.))
                .flex()
                .items_center()
                .justify_center()
                .child(gpui::svg().path(crate::icons::path("gift")).size(z(96.)).text_color(gpui::white().opacity(0.8))),
        )
        .into_any_element()
}

/// Redeem field (#container_VhFsFg): label body-md/medium --text-strong,
/// description body-sm --text-muted, 8 below; 40px input (--input-background
/// -default, 1px --input-border-default, r8) + md primary "Redeem", gap 8.
pub(super) fn redeem(_: &PhoenixApp, _: &mut Context<'_, PhoenixApp>) -> AnyElement {
    div()
        .py(z(10.))
        .flex()
        .items_end()
        .gap(z(8.))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(z(4.))
                        .mb(z(8.))
                        .child(text(16., 20., FontWeight::MEDIUM, idx::TEXT_STRONG, tr("sp.gi.code_label")))
                        .child(text(14., 18., FontWeight::NORMAL, idx::TEXT_MUTED, tr("sp.gi.code_desc"))),
                )
                .child(
                    div()
                        .h(z(40.))
                        .rounded(z(8.))
                        .border_1()
                        .border_color(c(tok(idx::INPUT_BORDER_DEFAULT)))
                        .bg(c(tok(idx::INPUT_BACKGROUND_DEFAULT))),
                ),
        )
        .child(controls::button(eid("gift-redeem"), tr("sp.gi.redeem"), BtnKind::Primary))
        .into_any_element()
}

const BADGES: &[(&str, u32)] = &[
    ("sp.gi.t1", 1),
    ("sp.gi.t2", 2),
    ("sp.gi.t3", 3),
    ("sp.gi.t4", 6),
    ("sp.gi.t5", 10),
    ("sp.gi.t6", 20),
];

/// `container_qZFuGA`: content section (--background-mod-subtle, r 12 12 0 0,
/// p32) with a row of 96px `badgeItem_` (p8 gap8, 44px icon, name body-sm
/// /semibold --text-muted, "Gifted: Nx" body-xs --text-subtle), footer
/// text body-xs; footer section (--background-surface-high, r 0 0 12 12,
/// p24) with the md primary "Send a gift" button on the right.
pub(super) fn badges(_: &PhoenixApp, _: &mut Context<'_, PhoenixApp>) -> AnyElement {
    let mut grid = div().flex().justify_center().gap(z(8.));
    for (k, n) in BADGES {
        grid = grid.child(
            div()
                .w(z(96.))
                .p(z(8.))
                .flex()
                .flex_col()
                .items_center()
                .gap(z(8.))
                .child(gpui::svg().path(crate::icons::path("gift")).size(z(44.)).text_color(c(tok(idx::ICON_MUTED))))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(z(2.))
                        .child(text(14., 18., FontWeight::SEMIBOLD, idx::TEXT_MUTED, tr(k)))
                        .child(text(12., 16., FontWeight::NORMAL, idx::TEXT_SUBTLE, crate::t!("sp.gi.gifted", n = n))),
                ),
        );
    }
    div()
        .py(z(10.))
        .flex()
        .flex_col()
        .child(
            div()
                .p(z(32.))
                .rounded_t(z(12.))
                .bg(c(tok(idx::BACKGROUND_MOD_SUBTLE)))
                .flex()
                .flex_col()
                .gap(z(16.))
                .child(grid)
                .child(text(12., 16., FontWeight::NORMAL, idx::TEXT_SUBTLE, tr("sp.gi.footer")).text_center()),
        )
        .child(
            div()
                .p(z(24.))
                .rounded_b(z(12.))
                .bg(c(tok(idx::BACKGROUND_SURFACE_HIGH)))
                .flex()
                .justify_end()
                .child(controls::button_icon(eid("gift-send"), "gift", tr("sp.gi.send"), BtnKind::Primary)),
        )
        .into_any_element()
}

/// Empty "Gifts I purchased": "No gifts." + the generous-soul line.
pub(super) fn purchased(_: &PhoenixApp, _: &mut Context<'_, PhoenixApp>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(z(8.))
        .child(text(16., 20., FontWeight::MEDIUM, idx::TEXT_DEFAULT, tr("sp.gi.none")))
        .child(text(14., 18., FontWeight::NORMAL, idx::TEXT_MUTED, tr("sp.gi.none_desc")))
        .into_any_element()
}

/// "Abonnements" (dump vanilla-settings-16-2, `content_NedsCW` gap 32):
/// heading-lg/semibold 20/24 + 16px margin, `sectionDescription_4VCFtm`
/// 16/20 --text-default; then the "Subscription credit" fieldset: legend
/// body-lg/medium 20/24 (mb 12), body-sm description (mt -8). Subscription
/// cards and credit rows need billing data Phoenix doesn't fetch.
pub(super) fn subscriptions(_: &PhoenixApp, _: &mut Context<'_, PhoenixApp>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(z(32.))
        .child(
            div()
                .flex()
                .flex_col()
                .child(text(20., 24., FontWeight::SEMIBOLD, idx::TEXT_STRONG, tr("sp.su.title")).mb(z(16.)))
                .child(text(16., 20., FontWeight::NORMAL, idx::TEXT_DEFAULT, tr("sp.su.desc"))),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .child(text(20., 24., FontWeight::MEDIUM, idx::TEXT_STRONG, tr("sp.su.credit")).mb(z(12.)))
                .child(text(14., 18., FontWeight::NORMAL, idx::TEXT_DEFAULT, tr("sp.su.credit_desc")).mt(z(-8.))),
        )
        .into_any_element()
}
