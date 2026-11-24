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
    let hit = ix.and_then(|i| {
        ranges
            .iter()
            .position(|r| r.contains(&i))
            .filter(|&k| matches!(targets[k], ClickTarget::Mention(_)))
    });
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
    // Role mentions carry an inline role color which wins over `.interactive:hover`.
    if matches!(targets.get(k), Some(ClickTarget::Mention(Mention::Role(_)))) {
        return None;
    }
    let mut out = runs.to_vec();
    let mut at = 0;
    for run in &mut out {
        let span = at..at + run.len;
        if span.start < r.end && r.start < span.end {
            run.background_color = Some(c(tok(idx::BRAND_500))); // .interactive:hover --brand-500
            run.color = c(tok(idx::WHITE)); // .interactive:hover --white
        }
        at = span.end;
    }
    Some(out)
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
}
