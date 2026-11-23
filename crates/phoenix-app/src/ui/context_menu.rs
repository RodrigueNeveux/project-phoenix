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
    SharedString, Styled, Window, anchored, deferred, div, prelude::*, svg,
};

use crate::theme::{c, generated::idx, tok};

pub type Handler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
pub type Dismiss = Rc<dyn Fn(&mut Window, &mut App)>;

pub enum MenuItem {
    /// `icon` sits on the left (iconContainerLeft_M3qk4j); `subtext` is the
    /// muted second line (subtext_M3qk4j), e.g. "Only visible to you".
    Action { label: SharedString, icon: Option<SharedString>, subtext: Option<SharedString>, danger: bool, on_click: Handler },
    Separator,
    /// Arbitrary row (e.g. the quick-reaction strip).
    Custom(AnyElement),
    /// Row with a chevron that opens a nested menu on hover.
    /// `subtext`: muted second line under the label (e.g. the current input
    /// device in the user panel's mic menu).
    Submenu { label: SharedString, items: Vec<MenuItem>, icon: Option<SharedString>, subtext: Option<SharedString> },
    /// Checkbox (`radio: false`) or radio row; the menu stays open on click
    /// for checkboxes, like Discord.
    Check { label: SharedString, checked: bool, radio: bool, subtext: Option<SharedString>, on_click: Handler },
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

/// Opens the submenu of row `ix` (offscreen screenshots only).
#[doc(hidden)]
pub fn debug_open_submenu(ix: usize) {
    OPEN_SUB.with(|c| c.set(Some(ix)));
    HL_ROOT.with(|c| c.set(Some(ix)));
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
        Self::Action { label: label.into(), icon, subtext: None, danger: false, on_click: Rc::new(on_click) }
    }

    pub fn danger(
        label: impl Into<SharedString>,
        icon: Option<SharedString>,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self::Action { label: label.into(), icon, subtext: None, danger: true, on_click: Rc::new(on_click) }
    }

    pub fn check(
        label: impl Into<SharedString>,
        checked: bool,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self::Check { label: label.into(), checked, radio: false, subtext: None, on_click: Rc::new(on_click) }
    }

    pub fn radio(
        label: impl Into<SharedString>,
        checked: bool,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self::Check { label: label.into(), checked, radio: true, subtext: None, on_click: Rc::new(on_click) }
    }

    /// Adds the muted second line under an action's label.
    pub fn with_subtext(mut self, text: impl Into<SharedString>) -> Self {
        if let Self::Action { subtext, .. } | Self::Submenu { subtext, .. } | Self::Check { subtext, .. } = &mut self {
            *subtext = Some(text.into());
        }
        self
    }

    /// Left icon by icon-provider name (iconContainerLeft_M3qk4j).
    pub fn with_icon(mut self, name: &str) -> Self {
        if let Self::Action { icon, .. } | Self::Submenu { icon, .. } = &mut self {
            *icon = Some(crate::icons::path(name));
        }
        self
    }

    pub fn submenu(label: impl Into<SharedString>, items: Vec<MenuItem>) -> Self {
        Self::Submenu { label: label.into(), items, icon: None, subtext: None }
    }

    /// Submenu row with a left icon (e.g. "Add to Favorites" with a star).
    pub fn submenu_with_icon(label: impl Into<SharedString>, icon: &str, items: Vec<MenuItem>) -> Self {
        Self::Submenu { label: label.into(), items, icon: Some(crate::icons::path(icon)), subtext: None }
    }
}

// Values from menu_M3qk4j (476662.*.css) and the live DM/message/guild
// context menus (dumps "user-context", "message", "guild-context"; see
// docs/parity/popouts.md):
// - .menu_ flexible_: content width, min --custom-menu-flexible-min-width
//   (188) max 320; 1px (--menu-border-width) --border-subtle; radius 8;
//   --background-surface-higher; --shadow-high; max-height viewport - 2*16.
// - .scroller_: padding-block --menu-scroller-block-padding (8), inline 8
//   (end 0 when the scrollbar shows: scrollerWithScrollbar_).
// - .labelContainer_: min-height 32, padding 8; label body-sm/medium 14/18
//   --text-strong; item color --interactive-text-default (icons);
//   .subtext_ 12/16 margin-top 2 --text-muted. Rows measure 34 (text),
//   36 (with a 20px icon/caret/checkbox), 52 (with subtext).
// - .iconContainerLeft_ 20x20, margin-inline-end 8; caret 20px --text-muted.
// - .focused_: radius 4, --interactive-background-hover, text --text-strong,
//   caret/subtext white / --text-subtle; danger: --text-feedback-critical on
//   --background-feedback-mod-critical. Plain :hover: --background-mod-subtle.
// - .separator_: 1px --border-subtle, margin 8px 0.
// - submenu (live): left = parent right - 4, top = row top - 9 (its scroller
//   padding + border so the first row lines up), min width 188.
// - checkboxIndicator_vTUVyc: 20px, radius 4, 1px --checkbox-border-default
//   on --checkbox-background-default; selected uses the *-selected tokens.
pub const MENU_W: f32 = 220.;
const MENU_MIN_W: f32 = 188.; // --custom-menu-flexible-min-width
const MENU_MAX_W: f32 = 320.; // flexible_M3qk4j max-width
const VIEWPORT_PAD: f32 = 16.; // --custom-menu-viewport-padding
const SUB_OVERLAP: f32 = 4.; // live: submenu x = parent right - 4
const CHEVRON: &str = "chevron-right";
const CHECK: &str = "check";

thread_local! {
    /// Window height seen on the last paint (menu max-height).
    static VIEW_H: std::cell::Cell<f32> = const { std::cell::Cell::new(700.) };
}

/// Height of one row, as laid out by the official client.
fn row_h(item: &MenuItem) -> f32 {
    match item {
        MenuItem::Separator => 17.,
        MenuItem::Custom(_) => 60.,
        MenuItem::Action { icon, subtext, .. } => {
            let text: f32 = 18. + if subtext.is_some() { 18. } else { 0. };
            16. + text.max(if icon.is_some() { 20. } else { 0. })
        }
        MenuItem::Submenu { subtext: Some(_), .. } | MenuItem::Check { subtext: Some(_), .. } => 52.,
        MenuItem::Submenu { .. } | MenuItem::Check { .. } => 36.,
    }
}

/// Menu panel only (no backdrop), for embedding.
pub fn panel(id: &'static str, items: Vec<MenuItem>, dismiss: Dismiss) -> gpui::Stateful<gpui::Div> {
    panel_at(id, 0, items, dismiss)
}

fn row(id: (&'static str, usize), lit: bool, danger: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex_none()
        .min_h(z(32.))
        .p(z(8.))
        .rounded(z(if lit { 4. } else { 2. }))
        .flex()
        .items_center()
        .cursor_pointer()
        .text_size(z(14.))
        .line_height(z(18.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(c(tok(if danger { idx::TEXT_FEEDBACK_CRITICAL } else { idx::TEXT_STRONG })))
        .when(lit, |d| {
            d.bg(c(tok(if danger { idx::BACKGROUND_FEEDBACK_MOD_CRITICAL } else { idx::INTERACTIVE_BACKGROUND_HOVER })))
        })
        .when(!lit, |d| d.hover(|s| s.bg(c(tok(idx::BACKGROUND_MOD_SUBTLE)))))
}

fn label_box(label: SharedString) -> gpui::Div {
    div().flex_1().min_w_0().overflow_hidden().text_ellipsis().whitespace_nowrap().child(label)
}

fn panel_at(id: &'static str, depth: u8, items: Vec<MenuItem>, dismiss: Dismiss) -> gpui::Stateful<gpui::Div> {
    let hl = if depth == 0 { HL_ROOT.with(|c| c.get()) } else { HL_SUB.with(|c| c.get()) };
    let open_sub = if depth == 0 { OPEN_SUB.with(|c| c.get()) } else { None };
    let max_h = VIEW_H.with(|v| v.get()) - VIEWPORT_PAD * 2.;
    let mut list = div()
        .id(SharedString::from(format!("{id}-scroller")))
        .flex()
        .flex_col()
        .py(z(8.))
        .px(z(8.))
        .max_h(z(max_h - 2.))
        .overflow_y_scroll();
    let mut sub_panel = None;
    let mut y = 0.;
    for (ix, item) in items.into_iter().enumerate() {
        let lit = hl == Some(ix);
        let top = y;
        y += row_h(&item);
        list = list.child(match item {
            MenuItem::Separator => div().flex_none().my(z(8.)).h(z(1.)).bg(c(tok(idx::BORDER_SUBTLE))).into_any_element(),
            MenuItem::Custom(el) => el,
            MenuItem::Submenu { label, items, icon, subtext } => {
                let open = open_sub == Some(ix);
                if open {
                    sub_panel = Some(
                        div()
                            .absolute()
                            .top(z(top))
                            .left_full()
                            .ml(z(-SUB_OVERLAP))
                            .child(panel_at("context-submenu", 1, items, dismiss.clone())),
                    );
                }
                row(("menu-sub", ix), open || lit, false)
                    .on_hover(move |h, w, _| {
                        if *h {
                            set_open_sub(Some(ix), w);
                            set_hl(0, Some(ix), w);
                            set_hl(1, None, w);
                        }
                    })
                    .when_some(icon, |d, path| {
                        let fg = if open || lit { idx::INTERACTIVE_TEXT_ACTIVE } else { idx::INTERACTIVE_TEXT_DEFAULT };
                        d.child(svg().path(path).mr(z(8.)).size(z(20.)).flex_none().text_color(c(tok(fg))))
                    })
                    .child(div().flex_1().min_w_0().flex().flex_col().child(label_box(label)).when_some(subtext, |d, s| {
                        // defaultColor_ subtext: 12/16 --text-muted, margin-top 2
                        d.child(
                            div()
                                .mt(z(2.))
                                .text_size(z(12.))
                                .line_height(z(16.))
                                .font_weight(FontWeight::NORMAL)
                                .text_color(c(tok(if open || lit { idx::TEXT_SUBTLE } else { idx::TEXT_MUTED })))
                                .child(s),
                        )
                    }))
                    // iconContainer_ margin-inline-start 8; caret_ --text-muted (white when focused)
                    .child(
                        svg()
                            .path(crate::icons::path(CHEVRON))
                            .ml(z(8.))
                            .size(z(20.))
                            .flex_none()
                            .text_color(if open || lit { gpui::white() } else { c(tok(idx::TEXT_MUTED)) }),
                    )
                    .into_any_element()
            }
            MenuItem::Check { label, checked, radio, subtext, on_click } => {
                let dismiss = dismiss.clone();
                let mark = if radio {
                    // standaloneRadioIndicator_oJuRb (guild notification submenu dump)
                    crate::ui::controls::radio_indicator(checked)
                } else {
                    let (bg, border) = if checked {
                        (idx::CHECKBOX_BACKGROUND_SELECTED_DEFAULT, idx::CHECKBOX_BORDER_SELECTED_DEFAULT)
                    } else {
                        (idx::CHECKBOX_BACKGROUND_DEFAULT, idx::CHECKBOX_BORDER_DEFAULT)
                    };
                    div()
                        .size(z(20.))
                        .flex_none()
                        .rounded(z(4.))
                        .border_1()
                        .border_color(c(tok(border)))
                        .bg(c(tok(bg)))
                        .flex()
                        .items_center()
                        .justify_center()
                        .when(checked, |d| {
                            d.child(svg().path(crate::icons::path(CHECK)).size(z(18.)).text_color(gpui::white()))
                        })
                };
                row(("menu-check", ix), lit, false)
                    .on_hover(move |h, w, _| {
                        if *h {
                            if depth == 0 {
                                set_open_sub(None, w);
                            }
                            set_hl(depth, Some(ix), w);
                        }
                    })
                    .child(div().flex_1().min_w_0().flex().flex_col().child(label_box(label)).when_some(subtext, |d, s| {
                        // vanilla-menu-channel-notify default radio: subtext_ 12/16 --text-muted, mt 2
                        d.child(
                            div()
                                .mt(z(2.))
                                .text_size(z(12.))
                                .line_height(z(16.))
                                .text_color(c(tok(if lit { idx::TEXT_SUBTLE } else { idx::TEXT_MUTED })))
                                .child(s),
                        )
                    }))
                    .child(div().ml(z(8.)).child(mark))
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
            MenuItem::Action { label, icon, subtext, danger, on_click } => {
                let dismiss = dismiss.clone();
                let icon_fg = if danger {
                    c(tok(idx::TEXT_FEEDBACK_CRITICAL))
                } else if lit {
                    c(tok(idx::INTERACTIVE_TEXT_ACTIVE))
                } else {
                    c(tok(idx::INTERACTIVE_TEXT_DEFAULT))
                };
                row(("menu-item", ix), lit, danger)
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
                    .when_some(icon, |d, name| {
                        let path: SharedString =
                            if name.ends_with(".svg") { name } else { crate::icons::path(&name) };
                        d.child(svg().path(path).mr(z(8.)).size(z(20.)).flex_none().text_color(icon_fg))
                    })
                    .child(
                        div().flex_1().min_w_0().flex().flex_col().child(label_box(label)).when_some(subtext, |d, s| {
                            d.child(
                                div()
                                    .mt(z(2.))
                                    .text_size(z(12.))
                                    .line_height(z(16.))
                                    .font_weight(FontWeight::NORMAL)
                                    .text_color(c(tok(if lit { idx::TEXT_SUBTLE } else { idx::TEXT_MUTED })))
                                    .child(s),
                            )
                        }),
                    )
                    .on_click(move |ev, w, cx| {
                        on_click(ev, w, cx);
                        dismiss(w, cx);
                    })
                    .into_any_element()
            }
        });
    }
    div()
        .id(id)
        .relative()
        .min_w(z(MENU_MIN_W))
        .max_w(z(MENU_MAX_W))
        .rounded(z(8.))
        .border_1()
        .border_color(c(tok(idx::BORDER_SUBTLE)))
        .bg(c(tok(idx::BACKGROUND_SURFACE_HIGHER)))
        // --shadow-high: 0 12px 24px 0 rgb(0 0 0 / .24)
        .shadow(vec![gpui::BoxShadow {
            color: gpui::hsla(0., 0., 0., 0.24),
            offset: gpui::point(z(0.), z(12.)),
            blur_radius: z(24.),
            spread_radius: z(0.),
            inset: false,
        }])
        .flex()
        .flex_col()
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
        .child(list)
        .children(sub_panel)
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
            VIEW_H.with(|v| v.set(f32::from(w.viewport_size().height) / f32::from(z(1.))));
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
                    .snap_to_window_with_margin(z(VIEWPORT_PAD))
                    .child(panel("context-menu", items, dismiss)),
            ),
    )
    .with_priority(10)
    .into_any_element()
}
