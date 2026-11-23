//! Own status menu, opened from the avatar in the bottom user panel:
//! Online / Idle / Do Not Disturb / Invisible + custom status text.
//! Applied through op 3 and `PATCH /users/@me/settings`.

use crate::theme::z;
use gpui::{AnyElement, Context, FontWeight, IntoElement, ParentElement, Styled, Window, div, prelude::*, px};
use gpui_component::input::{Input, InputEvent, InputState};

use crate::{
    app::PhoenixApp,
    model::Presence,
    presence::status_label,
    theme::{Palette, c, m},
    ui::status_dot,
};

impl PhoenixApp {
    pub fn toggle_status_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let open = !self.people.status_picker;
        self.people.close_overlays();
        if open {
            let cur = self.people.custom_status.clone().unwrap_or_default();
            let input = cx.new(|cx| InputState::new(window, cx).placeholder("Set a custom status").default_value(cur));
            let sub = cx.subscribe_in(&input, window, |this, input, ev: &InputEvent, _, cx| {
                if let InputEvent::PressEnter { .. } = ev {
                    let t = input.read(cx).value().trim().to_owned();
                    this.set_custom_status((!t.is_empty()).then_some(t), cx);
                    this.people.close_overlays();
                }
            });
            self.people.custom_input = Some(input);
            self.people.custom_sub = Some(sub);
            self.people.status_picker = true;
        }
        cx.notify();
    }

    pub fn set_own_status(&mut self, p: Presence, cx: &mut Context<Self>) {
        self.presence = p;
        let custom = self.people.custom_status.as_ref().map(|s| s.to_string());
        cx.background_spawn(self.backend.set_status(p, custom)).detach();
        self.people.close_overlays();
        cx.notify();
    }

    pub fn set_custom_status(&mut self, text: Option<String>, cx: &mut Context<Self>) {
        self.people.custom_status = text.clone().map(Into::into);
        cx.background_spawn(self.backend.set_status(self.presence, text)).detach();
        cx.notify();
    }
}

const ITEMS: [(Presence, Option<&str>); 4] = [
    (Presence::Online, None),
    (Presence::Idle, None),
    (Presence::Dnd, Some("You will not receive any desktop notifications.")),
    (Presence::Offline, Some("You will not appear online, but will have full access to all of Discord.")),
];

pub fn render(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let bg = c(Palette::BG_FLOATING);
    let mut menu = div()
        .id("status-picker")
        .absolute()
        .left(px(m::SERVER_BAR_W + 8.))
        .bottom(px(m::USER_PANEL_H + 4.))
        .w(z(260.))
        .p(z(8.))
        .rounded(z(8.))
        .bg(bg)
        .border_1()
        .border_color(c(Palette::PANEL_BORDER))
        .shadow_lg()
        .occlude()
        .flex()
        .flex_col()
        .gap(z(2.));
    if let Some(input) = &app.people.custom_input {
        let has = app.people.custom_status.is_some();
        menu = menu
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(z(6.))
                    .px(z(8.))
                    .h(z(36.))
                    .rounded(z(4.))
                    .bg(c(Palette::BG_TERTIARY))
                    .text_size(z(14.))
                    .child(div().flex_1().min_w_0().child(Input::new(input).appearance(false).bordered(false)))
                    .when(has, |d| {
                        d.child(
                            div()
                                .id("custom-clear")
                                .text_size(z(12.))
                                .text_color(c(Palette::TEXT_MUTED))
                                .cursor_pointer()
                                .hover(|s| s.text_color(c(Palette::TEXT_NORMAL)))
                                .on_click(cx.listener(|a, _, _, cx| {
                                    a.set_custom_status(None, cx);
                                    a.people.close_overlays();
                                }))
                                .child("Clear"),
                        )
                    }),
            )
            .child(div().h(z(1.)).mx(z(4.)).my(z(6.)).bg(c(Palette::DIVIDER)));
    }
    for (p, desc) in ITEMS {
        let selected = app.presence == p;
        menu = menu.child(
            div()
                .id(("status", p as usize))
                .px(z(8.))
                .py(z(6.))
                .rounded(z(4.))
                .cursor_pointer()
                .flex()
                .gap(z(10.))
                .when(selected, |d| d.bg(c(Palette::BG_MOD_HOVER)))
                .hover(|s| s.bg(c(Palette::BLURPLE)))
                .on_click(cx.listener(move |a, _, _, cx| a.set_own_status(p, cx)))
                .child(div().pt(z(3.)).child(status_dot(p, 10., bg)))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .min_w_0()
                        .child(
                            div()
                                .text_size(z(14.))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(c(Palette::TEXT_NORMAL))
                                .child(status_label(p)),
                        )
                        .when_some(desc, |d, t| {
                            d.child(div().text_size(z(12.)).line_height(z(16.)).text_color(c(Palette::TEXT_MUTED)).child(t))
                        }),
                ),
        );
    }
    menu.into_any_element()
}
