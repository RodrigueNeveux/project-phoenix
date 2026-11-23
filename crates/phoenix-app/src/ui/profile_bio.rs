//! Profile bio and badge-row helpers shared by the DM profile panel, the
//! user popout and the full profile modal.
//!
//! Bio: `bioText_qjJ3Wv` (body-sm/normal 14/18) is rendered by the client's
//! message markdown parser restricted to inline rules (bold, italic,
//! underline, strike, inline code, links, custom and Unicode emoji). We reuse
//! `crate::markdown::parse_inline` + `richtext::rich_inline`, line by line, so
//! the 3-line clamp (`descriptionClamp_BcddaK`, -webkit-line-clamp 3) stays
//! one source line per clamped line.
//!
//! Badges: module `UserProfileBadgeListRows` of the 2026 client. With the
//! "2026-08-badge-management" experiment on (vanilla dumps show the
//! `managed_IqWmhA` container, so it is on) and a non-bot target, the row
//! shows the first 6 badges then an `overflowSlot_IqWmhA` "+N"
//! (`hiddenCount_IqWmhA`, body-sm/normal, --text-default) — in every layout
//! (popout, sidebar, modal). Order is `displayProfile.getBadges()`, i.e. the
//! API `badges` array order, untouched.

use gpui::{div, prelude::*, Div, Hsla, InteractiveText, SharedString, StyledText, TextRun};

use crate::{
    markdown::{parse_inline, NoResolve},
    richtext::{rich_inline, ClickTarget, RichText},
    theme::{c, generated::idx, tok, z},
};

/// Badges shown before the "+N" slot (`S.slice(0,6)`, `max(len-6,0)`).
pub const MAX_VISIBLE_BADGES: usize = 6;

/// (visible badges, hidden count) for a badge list.
pub fn split_badges<T>(badges: &[T], bot: bool) -> (&[T], usize) {
    if bot || badges.len() <= MAX_VISIBLE_BADGES {
        return (badges, 0);
    }
    (&badges[..MAX_VISIBLE_BADGES], badges.len() - MAX_VISIBLE_BADGES)
}

/// `overflowSlot_IqWmhA` (flex, center, height 20px, min-width 20px,
/// padding 0 2px, --text-default) holding `hiddenCount_IqWmhA` "+N"
/// (body-sm/normal 14/18).
pub fn overflow_slot(n: usize) -> Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .h(z(20.))
        .min_w(z(20.))
        .px(z(2.))
        .text_size(z(14.))
        .line_height(z(18.))
        .text_color(c(tok(idx::TEXT_DEFAULT)))
        .child(format!("+{n}"))
}

fn slice_runs(runs: &[TextRun], range: std::ops::Range<usize>) -> Vec<TextRun> {
    let mut out = Vec::new();
    let mut pos = 0;
    for r in runs {
        let (s, e) = (pos, pos + r.len);
        pos = e;
        let (a, b) = (s.max(range.start), e.min(range.end));
        if a < b {
            let mut n = r.clone();
            n.len = b - a;
            out.push(n);
        }
    }
    out
}

fn line_rich(line: &str, color: Hsla, ellipsis: bool) -> RichText {
    let mut n = 0;
    let src = if ellipsis { format!("{line}…") } else { line.to_owned() };
    rich_inline(&parse_inline(&src, &NoResolve, &mut n), &|_| true, color)
}

/// Text slice as StyledText, with clickable links.
fn text_el(id: u64, t: &RichText, range: std::ops::Range<usize>) -> gpui::AnyElement {
    let styled = StyledText::new(SharedString::from(t.text[range.clone()].to_owned())).with_runs(slice_runs(&t.runs, range.clone()));
    let mut ranges = Vec::new();
    let mut urls = Vec::new();
    for (cr, tg) in t.click_ranges.iter().zip(&t.click_targets) {
        if let ClickTarget::Link(u) = tg
            && cr.start >= range.start
            && cr.end <= range.end
        {
            ranges.push(cr.start - range.start..cr.end - range.start);
            urls.push(u.clone());
        }
    }
    if ranges.is_empty() {
        return styled.into_any_element();
    }
    InteractiveText::new(("profile-bio", id), styled).on_click(ranges, move |ix, _w, cx| cx.open_url(&urls[ix])).into_any_element()
}

/// One bio line: text runs + inline emoji (`img.emoji` 1.375em = 19.25px,
/// margin 0 .05em 0 .1em). `wrap` lets it flow on several lines.
fn line_el(id: u64, t: &RichText, wrap: bool) -> Div {
    let size = 19.25;
    let mut row = div().flex().min_w_0().items_end();
    row = if wrap { row.flex_wrap() } else { row.overflow_hidden().whitespace_nowrap().h(z(if t.emoji.is_empty() { 18. } else { size })) };
    let mut last = 0;
    let mut k = 0u64;
    for (r, token) in &t.emoji {
        if last < r.start {
            row = row.child(text_el(id << 8 | k, t, last..r.start));
            k += 1;
        }
        row = row.child(div().flex_none().ml(z(1.4)).mr(z(0.7)).child(crate::media::emoji::emoji_element(token, size)));
        last = r.end;
    }
    if last < t.text.len() {
        row = row.child(text_el(id << 8 | k, t, last..t.text.len()));
    }
    row
}

/// Bio with inline markdown and emoji. `clamp` keeps at most 3 lines with
/// "…" on the third when more follow.
pub fn bio(bio: &str, clamp: bool, color: Hsla) -> Div {
    let lines: Vec<&str> = bio.lines().filter(|l| !l.trim().is_empty()).collect();
    let id = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        bio.hash(&mut h);
        h.finish() >> 16
    };
    let mut col = div().flex().flex_col().min_w_0().text_size(z(14.)).line_height(z(18.)).text_color(color);
    if lines.len() == 1 || !clamp {
        for (n, l) in lines.iter().enumerate() {
            let t = line_rich(l, color, false);
            let el = if t.emoji.is_empty() {
                div().child(text_el(id ^ n as u64, &t, 0..t.text.len()))
            } else {
                line_el(id ^ n as u64, &t, true)
            };
            col = col.child(if clamp { el.line_clamp(3).max_h(z(54.)).overflow_hidden() } else { el });
        }
        return col;
    }
    let more = lines.len() > 3;
    for (n, l) in lines.iter().take(3).enumerate() {
        let t = line_rich(l, color, more && n == 2);
        col = col.child(line_el(id ^ n as u64, &t, false));
    }
    col
}

/// Whether the clamped bio hides content ("View Full Bio").
pub fn bio_is_long(bio: &str) -> bool {
    let lines: Vec<&str> = bio.lines().filter(|l| !l.trim().is_empty()).collect();
    lines.len() > 3 || (lines.len() == 1 && bio.chars().count() > 120)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_then_overflow() {
        let b: Vec<u8> = (0..9).collect();
        assert_eq!(split_badges(&b, false), (&b[..6], 3));
        assert_eq!(split_badges(&b, true), (&b[..], 0));
        assert_eq!(split_badges(&b[..6], false).1, 0);
    }

    #[test]
    fn bio_markdown_runs() {
        let t = line_rich("**bold** *it* __u__ ~~s~~ `c` 😀", gpui::black(), false);
        assert_eq!(t.text.as_ref(), "bold it u s c 😀");
        assert_eq!(t.emoji.len(), 1);
        assert!(t.runs.iter().any(|r| r.font.weight == gpui::FontWeight::BOLD));
        assert!(t.runs.iter().any(|r| r.underline.is_some()));
        assert!(t.runs.iter().any(|r| r.strikethrough.is_some()));
    }
}
