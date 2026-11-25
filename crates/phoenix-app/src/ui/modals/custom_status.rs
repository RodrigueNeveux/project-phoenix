use chrono::{Duration, Local};
use gpui::{AnyElement, Context, Entity, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, prelude::*};
use gpui_component::input::{Input, InputState};

use crate::{
    actions::{open_modal, with_modal},
    app::PhoenixApp,
    theme::{c, generated::idx, tok, z},
    ui::{
        controls::{BtnKind, button},
        modals::{Modal, card},
        profile::{avatar_block, banner, info, name_block},
    },
};

pub const MAX_LEN: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClearAfter {
    Today,
    Hours4,
    Hours1,
    Minutes30,
    Never,
}

pub const CHOICES: [ClearAfter; 5] = [ClearAfter::Today, ClearAfter::Hours4, ClearAfter::Hours1, ClearAfter::Minutes30, ClearAfter::Never];

impl ClearAfter {
    pub fn duration(self) -> Option<Duration> {
        match self {
            Self::Today => Some(Duration::hours(24)),
            Self::Hours4 => Some(Duration::hours(4)),
            Self::Hours1 => Some(Duration::hours(1)),
            Self::Minutes30 => Some(Duration::minutes(30)),
            Self::Never => None,
        }
    }
}

pub struct CustomStatus {
    pub input: Option<Entity<InputState>>,
    pub initial: String,
    pub clear_after: ClearAfter,
    pub select_open: bool,
}

fn tk(i: usize) -> gpui::Hsla {
    c(tok(i))
}

fn when_label(d: Duration, tomorrow_key: &str, today_key: &str) -> SharedString {
    let now = Local::now();
    let at = now + d;
    let time = crate::i18n::clock(at);
    if at.date_naive() == now.date_naive() { crate::t!(today_key, time = time) } else { crate::t!(tomorrow_key, time = time) }
}

pub fn selected_label(c: ClearAfter) -> SharedString {
    match c.duration() {
        Some(d) => when_label(d, "cstatus.clear_tomorrow_at", "cstatus.clear_at"),
        None => crate::t!("cstatus.dont_clear"),
    }
}

pub fn option_label(c: ClearAfter) -> SharedString {
    let span = match c {
        ClearAfter::Today => crate::t!("cstatus.hours", n = 24),
        ClearAfter::Hours4 => crate::t!("cstatus.hours", n = 4),
        ClearAfter::Hours1 => crate::t!("cstatus.hour", n = 1),
        ClearAfter::Minutes30 => crate::t!("cstatus.minutes", n = 30),
        ClearAfter::Never => return crate::t!("cstatus.dont_clear"),
    };
    let when = c.duration().map(|d| when_label(d, "cstatus.tomorrow_at", "cstatus.at")).unwrap_or_default();
    format!("{span} ({when})").into()
}

fn set(f: impl FnOnce(&mut CustomStatus)) {
    with_modal(|m| {
        if let Some(Modal::CustomStatus(s)) = m {
            f(s);
        }
    });
}

pub fn prepare(s: &mut CustomStatus, mk: &mut dyn FnMut(SharedString, &str, bool) -> Entity<InputState>) {
    if s.input.is_none() {
        let initial = s.initial.clone();
        s.input = Some(mk(crate::t!("cstatus.placeholder"), &initial, true));
    }
}

fn preview(app: &PhoenixApp, text: &str) -> AnyElement {
    let i = info(app, app.me.id, None);
    let bubble = div()
        .absolute()
        .left(z(105.))
        .top(z(98.))
        .right(z(14.))
        .px(z(12.))
        .py(z(4.))
        .rounded(z(16.))
        .border_1()
        .border_color(tk(idx::BORDER_MUTED))
        .bg(tk(idx::BACKGROUND_SURFACE_HIGHEST))
        .shadow_sm()
        .overflow_hidden()
        .text_ellipsis()
        .whitespace_nowrap()
        .text_size(z(14.))
        .line_height(z(18.))
        .text_color(tk(if text.is_empty() { idx::TEXT_MUTED } else { idx::TEXT_STRONG }))
        .child(if text.is_empty() { crate::t!("cstatus.placeholder") } else { SharedString::from(text.to_owned()) });
    div()
        .w(z(300.))
        .rounded(z(8.))
        .overflow_hidden()
        .bg(tk(idx::BACKGROUND_SURFACE_HIGH))
        .border_1()
        .border_color(tk(idx::BORDER_SUBTLE))
        .child(
            div()
                .relative()
                .h(z(151.))
                .child(banner(&i, 105., 6.))
                .child(div().absolute().left(z(16.)).top(z(61.)).child(avatar_block(&i, 80., 6., tok(idx::BACKGROUND_SURFACE_HIGH))))
                .child(bubble),
        )
        .child(div().pt(z(8.)).px(z(16.)).pb(z(16.)).child(name_block(&i, false, None)))
        .into_any_element()
}

pub fn render(app: &PhoenixApp, s: &CustomStatus, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let text = s.input.as_ref().map(|i| i.read(cx).value().to_string()).unwrap_or_default();
    let has = !text.is_empty();
    let field = div()
        .mt(z(4.))
        .relative()
        .h(z(46.))
        .rounded(z(8.))
        .border_1()
        .border_color(tk(idx::INPUT_BORDER_DEFAULT))
        .bg(tk(idx::INPUT_BACKGROUND_DEFAULT))
        .flex()
        .items_center()
        .pl(z(48.))
        .pr(z(36.))
        .text_size(z(16.))
        .text_color(tk(idx::TEXT_DEFAULT))
        .children(s.input.as_ref().map(|i| div().flex_1().min_w_0().child(Input::new(i).appearance(false).bordered(false))))
        .child(
            div()
                .absolute()
                .left_0()
                .top_0()
                .h(z(46.))
                .w(z(42.))
                .pl(z(4.))
                .flex()
                .items_center()
                .justify_center()
                .child(gpui::svg().path(crate::icons::path("emoji")).size(z(24.)).text_color(tk(idx::INTERACTIVE_TEXT_DEFAULT))),
        )
        .when(has, |d| {
            d.child(
                div()
                    .id("cstatus-clear")
                    .absolute()
                    .right_0()
                    .top_0()
                    .h(z(46.))
                    .w(z(40.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .opacity(0.6)
                    .hover(|st| st.opacity(1.))
                    .on_click(cx.listener(|_, _, w, cx| {
                        set(|s| {
                            if let Some(i) = s.input.clone() {
                                i.update(cx, |st, cx| st.set_value("", w, cx));
                            }
                        });
                        cx.notify();
                    }))
                    .child(gpui::svg().path(crate::icons::path("settings/circle-x")).size(z(16.)).text_color(tk(idx::INTERACTIVE_TEXT_DEFAULT))),
            )
        });
    let body = div()
        .flex()
        .flex_col()
        .child(div().mt(z(8.)).mb(z(36.)).w_full().flex().items_center().justify_center().child(preview(app, &text)))
        .child(
            div()
                .mb(z(20.))
                .relative()
                .child(div().w_full().text_size(z(16.)).line_height(z(20.)).font_weight(FontWeight::SEMIBOLD).text_color(tk(idx::TEXT_STRONG)).child(crate::t!("cstatus.label")))
                .child(field),
        );
    let select = div()
        .id("cstatus-clear-after")
        .relative()
        .ml(z(-12.))
        .h(z(40.))
        .px(z(12.))
        .rounded(z(8.))
        .flex()
        .items_center()
        .gap(z(4.))
        .cursor_pointer()
        .text_size(z(14.))
        .line_height(z(18.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(tk(idx::TEXT_DEFAULT))
        .hover(|st| st.bg(tk(idx::BACKGROUND_MOD_SUBTLE)))
        .on_click(cx.listener(|_, _, _, cx| {
            set(|s| s.select_open = !s.select_open);
            cx.notify();
        }))
        .child(selected_label(s.clear_after))
        .child(gpui::svg().path(crate::icons::path("chevron")).size(z(16.)).text_color(tk(idx::INTERACTIVE_TEXT_DEFAULT)))
        .when(s.select_open, |d| d.child(clear_after_menu(s.clear_after, cx)));
    let save = button("cstatus-save", crate::t!("cstatus.save"), BtnKind::Primary).px(z(16.)).on_click(cx.listener(|a, _, _, cx| a.save_custom_status(cx)));
    let footer = div().w_full().flex().items_center().justify_between().child(select).child(save);
    card(480., crate::t!("cstatus.title"), None, body.into_any_element(), Some(footer.into_any_element()), cx)
}

fn clear_after_menu(current: ClearAfter, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let mut menu = div()
        .id("cstatus-clear-menu")
        .occlude()
        .absolute()
        .bottom(z(44.))
        .left_0()
        .p(z(8.))
        .rounded(z(8.))
        .border_1()
        .border_color(tk(idx::BORDER_SUBTLE))
        .bg(tk(idx::BACKGROUND_SURFACE_HIGHER))
        .shadow_lg()
        .flex()
        .flex_col()
        .child(div().min_w(z(200.)).px(z(8.)).py(z(8.)).text_size(z(14.)).line_height(z(18.)).font_weight(FontWeight::MEDIUM).text_color(tk(idx::TEXT_MUTED)).child(crate::t!("cstatus.clear_after")));
    for (n, ch) in CHOICES.into_iter().enumerate() {
        let sel = ch == current;
        menu = menu.child(
            div()
                .id(("cstatus-opt", n))
                .min_w(z(200.))
                .px(z(8.))
                .py(z(8.))
                .rounded(z(4.))
                .flex()
                .justify_between()
                .gap(z(8.))
                .cursor_pointer()
                .text_size(z(14.))
                .line_height(z(18.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(tk(idx::TEXT_STRONG))
                .hover(|st| st.bg(tk(idx::INTERACTIVE_BACKGROUND_HOVER)))
                .on_click(cx.listener(move |_, _, _, cx| {
                    set(|s| {
                        s.clear_after = ch;
                        s.select_open = false;
                    });
                    cx.notify();
                }))
                .child(option_label(ch))
                .when(sel, |d| d.child(gpui::svg().path(crate::icons::path("check")).size(z(18.)).text_color(tk(idx::BRAND_500)))),
        );
    }
    gpui::deferred(menu).with_priority(30).into_any_element()
}

impl PhoenixApp {
    pub fn open_custom_status(&mut self, cx: &mut Context<Self>) {
        let initial = self.people.custom_status.as_ref().map(|s| s.to_string()).unwrap_or_default();
        self.people.close_overlays();
        open_modal(Modal::CustomStatus(CustomStatus { input: None, initial, clear_after: ClearAfter::Today, select_open: false }), None);
        cx.notify();
    }

    fn save_custom_status(&mut self, cx: &mut Context<Self>) {
        let mut text = None;
        set(|s| text = s.input.as_ref().map(|i| i.read(cx).value().trim().chars().take(MAX_LEN).collect::<String>()));
        crate::actions::close_modal();
        self.set_custom_status(text.filter(|t| !t.is_empty()), cx);
    }
}
