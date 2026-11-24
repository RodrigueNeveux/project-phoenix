//! Frameless window chrome (client-side decorations), like the desktop client's
//! frameless Electron window on Linux: the app top bar is the drag region
//! (`bar_3zSH5d` `-webkit-app-region: drag`, buttons `no-drag`), a double click
//! on it toggles maximize, and thin invisible strips along the edges resize the
//! window through the compositor (snapping/tiling is the compositor's job).

use std::sync::atomic::{AtomicBool, Ordering};

use gpui::{
    CursorStyle, Decorations, InteractiveElement, IntoElement, MouseButton, ParentElement, ResizeEdge, Styled, Window, div,
    prelude::*, px,
};

/// Width of the resize strips inside the window edge. Chromium's frameless Linux
/// windows use a 5px inner resize border (Electron `frame: false`).
const EDGE: f32 = 5.;
/// Corner hit squares (diagonal cursors) are larger so corners are easy to grab.
const CORNER: f32 = 16.;

/// Set on a primary press in the drag region; the move starts on the first drag
/// motion so a double click still reaches the click handler (toggle maximize).
static PRESSED: AtomicBool = AtomicBool::new(false);

/// Makes `el` a window drag region: press-and-drag moves the window, double
/// click toggles maximize. Interactive children must `.occlude()`.
pub fn drag_region<E: InteractiveElement>(el: E) -> E {
    el.on_mouse_down(MouseButton::Left, |ev, w, _| {
        if ev.click_count >= 2 {
            PRESSED.store(false, Ordering::Relaxed);
            w.zoom_window();
        } else {
            PRESSED.store(true, Ordering::Relaxed);
        }
    })
    .on_mouse_up(MouseButton::Left, |_, _, _| PRESSED.store(false, Ordering::Relaxed))
    .on_mouse_move(|ev, w, _| {
        if ev.pressed_button == Some(MouseButton::Left) && PRESSED.swap(false, Ordering::Relaxed) {
            w.start_window_move();
        }
    })
}

fn strip(id: &'static str, edge: ResizeEdge, cursor: CursorStyle) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .absolute()
        .occlude()
        .cursor(cursor)
        .on_mouse_down(MouseButton::Left, move |_, w, cx| {
            cx.stop_propagation();
            w.start_window_resize(edge);
        })
}

/// Invisible resize strips and corners, drawn above everything. Only with
/// client-side decorations, and not on tiled/maximized/fullscreen edges.
pub fn resize_handles(window: &Window) -> Option<impl IntoElement> {
    let tiling = match window.window_decorations() {
        Decorations::Client { tiling } => tiling,
        Decorations::Server => return None,
    };
    if window.is_maximized() || window.is_fullscreen() {
        return None;
    }
    let (t, b, l, r) = (!tiling.top, !tiling.bottom, !tiling.left, !tiling.right);
    let e = px(EDGE);
    let k = px(CORNER);
    Some(
        div()
            .absolute()
            .inset_0()
            .when(t, |d| d.child(strip("rs-top", ResizeEdge::Top, CursorStyle::ResizeUpDown).top_0().left(k).right(k).h(e)))
            .when(b, |d| d.child(strip("rs-bottom", ResizeEdge::Bottom, CursorStyle::ResizeUpDown).bottom_0().left(k).right(k).h(e)))
            .when(l, |d| d.child(strip("rs-left", ResizeEdge::Left, CursorStyle::ResizeLeftRight).left_0().top(k).bottom(k).w(e)))
            .when(r, |d| d.child(strip("rs-right", ResizeEdge::Right, CursorStyle::ResizeLeftRight).right_0().top(k).bottom(k).w(e)))
            // Corners: an L of EDGE-thick bars CORNER long, so the window buttons in the
            // top-right corner stay clickable except for their outer 5px.
            .when(t && l, |d| {
                d.child(strip("rs-tl-h", ResizeEdge::TopLeft, CursorStyle::ResizeUpLeftDownRight).top_0().left_0().w(k).h(e))
                    .child(strip("rs-tl-v", ResizeEdge::TopLeft, CursorStyle::ResizeUpLeftDownRight).top_0().left_0().w(e).h(k))
            })
            .when(t && r, |d| {
                d.child(strip("rs-tr-h", ResizeEdge::TopRight, CursorStyle::ResizeUpRightDownLeft).top_0().right_0().w(k).h(e))
                    .child(strip("rs-tr-v", ResizeEdge::TopRight, CursorStyle::ResizeUpRightDownLeft).top_0().right_0().w(e).h(k))
            })
            .when(b && l, |d| {
                d.child(strip("rs-bl-h", ResizeEdge::BottomLeft, CursorStyle::ResizeUpRightDownLeft).bottom_0().left_0().w(k).h(e))
                    .child(strip("rs-bl-v", ResizeEdge::BottomLeft, CursorStyle::ResizeUpRightDownLeft).bottom_0().left_0().w(e).h(k))
            })
            .when(b && r, |d| {
                d.child(strip("rs-br-h", ResizeEdge::BottomRight, CursorStyle::ResizeUpLeftDownRight).bottom_0().right_0().w(k).h(e))
                    .child(strip("rs-br-v", ResizeEdge::BottomRight, CursorStyle::ResizeUpLeftDownRight).bottom_0().right_0().w(e).h(k))
            }),
    )
}
