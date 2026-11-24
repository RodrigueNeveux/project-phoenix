//! Mention pills in message text: hover state and click actions.
//!
//! Static style (`.mention` / `wrapper_NDaLCQ`: --mention-background, --mention-foreground,
//! radius 3px, padding 0 2px, weight medium) is baked into the runs by `richtext`. Here:
//! `.interactive:hover` (background --brand-500, color --white, 50ms ease-out) and the click
//! (user mention opens the profile popout, channel mention opens the channel).

use std::cell::Cell;
use std::ops::Range;

use gpui::{App, Entity, TextRun, Window};

use crate::{
    app::PhoenixApp,
    markdown::Mention,
    model::Id,
    richtext::ClickTarget,
    theme::{c, generated::idx, tok},
};

thread_local! {
    /// (message, block, element seed, hovered click-range index).
    static HOVER: Cell<Option<(u64, usize, u64, usize)>> = const { Cell::new(None) };
    /// Link of the message text under the cursor: (message, block, seed, url), for the
    /// link rows of the message menu.
    static HOVER_LINK: std::cell::RefCell<Option<(u64, usize, u64, gpui::SharedString)>> = const { std::cell::RefCell::new(None) };
}

/// The text link under the cursor in this block, if any.
pub fn hovered_link(msg: Id, bi: usize, seed: u64) -> Option<gpui::SharedString> {
    HOVER_LINK.with(|h| h.borrow().as_ref().filter(|(m, b, s, _)| *m == msg.0 && *b == bi && *s == seed).map(|(_, _, _, u)| u.clone()))
}

/// Hover listener body for an `InteractiveText`: tracks which mention range is under the mouse.
pub fn on_hover(
    msg: Id,
    bi: usize,
    seed: u64,
    ranges: &[Range<usize>],
    targets: &[ClickTarget],
    ix: Option<usize>,
    window: &mut Window,
) {
    let at = ix.and_then(|i| ranges.iter().position(|r| r.contains(&i)));
    HOVER_LINK.with(|h| {
        let mut h = h.borrow_mut();
        match at.map(|k| &targets[k]) {
            Some(ClickTarget::Link(url)) => *h = Some((msg.0, bi, seed, url.clone())),
            _ if h.as_ref().is_some_and(|(m, b, s, _)| *m == msg.0 && *b == bi && *s == seed) => *h = None,
            _ => {}
        }
    });
    let hit = at.filter(|&k| matches!(targets[k], ClickTarget::Mention(_)));
    let new = hit.map(|k| (msg.0, bi, seed, k));
    let cur = HOVER.with(Cell::get);
    let mine = cur.is_some_and(|(m, b, s, _)| m == msg.0 && b == bi && s == seed);
    if new != cur && (new.is_some() || mine) {
        HOVER.with(|h| h.set(new));
        window.refresh();
    }
}

/// Runs with the hovered mention (if it is in this text) painted in its hover state.
/// `base` is the byte offset of `runs` within the ranges' coordinate space.
pub fn hovered_runs(
    msg: Id,
    bi: usize,
    seed: u64,
    ranges: &[Range<usize>],
    targets: &[ClickTarget],
    runs: &[TextRun],
) -> Option<Vec<TextRun>> {
    let (m, b, s, k) = HOVER.with(Cell::get)?;
    if m != msg.0 || b != bi || s != seed {
        return None;
    }
    let r = ranges.get(k)?.clone();
    let role = matches!(targets.get(k), Some(ClickTarget::Mention(Mention::Role(_))));
    let mut out = runs.to_vec();
    let mut at = 0;
    for run in &mut out {
        let span = at..at + run.len;
        if span.start < r.end && r.start < span.end {
            match run.background_color {
                // A coloured role mention (inline role colour, background at 10 %): the
                // live client keeps the text colour and raises the background to 30 %
                // (inbox Mentions tab, 2026-11-24).
                Some(bg) if role && is_role_tint(bg, run.color) => run.background_color = Some(gpui::Hsla { a: 0.3, ..bg }),
                // User, channel and uncoloured role mentions: `.interactive:hover`.
                _ => {
                    run.background_color = Some(c(tok(idx::BRAND_500))); // --brand-500
                    run.color = c(tok(idx::WHITE)); // --white
                }
            }
        }
        at = span.end;
    }
    Some(out)
}

/// `bg` is the role colour `fg` at 10 % (set by `richtext::color_role_mentions`).
fn is_role_tint(bg: gpui::Hsla, fg: gpui::Hsla) -> bool {
    (bg.a - 0.1).abs() < 0.01 && (bg.h - fg.h).abs() < 1e-3 && (bg.s - fg.s).abs() < 1e-3 && (bg.l - fg.l).abs() < 1e-3
}

/// Click on a mention pill.
pub fn click(entity: &Entity<PhoenixApp>, m: &Mention, window: &mut Window, cx: &mut App) {
    match *m {
        Mention::User(u) => {
            let at = window.mouse_position();
            entity.update(cx, |a, cx| {
                let g = a.selected_guild;
                a.open_profile(Id(u), g, at, window, cx);
            });
        }
        Mention::Channel(ch) => entity.update(cx, |a, cx| {
            // Text chat only: a mention never joins voice.
            if a.channels.contains_key(&Id(ch)) {
                a.open_channel_chat(Id(ch), cx);
            }
        }),
        Mention::Role(_) | Mention::Everyone | Mention::Here => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(len: usize) -> TextRun {
        TextRun {
            len,
            font: gpui::font("x"),
            color: gpui::black(),
            background_color: None,
            underline: None,
            strikethrough: None,
        }
    }

    #[test]
    fn hover_paints_only_the_mention_run() {
        let ranges = std::slice::from_ref(&(3..8)).to_vec();
        let targets = vec![ClickTarget::Mention(Mention::User(1))];
        HOVER.with(|h| h.set(Some((7, 0, 0, 0))));
        let out = hovered_runs(Id(7), 0, 0, &ranges, &targets, &[run(3), run(5), run(2)]).unwrap();
        assert!(out[0].background_color.is_none());
        assert!(out[1].background_color.is_some());
        assert!(out[2].background_color.is_none());
        assert!(hovered_runs(Id(8), 0, 0, &ranges, &targets, &[run(3)]).is_none());
        HOVER.with(|h| h.set(None));
    }

    #[test]
    fn coloured_role_hover_raises_the_tint() {
        let ranges = vec![0..4, 4..8];
        let targets = vec![ClickTarget::Mention(Mention::Role(1)), ClickTarget::Mention(Mention::Role(2))];
        let orange: gpui::Hsla = gpui::rgb(0xe67e22).into();
        let mut tinted = run(4);
        tinted.color = orange;
        tinted.background_color = Some(gpui::Hsla { a: 0.1, ..orange });
        let runs = [tinted, run(4)];
        HOVER.with(|h| h.set(Some((7, 0, 0, 0))));
        let out = hovered_runs(Id(7), 0, 0, &ranges, &targets, &runs).unwrap();
        assert_eq!(out[0].color, orange);
        assert!((out[0].background_color.unwrap().a - 0.3).abs() < 1e-6);
        HOVER.with(|h| h.set(Some((7, 0, 0, 1))));
        let out = hovered_runs(Id(7), 0, 0, &ranges, &targets, &runs).unwrap();
        // Uncoloured role: same pill hover as a user mention.
        assert_eq!(out[1].color, c(tok(idx::WHITE)));
        HOVER.with(|h| h.set(None));
    }
}
