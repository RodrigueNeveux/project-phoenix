use std::cell::Cell;

use gpui::{AnyElement, Context, FontWeight, Hsla, IntoElement, ParentElement, SharedString, Styled, Window, div, img, prelude::*, rgb, rgba};

use crate::{
    app::PhoenixApp,
    model::Id,
    theme::{c, generated::idx, tok, z},
};

pub const HEADER_IMAGE: &str = "https://cdn.discordapp.com/assets/content/f91111a24ca4c59e87a462e8a3523938628e03e3723c31e5681991a07b0acf48.png";
pub const CONTENT_MAX_W: f32 = 1224.;
pub const TIER_BOOSTS: [u32; 4] = [0, 2, 7, 14];
pub const STACK_BELOW: f32 = 1100.;

thread_local! {
    static OPEN: Cell<Option<Id>> = const { Cell::new(None) };
    static SCROLL: gpui::ScrollHandle = gpui::ScrollHandle::new();
}

pub fn open(guild: Id) {
    OPEN.with(|o| o.set(Some(guild)));
    SCROLL.with(|s| s.set_offset(gpui::point(gpui::px(0.), gpui::px(0.))));
}

pub fn close() {
    OPEN.with(|o| o.set(None));
}

pub fn is_open() -> bool {
    OPEN.with(|o| o.get()).is_some()
}

fn tk(i: usize) -> Hsla {
    c(tok(i))
}

fn text(size: f32, line: f32, weight: FontWeight, color: Hsla, s: impl Into<SharedString>) -> gpui::Div {
    div().text_size(z(size)).line_height(z(line)).font_weight(weight).text_color(color).child(s.into())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Perk {
    pub icon: &'static str,
    pub key: &'static str,
}

pub const TIERS: [(u8, &[Perk]); 3] = [
    (
        1,
        &[
            Perk { icon: "chat/add-reaction", key: "boost.perk.emoji" },
            Perk { icon: "voice/soundboard", key: "boost.perk.soundboard" },
            Perk { icon: "gif", key: "boost.perk.animated_icon" },
            Perk { icon: "headphones", key: "boost.perk.audio" },
        ],
    ),
    (
        2,
        &[
            Perk { icon: "voice/screen", key: "boost.perk.hd_stream" },
            Perk { icon: "chat/upload", key: "boost.perk.upload_50" },
            Perk { icon: "settings/shield-user", key: "boost.perk.role_icons" },
            Perk { icon: "images", key: "boost.perk.banner" },
        ],
    ),
    (
        3,
        &[
            Perk { icon: "star", key: "boost.perk.vanity" },
            Perk { icon: "chat/upload", key: "boost.perk.upload_100" },
            Perk { icon: "headphones", key: "boost.perk.audio_max" },
            Perk { icon: "gif", key: "boost.perk.animated_banner" },
            Perk { icon: "stage", key: "boost.perk.stage" },
        ],
    ),
];

pub fn boosts_label(n: u32) -> SharedString {
    match n {
        0 => crate::t!("boost.none"),
        1 => crate::t!("boost.one", n = n),
        _ => crate::t!("boost.many", n = n),
    }
}

pub fn perk_text(key: &str) -> SharedString {
    match key {
        "boost.perk.emoji" => crate::t!(key, n = 100),
        "boost.perk.soundboard" => crate::t!(key, n = 24),
        "boost.perk.stage" => crate::t!(key, n = 300),
        _ => crate::t!(key),
    }
}

fn gem(size: f32, color: Hsla) -> gpui::Svg {
    gpui::svg().path(crate::icons::path("boost")).size(z(size)).text_color(color)
}

fn tier_card(tier: u8, perks: &[Perk], active: bool) -> AnyElement {
    let pink = tk(idx::GUILD_BOOSTING_PINK_REFRESH);
    let muted: Hsla = rgb(0x94959c).into();
    let bar = {
        let mut b = div().absolute().top(z(40.)).h(z(8.)).left_0().right_0();
        if tier == 1 {
            b = b.left(z(if active { 64. } else { 40. })).rounded_l(z(100.));
        }
        if tier == 3 {
            b = b.right(z(24.)).rounded_r(z(100.));
        }
        if active {
            b.bg(gpui::linear_gradient(90., gpui::linear_color_stop(rgb(0xff4ebb), 0.), gpui::linear_color_stop(rgba(0xff4ebb66), 1.)))
        } else {
            b.bg(rgb(0x474464))
        }
    };
    let border = match (active, tier) {
        (true, _) | (false, 1) => rgb(0x202024),
        (false, 2) => rgb(0x211e40),
        _ => rgb(0x201d3d),
    };
    let gem_box = div()
        .absolute()
        .left(z(24.))
        .top(z(24.))
        .size(z(40.))
        .rounded_full()
        .bg(border)
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .size(z(30.))
                .rounded_full()
                .bg(if active { pink } else { rgb(0x474464).into() })
                .flex()
                .items_center()
                .justify_center()
                .child(gem(24., gpui::white())),
        );
    let title = text(24., 30., FontWeight::SEMIBOLD, if active { tk(idx::TEXT_STRONG) } else { muted }, crate::t!(match tier {
        1 => "boost.level1",
        2 => "boost.level2",
        _ => "boost.level3",
    }))
    .flex_none();
    let badge = div()
        .h(z(30.))
        .px(z(8.))
        .rounded(z(8.))
        .bg(tk(idx::BACKGROUND_MOD_SUBTLE))
        .flex()
        .items_center()
        .gap(z(4.))
        .text_color(rgb(0xaaaab1))
        .child(gem(12., rgb(0xaaaab1).into()))
        .child(text(16., 20., FontWeight::MEDIUM, rgb(0xaaaab1).into(), boosts_label(TIER_BOOSTS[tier as usize])));
    let mut list = div().flex().flex_col().gap(z(12.));
    for p in perks.iter().take(4) {
        let col = if active { tk(idx::TEXT_DEFAULT) } else { muted };
        list = list.child(
            div()
                .flex()
                .items_center()
                .gap(z(8.))
                .child(gpui::svg().path(crate::icons::path(p.icon)).flex_none().size(z(20.)).text_color(col))
                .child(text(16., 20., FontWeight::MEDIUM, col, perk_text(p.key))),
        );
    }
    list = list.child(text(16., 20., FontWeight::MEDIUM, muted, crate::t!("boost.and_more")));
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .overflow_hidden()
        .rounded(z(16.))
        .border_1()
        .border_color(if active { gpui::hsla(240. / 360., 0.04, 0.61, 0.04) } else { tk(idx::BORDER_SUBTLE) })
        .bg(tk(idx::BACKGROUND_MOD_SUBTLE))
        .shadow_sm()
        .child(div().relative().h(z(64.)).child(bar).child(gem_box))
        .child(div().p(z(24.)).flex().flex_col().gap(z(12.)).child(div().flex().items_center().justify_between().child(title).child(badge)).child(list))
        .into_any_element()
}

pub fn render(app: &PhoenixApp, window: &mut Window, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    let gid = OPEN.with(|o| o.get())?;
    let g = app.guilds.iter().find(|x| x.id == gid)?;
    let white = gpui::white();
    let vw = f32::from(window.viewport_size().width) / crate::theme::zoom();
    let pink = tk(idx::GUILD_BOOSTING_PINK_REFRESH);
    let icon = div()
        .size(z(70.))
        .rounded(z(19.))
        .overflow_hidden()
        .border_1()
        .border_color(gpui::hsla(0., 0., 1., 0.16))
        .shadow_sm()
        .child(crate::ui::invite::guild_icon(Some(&g.icon), &g.name, f32::from(z(70.))));
    let count = div()
        .flex()
        .items_center()
        .gap(z(2.))
        .child(gem(18., if g.boosts > 0 { pink } else { gpui::hsla(0., 0., 1., 0.6) }).flex_none())
        .child(text(16., 20., FontWeight::SEMIBOLD, white, boosts_label(g.boosts)));
    let server = div()
        .flex()
        .flex_col()
        .items_center()
        .gap(z(12.))
        .child(icon)
        .child(div().flex().flex_col().items_center().gap(z(4.)).child(text(20., 24., FontWeight::SEMIBOLD, white, g.name.clone())).child(count));
    let boost_btn = crate::ui::controls::button("boost-layer-cta", crate::t!("boost.cta"), crate::ui::controls::BtnKind::Expressive)
        .gap(z(8.))
        .child(gem(16., tk(idx::CONTROL_EXPRESSIVE_ICON_DEFAULT)))
        .on_click(cx.listener(|a, _, _, cx| a.stub(&crate::t!("boost.cta"), cx)));
    let nitro_btn = crate::ui::controls::button("boost-layer-nitro", crate::t!("boost.get_nitro"), crate::ui::controls::BtnKind::Secondary)
        .on_click(cx.listener(|a, _, _, cx| a.stub(&crate::t!("boost.get_nitro"), cx)));
    let hero = div()
        .flex()
        .flex_col()
        .items_center()
        .gap(z(40.))
        .child(
            div()
                .max_w(z(770.))
                .text_center()
                .text_size(z(42.))
                .line_height(z(46.2))
                .font_weight(FontWeight::BOLD)
                .text_color(tk(idx::TEXT_STRONG))
                .child(crate::t!("boost.heading").to_uppercase()),
        )
        .child(div().flex().justify_center().gap(z(12.)).child(boost_btn).child(nitro_btn));
    let top = div().flex().flex_col().items_center().gap(z(24.)).mb(z(40.)).child(server).child(hero);
    let mut cards = div().w_full().flex().gap(z(24.));
    if vw <= STACK_BELOW {
        cards = cards.flex_col();
    }
    for (tier, perks) in TIERS {
        cards = cards.child(tier_card(tier, perks, g.boost_tier >= tier));
    }
    let background = div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .min_h(z(1280.))
        .overflow_hidden()
        .child(
            div().absolute().inset_0().bg(tk(idx::BACKGROUND_BASE_LOWEST)).child(div().absolute().inset_0().bg(gpui::linear_gradient(
                180.,
                gpui::linear_color_stop(rgba(0x9c59d199), 0.),
                gpui::linear_color_stop(rgba(0x9c59d100), 0.8),
            ))),
        )
        .child(img(HEADER_IMAGE).w_full().object_fit(gpui::ObjectFit::Cover));
    let content = div().relative().w_full().max_w(z(CONTENT_MAX_W)).mx_auto().pt(z(64.)).px(z(32.)).pb(z(32.)).flex().flex_col().child(top).child(cards);
    let scroller = SCROLL.with(|s| {
        div()
            .id("boost-layer-scroll")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(s)
            .bg(tk(idx::BACKGROUND_BASE_LOWEST))
            .child(div().relative().w_full().child(background).child(content).child(div().h(z(120.))))
    });
    let close_btn = div()
        .id("boost-layer-close")
        .absolute()
        .top(z(24.))
        .right(z(24.))
        .size(z(40.))
        .rounded(z(12.))
        .bg(rgba(0x10101280))
        .hover(|s| s.bg(rgba(0x101012b3)))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .tooltip(crate::ui::tooltip::tip(crate::t!("common.close")))
        .on_click(cx.listener(|_, _, _, cx| {
            close();
            cx.notify();
        }))
        .child(gpui::svg().path(crate::icons::path("close-small")).size(z(24.)).text_color(tk(idx::ICON_DEFAULT)));
    let keys = gpui::canvas(
        |_, _, _| (),
        |_, _, w, _| {
            w.on_key_event(|ev: &gpui::KeyDownEvent, phase, w, cx| {
                if phase == gpui::DispatchPhase::Capture && ev.keystroke.key == "escape" && is_open() {
                    close();
                    w.refresh();
                    cx.stop_propagation();
                }
            });
        },
    )
    .absolute()
    .size_0();
    Some(
        gpui::deferred(div().id("boost-layer").absolute().inset_0().occlude().child(scroller).child(close_btn).child(keys))
            .with_priority(5)
            .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_follow_the_client_definitions() {
        assert_eq!(TIER_BOOSTS, [0, 2, 7, 14]);
        assert_eq!(TIERS.map(|(t, p)| (t, p.len())), [(1, 4), (2, 4), (3, 5)]);
        assert_eq!(TIERS[0].1[1].key, "boost.perk.soundboard");
        assert_eq!(TIERS[2].1[4].icon, "stage");
    }

    #[test]
    fn layer_opens_and_closes() {
        open(Id(5));
        assert!(is_open());
        close();
        assert!(!is_open());
    }
}
