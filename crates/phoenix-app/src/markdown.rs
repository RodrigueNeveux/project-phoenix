//! Discord-flavoured markdown subset, parsed once per message (never per
//! frame). Output is a list of blocks whose inline text carries
//! non-overlapping styled spans, ready to be turned into GPUI text runs.

use std::ops::Range;

/// Bit flags for inline styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub struct Style(pub u16);

impl Style {
    pub const PLAIN: Style = Style(0);
    pub const BOLD: Style = Style(1);
    pub const ITALIC: Style = Style(1 << 1);
    pub const UNDERLINE: Style = Style(1 << 2);
    pub const STRIKE: Style = Style(1 << 3);
    pub const CODE: Style = Style(1 << 4);
    pub const SPOILER: Style = Style(1 << 5);
    pub const LINK: Style = Style(1 << 6);
    pub const MENTION: Style = Style(1 << 7);

    pub fn has(self, other: Style) -> bool {
        self.0 & other.0 == other.0 && other.0 != 0
    }
}

impl std::ops::BitOr for Style {
    type Output = Style;
    fn bitor(self, rhs: Style) -> Style {
        Style(self.0 | rhs.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mention {
    User(u64),
    Channel(u64),
    Role(u64),
    Everyone,
    Here,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub range: Range<usize>,
    pub style: Style,
    pub link: Option<String>,
    pub mention: Option<Mention>,
    /// Spoiler group index (for click-to-reveal), if inside a spoiler.
    pub spoiler: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Inline {
    pub text: String,
    pub spans: Vec<Span>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Paragraph(Inline),
    Quote(Inline),
    Code { lang: Option<String>, code: String },
}

/// Resolves `<@id>` / `<#id>` to display text (without the sigil).
pub trait MentionResolver {
    fn user(&self, id: u64) -> Option<String>;
    fn channel(&self, id: u64) -> Option<String>;
    fn role(&self, _id: u64) -> Option<String> {
        None
    }
}

pub struct NoResolve;
impl MentionResolver for NoResolve {
    fn user(&self, _: u64) -> Option<String> {
        None
    }
    fn channel(&self, _: u64) -> Option<String> {
        None
    }
}

pub fn parse(src: &str, r: &dyn MentionResolver) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut rest = src;
    let mut para = String::new();
    let mut quote = String::new();
    let mut spoilers = 0usize;

    let flush = |buf: &mut String, quote: bool, blocks: &mut Vec<Block>, sp: &mut usize| {
        let t = buf.trim_end_matches('\n');
        if !t.is_empty() {
            let inline = parse_inline(t, r, sp);
            blocks.push(if quote { Block::Quote(inline) } else { Block::Paragraph(inline) });
        }
        buf.clear();
    };

    while !rest.is_empty() {
        // Fenced code block.
        if let Some(after) = rest.strip_prefix("```")
            && let Some(end) = after.find("```")
        {
            flush(&mut para, false, &mut blocks, &mut spoilers);
            flush(&mut quote, true, &mut blocks, &mut spoilers);
            let body = &after[..end];
            let (lang, code) = match body.find('\n') {
                Some(nl)
                    if !body[..nl].trim().is_empty()
                        && !body[..nl].trim().contains(char::is_whitespace) =>
                {
                    (Some(body[..nl].trim().to_string()), &body[nl + 1..])
                }
                Some(0) => (None, &body[1..]),
                _ => (None, body),
            };
            blocks.push(Block::Code {
                lang,
                code: code.trim_end_matches('\n').to_string(),
            });
            rest = &after[end + 3..];
            rest = rest.strip_prefix('\n').unwrap_or(rest);
            continue;
        }
        let (line, next) = match rest.find('\n') {
            Some(i) => (&rest[..i], &rest[i + 1..]),
            None => (rest, ""),
        };
        // A code fence may start mid-line; only handle line-level constructs here.
        if line.contains("```") && rest.find("```").is_some_and(|i| i < line.len()) {
            let i = rest.find("```").unwrap_or(0);
            if i > 0 {
                para.push_str(&rest[..i]);
                rest = &rest[i..];
                continue;
            }
        }
        if let Some(q) = line.strip_prefix("> ").or_else(|| (line == ">").then_some("")) {
            flush(&mut para, false, &mut blocks, &mut spoilers);
            quote.push_str(q);
            quote.push('\n');
        } else {
            flush(&mut quote, true, &mut blocks, &mut spoilers);
            para.push_str(line);
            para.push('\n');
        }
        rest = next;
    }
    flush(&mut para, false, &mut blocks, &mut spoilers);
    flush(&mut quote, true, &mut blocks, &mut spoilers);
    blocks
}

/// Parse inline markdown into flattened spans.
pub fn parse_inline(src: &str, r: &dyn MentionResolver, spoiler_counter: &mut usize) -> Inline {
    let mut out = Inline::default();
    let mut ctx = Ctx {
        out: &mut out,
        r,
        spoilers: spoiler_counter,
    };
    ctx.run(src, Style::PLAIN, None);
    out
}

struct Ctx<'a> {
    out: &'a mut Inline,
    r: &'a dyn MentionResolver,
    spoilers: &'a mut usize,
}

const DELIMS: &[(&str, Style)] = &[
    ("**", Style::BOLD),
    ("__", Style::UNDERLINE),
    ("~~", Style::STRIKE),
    ("||", Style::SPOILER),
    ("*", Style::ITALIC),
    ("_", Style::ITALIC),
];

impl Ctx<'_> {
    fn push(&mut self, text: &str, style: Style, spoiler: Option<usize>) {
        self.push_full(text, style, None, None, spoiler)
    }

    fn push_full(
        &mut self,
        text: &str,
        style: Style,
        link: Option<String>,
        mention: Option<Mention>,
        spoiler: Option<usize>,
    ) {
        if text.is_empty() {
            return;
        }
        let start = self.out.text.len();
        self.out.text.push_str(text);
        let end = self.out.text.len();
        if let Some(last) = self.out.spans.last_mut()
            && last.style == style
            && last.link.is_none()
            && link.is_none()
            && last.mention.is_none()
            && mention.is_none()
            && last.spoiler == spoiler
            && last.range.end == start
        {
            last.range.end = end;
            return;
        }
        self.out.spans.push(Span {
            range: start..end,
            style,
            link,
            mention,
            spoiler,
        });
    }

    fn run(&mut self, src: &str, style: Style, spoiler: Option<usize>) {
        let bytes = src.as_bytes();
        let mut i = 0;
        let mut plain_start = 0;
        while i < bytes.len() {
            let rest = &src[i..];
            let c = bytes[i];

            // Escapes.
            if c == b'\\' && i + 1 < bytes.len() && !bytes[i + 1].is_ascii_alphanumeric() {
                self.push(&src[plain_start..i], style, spoiler);
                let ch_len = src[i + 1..].chars().next().map_or(1, char::len_utf8);
                self.push(&src[i + 1..i + 1 + ch_len], style, spoiler);
                i += 1 + ch_len;
                plain_start = i;
                continue;
            }

            // Inline code: no nested formatting.
            if c == b'`' {
                let ticks = if rest.starts_with("``") { 2 } else { 1 };
                let close = &rest[..ticks];
                if let Some(end) = rest[ticks..].find(close)
                    && end > 0
                {
                    self.push(&src[plain_start..i], style, spoiler);
                    let inner = &rest[ticks..ticks + end];
                    self.push(inner, style | Style::CODE, spoiler);
                    i += ticks * 2 + end;
                    plain_start = i;
                    continue;
                }
            }

            // Mentions <@123> <@!123> <#123> <@&123>.
            if c == b'<'
                && let Some(end) = rest.find('>')
            {
                let inner = &rest[1..end];
                let parsed = if let Some(id) = inner.strip_prefix("@&") {
                    id.parse().ok().map(|id| {
                        let n = self.r.role(id).unwrap_or_else(|| "unknown-role".into());
                        (Mention::Role(id), format!("@{n}"))
                    })
                } else if let Some(id) = inner.strip_prefix("@!").or_else(|| inner.strip_prefix('@')) {
                    id.parse().ok().map(|id| {
                        let n = self.r.user(id).unwrap_or_else(|| "Unknown User".into());
                        (Mention::User(id), format!("@{n}"))
                    })
                } else if let Some(id) = inner.strip_prefix('#') {
                    id.parse().ok().map(|id| {
                        let n = self.r.channel(id).unwrap_or_else(|| "unknown".into());
                        (Mention::Channel(id), format!("#{n}"))
                    })
                } else {
                    None
                };
                if let Some((m, label)) = parsed {
                    self.push(&src[plain_start..i], style, spoiler);
                    self.push_full(&label, style | Style::MENTION, None, Some(m), spoiler);
                    i += end + 1;
                    plain_start = i;
                    continue;
                }
            }
            if c == b'@' {
                let (tok, m) = if rest.starts_with("@everyone") {
                    ("@everyone", Some(Mention::Everyone))
                } else if rest.starts_with("@here") {
                    ("@here", Some(Mention::Here))
                } else {
                    ("", None)
                };
                if let Some(m) = m {
                    self.push(&src[plain_start..i], style, spoiler);
                    self.push_full(tok, style | Style::MENTION, None, Some(m), spoiler);
                    i += tok.len();
                    plain_start = i;
                    continue;
                }
            }

            // Masked link [text](https://...)
            if c == b'['
                && let Some(close) = rest.find("](")
                && let Some(paren) = rest[close + 2..].find(')')
            {
                let url = &rest[close + 2..close + 2 + paren];
                if url.starts_with("http://") || url.starts_with("https://") {
                    self.push(&src[plain_start..i], style, spoiler);
                    let label = &rest[1..close];
                    self.push_full(label, style | Style::LINK, Some(url.to_string()), None, spoiler);
                    i += close + 3 + paren;
                    plain_start = i;
                    continue;
                }
            }

            // Bare links.
            if (c == b'h') && (rest.starts_with("https://") || rest.starts_with("http://")) {
                let at_boundary = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
                if at_boundary {
                    let end = rest
                        .find(|ch: char| ch.is_whitespace() || ch == '<' || ch == '>')
                        .unwrap_or(rest.len());
                    let mut url = &rest[..end];
                    while url.ends_with(['.', ',', ')', '!', '?', ':', ';']) {
                        url = &url[..url.len() - 1];
                    }
                    self.push(&src[plain_start..i], style, spoiler);
                    self.push_full(url, style | Style::LINK, Some(url.to_string()), None, spoiler);
                    i += url.len();
                    plain_start = i;
                    continue;
                }
            }

            // Paired delimiters.
            let mut matched = false;
            for &(d, s) in DELIMS {
                if !rest.starts_with(d) {
                    continue;
                }
                // `_` italics must sit on word boundaries (snake_case stays plain).
                if d == "_" && i > 0 && bytes[i - 1].is_ascii_alphanumeric() {
                    continue;
                }
                let body = &rest[d.len()..];
                if body.starts_with(char::is_whitespace) && d.len() == 1 {
                    continue;
                }
                let Some(end) = find_close(body, d) else { continue };
                if end == 0 {
                    continue;
                }
                if d == "_" {
                    let after = body.as_bytes().get(end + 1);
                    if after.is_some_and(|b| b.is_ascii_alphanumeric()) {
                        continue;
                    }
                }
                self.push(&src[plain_start..i], style, spoiler);
                let sp = if s == Style::SPOILER {
                    let n = *self.spoilers;
                    *self.spoilers += 1;
                    Some(n)
                } else {
                    spoiler
                };
                self.run(&body[..end], style | s, sp);
                i += d.len() * 2 + end;
                plain_start = i;
                matched = true;
                break;
            }
            if matched {
                continue;
            }
            i += src[i..].chars().next().map_or(1, char::len_utf8);
        }
        self.push(&src[plain_start..], style, spoiler);
    }
}

/// Find a closing delimiter, skipping over inline code spans.
fn find_close(body: &str, d: &str) -> Option<usize> {
    let b = body.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'`'
            && let Some(e) = body[i + 1..].find('`')
        {
            i += e + 2;
            continue;
        }
        if body[i..].starts_with(d) {
            // For single `*`, don't treat the first half of `**` as a close.
            if d == "*" && body[i..].starts_with("**") {
                i += 2;
                continue;
            }
            return Some(i);
        }
        i += body[i..].chars().next().map_or(1, char::len_utf8);
    }
    None
}

/// Ranges in `inline.text` that are clickable (links + spoilers), in order.
pub fn clickable(inline: &Inline) -> impl Iterator<Item = &Span> {
    inline
        .spans
        .iter()
        .filter(|s| s.link.is_some() || s.spoiler.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct R;
    impl MentionResolver for R {
        fn user(&self, id: u64) -> Option<String> {
            (id == 42).then(|| "wumpus".into())
        }
        fn channel(&self, id: u64) -> Option<String> {
            (id == 7).then(|| "general".into())
        }
    }

    fn inline(s: &str) -> Inline {
        let mut n = 0;
        parse_inline(s, &R, &mut n)
    }

    fn styled(i: &Inline) -> Vec<(&str, Style)> {
        i.spans.iter().map(|s| (&i.text[s.range.clone()], s.style)).collect()
    }

    #[test]
    fn plain_text() {
        let i = inline("hello world");
        assert_eq!(styled(&i), vec![("hello world", Style::PLAIN)]);
    }

    #[test]
    fn bold_italic_nested() {
        let i = inline("a **b *c* d** e");
        assert_eq!(i.text, "a b c d e");
        assert_eq!(
            styled(&i),
            vec![
                ("a ", Style::PLAIN),
                ("b ", Style::BOLD),
                ("c", Style::BOLD | Style::ITALIC),
                (" d", Style::BOLD),
                (" e", Style::PLAIN),
            ]
        );
    }

    #[test]
    fn underline_strike_spoiler() {
        let i = inline("__u__ ~~s~~ ||x||");
        assert_eq!(i.text, "u s x");
        assert_eq!(i.spans[0].style, Style::UNDERLINE);
        assert_eq!(i.spans[2].style, Style::STRIKE);
        assert_eq!(i.spans[4].style, Style::SPOILER);
        assert_eq!(i.spans[4].spoiler, Some(0));
    }

    #[test]
    fn inline_code_is_literal() {
        let i = inline("run `**not bold**` now");
        assert_eq!(i.text, "run **not bold** now");
        assert_eq!(styled(&i)[1], ("**not bold**", Style::CODE));
    }

    #[test]
    fn snake_case_not_italic() {
        let i = inline("my_var_name and _it_");
        assert_eq!(i.text, "my_var_name and it");
        assert_eq!(styled(&i).last().unwrap(), &("it", Style::ITALIC));
    }

    #[test]
    fn unmatched_delimiters_stay() {
        let i = inline("2 * 3 = 6 and **open");
        assert_eq!(i.text, "2 * 3 = 6 and **open");
        assert_eq!(i.spans.len(), 1);
    }

    #[test]
    fn links_bare_and_masked() {
        let i = inline("see https://example.com/a, or [docs](https://docs.rs).");
        assert_eq!(i.text, "see https://example.com/a, or docs.");
        let links: Vec<_> = i.spans.iter().filter_map(|s| s.link.as_deref()).collect();
        assert_eq!(links, vec!["https://example.com/a", "https://docs.rs"]);
        assert_eq!(clickable(&i).count(), 2);
    }

    #[test]
    fn mentions() {
        let i = inline("hi <@42> in <#7>, <@!99> @everyone");
        assert_eq!(i.text, "hi @wumpus in #general, @Unknown User @everyone");
        let m: Vec<_> = i.spans.iter().filter_map(|s| s.mention.clone()).collect();
        assert_eq!(
            m,
            vec![Mention::User(42), Mention::Channel(7), Mention::User(99), Mention::Everyone]
        );
    }

    #[test]
    fn escapes() {
        let i = inline(r"\*not italic\*");
        assert_eq!(i.text, "*not italic*");
        assert_eq!(i.spans.len(), 1);
    }

    #[test]
    fn code_block_with_lang() {
        let b = parse("before\n```rust\nfn main() {}\n```\nafter", &R);
        assert_eq!(b.len(), 3);
        assert!(matches!(&b[0], Block::Paragraph(i) if i.text == "before"));
        assert_eq!(
            b[1],
            Block::Code { lang: Some("rust".into()), code: "fn main() {}".into() }
        );
        assert!(matches!(&b[2], Block::Paragraph(i) if i.text == "after"));
    }

    #[test]
    fn code_block_inline_fence() {
        let b = parse("```just code```", &R);
        assert_eq!(b, vec![Block::Code { lang: None, code: "just code".into() }]);
    }

    #[test]
    fn quotes() {
        let b = parse("> quoted **line**\n> two\nnormal", &R);
        assert_eq!(b.len(), 2);
        match &b[0] {
            Block::Quote(i) => {
                assert_eq!(i.text, "quoted line\ntwo");
                assert!(i.spans.iter().any(|s| s.style == Style::BOLD));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn multiple_spoilers_are_numbered() {
        let b = parse("||a|| and ||b||", &NoResolve);
        let Block::Paragraph(i) = &b[0] else { panic!() };
        let ids: Vec<_> = i.spans.iter().filter_map(|s| s.spoiler).collect();
        assert_eq!(ids, vec![0, 1]);
    }

    #[test]
    fn unicode_safe() {
        let i = inline("héllo **wörld** 🎉 _ok_");
        assert_eq!(i.text, "héllo wörld 🎉 ok");
    }
}
