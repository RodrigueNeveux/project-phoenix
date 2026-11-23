//! Tiny UI translation layer.
//!
//! Locale files live in `crates/phoenix-app/locales/<code>.yml` and are flat
//! `dotted.key: "value"` maps (a strict subset of YAML) embedded at compile
//! time. `{name}` placeholders are filled from the `t!` arguments. Lookups fall
//! back to English, then to the key itself. To add a language, drop a new file
//! in `locales/` and list it in [`LOCALES`].

use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use gpui::SharedString;

/// (code prefix, file contents). Index 0 is the English fallback.
const LOCALES: &[(&str, &str)] = &[
    ("en", include_str!("../locales/en.yml")),
    ("fr", include_str!("../locales/fr.yml")),
];

static CURRENT: AtomicUsize = AtomicUsize::new(0);

type Table = HashMap<&'static str, &'static str>;

fn tables() -> &'static [Table] {
    static T: OnceLock<Vec<Table>> = OnceLock::new();
    T.get_or_init(|| LOCALES.iter().map(|(_, src)| parse(src)).collect())
}

/// Parses `key: "value"` lines; `#` comments and blank lines are skipped.
fn parse(src: &'static str) -> Table {
    let mut m = HashMap::new();
    for line in src.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let Some((k, v)) = l.split_once(':') else { continue };
        let v = v.trim();
        let v = v.strip_prefix('"').and_then(|v| v.strip_suffix('"')).unwrap_or(v);
        m.insert(k.trim(), v);
    }
    m
}

fn index_of(code: &str) -> usize {
    let lang = code.split(['-', '_']).next().unwrap_or("").to_ascii_lowercase();
    LOCALES.iter().position(|(c, _)| *c == lang).unwrap_or(0)
}

/// Switches the UI language from a Discord locale code ("fr", "en-US", ...).
/// `PHOENIX_LOCALE` (dev/screenshots) wins over the account locale.
pub fn set_locale(code: &str) {
    let forced = env_locale();
    CURRENT.store(index_of(forced.as_deref().unwrap_or(code)), Ordering::Relaxed);
}

fn env_locale() -> Option<String> {
    std::env::var("PHOENIX_LOCALE").ok().filter(|s| !s.is_empty())
}

fn current() -> usize {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        if let Some(l) = env_locale() {
            CURRENT.store(index_of(&l), Ordering::Relaxed);
        }
    });
    CURRENT.load(Ordering::Relaxed)
}

/// The active language prefix ("en", "fr").
pub fn locale() -> &'static str {
    LOCALES[current()].0
}

fn lookup(idx: usize, key: &str) -> Option<&'static str> {
    let t = tables();
    t[idx].get(key).or_else(|| t[0].get(key)).copied()
}

fn fill(s: &str, args: &[(&str, &str)]) -> String {
    let mut out = s.replace("\\n", "\n").replace("\\\"", "\"");
    for (k, v) in args {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

/// Translates `key` in the active locale, filling `{name}` placeholders.
pub fn t(key: &str, args: &[(&str, &str)]) -> SharedString {
    t_in(current(), key, args)
}

fn t_in(idx: usize, key: &str, args: &[(&str, &str)]) -> SharedString {
    match lookup(idx, key) {
        Some(s) if args.is_empty() && !s.contains('\\') => SharedString::new_static(s),
        Some(s) => fill(s, args).into(),
        None => SharedString::from(key.to_owned()),
    }
}

/// `t!("key")` or `t!("key", name = value, ...)` -> `SharedString`.
#[macro_export]
macro_rules! t {
    ($key:expr) => { $crate::i18n::t($key, &[]) };
    ($key:expr, $($name:ident = $val:expr),+ $(,)?) => {
        $crate::i18n::t($key, &[$((stringify!($name), &$val.to_string())),+])
    };
}

// ---- dates and times -------------------------------------------------------

/// "Today at 01:30" / "Yesterday at 01:30" / localized full date for older ones.
pub fn calendar(dt: chrono::DateTime<chrono::Local>, now: chrono::DateTime<chrono::Local>) -> SharedString {
    let time = clock(dt);
    let days = (now.date_naive() - dt.date_naive()).num_days();
    match days {
        0 => t!("time.today_at", time = time),
        1 => t!("time.yesterday_at", time = time),
        _ => format!("{} {}", short_date(dt), time).into(),
    }
}

/// Wall-clock time in the locale's style ("1:30 AM" / "01:30").
pub fn clock<Tz: chrono::TimeZone>(dt: chrono::DateTime<Tz>) -> String
where
    Tz::Offset: std::fmt::Display,
{
    if locale() == "en" { dt.format("%-I:%M %p").to_string() } else { dt.format("%H:%M").to_string() }
}

/// "11/23/2026" / "23/11/2026".
pub fn short_date<Tz: chrono::TimeZone>(dt: chrono::DateTime<Tz>) -> String
where
    Tz::Offset: std::fmt::Display,
{
    if locale() == "en" { dt.format("%m/%d/%Y").to_string() } else { dt.format("%d/%m/%Y").to_string() }
}

/// "November 23, 2026" / "23 novembre 2026" (date dividers, profiles).
pub fn long_date(date: chrono::NaiveDate) -> String {
    use chrono::Datelike;
    let month = t(&format!("month.{}", date.month()), &[]);
    if locale() == "en" {
        format!("{month} {}, {}", date.day(), date.year())
    } else {
        format!("{} {month} {}", date.day(), date.year())
    }
}

/// "Nov 23, 2026" / "23 nov. 2026".
pub fn medium_date(date: chrono::NaiveDate) -> String {
    use chrono::Datelike;
    let month = t(&format!("month_short.{}", date.month()), &[]);
    if locale() == "en" {
        format!("{month} {}, {}", date.day(), date.year())
    } else {
        format!("{} {month} {}", date.day(), date.year())
    }
}

/// "Monday, November 23, 2026 at 3:04 PM" / "lundi 23 novembre 2026 à 15:04".
pub fn full_datetime(dt: chrono::DateTime<chrono::Local>) -> String {
    use chrono::Datelike;
    let wd = t(&format!("weekday.{}", dt.weekday().num_days_from_monday()), &[]);
    t("time.full", &[("weekday", &wd), ("date", &long_date(dt.date_naive())), ("time", &clock(dt))]).to_string()
}

/// "in an hour" / "2 minutes ago" (`<t:..:R>`), localized.
pub fn relative_long(delta: i64) -> String {
    let a = delta.unsigned_abs();
    let (n, unit) = match a {
        0..60 => (a, "second"),
        60..3600 => (a / 60, "minute"),
        3600..86400 => (a / 3600, "hour"),
        86400..2_592_000 => (a / 86400, "day"),
        2_592_000..31_536_000 => (a / 2_592_000, "month"),
        _ => (a / 31_536_000, "year"),
    };
    let what = if n == 1 && unit != "second" {
        t(&format!("unit.{unit}_a"), &[]).to_string()
    } else {
        let u = t(&format!("unit.{unit}_{}", if n == 1 { "one" } else { "many" }), &[]);
        format!("{n} {u}")
    };
    t(if delta >= 0 { "time.in" } else { "time.ago" }, &[("what", &what)]).to_string()
}

/// Short elapsed time: "now", "5m ago", "2h ago", "2d ago", "3mo ago".
pub fn ago_short(secs: i64) -> SharedString {
    let s = secs.max(0);
    match s {
        0..60 => t!("time.now"),
        60..3600 => t!("time.minutes_ago_short", n = s / 60),
        3600..86_400 => t!("time.hours_ago_short", n = s / 3600),
        86_400..2_592_000 => t!("time.days_ago_short", n = s / 86_400),
        _ => t!("time.months_ago_short", n = s / 2_592_000),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_english_key_exists_in_every_locale() {
        let t = tables();
        for (i, (code, _)) in LOCALES.iter().enumerate().skip(1) {
            let missing: Vec<_> = t[0].keys().filter(|k| !t[i].contains_key(*k)).collect();
            assert!(missing.is_empty(), "{code}.yml is missing keys: {missing:?}");
            let extra: Vec<_> = t[i].keys().filter(|k| !t[0].contains_key(*k)).collect();
            assert!(extra.is_empty(), "{code}.yml has keys not in en.yml: {extra:?}");
        }
    }

    #[test]
    fn placeholders_match_between_locales() {
        let holes = |s: &str| {
            let mut v: Vec<String> = s.split('{').skip(1).filter_map(|p| p.split_once('}').map(|x| x.0.to_owned())).collect();
            v.sort();
            v
        };
        let t = tables();
        for (i, (code, _)) in LOCALES.iter().enumerate().skip(1) {
            for (k, en) in &t[0] {
                if let Some(o) = t[i].get(k) {
                    assert_eq!(holes(en), holes(o), "placeholder mismatch for {k} in {code}");
                }
            }
        }
    }

    #[test]
    fn lookup_and_fallback() {
        let fr = index_of("fr");
        assert_eq!(index_of("en-US"), 0);
        assert_eq!(index_of("xx"), 0);
        assert_eq!(t_in(fr, "status.online", &[]).as_ref(), "En ligne");
        assert_eq!(t_in(fr, "chat.typing_one", &[("a", "Bob")]).as_ref(), "Bob est en train d'écrire…");
        assert_eq!(t_in(fr, "no.such.key", &[]).as_ref(), "no.such.key");
    }
}
