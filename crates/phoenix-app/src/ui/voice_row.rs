//! Voice channel member row (channel list) interactions: profile popout anchor and
//! the row bounds it needs.
//!
//! Live client (vanilla, 2026-11-24): clicking `voiceUser_YUoOxm` opens the standard
//! `user-profile-popout` (300px) with its left edge 8px right of the row and its top
//! aligned with the row; while open the row carries `selected_YUoOxm`. Clicking the
//! row never joins the channel.

use std::cell::RefCell;
use std::collections::HashMap;

use gpui::{AnyElement, Bounds, InteractiveElement, IntoElement, ParentElement, Pixels, Point, StatefulInteractiveElement, Styled, div, point, px, svg};

use crate::theme::{c, generated::idx, tok, z};

use crate::model::Id;

thread_local! {
    static ROWS: RefCell<HashMap<u64, Bounds<Pixels>>> = RefCell::new(HashMap::new());
}

/// Remember where the row for `user` was painted.
pub fn record(user: Id, b: Bounds<Pixels>) {
    ROWS.with(|r| r.borrow_mut().insert(user.0, b));
}

/// `open_profile` anchor that lands the popout at (row right + 8, row top).
/// profile.rs places the card at (at.x - 300 - 16, at.y - 60).
pub fn popout_anchor(user: Id, fallback: Point<Pixels>) -> Point<Pixels> {
    let b = ROWS.with(|r| r.borrow().get(&user.0).copied());
    match b {
        Some(b) => point(b.right() + px(8. + 300. + 16.), b.top() + px(60.)),
        None => point(fallback.x + px(300. + 16. + 8.), fallback.y + px(60.)),
    }
}

/// optionsButton_YUoOxm: 24x24, radius 4px, hover/active --interactive-background-active.
/// Opens the user's context menu at the button (voice menu when in the call).
pub fn options_button(user: Id, guild: Option<Id>, in_call: bool) -> AnyElement {
    div()
        .id(("voice-user-options", user.0))
        .size(z(24.))
        .flex_none()
        .rounded(z(4.))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|s| s.bg(c(tok(idx::INTERACTIVE_BACKGROUND_ACTIVE))))
        .child(
            svg()
                .path(crate::icons::path("more-horizontal"))
                .size(z(16.))
                .text_color(c(tok(idx::CHANNELS_DEFAULT))), // .icon_YUoOxm --channels-default
        )
        .on_click(move |ev, window, cx| {
            cx.stop_propagation();
            if in_call {
                crate::ui::voice_panel::open_user_menu(user, ev.position(), cx);
            } else {
                crate::ui::user_menu::open(user, guild, ev.position(), window);
            }
        })
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::size;

    #[test]
    fn anchor_is_right_of_row() {
        record(Id(5), Bounds::new(point(px(118.), px(290.)), size(px(289.), px(32.))));
        let a = popout_anchor(Id(5), point(px(0.), px(0.)));
        assert_eq!(a.x - px(316.), px(415.));
        assert_eq!(a.y - px(60.), px(290.));
    }
}
