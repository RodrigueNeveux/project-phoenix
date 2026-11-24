//! Voice channel member row (channel list) interactions: profile popout anchor and
//! the row bounds it needs.
//!
//! Live client (vanilla, 2026-11-24): clicking `voiceUser_YUoOxm` opens the standard
//! `user-profile-popout` (300px) with its left edge 8px right of the row and its top
//! aligned with the row; while open the row carries `selected_YUoOxm`. Clicking the
//! row never joins the channel.

use std::cell::RefCell;
use std::collections::HashMap;

use gpui::{Bounds, Pixels, Point, point, px};

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
