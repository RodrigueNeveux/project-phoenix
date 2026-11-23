//! Avatar decoration overlay for the generic avatar component.
//! `avatarDecorationContainer_XpIUHy`: absolute, inset-inline-start/top
//! --custom-avatar-avatar-decoration-border-position
//! (`(1 - --decoration-to-avatar-ratio) / 2 * 100%`), size avatar ×
//! --decoration-to-avatar-ratio (1.2), pointer-events none, drawn above the
//! avatar and its status cut-out.

use gpui::{img, px, Div, ParentElement, Styled};

use crate::model::User;

/// --decoration-to-avatar-ratio
pub const RATIO: f32 = 1.2;

/// Decoration box for an avatar of `size`: (offset, side).
pub fn geometry(size: f32) -> (f32, f32) {
    let side = size * RATIO;
    ((size - side) / 2., side)
}

/// Adds the user's decoration on top of `avatar` (a `relative` box of `size`).
pub fn decorate(avatar: Div, user: Option<&User>, size: f32) -> Div {
    match user.and_then(|u| u.decoration.clone()) {
        Some(deco) => {
            let (off, side) = geometry(size);
            avatar.child(img(deco).absolute().left(px(off)).top(px(off)).size(px(side)))
        }
        None => avatar,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn border_position_is_minus_ten_percent() {
        let (off, side) = super::geometry(32.);
        assert!((off + 3.2).abs() < 1e-4 && (side - 38.4).abs() < 1e-4);
    }
}
