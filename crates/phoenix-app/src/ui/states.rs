//! Interactive states shared by Discord's chat-input buttons (button_hfM7yP):
//! rest `--interactive-text-default`; hover `--interactive-text-hover` on a
//! `--interactive-background-selected` 8px tile; pressed `--interactive-text-active`.
//! Audit: docs/parity/states.md.

use crate::theme::{c, generated::idx, tok, z};
use crate::ui::Icon;
use gpui::{Div, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled, div, svg};

/// 32px composer button (attach, gift, GIF, sticker, emoji, apps), 20px glyph
/// (`--chat-input-icon-size`). The glyph follows the button's hover/active state.
pub fn composer_button(id: &'static str, i: Icon) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .group(id)
        .size(z(32.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(z(8.)) // button_hfM7yP:not(.noHover) border-radius 8px
        .cursor_pointer()
        // button_hfM7yP:not(.noHover):hover background-color
        .hover(|s| s.bg(c(tok(idx::INTERACTIVE_BACKGROUND_SELECTED))))
        .child(
            svg()
                .path(i.path())
                .size(z(20.))
                .flex_none()
                .text_color(c(tok(idx::INTERACTIVE_TEXT_DEFAULT))) // button_hfM7yP color
                .group_hover(id, |s| s.text_color(c(tok(idx::INTERACTIVE_TEXT_HOVER)))) // :hover color
                .id("glyph")
                .group_active(id, |s| s.text_color(c(tok(idx::INTERACTIVE_TEXT_ACTIVE)))), // :active color
        )
}
