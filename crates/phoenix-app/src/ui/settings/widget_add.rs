use std::cell::Cell;

use gpui::{AnyElement, Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, prelude::*};
use serde_json::{Value, json};

use crate::{
    app::PhoenixApp,
    theme::{c, generated::idx, tok, z},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Interests,
    GameStats,
}

pub const INTERESTS: [(&str, &str); 4] = [
    ("favorite_games", "pe.widget_favorite"),
    ("played_games", "pe.widget_played"),
    ("current_games", "pe.widget_current"),
    ("want_to_play_games", "pe.widget_want"),
];

thread_local! {
    static OPEN: Cell<Option<Category>> = const { Cell::new(None) };
}

pub fn open() {
    OPEN.with(|o| o.set(Some(Category::Interests)));
}

pub fn close() -> bool {
    OPEN.with(|o| o.take().is_some())
}

pub fn is_open() -> Option<Category> {
    OPEN.with(Cell::get)
}

pub fn submission(existing: &[u64], kind: &str) -> Value {
    let mut out: Vec<Value> = existing.iter().map(|id| json!({ "data": { "type": "application", "application_id": id.to_string() } })).collect();
    out.push(json!({ "data": { "type": kind, "games": [] } }));
    Value::Array(out)
}

fn nav_item(id: usize, label: SharedString, on: bool, cat: Category, cx: &mut Context<PhoenixApp>) -> AnyElement {
    div()
        .id(("widget-add-nav", id))
        .flex()
        .items_center()
        .gap(z(12.))
        .pt(z(8.))
        .pb(z(8.))
        .pl(z(8.))
        .pr(z(12.))
        .rounded(z(8.))
        .text_size(z(16.))
        .line_height(z(24.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(c(tok(if on { idx::INTERACTIVE_TEXT_ACTIVE } else { idx::INTERACTIVE_TEXT_DEFAULT })))
        .cursor_pointer()
        .when(on, |d| d.bg(c(tok(idx::BACKGROUND_MOD_NORMAL))).hover(|s| s.bg(c(tok(idx::BACKGROUND_MOD_STRONG)))))
        .when(!on, |d| d.hover(|s| s.bg(c(tok(idx::INTERACTIVE_BACKGROUND_HOVER))).text_color(c(tok(idx::INTERACTIVE_TEXT_HOVER)))))
        .on_click(cx.listener(move |_, _, _, cx| {
            OPEN.with(|o| o.set(Some(cat)));
            cx.notify();
        }))
        .child(label)
        .into_any_element()
}

fn interest_card(ix: usize, kind: &'static str, key: &'static str, existing: Vec<u64>, cx: &mut Context<PhoenixApp>) -> AnyElement {
    div()
        .id(("widget-add-interest", ix))
        .w_full()
        .p(z(16.))
        .rounded(z(12.))
        .bg(c(tok(idx::BACKGROUND_MOD_SUBTLE)))
        .border_1()
        .border_color(c(tok(idx::BORDER_SUBTLE)))
        .flex()
        .items_center()
        .gap(z(12.))
        .cursor_pointer()
        .hover(|s| s.bg(c(tok(idx::BACKGROUND_MOD_NORMAL))))
        .on_click(cx.listener(move |a, _, _, cx| {
            close();
            a.blocked_write(crate::actions::guard::GuardedWrite::put_profile_widgets(submission(&existing, kind)), cx);
            cx.notify();
        }))
        .child(div().flex_1().min_w_0().text_size(z(16.)).line_height(z(20.)).font_weight(FontWeight::SEMIBOLD).text_color(c(tok(idx::TEXT_STRONG))).child(crate::t!(key)))
        .child(gpui::svg().path(crate::icons::path("plus")).flex_none().size(z(20.)).text_color(c(tok(idx::ICON_SUBTLE))))
        .into_any_element()
}

pub fn modal(existing: Vec<u64>, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    let cat = is_open()?;
    let e = cx.entity();
    let close_action: crate::ui::modal::Action = std::rc::Rc::new(move |_, cx| {
        close();
        e.update(cx, |_, cx| cx.notify());
    });
    let close_btn = close_action.clone();
    let pane: AnyElement = match cat {
        Category::Interests => div()
            .py(z(24.))
            .pl(z(24.))
            .pr(z(8.))
            .flex()
            .flex_col()
            .gap(z(12.))
            .children(INTERESTS.iter().enumerate().map(|(i, (kind, key))| interest_card(i, kind, key, existing.clone(), cx)))
            .into_any_element(),
        Category::GameStats => div()
            .pt(z(64.))
            .px(z(48.))
            .text_center()
            .text_size(z(16.))
            .line_height(z(20.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(c(tok(idx::TEXT_SUBTLE)))
            .child(crate::t!("pe.widget_all_added", tabName = crate::t!("pe.widget_cat_stats")))
            .into_any_element(),
    };
    let card = div()
        .id("widget-add-modal")
        .relative()
        .w(z(800.))
        .rounded(z(12.))
        .bg(c(tok(idx::BACKGROUND_SURFACE_HIGH)))
        .border_1()
        .border_color(c(tok(idx::BORDER_SUBTLE)))
        .overflow_hidden()
        .occlude()
        .flex()
        .flex_col()
        .child(
            div()
                .pt(z(24.))
                .px(z(24.))
                .pb(z(16.))
                .pr(z(60.))
                .text_size(z(20.))
                .line_height(z(24.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c(tok(idx::TEXT_STRONG)))
                .child(crate::t!("pe.widget_add_title")),
        )
        .child(
            div()
                .id("widget-add-close")
                .absolute()
                .top(z(12.))
                .right(z(12.))
                .size(z(40.))
                .rounded(z(8.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(|s| s.bg(c(tok(idx::CONTROL_ICON_ONLY_BACKGROUND_HOVER))))
                .on_click(move |_, w, cx| close_btn(w, cx))
                .child(gpui::svg().path(crate::icons::path("close")).size(z(20.)).text_color(c(tok(idx::CONTROL_ICON_ONLY_ICON_DEFAULT)))),
        )
        .child(
            div()
                .mt(z(4.))
                .h(z(540.))
                .border_t_1()
                .border_color(c(tok(idx::BORDER_SUBTLE)))
                .flex()
                .child(
                    div()
                        .w(z(180.))
                        .flex_none()
                        .py(z(24.))
                        .px(z(16.))
                        .border_r_1()
                        .border_color(c(tok(idx::BORDER_SUBTLE)))
                        .flex()
                        .flex_col()
                        .child(nav_item(0, crate::t!("pe.widget_cat_interests"), cat == Category::Interests, Category::Interests, cx))
                        .child(nav_item(1, crate::t!("pe.widget_cat_stats"), cat == Category::GameStats, Category::GameStats, cx)),
                )
                .child(div().id("widget-add-pane").flex_1().min_w_0().overflow_y_scroll().child(pane)),
        );
    Some(crate::ui::modal::frame("widget-add-backdrop", close_action, card))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interest_widgets_follow_the_client_order() {
        assert_eq!(INTERESTS.map(|(k, _)| k), ["favorite_games", "played_games", "current_games", "want_to_play_games"]);
        let body = submission(&[42], "favorite_games");
        assert_eq!(body, json!([{ "data": { "type": "application", "application_id": "42" } }, { "data": { "type": "favorite_games", "games": [] } }]));
        let w = crate::actions::guard::GuardedWrite::put_profile_widgets(body);
        assert!(!crate::actions::guard::read_only_write(&w));
    }
}
