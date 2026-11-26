use gpui::{AnyElement, Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, img, prelude::*};

use crate::{
    app::PhoenixApp,
    theme::{c, generated::idx, tok, z},
    ui::{
        controls::{BtnKind, button},
        modals::{Modal, card},
        quests_page::{Quest, privacy_url},
    },
};

pub const VIDEO_MAX_W: f32 = 960.;
pub const FOOTER_H: f32 = 80.;
pub const RING: f32 = 48.;

fn tk(i: usize) -> gpui::Hsla {
    c(tok(i))
}

pub fn video_size(viewport_w: f32, viewport_h: f32, portrait: bool) -> (f32, f32) {
    let avail_w = (viewport_w - 80.).max(240.);
    let avail_h = (viewport_h - FOOTER_H - 80.).max(160.);
    let ratio = if portrait { 9. / 16. } else { 16. / 9. };
    let w = avail_w.min(VIDEO_MAX_W).min(avail_h * ratio);
    (w.round(), (w / ratio).round())
}

fn ring(q: &Quest) -> AnyElement {
    let pct = if q.completed { 1. } else { q.progress.min(0.99) };
    div()
        .size(z(RING))
        .flex_none()
        .rounded_full()
        .border_2()
        .border_color(if pct > 0. { tk(idx::TEXT_BRAND) } else { tk(idx::BORDER_NORMAL) })
        .bg(tk(idx::BACKGROUND_MOD_SUBTLE))
        .flex()
        .items_center()
        .justify_center()
        .child(img(crate::ui::quests_page::REWARD_ART_SMALL).size(z(36.)).rounded_full().object_fit(gpui::ObjectFit::Cover))
        .into_any_element()
}

pub fn render_video(q: &Quest, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let (w, h) = video_size(crate::ui::title_bar::viewport_w_css(), crate::ui::title_bar::viewport_h_css(), q.video_portrait);
    let id = q.id;
    let play = div()
        .id("quest-video-play")
        .size(z(64.))
        .rounded_full()
        .bg(gpui::black().opacity(0.6))
        .hover(|s| s.bg(gpui::black().opacity(0.8)))
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .on_click(cx.listener(move |a, _, _, cx| a.blocked_write(crate::actions::guard::GuardedWrite::quest_video_progress(id, 0.), cx)))
        .child(gpui::svg().path(crate::icons::path("play")).size(z(32.)).text_color(gpui::white()));
    let video = div()
        .relative()
        .w(z(w))
        .h(z(h))
        .rounded_t(z(12.))
        .overflow_hidden()
        .bg(gpui::black())
        .children(q.video_thumb.clone().map(|u| img(u).absolute().inset_0().size_full().object_fit(gpui::ObjectFit::Cover)))
        .child(div().absolute().inset_0().flex().items_center().justify_center().child(play));
    let more = {
        let target = crate::ui::extra_menus::Target::QuestMore(Box::new(q.clone()));
        div()
            .id("quest-video-more")
            .size(z(40.))
            .flex_none()
            .rounded(z(8.))
            .cursor_pointer()
            .bg(tk(idx::CONTROL_SECONDARY_BACKGROUND_DEFAULT))
            .hover(|s| s.bg(tk(idx::CONTROL_SECONDARY_BACKGROUND_HOVER)))
            .flex()
            .items_center()
            .justify_center()
            .on_click(move |ev, win, _| crate::ui::extra_menus::open(target.clone(), ev.position(), win))
            .child(gpui::svg().path(crate::icons::path("more-horizontal")).size(z(20.)).text_color(tk(idx::INTERACTIVE_TEXT_DEFAULT)))
    };
    let details = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .child(div().truncate().text_size(z(16.)).line_height(z(20.)).font_weight(FontWeight::SEMIBOLD).text_color(tk(idx::TEXT_STRONG)).child(crate::t!("quests.quest_name", name = q.name.clone())))
        .child(
            div()
                .flex()
                .items_center()
                .gap(z(4.))
                .text_size(z(14.))
                .line_height(z(18.))
                .text_color(tk(idx::TEXT_MUTED))
                .child(gpui::svg().path(crate::icons::path("circle-check")).size(z(14.)).text_color(tk(idx::TEXT_MUTED)))
                .child(div().truncate().child(q.publisher.clone())),
        );
    let primary = q.cta.clone().zip(q.cta_link.clone()).map(|(label, url)| {
        button("quest-video-cta", label, BtnKind::Primary).flex_none().on_click(move |_, _, cx| cx.open_url(&url)).into_any_element()
    });
    let footer = div()
        .w(z(w))
        .h(z(FOOTER_H))
        .px(z(16.))
        .rounded_b(z(12.))
        .bg(tk(idx::BACKGROUND_SURFACE_HIGH))
        .flex()
        .items_center()
        .gap(z(12.))
        .child(ring(q))
        .child(details)
        .child(more)
        .children(primary);
    let close = div()
        .id("quest-video-close")
        .absolute()
        .top(z(-44.))
        .right_0()
        .size(z(32.))
        .rounded_full()
        .cursor_pointer()
        .bg(tk(idx::CONTROL_OVERLAY_SECONDARY_BACKGROUND_DEFAULT))
        .hover(|s| s.bg(tk(idx::CONTROL_OVERLAY_SECONDARY_BACKGROUND_HOVER)))
        .flex()
        .items_center()
        .justify_center()
        .on_click(cx.listener(|_, _, _, cx| {
            crate::actions::close_modal();
            cx.notify();
        }))
        .child(gpui::svg().path(crate::icons::path("close")).size(z(18.)).text_color(gpui::white()));
    div().id("quest-video-modal").relative().flex().flex_col().child(close).child(video).child(footer).into_any_element()
}

pub fn render_disclosure(q: &Quest, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let url = privacy_url();
    let body = div()
        .flex()
        .flex_col()
        .gap(z(16.))
        .child(div().text_size(z(14.)).line_height(z(18.)).font_weight(FontWeight::MEDIUM).text_color(tk(idx::TEXT_DEFAULT)).child(crate::ui::quests_page::disclosure(q)))
        .child(
            div()
                .id("quest-privacy-link")
                .text_size(z(12.))
                .line_height(z(16.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(tk(idx::TEXT_MUTED))
                .cursor_pointer()
                .on_click(move |_, _, cx| cx.open_url(&url))
                .child(crate::t!("quests.privacy_hint")),
        );
    let close = crate::ui::modals::close_action(cx);
    let footer = div().flex().justify_end().child(button("quest-why-close", crate::t!("common.close"), BtnKind::Primary).on_click(move |_, w, cx| close(w, cx))).into_any_element();
    card(440., crate::t!("quests.why"), None, body.into_any_element(), Some(footer), cx)
}

impl PhoenixApp {
    pub fn open_quest_video(&mut self, q: Quest, cx: &mut Context<Self>) {
        crate::ui::extra_menus::close();
        crate::actions::open_modal(Modal::QuestVideo(Box::new(q)), None);
        cx.notify();
    }

    pub fn open_quest_disclosure(&mut self, q: Quest, cx: &mut Context<Self>) {
        crate::ui::extra_menus::close();
        crate::actions::open_modal(Modal::QuestDisclosure(Box::new(q)), None);
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_fits_the_window_and_keeps_its_ratio() {
        assert_eq!(video_size(1600., 1000., false), (960., 540.));
        let (w, h) = video_size(800., 700., false);
        assert_eq!((w, h), (720., 405.));
        let (w, h) = video_size(1600., 1000., true);
        assert!(h <= 1000. - FOOTER_H - 80. && (w / h - 9. / 16.).abs() < 0.01);
    }
}
