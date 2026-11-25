use gpui::{AnyElement, App, Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, prelude::*};

use crate::{
    app::PhoenixApp,
    theme::{c, generated::idx, tok, z},
    ui::{
        context_menu::MenuItem,
        controls::{BtnKind, button},
        modals::card,
    },
};

thread_local! {
    static MENU: std::cell::Cell<Option<gpui::Point<gpui::Pixels>>> = const { std::cell::Cell::new(None) };
}

fn account_card(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> gpui::Div {
    let me = app.users.get(&app.me.id);
    let name = SharedString::from(app.me.name.to_string());
    let more = div()
        .id("acct-more")
        .size(z(32.))
        .flex_none()
        .rounded(z(8.))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|s| s.bg(c(tok(idx::BACKGROUND_MOD_SUBTLE))))
        .on_click(cx.listener(|_, ev: &gpui::ClickEvent, w, cx| {
            MENU.with(|m| m.set(Some(ev.position())));
            w.refresh();
            cx.notify();
        }))
        .child(gpui::svg().path(crate::icons::path("more-horizontal")).size(z(20.)).text_color(c(tok(idx::INTERACTIVE_TEXT_DEFAULT))));
    div().flex().px(z(16.)).py(z(12.)).child(
        div()
            .w_full()
            .flex()
            .items_center()
            .child(crate::ui::avatar(me, 40.))
            .child(
                div()
                    .ml(z(8.))
                    .max_w(gpui::relative(0.76))
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .text_size(z(16.))
                            .line_height(z(20.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(c(tok(idx::TEXT_STRONG)))
                            .child(name),
                    )
                    .child(
                        div()
                            .text_size(z(14.))
                            .line_height(z(18.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(c(tok(idx::TEXT_FEEDBACK_POSITIVE)))
                            .child(crate::t!("accounts.active")),
                    ),
            )
            .child(div().ml_auto().flex().items_center().gap(z(8.)).child(more)),
    )
}

pub fn render(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let list = div().flex().flex_col().rounded(z(8.)).bg(c(tok(idx::BACKGROUND_MOD_SUBTLE))).child(account_card(app, cx));
    let add = button("acct-add", crate::t!("status.add_account"), BtnKind::Secondary)
        .flex_1()
        .on_click(cx.listener(|a, _, _, cx| a.stub(&crate::t!("status.add_account"), cx)));
    let footer = div().w_full().flex().child(add);
    let mut body = div().relative().pb(z(16.)).child(list);
    if let Some(pos) = MENU.with(|m| m.get()) {
        let e = cx.entity();
        let items = vec![MenuItem::danger(crate::t!("accounts.log_out"), None, move |_, _, cx: &mut App| {
            MENU.with(|m| m.set(None));
            e.update(cx, |a, cx| a.stub(&crate::t!("accounts.log_out"), cx));
        })];
        let dismiss: crate::ui::context_menu::Dismiss = std::rc::Rc::new(|w, _| {
            MENU.with(|m| m.set(None));
            w.refresh();
        });
        body = body.child(crate::ui::context_menu::render_anchored_at(pos, gpui::Anchor::TopLeft, items, dismiss, 40));
    }
    card(480., crate::t!("status.manage_accounts"), Some(crate::t!("accounts.subtitle")), body.into_any_element(), Some(footer.into_any_element()), cx)
}

impl PhoenixApp {
    pub fn open_switch_accounts(&mut self, cx: &mut Context<Self>) {
        self.people.close_overlays();
        MENU.with(|m| m.set(None));
        crate::actions::open_modal(crate::ui::modals::Modal::SwitchAccounts, None);
        cx.notify();
    }
}
