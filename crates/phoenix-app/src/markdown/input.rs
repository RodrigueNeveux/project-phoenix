use std::ops::Range;

pub const BOLD: u16 = 1;
pub const ITALIC: u16 = 1 << 1;
pub const UNDERLINE: u16 = 1 << 2;
pub const STRIKE: u16 = 1 << 3;
pub const SPOILER: u16 = 1 << 4;
pub const CODE: u16 = 1 << 5;
pub const LINK: u16 = 1 << 6;
pub const SYNTAX: u16 = 1 << 7;
pub const LIGHT: u16 = 1 << 8;
pub const FENCE: u16 = 1 << 9;
pub const LANG: u16 = 1 << 10;
pub const SUBTEXT: u16 = 1 << 11;
pub const SUBTEXT_MARK: u16 = 1 << 12;

const DELIMS: &[(&str, u16)] = &[("**", BOLD), ("__", UNDERLINE), ("~~", STRIKE), ("||", SPOILER), ("*", ITALIC), ("_", ITALIC)];

struct Out {
    spans: Vec<(Range<usize>, u16)>,
}

impl Out {
    fn push(&mut self, range: Range<usize>, attrs: u16) {
        if range.is_empty() || attrs == 0 {
            return;
        }
        if let Some((last, a)) = self.spans.last_mut()
            && *a == attrs
            && last.end == range.start
        {
            last.end = range.end;
            return;
        }
        self.spans.push((range, attrs));
    }
}

pub fn spans(text: &str) -> Vec<(Range<usize>, u16)> {
    let mut out = Out { spans: Vec::new() };
    let mut i = 0;
    while i < text.len() {
        let rest = &text[i..];
        if let Some(after) = rest.strip_prefix("```")
            && let Some(end) = after.find("```")
        {
            out.push(i..i + 3, FENCE);
            let body_start = i + 3;
            let body = &after[..end];
            if let Some(nl) = body.find('\n') {
                let lang = &body[..nl];
                if !lang.is_empty() && lang.bytes().all(|b| b.is_ascii_alphanumeric() || b"_+-.#".contains(&b)) {
                    out.push(body_start..body_start + nl, LANG);
                }
            }
            let close = body_start + end;
            out.push(close..close + 3, FENCE);
            i = close + 3;
            continue;
        }
        let line_end = rest.find('\n').map_or(text.len(), |n| i + n);
        let next_fence = rest.find("```").map(|n| i + n).filter(|&f| f < line_end && text[f + 3..].contains("```"));
        let stop = next_fence.unwrap_or(line_end);
        line(text, i, stop, &mut out);
        i = if next_fence.is_some() { stop } else { line_end + 1 };
    }
    out.spans
}

fn line(text: &str, start: usize, end: usize, out: &mut Out) {
    let src = &text[start..end];
    if let Some(body) = src.strip_prefix("-# ")
        && !body.trim().is_empty()
        && (start == 0 || text.as_bytes()[start - 1] == b'\n')
    {
        out.push(start..start + 3, SUBTEXT_MARK);
        run(text, start + 3, end, SUBTEXT, out);
        return;
    }
    run(text, start, end, 0, out);
}

fn token_end(rest: &str) -> Option<usize> {
    let end = rest.find('>')?;
    let inner = &rest[1..end];
    if inner.contains('<') {
        return None;
    }
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let ok = inner.strip_prefix("@&").is_some_and(digits)
        || inner.strip_prefix("@!").or_else(|| inner.strip_prefix('@')).is_some_and(digits)
        || inner.strip_prefix('#').is_some_and(digits)
        || inner.strip_prefix("t:").is_some_and(|t| digits(t.split(':').next().unwrap_or("")))
        || inner.strip_prefix("sound:").is_some_and(|t| t.split(':').all(digits))
        || inner.strip_prefix("id:").is_some()
        || {
            let e = inner.strip_prefix("a:").or_else(|| inner.strip_prefix(':'));
            e.and_then(|e| e.split_once(':')).is_some_and(|(n, id)| !n.is_empty() && digits(id))
        };
    ok.then_some(end + 1)
}

fn marker(outer: u16, own: u16) -> u16 {
    match own {
        UNDERLINE => SYNTAX | LIGHT | UNDERLINE,
        STRIKE | SPOILER => SYNTAX | LIGHT | (outer & UNDERLINE),
        CODE => SYNTAX | CODE | (outer & UNDERLINE),
        _ => SYNTAX,
    }
}

fn run(text: &str, start: usize, end: usize, attrs: u16, out: &mut Out) {
    let src = &text[start..end];
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut plain = 0;
    while i < bytes.len() {
        let rest = &src[i..];
        let c = bytes[i];
        let at = start + i;

        if c == b'\\' && i + 1 < bytes.len() && !bytes[i + 1].is_ascii_alphanumeric() {
            out.push(start + plain..at, attrs);
            out.push(at..at + 1, SYNTAX | LIGHT);
            let len = src[i + 1..].chars().next().map_or(1, char::len_utf8);
            out.push(at + 1..at + 1 + len, attrs);
            i += 1 + len;
            plain = i;
            continue;
        }

        if c == b'`' {
            let ticks = if rest.starts_with("``") { 2 } else { 1 };
            if let Some(n) = rest[ticks..].find(&rest[..ticks])
                && n > 0
            {
                out.push(start + plain..at, attrs);
                let m = marker(attrs, CODE);
                out.push(at..at + ticks, m);
                out.push(at + ticks..at + ticks + n, (attrs & !LINK) | CODE);
                out.push(at + ticks + n..at + ticks * 2 + n, m);
                i += ticks * 2 + n;
                plain = i;
                continue;
            }
        }

        if c == b'<'
            && let Some(len) = token_end(rest)
        {
            out.push(start + plain..at, attrs);
            i += len;
            plain = i;
            continue;
        }

        if c == b'['
            && let Some(m) = super::link::match_at(rest)
        {
            let label = &rest[m.label.clone()];
            if super::link::validate_url(&rest[m.url.clone()]).is_some() && super::link::label_ok(label, false) {
                out.push(start + plain..at, attrs);
                out.push(at..at + 1, SYNTAX | LIGHT);
                run(text, at + m.label.start, at + m.label.end, attrs | LINK, out);
                out.push(at + m.label.end..at + m.len, SYNTAX | LIGHT);
                i += m.len;
                plain = i;
                continue;
            }
        }

        if c == b'h' && (rest.starts_with("https://") || rest.starts_with("http://")) && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric()) {
            let mut len = rest.find(|ch: char| ch.is_whitespace() || ch == '<' || ch == '>').unwrap_or(rest.len());
            while rest[..len].ends_with(['.', ',', ')', '!', '?', ':', ';']) {
                len -= 1;
            }
            out.push(start + plain..at, attrs);
            out.push(at..at + len, attrs | LINK);
            i += len;
            plain = i;
            continue;
        }

        let mut matched = false;
        for &(d, s) in DELIMS {
            if !rest.starts_with(d) {
                continue;
            }
            if d == "_" && i > 0 && bytes[i - 1].is_ascii_alphanumeric() {
                continue;
            }
            let body = &rest[d.len()..];
            if d.len() == 1 && body.starts_with(char::is_whitespace) {
                continue;
            }
            let Some(n) = super::find_close(body, d) else { continue };
            if n == 0 {
                continue;
            }
            if d == "_" && body.as_bytes().get(n + 1).is_some_and(u8::is_ascii_alphanumeric) {
                continue;
            }
            out.push(start + plain..at, attrs);
            let m = marker(attrs, s);
            let dl = d.len();
            out.push(at..at + dl, m);
            run(text, at + dl, at + dl + n, attrs | s, out);
            out.push(at + dl + n..at + dl * 2 + n, m);
            i += dl * 2 + n;
            plain = i;
            matched = true;
            break;
        }
        if matched {
            continue;
        }
        i += src[i..].chars().next().map_or(1, char::len_utf8);
    }
    out.push(start + plain..end, attrs);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str, needle: &str) -> u16 {
        let p = text.find(needle).expect("needle");
        spans(text).into_iter().find(|(r, _)| r.contains(&p)).map_or(0, |(_, a)| a)
    }

    #[test]
    fn bold_content_and_semibold_markers() {
        let t = "a **bold** b";
        assert_eq!(spans(t), vec![(2..4, SYNTAX), (4..8, BOLD), (8..10, SYNTAX)]);
    }

    #[test]
    fn each_style_gets_its_attribute() {
        let t = "*it* __un__ ~~st~~ ||sp|| `co`";
        assert_eq!(at(t, "it"), ITALIC);
        assert_eq!(at(t, "un"), UNDERLINE);
        assert_eq!(at(t, "st"), STRIKE);
        assert_eq!(at(t, "sp"), SPOILER);
        assert_eq!(at(t, "co"), CODE);
        assert_eq!(at(t, "__"), SYNTAX | LIGHT | UNDERLINE);
        assert_eq!(at(t, "~~"), SYNTAX | LIGHT);
        assert_eq!(at(t, "||"), SYNTAX | LIGHT);
        assert_eq!(at(t, "`"), SYNTAX | CODE);
        assert_eq!(at(t, "*it"), SYNTAX);
    }

    #[test]
    fn nesting_combines_content_but_not_markers() {
        let t = "**a ~~b~~**";
        assert_eq!(at(t, "a "), BOLD);
        assert_eq!(at(t, "b"), BOLD | STRIKE);
        assert_eq!(at(t, "~~"), SYNTAX | LIGHT);
        let u = "__x ~~y~~__";
        assert_eq!(at(u, "~~"), SYNTAX | LIGHT | UNDERLINE);
    }

    #[test]
    fn tokens_are_opaque() {
        let t = "<:a_b_c:123> _no_";
        assert_eq!(at(t, "a_b"), 0);
        assert_eq!(at(t, "no"), ITALIC);
        assert_eq!(at("<@12> **x**", "x"), BOLD);
    }

    #[test]
    fn unclosed_or_empty_delimiters_stay_plain() {
        assert!(spans("**open").is_empty());
        assert!(spans("snake_case_name").is_empty());
        assert!(spans("* not italic*").is_empty());
    }

    #[test]
    fn escapes_links_and_subtext() {
        let t = r"\*x\*";
        assert_eq!(spans(t), vec![(0..1, SYNTAX | LIGHT), (3..4, SYNTAX | LIGHT)]);
        assert_eq!(at("see https://x.io/a.", "x.io"), LINK);
        assert_eq!(at("see https://x.io/a.", "a."), LINK);
        assert_eq!(spans("see https://x.io/a.").last().map(|(r, _)| r.end), Some(18));
        let s = "-# small **b**";
        assert_eq!(at(s, "-#"), SUBTEXT_MARK);
        assert_eq!(at(s, "small"), SUBTEXT);
        assert_eq!(at(s, "b"), SUBTEXT | BOLD);
    }

    #[test]
    fn code_blocks_mark_fences_and_language_only() {
        let t = "x\n```rust\nlet **a** = 1;\n```\n**y**";
        let s = spans(t);
        assert!(s.contains(&(2..5, FENCE)));
        assert!(s.contains(&(5..9, LANG)));
        assert_eq!(at(t, "a**"), 0);
        assert_eq!(at(t, "y"), BOLD);
        assert_eq!(at("```\n**a**", "a"), BOLD);
    }

    #[test]
    fn markdown_does_not_cross_lines() {
        assert!(spans("**a\nb**").is_empty());
    }
}
