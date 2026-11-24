//! Unicode emoji inside plain UI labels (channel / category / thread names).
//!
//! Live client (vanilla, 2026-11-24): channel names are plain text in
//! `"gg sans","Noto Sans",...`, and Chromium draws emoji-presentation characters
//! with the system color emoji font (`fc-match emoji` = Noto Color Emoji), e.g. the
//! red "❓" in "〢❓・aide-membres". GPUI's per-glyph fallback only checks coverage,
//! so "❓" came from a monochrome symbol font and some emoji (📞) were missing.
//! Here the emoji sequences found by `richtext::emoji_ranges` get the system color
//! emoji font (`fonts::fallback::EMOJI_FAMILY`) with no UI fallback chain;
//! everything else stays inherited text.

use gpui::{AnyElement, IntoElement, ParentElement, SharedString, Styled, div};

/// `text` (drawn at `font_px`) with its Unicode emoji as images.
pub fn label(text: SharedString, _font_px: f32) -> AnyElement {
    let ranges: Vec<_> = crate::richtext::emoji_ranges(&text)
        .into_iter()
        .filter(|(r, _)| !text[r.clone()].starts_with('<')) // custom `<:name:id>` tokens stay text
        .map(|(r, _)| r)
        .collect();
    // PHX_NO_EMOJI_FONT=1: old behaviour, for before/after renders.
    if ranges.is_empty() || std::env::var_os("PHX_NO_EMOJI_FONT").is_some() {
        return text.into_any_element();
    }
    let mut row = div().flex().items_center().min_w_0().overflow_hidden();
    let mut at = 0;
    for r in ranges {
        if r.start > at {
            row = row.child(div().flex_none().child(SharedString::from(text[at..r.start].to_string())));
        }
        // Keep the UI font (gg sans has no emoji) but make the color emoji font the
        // only fallback, so "❓" no longer resolves to DejaVu Sans' monochrome glyph.
        let mut e = div().flex_none();
        e.text_style().font_fallbacks =
            Some(gpui::FontFallbacks::from_fonts(vec![crate::fonts::fallback::EMOJI_FAMILY.to_string()]));
        row = row.child(e.child(SharedString::from(text[r.clone()].to_string())));
        at = r.end;
    }
    if at < text.len() {
        row = row.child(div().min_w_0().text_ellipsis().child(SharedString::from(text[at..].to_string())));
    }
    row.into_any_element()
}

#[cfg(test)]
mod tests {
    #[test]
    fn finds_emoji_in_channel_names() {
        let t = "〢❓・aide-membres";
        let r = crate::richtext::emoji_ranges(t);
        assert_eq!(r.len(), 1);
        assert_eq!(&t[r[0].0.clone()], "❓");
        assert!(crate::richtext::emoji_ranges("〢・general").is_empty());
    }
}
