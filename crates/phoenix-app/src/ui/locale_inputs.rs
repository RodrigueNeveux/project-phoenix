//! Input placeholders that follow the UI language.
//!
//! The locale comes from the synced settings, which load after READY. Inputs created by an
//! earlier render kept the English placeholder ("Search", "Search or create a post...") in
//! the French UI. [`keep_placeholder`] re-applies the translated text whenever the active
//! locale differs from the one the input last got.

use std::{cell::RefCell, collections::HashMap};

use gpui::{App, Entity, EntityId, Window};
use gpui_component::input::InputState;

thread_local! {
    static APPLIED: RefCell<HashMap<EntityId, &'static str>> = RefCell::default();
}

/// Sets `input`'s placeholder to `t!(key)` if the locale changed since the last call.
pub fn keep_placeholder(input: &Entity<InputState>, key: &str, window: &mut Window, cx: &mut App) {
    let locale = crate::i18n::locale();
    let stale = APPLIED.with_borrow_mut(|m| m.insert(input.entity_id(), locale) != Some(locale));
    if stale {
        let text = crate::i18n::t(key, &[]);
        input.update(cx, |s, cx| s.set_placeholder(text, window, cx));
    }
}
