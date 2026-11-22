//! Converts parsed markdown into cached GPUI text runs. Built once per
//! message (or on spoiler reveal), never in the render path.

use std::ops::Range;

use gpui::{
    Font, FontStyle, FontWeight, Hsla, SharedString, StrikethroughStyle, TextRun, UnderlineStyle,
    font, px,
};

use crate::{
    markdown::{self, Block, Inline, Mention, MentionResolver, Style},
    model::Id,
    theme::{FONT_MONO, FONT_UI, Palette, c, ca},
};

#[derive(Clone, Debug)]
pub enum ClickTarget {
    Link(SharedString),
    Spoiler(usize),
    Mention(Mention),
}

#[derive(Clone)]
pub struct RichText {
    pub text: SharedString,
    pub runs: Vec<TextRun>,
    pub click_ranges: Vec<Range<usize>>,
    pub click_targets: Vec<ClickTarget>,
}

#[derive(Clone)]
pub enum RBlock {
    Text(RichText),
    Quote(RichText),
    Code { lang: Option<SharedString>, code: SharedString },
}

pub fn ui_font() -> Font {
    font(FONT_UI)
}

fn run_for(len: usize, style: Style, revealed: bool, base: Hsla) -> TextRun {
    let mut f = if style.has(Style::CODE) { font(FONT_MONO) } else { ui_font() };
    if style.has(Style::BOLD) {
        f.weight = FontWeight::BOLD;
    }
    if style.has(Style::ITALIC) {
        f.style = FontStyle::Italic;
    }
    let mut color = base;
    let mut background_color = None;
    if style.has(Style::CODE) {
        background_color = Some(c(Palette::CODE_BG));
    }
    if style.has(Style::MENTION) {
        color = c(Palette::MENTION_FG);
        background_color = Some(ca(Palette::MENTION_BG));
        f.weight = FontWeight::MEDIUM;
    }
    if style.has(Style::LINK) {
        color = c(Palette::TEXT_LINK);
    }
    let underline = style.has(Style::UNDERLINE).then(|| UnderlineStyle {
        thickness: px(1.),
        color: Some(color),
        wavy: false,
    });
    if style.has(Style::SPOILER) {
        if revealed {
            background_color = Some(ca(Palette::SPOILER_REVEALED));
        } else {
            color = c(Palette::SPOILER_BG);
            background_color = Some(c(Palette::SPOILER_BG));
        }
    }
    TextRun {
        len,
        font: f,
        color,
        background_color,
        underline,
        strikethrough: style.has(Style::STRIKE).then(|| StrikethroughStyle {
            thickness: px(1.),
            color: Some(color),
        }),
    }
}

pub fn rich_inline(inline: &Inline, revealed: &dyn Fn(usize) -> bool, base: Hsla) -> RichText {
    let mut runs = Vec::with_capacity(inline.spans.len());
    let mut click_ranges = Vec::new();
    let mut click_targets = Vec::new();
    for s in &inline.spans {
        let rev = s.spoiler.is_some_and(revealed);
        runs.push(run_for(s.range.len(), s.style, rev, base));
        if let Some(sp) = s.spoiler
            && !rev
        {
            click_ranges.push(s.range.clone());
            click_targets.push(ClickTarget::Spoiler(sp));
        } else if let Some(l) = &s.link {
            click_ranges.push(s.range.clone());
            click_targets.push(ClickTarget::Link(l.clone().into()));
        } else if let Some(m) = &s.mention {
            click_ranges.push(s.range.clone());
            click_targets.push(ClickTarget::Mention(m.clone()));
        }
    }
    RichText {
        text: inline.text.clone().into(),
        runs,
        click_ranges,
        click_targets,
    }
}

pub fn render_blocks(
    blocks: &[Block],
    revealed: &dyn Fn(usize) -> bool,
    base: Hsla,
) -> Vec<RBlock> {
    blocks
        .iter()
        .map(|b| match b {
            Block::Paragraph(i) => RBlock::Text(rich_inline(i, revealed, base)),
            Block::Quote(i) => RBlock::Quote(rich_inline(i, revealed, base)),
            Block::Code { lang, code } => RBlock::Code {
                lang: lang.clone().map(Into::into),
                code: code.clone().into(),
            },
        })
        .collect()
}

/// Everything needed to draw one message row without further work.
#[derive(Clone)]
pub struct RenderedMessage {
    pub group_start: bool,
    pub day_divider: Option<SharedString>,
    pub time_full: SharedString,
    pub time_short: SharedString,
    pub blocks: Vec<RBlock>,
    pub mentions_me: bool,
    /// One-line preview used when other messages reply to this one.
    pub preview: SharedString,
    pub parsed: Vec<Block>,
}

pub fn build(
    content: &str,
    resolver: &dyn MentionResolver,
    me: Id,
    revealed: &dyn Fn(usize) -> bool,
) -> (Vec<Block>, Vec<RBlock>, bool, SharedString) {
    let parsed = markdown::parse(content, resolver);
    let mentions_me = parsed.iter().any(|b| match b {
        Block::Paragraph(i) | Block::Quote(i) => i.spans.iter().any(|s| {
            matches!(s.mention, Some(Mention::User(u)) if u == me.0)
                || matches!(s.mention, Some(Mention::Everyone | Mention::Here))
        }),
        _ => false,
    });
    let preview: String = parsed
        .iter()
        .map(|b| match b {
            Block::Paragraph(i) | Block::Quote(i) => i.text.as_str(),
            Block::Code { code, .. } => code.as_str(),
        })
        .collect::<Vec<_>>()
        .join(" ")
        .replace('\n', " ")
        .chars()
        .take(120)
        .collect();
    let blocks = render_blocks(&parsed, revealed, c(Palette::TEXT_NORMAL));
    (parsed, blocks, mentions_me, preview.into())
}
