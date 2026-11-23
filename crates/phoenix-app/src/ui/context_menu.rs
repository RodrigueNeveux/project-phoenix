//! Discord-style right-click menu: dark floating panel anchored at the
//! cursor, 8px padding, 32px rows, blurple hover, red "danger" rows.
//!
//! The menu is rendered as a full-window overlay (transparent backdrop that
//! dismisses on any click outside the panel).

use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, FontWeight, IntoElement, MouseButton, ParentElement, Pixels, Point,
    SharedString, Styled, Window, anchored, deferred, div, prelude::*, px, svg,
};

use crate::theme::{Palette, c};

pub type Handler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
pub type Dismiss = Rc<dyn Fn(&mut Window, &mut App)>;

pub enum MenuItem {
    Action { label: SharedString, icon: Option<SharedString>, danger: bool, on_click: Handler },
    Separator,
    /// Arbitrary row (e.g. the quick-reaction strip).
    Custom(AnyElement),
}

impl MenuItem {
    pub fn action(
        label: impl Into<SharedString>,
        icon: Option<SharedString>,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self::Action { label: label.into(), icon, danger: false, on_click: Rc::new(on_click) }
    }

    pub fn danger(
        label: impl Into<SharedString>,
        icon: Option<SharedString>,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self::Action { label: label.into(), icon, danger: true, on_click: Rc::new(on_click) }
    }
}

pub const MENU_W: f32 = 220.;
const MENU_BG: u32 = 0x111214;
const HOVER_BG: u32 = 0x4752C4;

/// Menu panel only (no backdrop), for embedding.
pub fn panel(id: &'static str, items: Vec<MenuItem>, dismiss: Dismiss) -> gpui::Stateful<gpui::Div> {
    let mut list = div()
        .id(id)
        .w(px(MENU_W))
        .p(px(8.))
        .rounded(px(4.))
        .bg(c(MENU_BG))
        .shadow_lg()
        .flex()
        .flex_col()
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation());
    for (ix, item) in items.into_iter().enumerate() {
        list = list.child(match item {
            MenuItem::Separator => div().mx(px(4.)).my(px(4.)).h(px(1.)).bg(c(0x2E2F34)).into_any_element(),
            MenuItem::Custom(el) => el,
            MenuItem::Action { label, icon, danger, on_click } => {
                let fg = if danger { Palette::RED } else { 0xB5BAC1 };
                let dismiss = dismiss.clone();
                div()
                    .id(("menu-item", ix))
                    .group("menu-item")
                    .h(px(32.))
                    .px(px(8.))
                    .my(px(1.))
                    .rounded(px(2.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .text_size(px(14.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c(fg))
                    .hover(move |s| s.bg(c(if danger { Palette::RED } else { HOVER_BG })).text_color(gpui::white()))
                    .child(label)
                    .when_some(icon, |d, path| {
                        d.child(
                            svg()
                                .path(path)
                                .size(px(18.))
                                .flex_none()
                                .text_color(c(fg))
                                .group_hover("menu-item", |s| s.text_color(gpui::white())),
                        )
                    })
                    .on_click(move |ev, w, cx| {
                        on_click(ev, w, cx);
                        dismiss(w, cx);
                    })
                    .into_any_element()
            }
        });
    }
    list
}

/// Full-window overlay with the menu anchored at `pos`.
pub fn render(pos: Point<Pixels>, items: Vec<MenuItem>, dismiss: Dismiss) -> AnyElement {
    let d1 = dismiss.clone();
    let d2 = dismiss.clone();
    deferred(
        div()
            .id("context-menu-backdrop")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .occlude()
            .on_mouse_down(MouseButton::Left, move |_, w, cx| d1(w, cx))
            .on_mouse_down(MouseButton::Right, move |_, w, cx| d2(w, cx))
            .child(
                anchored()
                    .position(pos)
                    .snap_to_window_with_margin(px(8.))
                    .child(panel("context-menu", items, dismiss)),
            ),
    )
    .with_priority(10)
    .into_any_element()
}
