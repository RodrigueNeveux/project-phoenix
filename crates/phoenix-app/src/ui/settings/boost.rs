//! "Boost de serveur" settings page, laid out like the 2026 client
//! (dump vanilla-settings-16-1, `guildBoostingSettings_AioBH5`, 696px column):
//! header (title + lead | 300px video card), purchase notice pill, guild cards,
//! perks table, "Boost your community" cards and the FAQ list.
//! Phoenix doesn't buy boosts: buttons are inert like the rest of billing.

use gpui::{AnyElement, Context, Div, ElementId, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, prelude::*, relative};

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

fn icon(name: &str, size: f32, color: usize) -> gpui::Svg {
    gpui::svg().path(crate::icons::path(name)).flex_none().size(z(size)).text_color(c(tok(color)))
}

fn text(size: f32, line: f32, weight: FontWeight, color: usize, s: SharedString) -> Div {
    div().text_size(z(size)).line_height(z(line)).font_weight(weight).text_color(c(tok(color))).child(s)
}

/// heading-xl/medium 24/32 --text-strong (#2731, #2921, #2936).
fn heading_xl(s: SharedString) -> Div {
    text(24., 32., FontWeight::MEDIUM, idx::TEXT_STRONG, s)
}

/// Perks table rows (#2812..): label key, then the three tier values
/// (`None` = check icon, `Some("")` = empty cell).
const PERKS: &[(&str, [Option<&str>; 3])] = &[
    ("sp.bo.p_emoji", [Some("100"), Some("150"), Some("250")]),
    ("sp.bo.p_stickers", [Some("15"), Some("30"), Some("60")]),
    ("sp.bo.p_soundboard", [Some("24"), Some("36"), Some("48")]),
    ("sp.bo.p_stream", [Some("sp.bo.v_720"), Some("sp.bo.v_1440"), Some("sp.bo.v_1440")]),
    ("sp.bo.p_audio", [Some("128 ko/s"), Some("256 ko/s"), Some("384 ko/s")]),
    ("sp.bo.p_upload", [Some("10 Mo"), Some("50 Mo"), Some("100 Mo")]),
    ("sp.bo.p_video", [Some("50"), Some("150"), Some("300")]),
    ("sp.bo.p_anim_icon", [None, None, None]),
    ("sp.bo.p_invite_bg", [None, None, None]),
    ("sp.bo.p_banner", [Some(""), Some("sp.bo.v_static"), Some("sp.bo.v_animated")]),
    ("sp.bo.p_role_icons", [Some(""), Some(""), None]),
    ("sp.bo.p_vanity", [Some(""), Some(""), None]),
];

const FAQ: &[&str] = &[
    "sp.bo.q1", "sp.bo.q2", "sp.bo.q3", "sp.bo.q4", "sp.bo.q5", "sp.bo.q6", "sp.bo.q7", "sp.bo.q8", "sp.bo.q9",
];

fn value(v: &str) -> SharedString {
    if v.starts_with("sp.") { tr(v) } else { SharedString::from(v.to_string()) }
}

pub(super) fn page(app: &PhoenixApp, _: &mut Context<'_, PhoenixApp>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .w_full()
        .child(header())
        .child(notice())
        .child(
            // refreshSections_AioBH5: column, gap --space-32
            div()
                .flex()
                .flex_col()
                .gap(z(32.))
                .child(guild_cards(app))
                .child(perks_table())
                .child(community())
                .child(faq()),
        )
        .into_any_element()
}

/// header_xDn2Ui: row, centred, gap 32, margin-bottom 32; content 364 wide
/// (headingRow gap 10: 15x24 gem + heading-xl), lead body-sm 14/18
/// --text-subtle; videoThumbnail 300x169, radius 10, 1px --border-normal.
fn header() -> Div {
    div()
        .flex()
        .items_center()
        .gap(z(32.))
        .mb(z(32.))
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap(z(4.)) // content_xDn2Ui gap
                .child(div().flex().items_center().gap(z(10.)).child(icon("boost", 20., idx::GUILD_BOOSTING_PINK)).child(heading_xl(tr("sp.bo.title"))))
                .child(
                    div()
                        .text_size(z(14.))
                        .line_height(z(18.))
                        .text_color(c(tok(idx::TEXT_SUBTLE)))
                        .child(tr("sp.bo.lead"))
                        .child(div().text_color(c(tok(idx::TEXT_LINK))).child(tr("sp.bo.learn"))),
                ),
        )
        .child(
            div()
                .flex_none()
                .relative()
                .w(z(300.))
                .h(z(169.))
                .rounded(z(10.))
                .border_1()
                .border_color(c(tok(idx::BORDER_NORMAL)))
                .bg(c(tok(idx::BACKGROUND_SURFACE_HIGH)))
                .overflow_hidden()
                // videoTitle_DV7fsY at 12,12: body-sm/semibold white
                .child(div().absolute().top(z(12.)).left(z(12.)).child(text(14., 18., FontWeight::SEMIBOLD, idx::WHITE, tr("sp.bo.video"))))
                // playButton_DV7fsY: 48px, padding 12, 24px play glyph, centred
                .child(
                    div()
                        .absolute()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(div().size(z(48.)).rounded_full().bg(gpui::black().opacity(0.6)).flex().items_center().justify_center().child(icon("play", 24., idx::WHITE))),
                ),
        )
}

/// guildBoostUnavailableNotice_Jru8ph: centred pill, --white 10% fill, 2px
/// --white border, padding 8 16, margin-bottom 32; body-sm/medium uppercase.
fn notice() -> Div {
    div().flex().justify_center().mb(z(32.)).child(
        div()
            .px(z(16.))
            .py(z(8.))
            .rounded_full()
            .border_2()
            .border_color(gpui::white())
            .bg(gpui::white().opacity(0.1))
            .child(text(16., 20., FontWeight::MEDIUM, idx::TEXT_DEFAULT, tr("sp.bo.unavailable"))),
    )
}

/// wrapper_M7b2T5 (gap 16): heading-sm/semibold title, then `card_rH8mBz`
/// rows 8 apart: --background-surface-high, 1px --border-muted, r16, p12;
/// 40px guild icon, gap 12, name heading-sm/semibold; sm secondary
/// "Boost this server" button with the 16px boost glyph.
fn guild_cards(app: &PhoenixApp) -> Div {
    let mut cards = div().flex().flex_col().gap(z(8.));
    for g in app.guilds.iter().take(3) {
        cards = cards.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .p(z(12.))
                .rounded(z(16.))
                .border_1()
                .border_color(c(tok(idx::BORDER_MUTED)))
                .bg(c(tok(idx::BACKGROUND_SURFACE_HIGH)))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(z(12.))
                        .min_w_0()
                        .child(crate::ui::invite::guild_icon(Some(&g.icon), &g.name, 40.))
                        .child(text(16., 20., FontWeight::SEMIBOLD, idx::TEXT_DEFAULT, g.name.clone())),
                )
                .child(
                    controls::button_sm(eid(format!("boost-{}", g.id)), SharedString::default(), BtnKind::Secondary)
                        .gap(z(4.))
                        .child(icon("boost", 16., idx::CONTROL_SECONDARY_TEXT_DEFAULT))
                        .child(tr("sp.bo.boost_btn")),
                ),
        );
    }
    div()
        .flex()
        .flex_col()
        .gap(z(16.))
        .child(text(16., 20., FontWeight::SEMIBOLD, idx::TEXT_STRONG, tr("sp.bo.could")))
        .child(cards)
}

/// table_twYJsJ: rows padding 16 0 with a 1px --border-normal bottom; four
/// flex:1 cells padded 0 16. Header: heading-lg/semibold 20/24 + body-sm
/// /medium --text-subtle boosts count; perk label body-md/medium
/// --text-subtle; tier values body-md/semibold --text-strong or a 24px check.
fn perks_table() -> Div {
    let cell = || div().flex_1().min_w_0().px(z(16.));
    let tier = || cell().flex().flex_col().items_center().justify_center();
    let row = || div().flex().items_center().py(z(16.)).border_b_1().border_color(c(tok(idx::BORDER_NORMAL)));
    let mut head = row().child(cell().child(text(20., 24., FontWeight::SEMIBOLD, idx::TEXT_DEFAULT, tr("sp.bo.perks"))));
    for (lvl, n) in [("sp.bo.lvl1", "sp.bo.n2"), ("sp.bo.lvl2", "sp.bo.n7"), ("sp.bo.lvl3", "sp.bo.n14")] {
        head = head.child(
            tier()
                .child(text(20., 24., FontWeight::SEMIBOLD, idx::TEXT_DEFAULT, tr(lvl)))
                .child(text(14., 18., FontWeight::MEDIUM, idx::TEXT_SUBTLE, tr(n))),
        );
    }
    let mut t = div().flex().flex_col().child(head);
    for (label, vals) in PERKS {
        let mut r = row().child(cell().child(text(16., 20., FontWeight::MEDIUM, idx::TEXT_SUBTLE, tr(label))));
        for v in vals {
            r = r.child(match v {
                None => tier().child(icon("check", 24., idx::TEXT_STRONG)),
                Some(s) if s.is_empty() => tier(),
                Some(s) => tier().child(text(16., 20., FontWeight::SEMIBOLD, idx::TEXT_STRONG, value(s))),
            });
        }
        t = t.child(r);
    }
    t
}

/// wrapper_dgf5AJ (gap 16): heading-xl, then 2x2 `card_dgf5AJ` (flex 50%,
/// gap 16): --background-surface-high, 1px --border-subtle, r16, p12, gap 8,
/// centred 42px --icon-muted glyph + body-sm/medium --text-subtle text.
fn community() -> Div {
    let card = |ic: &'static str, k: &'static str| {
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .items_center()
            .gap(z(8.))
            .p(z(12.))
            .rounded(z(16.))
            .border_1()
            .border_color(c(tok(idx::BORDER_SUBTLE)))
            .bg(c(tok(idx::BACKGROUND_SURFACE_HIGH)))
            .child(icon(ic, 42., idx::ICON_MUTED))
            .child(text(14., 18., FontWeight::MEDIUM, idx::TEXT_SUBTLE, tr(k)).text_center())
    };
    div()
        .flex()
        .flex_col()
        .gap(z(16.))
        .child(heading_xl(tr("sp.bo.community")))
        .child(div().flex().gap(z(16.)).child(card("members", "sp.bo.c1")).child(card("medal", "sp.bo.c2")))
        .child(div().flex().gap(z(16.)).child(card("crown", "sp.bo.c3")).child(card("boost", "sp.bo.c4")))
}

/// wrapper_eICevE (gap 16): heading-xl + list gap 4 of `listItem_eICevE`
/// (--background-surface-high, 1px --border-subtle, r8, p12, pointer);
/// question heading-sm/semibold --text-subtle, 18px chevron, gap 12.
fn faq() -> Div {
    let mut list = div().flex().flex_col().gap(z(4.));
    for (i, q) in FAQ.iter().enumerate() {
        list = list.child(
            div()
                .id(eid(format!("boost-faq-{i}")))
                .flex()
                .items_center()
                .gap(z(12.))
                .p(z(12.))
                .rounded(z(8.))
                .border_1()
                .border_color(c(tok(idx::BORDER_SUBTLE)))
                .bg(c(tok(idx::BACKGROUND_SURFACE_HIGH)))
                .cursor_pointer()
                .child(div().flex_1().min_w_0().child(text(16., 20., FontWeight::SEMIBOLD, idx::TEXT_SUBTLE, tr(q))))
                .child(icon("chevron", 18., idx::TEXT_DEFAULT)),
        );
    }
    div().flex().flex_col().gap(z(16.)).w(relative(1.)).child(heading_xl(tr("sp.bo.faq"))).child(list)
}
