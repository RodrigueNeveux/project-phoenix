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
    /// Row with a chevron that opens a nested menu on hover.
    Submenu { label: SharedString, items: Vec<MenuItem> },
}

thread_local! {
    /// Index of the submenu row currently open (menus are modal, one at a time).
    static OPEN_SUB: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

fn set_open_sub(v: Option<usize>, w: &mut Window) {
    if OPEN_SUB.with(|c| c.replace(v)) != v {
        w.refresh();
    }
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

    pub fn submenu(label: impl Into<SharedString>, items: Vec<MenuItem>) -> Self {
        Self::Submenu { label: label.into(), items }
    }
}

// Observed: min-width 188 / max 320, padding 6px 8px, radius 4, 1px subtle
// border; items 32px min, padding 6px 8px, margin 2px 0, radius 2,
// 14px/500 with 18px line height; hover brand-560.
pub const MENU_W: f32 = 220.;
const MENU_BG: u32 = 0x111214;
const MENU_BORDER: u32 = 0x97979F1F;
const HOVER_BG: u32 = 0x4752C4;
const CHEVRON: &str = "icons/chevron-right.svg";

/// Menu panel only (no backdrop), for embedding.
pub fn panel(id: &'static str, items: Vec<MenuItem>, dismiss: Dismiss) -> gpui::Stateful<gpui::Div> {
    let mut list = div()
        .id(id)
        .w(px(MENU_W))
        .py(px(6.))
        .px(px(8.))
        .rounded(px(4.))
        .border_1()
        .border_color(crate::theme::ca(MENU_BORDER))
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
            MenuItem::Submenu { label, items } => {
                let open = OPEN_SUB.with(|c| c.get()) == Some(ix);
                div()
                    .id(("menu-sub", ix))
                    .relative()
                    .h(px(32.))
                    .px(px(8.))
                    .my(px(2.))
                    .rounded(px(2.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .text_size(px(14.))
                    .line_height(px(18.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c(if open { 0xFFFFFF } else { 0xB5BAC1 }))
                    .when(open, |d| d.bg(c(HOVER_BG)))
                    .on_hover(move |h, w, _| {
                        if *h {
                            set_open_sub(Some(ix), w)
                        }
                    })
                    .child(label)
                    .child(svg().path(CHEVRON).size(px(18.)).flex_none().text_color(c(if open { 0xFFFFFF } else { 0xB5BAC1 })))
                    .when(open, |d| {
                        d.child(
                            div()
                                .absolute()
                                .top(px(-6.))
                                .left(px(MENU_W - 8.))
                                .child(panel("context-submenu", items, dismiss.clone())),
                        )
                    })
                    .into_any_element()
            }
            MenuItem::Action { label, icon, danger, on_click } => {
                let fg = if danger { Palette::RED } else { 0xB5BAC1 };
                let dismiss = dismiss.clone();
                div()
                    .id(("menu-item", ix))
                    .group("menu-item")
                    .h(px(32.))
                    .px(px(8.))
                    .my(px(2.))
                    .rounded(px(2.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .text_size(px(14.))
                    .line_height(px(18.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c(fg))
                    .on_hover(|h, w, _| {
                        if *h {
                            set_open_sub(None, w)
                        }
                    })
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
    let inner = dismiss;
    let dismiss: Dismiss = Rc::new(move |w, cx| {
        OPEN_SUB.with(|c| c.set(None));
        inner(w, cx)
    });
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
