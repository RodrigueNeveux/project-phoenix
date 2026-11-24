//! Search term highlighting, as the client does it (local bundle
//! `current-client/js/698890330839a6c6.js`, module 697530, with the stemmer of
//! module 83956):
//!
//! - the query is split on `/\W+/` (so any non-ASCII letter splits a word:
//!   "règlement" gives "r" and "glement"), a trailing possessive `'s` is dropped,
//!   words are lowercased, English stop words removed, and each word is stemmed
//!   with the Snowball English (Porter2) stemmer;
//! - a text is split on `/(\W+)/` and every token whose stem is one of the query
//!   stems is highlighted, whole (`supportPartialMatches`, used for forum titles
//!   of 3 to 12 characters, matches a stem that merely contains a query stem);
//! - in a message the highlighter runs over every markdown node with a string
//!   content except code blocks (text, bold, italic, links, inline code…).
//!
//! The highlighted token is `span.highlight`: background hsl(--yellow-300 / .3).

use std::{collections::HashSet, ops::Range};

/// `.highlight` background: hsl(var(--yellow-300-hsl) / .3) = rgba(253, 184, 51, .3).
pub const HIGHLIGHT_RGBA: u32 = 0xfdb8334d;

/// The client's stop word list (English only).
const STOP: [&str; 33] = [
    "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "if", "in", "into", "is", "it", "no", "not", "of", "on", "or", "such",
    "that", "the", "their", "then", "there", "these", "they", "this", "to", "was", "will", "with",
];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Highlighter {
    words: HashSet<String>,
    partial: bool,
}

/// JS `\w`: ASCII letters, digits and `_`.
fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `s.replace(/('|’|＇)(s|S)$/, "")`.
fn strip_possessive(s: &str) -> &str {
    for q in ['\'', '\u{2019}', '\u{FF07}'] {
        for e in ['s', 'S'] {
            if let Some(base) = s.strip_suffix(e).and_then(|rest| rest.strip_suffix(q)) {
                return base;
            }
        }
    }
    s
}

/// Lowercased, possessive-free and stemmed token, or `None` when it can't match
/// (empty or a stop word).
fn normalize(token: &str) -> Option<String> {
    let lower = strip_possessive(token).to_lowercase();
    if lower.is_empty() || STOP.contains(&lower.as_str()) {
        return None;
    }
    Some(stem(&lower))
}

/// Split like JS `text.split(/(\W+)/g)`: alternating word and separator runs,
/// as byte ranges.
fn tokens(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut word = None;
    for (i, ch) in text.char_indices() {
        let w = is_word(ch);
        if word.is_some_and(|p| p != w) {
            out.push(start..i);
            start = i;
        }
        word = Some(w);
    }
    if start < text.len() {
        out.push(start..text.len());
    }
    out
}

impl Highlighter {
    /// `th(query, partial)`: partial matching only for queries of 3 to 12 characters.
    pub fn new(query: &str, partial: bool) -> Self {
        let words = query.split(|c: char| !is_word(c)).filter_map(normalize).collect();
        let n = query.chars().count();
        Self { words, partial: partial && (3..=12).contains(&n) }
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// Byte ranges of `text` to highlight (whole tokens, merged when adjacent).
    pub fn ranges(&self, text: &str) -> Vec<Range<usize>> {
        if self.words.is_empty() {
            return Vec::new();
        }
        let mut out: Vec<Range<usize>> = Vec::new();
        for r in tokens(text) {
            let Some(stemmed) = normalize(&text[r.clone()]) else { continue };
            let hit = if self.partial { self.words.iter().any(|w| stemmed.contains(w.as_str())) } else { self.words.contains(&stemmed) };
            if hit {
                out.push(r);
            }
        }
        out
    }
}

// ------------------------------------------------------------ Porter2 stemmer

fn vowel(c: u8) -> bool {
    matches!(c, b'a' | b'e' | b'i' | b'o' | b'u' | b'y')
}

/// Start of the region after the first non-vowel following a vowel, from `from`.
fn region(w: &[u8], from: usize) -> usize {
    let mut i = from;
    while i < w.len() && !vowel(w[i]) {
        i += 1;
    }
    while i < w.len() && vowel(w[i]) {
        i += 1;
    }
    (i + 1).min(w.len())
}

/// Short syllable ending at `end` (exclusive): non-vowel, vowel, non-vowel other
/// than w, x, Y; or vowel + non-vowel at the start of the word.
fn short_syllable_at(w: &[u8], end: usize) -> bool {
    if end == 2 {
        return vowel(w[0]) && !vowel(w[1]);
    }
    end >= 3 && !vowel(w[end - 3]) && vowel(w[end - 2]) && !vowel(w[end - 1]) && !matches!(w[end - 1], b'w' | b'x' | b'Y')
}

fn ends(w: &[u8], s: &str) -> bool {
    w.ends_with(s.as_bytes())
}

fn replace_end(w: &mut Vec<u8>, suffix_len: usize, with: &str) {
    w.truncate(w.len() - suffix_len);
    w.extend_from_slice(with.as_bytes());
}

fn has_vowel(w: &[u8]) -> bool {
    w.iter().copied().any(vowel)
}

/// Snowball English (Porter2) stem of a lowercase ASCII word; other input is
/// returned unchanged past the exceptions.
pub fn stem(word: &str) -> String {
    if word.len() < 3 || !word.is_ascii() {
        return word.to_owned();
    }
    match word {
        "skis" => return "ski".into(),
        "skies" => return "sky".into(),
        "dying" => return "die".into(),
        "lying" => return "lie".into(),
        "tying" => return "tie".into(),
        "idly" => return "idl".into(),
        "gently" => return "gentl".into(),
        "ugly" => return "ugli".into(),
        "early" => return "earli".into(),
        "only" => return "onli".into(),
        "singly" => return "singl".into(),
        "sky" | "news" | "howe" | "atlas" | "cosmos" | "bias" | "andes" => return word.into(),
        _ => {}
    }
    let mut w: Vec<u8> = word.trim_start_matches('\'').as_bytes().to_vec();
    // Initial y, and y after a vowel, are consonants (Y).
    for i in 0..w.len() {
        if w[i] == b'y' && (i == 0 || vowel(w[i - 1])) {
            w[i] = b'Y';
        }
    }
    let r1 = if w.starts_with(b"gener") || w.starts_with(b"arsen") {
        5
    } else if w.starts_with(b"commun") {
        6
    } else {
        region(&w, 0)
    };
    let r2 = region(&w, r1);

    // Step 0: 's' 's '
    for s in ["'s'", "'s", "'"] {
        if ends(&w, s) {
            w.truncate(w.len() - s.len());
            break;
        }
    }
    // Step 1a
    if ends(&w, "sses") {
        replace_end(&mut w, 4, "ss");
    } else if ends(&w, "ied") || ends(&w, "ies") {
        let with = if w.len() > 4 { "i" } else { "ie" };
        replace_end(&mut w, 3, with);
    } else if ends(&w, "us") || ends(&w, "ss") {
    } else if ends(&w, "s") && w.len() >= 2 && has_vowel(&w[..w.len() - 2]) {
        w.pop();
    }
    if matches!(&w[..], b"inning" | b"outing" | b"canning" | b"herring" | b"earring" | b"proceed" | b"exceed" | b"succeed") {
        return String::from_utf8(w).unwrap_or_default();
    }
    // Step 1b
    let eed = ["eedly", "eed"].into_iter().find(|s| ends(&w, s));
    let ed = ["ingly", "edly", "ing", "ed"].into_iter().find(|s| ends(&w, s));
    match (eed, ed) {
        (Some(s), _) => {
            if w.len() - s.len() >= r1 {
                replace_end(&mut w, s.len(), "ee");
            }
        }
        (None, Some(s)) if has_vowel(&w[..w.len() - s.len()]) => {
            w.truncate(w.len() - s.len());
            if ends(&w, "at") || ends(&w, "bl") || ends(&w, "iz") {
                w.push(b'e');
            } else if w.len() >= 2
                && w[w.len() - 1] == w[w.len() - 2]
                && matches!(w[w.len() - 1], b'b' | b'd' | b'f' | b'g' | b'm' | b'n' | b'p' | b'r' | b't')
            {
                w.pop();
            } else if r1 >= w.len() && short_syllable_at(&w, w.len()) {
                w.push(b'e');
            }
        }
        _ => {}
    }
    // Step 1c
    let n = w.len();
    if n > 2 && matches!(w[n - 1], b'y' | b'Y') && !vowel(w[n - 2]) {
        w[n - 1] = b'i';
    }
    // Step 2 (suffix in R1)
    const STEP2: [(&str, &str); 24] = [
        ("ization", "ize"),
        ("ational", "ate"),
        ("fulness", "ful"),
        ("ousness", "ous"),
        ("iveness", "ive"),
        ("tional", "tion"),
        ("biliti", "ble"),
        ("lessli", "less"),
        ("entli", "ent"),
        ("ation", "ate"),
        ("alism", "al"),
        ("aliti", "al"),
        ("ousli", "ous"),
        ("iviti", "ive"),
        ("fulli", "ful"),
        ("enci", "ence"),
        ("anci", "ance"),
        ("abli", "able"),
        ("izer", "ize"),
        ("ator", "ate"),
        ("alli", "al"),
        ("bli", "ble"),
        ("ogi", "og"),
        ("li", ""),
    ];
    if let Some(&(s, with)) = STEP2.iter().filter(|(s, _)| ends(&w, s)).max_by_key(|(s, _)| s.len()) {
        let at = w.len() - s.len();
        if at >= r1 {
            let ok = match s {
                "ogi" => at >= 1 && w[at - 1] == b'l',
                "li" => at >= 1 && matches!(w[at - 1], b'c' | b'd' | b'e' | b'g' | b'h' | b'k' | b'm' | b'n' | b'r' | b't'),
                _ => true,
            };
            if ok {
                replace_end(&mut w, s.len(), with);
            }
        }
    }
    // Step 3 (suffix in R1; ative in R2)
    const STEP3: [(&str, &str); 9] = [
        ("ational", "ate"),
        ("tional", "tion"),
        ("alize", "al"),
        ("icate", "ic"),
        ("iciti", "ic"),
        ("ative", ""),
        ("ical", "ic"),
        ("ness", ""),
        ("ful", ""),
    ];
    if let Some(&(s, with)) = STEP3.iter().filter(|(s, _)| ends(&w, s)).max_by_key(|(s, _)| s.len()) {
        let at = w.len() - s.len();
        if at >= r1 && (s != "ative" || at >= r2) {
            replace_end(&mut w, s.len(), with);
        }
    }
    // Step 4 (suffix in R2)
    const STEP4: [&str; 18] =
        ["ement", "ance", "ence", "able", "ible", "ment", "ant", "ent", "ism", "ate", "iti", "ous", "ive", "ize", "ion", "al", "er", "ic"];
    if let Some(&s) = STEP4.iter().filter(|s| ends(&w, s)).max_by_key(|s| s.len()) {
        let at = w.len() - s.len();
        if at >= r2 && (s != "ion" || (at >= 1 && matches!(w[at - 1], b's' | b't'))) {
            w.truncate(at);
        }
    }
    // Step 5
    let n = w.len();
    if n >= 1 && w[n - 1] == b'e' {
        let at = n - 1;
        if at >= r2 || (at >= r1 && !short_syllable_at(&w, at)) {
            w.pop();
        }
    } else if n >= 2 && w[n - 1] == b'l' && n - 1 >= r2 && w[n - 2] == b'l' {
        w.pop();
    }
    for c in &mut w {
        if *c == b'Y' {
            *c = b'y';
        }
    }
    String::from_utf8(w).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn porter2_vocabulary() {
        // Pairs from the Snowball English sample vocabulary.
        for (w, s) in [
            ("consign", "consign"),
            ("consigned", "consign"),
            ("consigning", "consign"),
            ("consignment", "consign"),
            ("consist", "consist"),
            ("consisted", "consist"),
            ("consistency", "consist"),
            ("consistent", "consist"),
            ("consistently", "consist"),
            ("consisting", "consist"),
            ("consists", "consist"),
            ("consolation", "consol"),
            ("consolations", "consol"),
            ("consolatory", "consolatori"),
            ("console", "consol"),
            ("consoled", "consol"),
            ("consoles", "consol"),
            ("consolidate", "consolid"),
            ("consolidated", "consolid"),
            ("consolidating", "consolid"),
            ("consoling", "consol"),
            ("consolingly", "consol"),
            ("consonant", "conson"),
            ("conspicuous", "conspicu"),
            ("conspicuously", "conspicu"),
            ("conspiracy", "conspiraci"),
            ("conspirator", "conspir"),
            ("conspirators", "conspir"),
            ("conspire", "conspir"),
            ("conspired", "conspir"),
            ("constable", "constabl"),
            ("constancy", "constanc"),
            ("constant", "constant"),
            ("caresses", "caress"),
            ("ponies", "poni"),
            ("ties", "tie"),
            ("cries", "cri"),
            ("gaps", "gap"),
            ("gas", "gas"),
            ("kiwis", "kiwi"),
            ("running", "run"),
            ("hopping", "hop"),
            ("hoped", "hope"),
            ("luxuriating", "luxuri"),
            ("generously", "generous"),
            ("communication", "communic"),
            ("configuration", "configur"),
            ("skies", "sky"),
            ("news", "news"),
            ("succeed", "succeed"),
            ("cry", "cri"),
            ("by", "by"),
            ("say", "say"),
        ] {
            assert_eq!(stem(w), s, "{w}");
        }
    }

    #[test]
    fn query_words_split_on_non_ascii() {
        // The live client highlighted "r" and "glement" in "règlement" for that query.
        let h = Highlighter::new("règlement", false);
        let t = "〢📜・règlement";
        let got: Vec<&str> = h.ranges(t).into_iter().map(|r| &t[r]).collect();
        assert_eq!(got, ["r", "glement"]);
    }

    #[test]
    fn whole_tokens_by_stem_without_stop_words() {
        let h = Highlighter::new("Configuration du serveur", false);
        let t = "La configuration du **serveur** (configurations) is the best";
        let got: Vec<&str> = h.ranges(t).into_iter().map(|r| &t[r]).collect();
        assert_eq!(got, ["configuration", "du", "serveur", "configurations"]);
        assert!(Highlighter::new("the of", false).is_empty());
        assert!(Highlighter::new("Bob's", false).ranges("bob").len() == 1);
    }

    #[test]
    fn partial_only_for_short_queries() {
        assert_eq!(Highlighter::new("pho", true).ranges("phoenix").len(), 1);
        assert_eq!(Highlighter::new("pho", false).ranges("phoenix").len(), 0);
        // 14 characters: exact stems only.
        assert_eq!(Highlighter::new("phoenixes rock", true).ranges("phoenix").len(), 1);
        assert_eq!(Highlighter::new("phoenixes rock", true).ranges("phoenixlike").len(), 0);
    }
}
