use gpui::{AnyElement, Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, prelude::*};
use phoenix_api::rest::collectibles::LookSku;
use serde_json::json;

use crate::{
    actions::with_modal,
    app::PhoenixApp,
    model::Id,
    theme::{c, generated::idx, tok, z},
    ui::{
        avatar,
        controls::{BtnKind, button},
        home::data::RelationshipKind,
        modals::{Modal, card},
        shop_page::{ORB_CURRENCY, Shown, card_of, format_price, shown_price},
    },
};

pub const WIDTH: f32 = 480.;
pub const FRIENDS_MAX_H: f32 = 220.;

pub struct State {
    pub look: LookSku,
    pub gift: bool,
    pub recipient: Option<Id>,
}

pub fn charge(l: &LookSku, premium: bool, balance: Option<u64>) -> Option<(i64, String)> {
    let k = card_of(&json!({ "sku_id": l.sku.to_string(), "name": l.name, "prices": l.prices }))?;
    match shown_price(&k, premium, balance)? {
        Shown::Fiat { price, .. } => Some((price.0, price.2)),
        Shown::Orbs(o) => Some((o as i64, ORB_CURRENCY.to_owned())),
    }
}

pub fn label(l: &LookSku, premium: bool, balance: Option<u64>) -> Option<String> {
    let k = card_of(&json!({ "sku_id": l.sku.to_string(), "name": l.name, "prices": l.prices }))?;
    Some(match shown_price(&k, premium, balance)? {
        Shown::Fiat { price, .. } => format_price(price.0, price.1, &price.2),
        Shown::Orbs(o) => crate::t!("shop.orbs_amount", n = o).to_string(),
    })
}

pub fn can_confirm(s: &State) -> bool {
    !s.gift || s.recipient.is_some()
}

fn heading(t: SharedString) -> impl IntoElement {
    div().mb(z(8.)).text_size(z(12.)).line_height(z(16.)).font_weight(FontWeight::SEMIBOLD).text_color(c(tok(idx::TEXT_MUTED))).child(t.to_uppercase())
}

pub fn render(app: &PhoenixApp, s: &State, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let premium = app.backend.has_premium();
    let balance = crate::ui::shop_page::orbs_balance(app, cx);
    let price = label(&s.look, premium, balance);
    let product = div()
        .mb(z(16.))
        .p(z(12.))
        .rounded(z(8.))
        .bg(c(tok(idx::BACKGROUND_MOD_MUTED)))
        .border_1()
        .border_color(c(tok(idx::BORDER_SUBTLE)))
        .flex()
        .items_center()
        .gap(z(12.))
        .child(div().size(z(48.)).flex_none().flex().items_center().justify_center().child(crate::ui::shop_look::preview(&s.look)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().text_size(z(16.)).line_height(z(20.)).font_weight(FontWeight::SEMIBOLD).text_color(c(tok(idx::TEXT_STRONG))).truncate().child(SharedString::from(s.look.name.clone())))
                .children(crate::ui::shop_look::type_label(s.look.kind).map(|t| div().text_size(z(14.)).line_height(z(18.)).text_color(c(tok(idx::TEXT_MUTED))).child(t))),
        )
        .children(price.clone().map(|p| div().flex_none().text_size(z(16.)).line_height(z(20.)).font_weight(FontWeight::SEMIBOLD).text_color(c(tok(idx::TEXT_STRONG))).child(p)));
    let mut body = div().flex().flex_col().pt(z(4.)).child(product);
    if s.gift {
        let friends: Vec<Id> = app.nav.relationships.iter().filter(|r| r.kind == RelationshipKind::Friend).map(|r| r.user).collect();
        let mut list = div().id("shop-gift-friends").max_h(z(FRIENDS_MAX_H)).overflow_y_scroll().flex().flex_col().gap(z(2.));
        for (i, uid) in friends.into_iter().enumerate() {
            let u = app.users.get(&uid);
            let selected = s.recipient == Some(uid);
            let name = u.map(|u| u.display_name.clone()).unwrap_or_else(|| SharedString::from(uid.0.to_string()));
            list = list.child(
                div()
                    .id(("shop-gift-friend", i))
                    .h(z(40.))
                    .px(z(8.))
                    .rounded(z(8.))
                    .flex()
                    .items_center()
                    .gap(z(10.))
                    .cursor_pointer()
                    .when(selected, |d| d.bg(c(tok(idx::BACKGROUND_MOD_STRONG))))
                    .hover(|st| st.bg(c(tok(idx::BACKGROUND_MOD_SUBTLE))))
                    .on_click(cx.listener(move |_, _, _, cx| {
                        with_modal(|m| {
                            if let Some(Modal::ShopPurchase(st)) = m {
                                st.recipient = Some(uid);
                            }
                        });
                        cx.notify();
                    }))
                    .child(avatar(u, 24.))
                    .child(div().flex_1().min_w_0().truncate().text_size(z(16.)).line_height(z(20.)).font_weight(FontWeight::MEDIUM).text_color(c(tok(idx::INTERACTIVE_TEXT_ACTIVE))).child(name)),
            );
        }
        body = body.child(heading(crate::t!("shop.gift_recipient"))).child(div().mb(z(16.)).child(list));
    }
    body = body.child(heading(crate::t!("shop.payment_method"))).child(
        div()
            .p(z(12.))
            .rounded(z(8.))
            .bg(c(tok(idx::BACKGROUND_MOD_MUTED)))
            .text_size(z(14.))
            .line_height(z(18.))
            .text_color(c(tok(idx::TEXT_DEFAULT)))
            .child(crate::t!("shop.no_purchase_notice")),
    );
    let ok = can_confirm(s);
    let close = crate::ui::modals::close_action(cx);
    let confirm = match (s.gift, &price) {
        (true, _) => crate::t!("shop.gift_send"),
        (false, Some(p)) => crate::t!("shop.buy_for", price = p.clone()),
        (false, None) => crate::t!("shop.buy"),
    };
    let footer = div()
        .flex()
        .gap(z(8.))
        .child(button("shop-purchase-cancel", crate::t!("common.cancel"), BtnKind::Secondary).flex_1().on_click(move |_, w, cx| close(w, cx)))
        .child(
            button("shop-purchase-confirm", confirm, BtnKind::Primary)
                .flex_1()
                .when(!ok, |d| d.opacity(0.5).cursor_not_allowed())
                .on_click(cx.listener(move |a, _, _, cx| a.confirm_shop_purchase(cx))),
        )
        .into_any_element();
    let title = if s.gift { crate::t!("shop.gift_item") } else { SharedString::from(s.look.name.clone()) };
    card(WIDTH, title, None, body.into_any_element(), Some(footer), cx)
}

impl PhoenixApp {
    pub fn open_shop_purchase(&mut self, look: LookSku, gift: bool, cx: &mut Context<Self>) {
        crate::ui::extra_menus::close();
        self.people.popout = None;
        crate::actions::open_modal(Modal::ShopPurchase(Box::new(State { look, gift, recipient: None })), None);
        cx.notify();
    }

    pub fn confirm_shop_purchase(&mut self, cx: &mut Context<Self>) {
        let premium = self.backend.has_premium();
        let balance = crate::ui::shop_page::orbs_balance(self, cx);
        let mut write = None;
        with_modal(|m| {
            if let Some(Modal::ShopPurchase(st)) = m
                && can_confirm(st)
            {
                write = charge(&st.look, premium, balance).map(|(amount, cur)| crate::actions::guard::GuardedWrite::sku_purchase(st.look.sku, st.gift, amount, &cur));
            }
        });
        if let Some(w) = write {
            self.blocked_write(w, cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look(prices: serde_json::Value) -> LookSku {
        LookSku { sku: 5, kind: 0, name: "Bunny".into(), shoppable: true, item: json!({}), summary: String::new(), background: None, prices }
    }

    #[test]
    fn the_charge_follows_the_shown_price() {
        let l = look(json!({
            "0": {"country_prices": {"prices": [{"amount": 599, "currency": "eur", "exponent": 2}, {"amount": 1200, "currency": "discord_orb", "exponent": 0}]}},
            "4": {"country_prices": {"prices": [{"amount": 449, "currency": "eur", "exponent": 2}]}}
        }));
        assert_eq!(charge(&l, false, None), Some((599, "eur".into())));
        assert_eq!(charge(&l, true, Some(10)), Some((449, "eur".into())));
        assert_eq!(charge(&l, false, Some(5000)), Some((1200, ORB_CURRENCY.into())));
        assert_eq!(charge(&look(serde_json::Value::Null), false, None), None);
    }

    #[test]
    fn gifts_need_a_recipient() {
        let mut s = State { look: look(serde_json::Value::Null), gift: true, recipient: None };
        assert!(!can_confirm(&s));
        s.recipient = Some(Id(3));
        assert!(can_confirm(&s));
        s.gift = false;
        s.recipient = None;
        assert!(can_confirm(&s));
    }
}
