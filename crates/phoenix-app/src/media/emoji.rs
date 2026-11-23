//! Emoji images: Discord custom emoji (`<:name:id>`, `<a:name:id>`) from the
//! CDN and Unicode emoji from the Twemoji set (graphics CC-BY 4.0, Twitter
//! Inc. and contributors, maintained at github.com/jdecked/twemoji), served
//! by jsDelivr.

use gpui::{AnyElement, ParentElement, SharedString, Styled, StyledImage, div, img, prelude::*, px};

pub const TWEMOJI_BASE: &str = "https://cdn.jsdelivr.net/gh/jdecked/twemoji@15.1.0/assets/72x72";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomEmoji {
    pub name: SharedString,
    pub id: u64,
    pub animated: bool,
}

/// Sizes the media CDN serves; any other `size` (e.g. 38, a zoomed 19.25px
/// bio emoji at 2x) answers 400 and the emoji stays on its text fallback.
const CDN_SIZES: [u32; 17] = [16, 20, 22, 24, 28, 32, 40, 44, 48, 56, 60, 64, 80, 96, 100, 128, 160];

/// Smallest CDN size ≥ `want` (or the largest listed / a power of two above it).
pub fn cdn_size(want: u32) -> u32 {
    CDN_SIZES.iter().copied().find(|s| *s >= want).unwrap_or_else(|| want.next_power_of_two().min(4096))
}

impl CustomEmoji {
    pub fn url(&self, size: u32) -> SharedString {
        let ext = if self.animated { "gif" } else { "webp" };
        format!("https://cdn.discordapp.com/emojis/{}.{ext}?size={}&quality=lossless", self.id, cdn_size(size)).into()
    }
}

/// Parse a full `<:name:id>` / `<a:name:id>` token.
pub fn parse_custom(s: &str) -> Option<CustomEmoji> {
    let inner = s.strip_prefix('<')?.strip_suffix('>')?;
    let (animated, rest) = match inner.strip_prefix("a:") {
        Some(r) => (true, r),
        None => (false, inner.strip_prefix(':')?),
    };
    let (name, id) = rest.split_once(':')?;
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    Some(CustomEmoji { name: name.to_owned().into(), id: id.parse().ok()?, animated })
}

/// Find custom emoji tokens in text: (byte range, emoji).
pub fn find_custom(text: &str) -> Vec<(std::ops::Range<usize>, CustomEmoji)> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(start) = text[i..].find('<').map(|p| p + i) {
        match text[start..].find('>').map(|p| p + start + 1) {
            Some(end) => {
                if let Some(e) = parse_custom(&text[start..end]) {
                    out.push((start..end, e));
                    i = end;
                } else {
                    i = start + 1;
                }
            }
            None => break,
        }
    }
    out
}

/// Twemoji file name for a Unicode emoji ("1f44d", "1f469-200d-1f4bb").
/// FE0F is dropped unless the sequence contains a ZWJ, as Twemoji does.
pub fn twemoji_code(e: &str) -> Option<String> {
    let has_zwj = e.contains('\u{200D}');
    let parts: Vec<String> = e
        .chars()
        .filter(|&c| has_zwj || c != '\u{FE0F}')
        .map(|c| format!("{:x}", c as u32))
        .collect();
    let first = e.chars().next()? as u32;
    // Only real emoji ranges; plain text is not looked up.
    let emojiish = first >= 0x1F000
        || (0x2000..0x3300).contains(&first)
        || e.contains('\u{FE0F}')
        || e.contains('\u{20E3}');
    (emojiish && !parts.is_empty()).then(|| parts.join("-"))
}

pub fn twemoji_url(e: &str) -> Option<SharedString> {
    twemoji_code(e).map(|c| format!("{TWEMOJI_BASE}/{c}.png").into())
}

/// Emoji as an inline image of `size` px: custom emoji tokens, `:name:` with
/// an id, or a Unicode emoji. Falls back to the text when nothing matches or
/// the image fails to load.
pub fn emoji_element(token: &str, size: f32) -> AnyElement {
    let url = parse_custom(token).map(|e| e.url((size * 2.) as u32)).or_else(|| twemoji_url(token));
    let text: SharedString = match parse_custom(token) {
        Some(e) => format!(":{}:", e.name).into(),
        None => token.to_owned().into(),
    };
    match url {
        Some(u) => {
            let fb = text.clone();
            img(u)
                .size(px(size))
                .flex_none()
                .with_fallback(move || div().text_size(px(size * 0.9)).child(fb.clone()).into_any_element())
                .into_any_element()
        }
        None => div().text_size(px(size)).child(text).into_any_element(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_tokens() {
        assert_eq!(parse_custom("<:blob:123>"), Some(CustomEmoji { name: "blob".into(), id: 123, animated: false }));
        assert!(parse_custom("<a:dance:9>").unwrap().animated);
        assert_eq!(parse_custom("<@123>"), None);
        let found = find_custom("hi <:a:1> and <a:b:2> <#3>");
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].0, 3..9);
    }

    #[test]
    fn cdn_sizes() {
        assert_eq!(cdn_size(38), 40);
        assert_eq!(cdn_size(44), 44);
        assert_eq!(cdn_size(1), 16);
        assert_eq!(cdn_size(200), 256);
    }

    #[test]
    fn twemoji_codes() {
        assert_eq!(twemoji_code("👍").as_deref(), Some("1f44d"));
        assert_eq!(twemoji_code("❤️").as_deref(), Some("2764"));
        assert_eq!(twemoji_code("👩‍💻").as_deref(), Some("1f469-200d-1f4bb"));
        assert_eq!(twemoji_code("a"), None);
    }
}
