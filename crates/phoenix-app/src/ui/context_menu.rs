//! Discord-style right-click menu: dark floating panel anchored at the
//! cursor, 8px padding, 32px rows, blurple hover, red "danger" rows.
//!
//! The menu is rendered as a full-window overlay (transparent backdrop that
//! dismisses on any click outside the panel). Keyboard: Up/Down move the
//! highlight, Enter/Space activate, Right/Left open/close a submenu, Esc closes.

use crate::theme::z;
use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, FontWeight, IntoElement, MouseButton, ParentElement, Pixels, Point,
    SharedString, Styled, Window, anchored, deferred, div, prelude::*, px, svg,
};

use crate::theme::{Palette, c, ca, generated::idx, tok};

pub type Handler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
pub type Dismiss = Rc<dyn Fn(&mut Window, &mut App)>;

pub enum MenuItem {
    Action { label: SharedString, icon: Option<SharedString>, danger: bool, on_click: Handler },
    Separator,
    /// Arbitrary row (e.g. the quick-reaction strip).
    Custom(AnyElement),
    /// Row with a chevron that opens a nested menu on hover.
    Submenu { label: SharedString, items: Vec<MenuItem> },
    /// Checkbox (`radio: false`) or radio row; the menu stays open on click
    /// for checkboxes, like Discord.
    Check { label: SharedString, checked: bool, radio: bool, on_click: Handler },
}

thread_local! {
    /// Index of the submenu row currently open (menus are modal, one at a time).
    static OPEN_SUB: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
    /// Highlighted row in the root menu (keyboard focus or mouse hover).
    static HL_ROOT: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
    /// Highlighted row inside the open submenu; `Some` means the keyboard is in it.
    static HL_SUB: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

fn set_hl(depth: u8, v: Option<usize>, w: &mut Window) {
    let cell = if depth == 0 { &HL_ROOT } else { &HL_SUB };
    if cell.with(|c| c.replace(v)) != v {
        w.refresh();
    }
}

fn reset_state() {
    OPEN_SUB.with(|c| c.set(None));
    HL_ROOT.with(|c| c.set(None));
    HL_SUB.with(|c| c.set(None));
}

/// What the keyboard can do with each row.
enum Nav {
    Skip,
    Act(Handler),
    Sub(Vec<Option<Handler>>),
}

fn nav_of(items: &[MenuItem]) -> Vec<Nav> {
    items
        .iter()
        .map(|i| match i {
            MenuItem::Action { on_click, .. } | MenuItem::Check { on_click, .. } => Nav::Act(on_click.clone()),
            MenuItem::Submenu { items, .. } => Nav::Sub(
                items
                    .iter()
                    .map(|i| match i {
                        MenuItem::Action { on_click, .. } | MenuItem::Check { on_click, .. } => {
                            Some(on_click.clone())
                        }
                        _ => None,
                    })
                    .collect(),
            ),
            _ => Nav::Skip,
        })
        .collect()
}

/// Next selectable index from `cur` in direction `dir` (wraps like Discord).
fn step(ok: &[bool], cur: Option<usize>, dir: isize) -> Option<usize> {
    let n = ok.len() as isize;
    if n == 0 || !ok.iter().any(|b| *b) {
        return None;
    }
    let mut i = match cur {
        Some(c) => c as isize,
        None if dir > 0 => -1,
        None => n,
    };
    for _ in 0..n {
        i = (i + dir).rem_euclid(n);
        if ok[i as usize] {
            return Some(i as usize);
        }
    }
    None
}

fn keyboard_click(pos: Point<Pixels>) -> ClickEvent {
    ClickEvent::Keyboard(gpui::KeyboardClickEvent {
        button: gpui::KeyboardButton::Enter,
        bounds: gpui::Bounds::new(pos, gpui::size(z(0.), z(0.))),
    })
}

/// Arrows move, Enter/Space activate, Right/Left enter/leave a submenu,
/// Escape closes the submenu then the menu.
fn handle_key(key: &str, nav: &[Nav], pos: Point<Pixels>, dismiss: &Dismiss, w: &mut Window, cx: &mut App) -> bool {
    let root = HL_ROOT.with(|c| c.get());
    let sub = HL_SUB.with(|c| c.get());
    let open = OPEN_SUB.with(|c| c.get());
    let sub_list = open.and_then(|o| match &nav[o] {
        Nav::Sub(v) => Some(v),
        _ => None,
    });
    let in_sub = sub.is_some() && sub_list.is_some();
    let root_ok: Vec<bool> = nav.iter().map(|n| !matches!(n, Nav::Skip)).collect();
    match key {
        "down" | "up" | "tab" => {
            let dir = if key == "up" { -1 } else { 1 };
            if in_sub {
                let ok: Vec<bool> = sub_list.unwrap().iter().map(Option::is_some).collect();
                set_hl(1, step(&ok, sub, dir), w);
            } else {
                let next = step(&root_ok, root, dir);
                set_hl(0, next, w);
                set_open_sub(None, w);
            }
        }
        "right" if !in_sub => {
            if let Some(r) = root
                && let Nav::Sub(v) = &nav[r]
            {
                set_open_sub(Some(r), w);
                let ok: Vec<bool> = v.iter().map(Option::is_some).collect();
                set_hl(1, step(&ok, None, 1), w);
            }
        }
        "left" if in_sub => {
            set_hl(1, None, w);
            set_open_sub(None, w);
        }
        "escape" => {
            if in_sub {
                set_hl(1, None, w);
                set_open_sub(None, w);
            } else {
                dismiss(w, cx);
            }
        }
        "enter" | "space" => {
            if in_sub {
                if let Some(Some(h)) = sub.and_then(|s| sub_list.unwrap().get(s)) {
                    h(&keyboard_click(pos), w, cx);
                    dismiss(w, cx);
                }
            } else if let Some(r) = root {
                match &nav[r] {
                    Nav::Act(h) => {
                        h(&keyboard_click(pos), w, cx);
                        dismiss(w, cx);
                    }
                    Nav::Sub(_) => return handle_key("right", nav, pos, dismiss, w, cx),
                    Nav::Skip => {}
                }
            }
        }
        _ => return false,
    }
    true
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

    pub fn check(
        label: impl Into<SharedString>,
        checked: bool,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self::Check { label: label.into(), checked, radio: false, on_click: Rc::new(on_click) }
    }

    pub fn radio(
        label: impl Into<SharedString>,
        checked: bool,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self::Check { label: label.into(), checked, radio: true, on_click: Rc::new(on_click) }
    }

    pub fn submenu(label: impl Into<SharedString>, items: Vec<MenuItem>) -> Self {
        Self::Submenu { label: label.into(), items }
    }
}

// menu_M3qk4j (476662): --background-surface-higher, 1px --border-subtle
// (--menu-border-width), radius 8px, --shadow-high, min-width 188 / submenu
// max 320; scroller_M3qk4j padding-block --menu-scroller-block-padding (8px),
// padding-inline 8px. labelContainer_M3qk4j min-height 32, padding 8;
// item radius 2 (4 when focused); focused bg --interactive-background-hover
// with --text-strong; danger --text-feedback-critical on
// --background-feedback-mod-critical; iconContainer/caret 20px;
// separator margin --custom-menu-separator-margin (8px).
pub const MENU_W: f32 = 220.;
const MENU_BG: u32 = tok(idx::BACKGROUND_SURFACE_HIGHER);
const MENU_BORDER: u32 = tok(idx::BORDER_SUBTLE);
const HOVER_BG: u32 = tok(idx::INTERACTIVE_BACKGROUND_HOVER);
const DANGER_BG: u32 = tok(idx::BACKGROUND_FEEDBACK_MOD_CRITICAL);
const CHEVRON: &str = "icons/chevron-right.svg";
const CHECK: &str = "icons/check.svg";

/// Menu panel only (no backdrop), for embedding.
pub fn panel(id: &'static str, items: Vec<MenuItem>, dismiss: Dismiss) -> gpui::Stateful<gpui::Div> {
    panel_at(id, 0, items, dismiss)
}

fn panel_at(id: &'static str, depth: u8, items: Vec<MenuItem>, dismiss: Dismiss) -> gpui::Stateful<gpui::Div> {
    let hl = if depth == 0 { HL_ROOT.with(|c| c.get()) } else { HL_SUB.with(|c| c.get()) };
    let mut list = div()
        .id(id)
        .w(px(MENU_W))
        .py(z(8.))
        .px(z(8.))
        .rounded(z(8.))
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
            MenuItem::Separator => div().m(z(8.)).h(z(1.)).bg(c(Palette::DIVIDER)).into_any_element(),
            MenuItem::Custom(el) => el,
            MenuItem::Submenu { label, items } => {
                let open = OPEN_SUB.with(|c| c.get()) == Some(ix);
                div()
                    .id(("menu-sub", ix))
                    .relative()
                    .h(z(32.))
                    .px(z(8.))
                    .rounded(z(4.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .text_size(z(14.))
                    .line_height(z(18.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c(Palette::TEXT_HEADER))
                    .when(open, |d| d.bg(ca(HOVER_BG)))
                    .on_hover(move |h, w, _| {
                        if *h {
                            set_open_sub(Some(ix), w);
                            set_hl(0, Some(ix), w);
                            set_hl(1, None, w);
                        }
                    })
                    .child(label)
                    .child(svg().path(CHEVRON).size(z(20.)).flex_none().text_color(c(Palette::TEXT_HEADER)))
                    .when(open, |d| {
                        d.child(
                            div()
                                .absolute()
                                .top(z(-6.))
                                .left(px(MENU_W - 8.))
                                .child(panel_at("context-submenu", 1, items, dismiss.clone())),
                        )
                    })
                    .into_any_element()
            }
            MenuItem::Check { label, checked, radio, on_click } => {
                let dismiss = dismiss.clone();
                let lit = hl == Some(ix);
                let mark = if radio {
                    div()
                        .size(z(18.))
                        .flex_none()
                        .rounded_full()
                        .border_2()
                        .border_color(if lit { gpui::white() } else { c(Palette::INTERACTIVE) })
                        .flex()
                        .items_center()
                        .justify_center()
                        .when(checked, |d| {
                            d.child(div().size(z(8.)).rounded_full().bg(if lit { gpui::white() } else { c(Palette::BLURPLE) }))
                        })
                } else {
                    div()
                        .size(z(18.))
                        .flex_none()
                        .rounded(z(4.))
                        .border_1()
                        .border_color(if checked { c(Palette::BLURPLE) } else if lit { gpui::white() } else { c(Palette::INTERACTIVE) })
                        .when(checked, |d| d.bg(c(Palette::BLURPLE)).child(svg().path(CHECK).size(z(14.)).text_color(gpui::white())))
                        .flex()
                        .items_center()
                        .justify_center()
                };
                div()
                    .id(("menu-check", ix))
                    .h(z(32.))
                    .px(z(8.))
                    .rounded(z(4.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .text_size(z(14.))
                    .line_height(z(18.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c(Palette::INTERACTIVE))
                    .on_hover(move |h, w, _| {
                        if *h {
                            if depth == 0 {
                                set_open_sub(None, w);
                            }
                            set_hl(depth, Some(ix), w);
                        }
                    })
                    .when(lit, |d| d.bg(ca(HOVER_BG)))
                    .hover(|s| s.bg(ca(HOVER_BG)))
                    .child(label)
                    .child(mark)
                    .on_click(move |ev, w, cx| {
                        on_click(ev, w, cx);
                        if radio {
                            dismiss(w, cx);
                        } else {
                            w.refresh();
                        }
                    })
                    .into_any_element()
            }
            MenuItem::Action { label, icon, danger, on_click } => {
                let fg = if danger { Palette::TEXT_CRITICAL } else { Palette::TEXT_HEADER };
                let dismiss = dismiss.clone();
                div()
                    .id(("menu-item", ix))
                    .group("menu-item")
                    .h(z(32.))
                    .px(z(8.))
                    .rounded(z(4.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .text_size(z(14.))
                    .line_height(z(18.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c(fg))
                    .on_hover(move |h, w, _| {
                        if *h {
                            if depth == 0 {
                                set_open_sub(None, w);
                            }
                            set_hl(depth, Some(ix), w);
                        } else if hl == Some(ix) {
                            set_hl(depth, None, w);
                        }
                    })
                    .when(hl == Some(ix), |d| {
                        d.bg(ca(if danger { DANGER_BG } else { HOVER_BG }))
                    })
                    .hover(move |s| s.bg(ca(if danger { DANGER_BG } else { HOVER_BG })))
                    .child(label)
                    .when_some(icon, |d, path| {
                        d.child(
                            svg()
                                .path(path)
                                .size(z(20.))
                                .flex_none()
                                .text_color(c(fg)),
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
        reset_state();
        inner(w, cx)
    });
    let nav = Rc::new(nav_of(&items));
    let kd = dismiss.clone();
    let keys = gpui::canvas(
        |_, _, _| (),
        move |_, _, w, _| {
            w.on_key_event(move |ev: &gpui::KeyDownEvent, phase, w, cx| {
                if phase == gpui::DispatchPhase::Capture
                    && !ev.keystroke.modifiers.modified()
                    && handle_key(&ev.keystroke.key, &nav, pos, &kd, w, cx)
                {
                    cx.stop_propagation();
                }
            });
        },
    )
    .absolute()
    .size_0();
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
            .child(keys)
            .child(
                anchored()
                    .position(pos)
                    .snap_to_window_with_margin(z(8.))
                    .child(panel("context-menu", items, dismiss)),
            ),
    )
    .with_priority(10)
    .into_any_element()
}
