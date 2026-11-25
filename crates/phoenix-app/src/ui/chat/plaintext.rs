use std::{cell::RefCell, collections::HashMap};

use futures::AsyncReadExt;
use gpui::{
    AnyElement, App, IntoElement, ParentElement, SharedString, StyledText, Styled, div,
    http_client::{AsyncBody, Request},
    prelude::*,
};

use crate::{
    model::Attachment,
    theme::{c, generated::idx, tok, z},
    ui::context_menu::MenuItem,
};

pub const LANGUAGES: &[&str] = &[
    "1c",
    "4d",
    "abnf",
    "accesslog",
    "ada",
    "arduino",
    "ino",
    "armasm",
    "arm",
    "avrasm",
    "actionscript",
    "as",
    "ass",
    "ssa",
    "alan",
    "ansi",
    "i",
    "log",
    "ln",
    "angelscript",
    "asc",
    "apache",
    "apacheconf",
    "applescript",
    "osascript",
    "arcade",
    "asciidoc",
    "adoc",
    "aspectj",
    "autohotkey",
    "autoit",
    "awk",
    "mawk",
    "nawk",
    "gawk",
    "bash",
    "sh",
    "zsh",
    "basic",
    "bbcode",
    "blade",
    "bnf",
    "brainfuck",
    "bf",
    "csharp",
    "cs",
    "c",
    "h",
    "cpp",
    "hpp",
    "cc",
    "hh",
    "c++",
    "h++",
    "cxx",
    "hxx",
    "cal",
    "cos",
    "cls",
    "cmake",
    "cmake.in",
    "coq",
    "csp",
    "css",
    "csv",
    "capnproto",
    "capnp",
    "chaos",
    "kaos",
    "chapel",
    "chpl",
    "cisco",
    "clojure",
    "clj",
    "coffeescript",
    "coffee",
    "cson",
    "iced",
    "cpc",
    "crmsh",
    "crm",
    "pcmk",
    "crystal",
    "cr",
    "cypher",
    "d",
    "dns",
    "zone",
    "bind",
    "dos",
    "bat",
    "cmd",
    "dart",
    "delphi",
    "dpr",
    "dfm",
    "pas",
    "pascal",
    "freepascal",
    "lazarus",
    "lpr",
    "lfm",
    "diff",
    "patch",
    "django",
    "jinja",
    "dockerfile",
    "docker",
    "dsconfig",
    "dts",
    "dust",
    "dst",
    "dylan",
    "ebnf",
    "elixir",
    "ex",
    "elm",
    "erlang",
    "erl",
    "extempore",
    "xtlang",
    "xtm",
    "fsharp",
    "fs",
    "fix",
    "fortran",
    "f90",
    "f95",
    "gcode",
    "nc",
    "gams",
    "gms",
    "gauss",
    "gss",
    "godot",
    "gdscript",
    "gherkin",
    "gn",
    "gni",
    "go",
    "golang",
    "gf",
    "golo",
    "gololang",
    "gradle",
    "groovy",
    "xml",
    "html",
    "xhtml",
    "rss",
    "atom",
    "xjb",
    "xsd",
    "xsl",
    "plist",
    "svg",
    "http",
    "https",
    "haml",
    "handlebars",
    "hbs",
    "html.hbs",
    "html.handlebars",
    "haskell",
    "hs",
    "haxe",
    "hx",
    "hy",
    "hylang",
    "ini",
    "toml",
    "inform7",
    "i7",
    "irpf90",
    "json",
    "java",
    "jsp",
    "javascript",
    "js",
    "jsx",
    "jolie",
    "iol",
    "ol",
    "julia",
    "julia-repl",
    "kotlin",
    "kt",
    "tex",
    "leaf",
    "lean",
    "lasso",
    "ls",
    "lassoscript",
    "less",
    "ldif",
    "lisp",
    "livecodeserver",
    "livescript",
    "lock",
    "ls",
    "lua",
    "makefile",
    "mk",
    "mak",
    "make",
    "markdown",
    "md",
    "mkdown",
    "mkd",
    "mathematica",
    "mma",
    "wl",
    "matlab",
    "maxima",
    "mel",
    "mercury",
    "mirc",
    "mrc",
    "mizar",
    "mojolicious",
    "monkey",
    "moonscript",
    "moon",
    "n1ql",
    "nsis",
    "never",
    "nginx",
    "nginxconf",
    "nim",
    "nimrod",
    "nix",
    "ocl",
    "ocaml",
    "ml",
    "objectivec",
    "mm",
    "objc",
    "obj-c",
    "obj-c++",
    "objective-c++",
    "glsl",
    "openscad",
    "scad",
    "ruleslanguage",
    "oxygene",
    "pf",
    "pf.conf",
    "php",
    "php3",
    "php4",
    "php5",
    "php6",
    "php7",
    "parser3",
    "perl",
    "pl",
    "pm",
    "plaintext",
    "txt",
    "text",
    "pony",
    "pgsql",
    "postgres",
    "postgresql",
    "powershell",
    "ps",
    "ps1",
    "processing",
    "prolog",
    "properties",
    "proto",
    "protobuf",
    "puppet",
    "pp",
    "python",
    "py",
    "gyp",
    "profile",
    "python-repl",
    "pycon",
    "k",
    "kdb",
    "qml",
    "r",
    "cshtml",
    "razor",
    "razor-cshtml",
    "reasonml",
    "re",
    "redbol",
    "rebol",
    "red",
    "red-system",
    "rib",
    "rsl",
    "graph",
    "instances",
    "robot",
    "rf",
    "rpm-specfile",
    "rpm",
    "spec",
    "rpm-spec",
    "specfile",
    "ruby",
    "rb",
    "gemspec",
    "podspec",
    "thor",
    "irb",
    "rust",
    "rs",
    "SAS",
    "sas",
    "scss",
    "sql",
    "p21",
    "step",
    "stp",
    "scala",
    "scheme",
    "scilab",
    "sci",
    "shexc",
    "shell",
    "console",
    "smali",
    "smalltalk",
    "st",
    "sml",
    "ml",
    "solidity",
    "sol",
    "stan",
    "stanfuncs",
    "stata",
    "iecst",
    "scl",
    "structured-text",
    "stylus",
    "styl",
    "subunit",
    "supercollider",
    "sc",
    "srt",
    "svelte",
    "swift",
    "tcl",
    "tk",
    "terraform",
    "tf",
    "hcl",
    "tap",
    "thrift",
    "tp",
    "tsql",
    "ttml",
    "twig",
    "craftcms",
    "typescript",
    "ts",
    "tsx",
    "unicorn-rails-log",
    "vbnet",
    "vb",
    "vba",
    "vbscript",
    "vbs",
    "vhdl",
    "vala",
    "verilog",
    "v",
    "vim",
    "vtt",
    "axapta",
    "x++",
    "x86asm",
    "xl",
    "tao",
    "xquery",
    "xpath",
    "xq",
    "yml",
    "yaml",
    "zephir",
    "zep",
];

pub const MARKDOWN: &[&str] = &["markdown", "md", "mkd", "mkdown"];
pub const RANGE: &str = "bytes=0-50000";
pub const COLLAPSED_LINES: usize = 6;
pub const EXPANDED_LINES: usize = 100;

pub fn extension(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or(name)
}

pub fn previewable(name: &str) -> bool {
    LANGUAGES.contains(&extension(name))
}

#[derive(Clone, Default)]
pub struct Entry {
    pub text: Option<String>,
    pub bytes_left: u64,
    pub failed: bool,
    pub expanded: bool,
    pub wrap: bool,
    pub markdown: bool,
}

thread_local! {
    static ENTRIES: RefCell<HashMap<SharedString, Entry>> = RefCell::new(HashMap::new());
    static MENU: RefCell<Option<(SharedString, gpui::Point<gpui::Pixels>)>> = const { RefCell::new(None) };
}

fn entry(url: &SharedString) -> Option<Entry> {
    ENTRIES.with(|e| e.borrow().get(url).cloned())
}

fn update(url: &SharedString, f: impl FnOnce(&mut Entry)) {
    ENTRIES.with(|e| f(e.borrow_mut().entry(url.clone()).or_insert_with(|| Entry { wrap: true, markdown: true, ..Default::default() })));
}

pub fn remaining(content_range: Option<&str>, received: u64) -> u64 {
    content_range.and_then(|r| r.rsplit('/').next()).and_then(|t| t.parse::<u64>().ok()).map_or(0, |total| total.saturating_sub(received))
}

pub fn visible(text: &str, expanded: bool, bytes_left: u64) -> (String, Option<(bool, usize)>) {
    let lines: Vec<&str> = text.split('\n').collect();
    let limit = if expanded { EXPANDED_LINES } else { COLLAPSED_LINES };
    let shown = lines.iter().take(limit).copied().collect::<Vec<_>>().join("\n");
    let whole = bytes_left == 0;
    let more = lines.len().saturating_sub(limit);
    let notice = if whole && expanded && more > 0 {
        Some((true, more))
    } else if !whole || more > 0 {
        Some((whole, more))
    } else {
        None
    };
    (shown, notice)
}

fn fetch(url: SharedString, cx: &App) {
    update(&url, |_| {});
    let http = cx.http_client();
    cx.spawn(async move |cx| {
        let req = Request::builder().uri(url.as_ref()).header("Range", RANGE).header("Accept", "text/plain").body(AsyncBody::default());
        let result = match req {
            Ok(req) => http.send(req).await.ok(),
            Err(_) => None,
        };
        let mut done = None;
        if let Some(resp) = result.filter(|r| r.status().is_success()) {
            let range = resp.headers().get("content-range").and_then(|v| v.to_str().ok()).map(str::to_owned);
            let mut body = resp.into_body();
            let mut bytes = Vec::new();
            if body.read_to_end(&mut bytes).await.is_ok() {
                let left = remaining(range.as_deref(), bytes.len() as u64);
                let mut text = String::from_utf8_lossy(&bytes).into_owned();
                if left > 0 {
                    text.pop();
                }
                done = Some((text, left));
            }
        }
        update(&url, |e| match done {
            Some((text, left)) => {
                e.text = Some(text);
                e.bytes_left = left;
            }
            None => e.failed = true,
        });
        cx.update(|cx| cx.refresh_windows());
    })
    .detach();
}

fn tk(i: usize) -> gpui::Hsla {
    c(tok(i))
}

fn icon_button(id: impl Into<gpui::ElementId>, icon: &str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex()
        .flex_none()
        .cursor_pointer()
        .text_color(tk(idx::INTERACTIVE_TEXT_DEFAULT))
        .hover(|s| s.text_color(tk(idx::INTERACTIVE_TEXT_HOVER)))
        .child(gpui::svg().path(crate::icons::path(icon)).size(z(20.)).text_color(tk(idx::INTERACTIVE_TEXT_DEFAULT)))
}

pub fn size_label(bytes: u64) -> SharedString {
    super::attach::human_size(bytes)
}

pub fn render(a: &Attachment, cx: &App) -> Option<AnyElement> {
    let url = a.url.clone();
    let Some(e) = entry(&url) else {
        fetch(url, cx);
        return Some(div().mt(z(4.)).min_w(z(302.)).h(z(118.)).rounded_t(z(4.)).border_1().border_color(tk(idx::BORDER_SUBTLE)).bg(tk(idx::BACKGROUND_BASE_LOWER)).into_any_element());
    };
    if e.failed {
        return None;
    }
    let lang = extension(&a.filename).to_owned();
    let text = e.text.clone().unwrap_or_default();
    let lines = text.split('\n').count();
    let (shown, notice) = visible(&text, e.expanded, e.bytes_left);
    let notice = notice.map(|(whole, more)| {
        if whole {
            if more == 1 { crate::t!("plaintext.line_left", n = more) } else { crate::t!("plaintext.lines_left", n = more) }
        } else {
            crate::t!("plaintext.bytes_left", size = size_label(e.bytes_left))
        }
    });
    let body = match &notice {
        Some(n) if e.expanded && e.bytes_left == 0 => format!("{shown}\n... {n}"),
        Some(n) => format!("{shown}... {n}"),
        None => shown,
    };
    let runs = crate::richtext::code_runs(&body, Some(&lang));
    let code = div()
        .min_h(z(100.))
        .p(z(8.))
        .font_family(crate::theme::FONT_MONO())
        .text_size(z(14.))
        .line_height(z(16.))
        .text_color(tk(idx::TEXT_CODE))
        .when(!e.wrap, |d| d.whitespace_nowrap())
        .child(StyledText::new(body).with_runs(runs));
    let max_h = if e.expanded { 500. } else { (COLLAPSED_LINES as f32 * 16.).max(100.) + 26. };
    let text_box = div()
        .id(("plaintext-body", a.size))
        .min_h(z(118.))
        .max_h(z(max_h))
        .overflow_hidden()
        .when(e.expanded, |d| d.overflow_y_scroll())
        .rounded_t(z(4.))
        .border_1()
        .border_color(tk(idx::BORDER_SUBTLE))
        .bg(tk(idx::BACKGROUND_BASE_LOWER))
        .child(code);
    let can_expand = e.expanded || COLLAPSED_LINES < lines;
    let count = if e.bytes_left == 0 {
        if lines == 1 { crate::t!("plaintext.line", n = lines) } else { crate::t!("plaintext.lines", n = lines) }
    } else {
        crate::t!("plaintext.more_than", n = lines)
    };
    let toggle_label: SharedString = format!("{} ({count})", if e.expanded { crate::t!("plaintext.collapse") } else { crate::t!("plaintext.expand") }).into();
    let u1 = url.clone();
    let u2 = url.clone();
    let expand = can_expand.then(|| {
        div().mr(z(8.)).child(
            div()
                .id("plaintext-expand")
                .size(z(32.))
                .rounded(z(8.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .border_1()
                .border_color(tk(idx::CONTROL_SECONDARY_BORDER_DEFAULT))
                .bg(tk(idx::CONTROL_SECONDARY_BACKGROUND_DEFAULT))
                .hover(|s| s.bg(tk(idx::CONTROL_SECONDARY_BACKGROUND_HOVER)))
                .tooltip(crate::ui::tooltip::tip(toggle_label))
                .on_click(move |_, w, _| {
                    update(&u1, |e| e.expanded = !e.expanded);
                    w.refresh();
                })
                .child(gpui::svg().path(crate::icons::path(if e.expanded { "chevron-up" } else { "chevron" })).size(z(20.)).text_color(tk(idx::CONTROL_SECONDARY_TEXT_DEFAULT))),
        )
    });
    let name = div()
        .min_w_0()
        .overflow_hidden()
        .flex()
        .flex_col()
        .child(div().min_w_0().overflow_hidden().text_ellipsis().whitespace_nowrap().text_size(z(14.)).line_height(z(18.)).text_color(tk(idx::TEXT_DEFAULT)).child(a.filename.clone()))
        .child(div().text_size(z(12.)).line_height(z(16.)).text_color(tk(idx::TEXT_SUBTLE)).child(size_label(a.size)));
    let open_url = a.url.clone();
    let footer = div()
        .flex()
        .items_center()
        .justify_end()
        .min_w_0()
        .p(z(12.))
        .rounded_b(z(8.))
        .border_1()
        .border_t_0()
        .border_color(tk(idx::BORDER_SUBTLE))
        .bg(tk(idx::BACKGROUND_SURFACE_HIGH))
        .text_size(z(14.))
        .text_color(tk(idx::TEXT_DEFAULT))
        .children(expand)
        .child(name)
        .child(div().flex_grow(200.).min_w(z(16.)))
        .child(icon_button("plaintext-lang", "angle-brackets").tooltip(crate::ui::tooltip::tip(crate::t!("plaintext.language"))))
        .child(icon_button("plaintext-full", "maximize").ml(z(16.)).tooltip(crate::ui::tooltip::tip(crate::t!("plaintext.whole_file"))).on_click(move |_, _, cx| cx.open_url(&open_url)))
        .child(icon_button("plaintext-more", "more-horizontal").ml(z(16.)).mr(z(4.)).tooltip(crate::ui::tooltip::tip(crate::t!("plaintext.more"))).on_click(move |ev, w, _| {
            let p = ev.position();
            MENU.with(|m| *m.borrow_mut() = Some((u2.clone(), p)));
            w.refresh();
        }));
    let menu = MENU.with(|m| m.borrow().clone()).filter(|(u, _)| *u == url).map(|(u, pos)| {
        let md = MARKDOWN.contains(&lang.as_str());
        let dl = u.clone();
        let wrap_url = u.clone();
        let md_url = u.clone();
        let mut items = vec![
            MenuItem::action(crate::t!("plaintext.download"), Some(crate::icons::path("download")), move |_, _, cx: &mut App| {
                MENU.with(|m| *m.borrow_mut() = None);
                cx.open_url(&dl);
            }),
            MenuItem::check(crate::t!("plaintext.wrap"), e.wrap, move |_, w, _| {
                update(&wrap_url, |e| e.wrap = !e.wrap);
                w.refresh();
            }),
        ];
        if md {
            items.push(MenuItem::check(crate::t!("plaintext.markdown"), e.markdown, move |_, w, _| {
                update(&md_url, |e| e.markdown = !e.markdown);
                w.refresh();
            }));
        }
        let dismiss: crate::ui::context_menu::Dismiss = std::rc::Rc::new(|w, _| {
            MENU.with(|m| *m.borrow_mut() = None);
            w.refresh();
        });
        crate::ui::context_menu::render_anchored(pos, gpui::Anchor::BottomLeft, items, dismiss)
    });
    Some(div().mt(z(4.)).min_w(z(302.)).max_w(z(560.)).w_full().flex().flex_col().child(text_box).child(footer).children(menu).into_any_element())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previewable_extensions_come_from_the_client_list() {
        assert!(previewable("main.rs"));
        assert!(previewable("notes.txt") == LANGUAGES.contains(&"txt"));
        assert!(previewable("README.md"));
        assert!(!previewable("photo.png"));
        assert_eq!(remaining(Some("bytes 0-50000/120000"), 50001), 69999);
        assert_eq!(remaining(None, 10), 0);
    }

    #[test]
    fn collapsed_view_keeps_six_lines() {
        let text = (1..=10).map(|n| n.to_string()).collect::<Vec<_>>().join("\n");
        let (shown, notice) = visible(&text, false, 0);
        assert_eq!(shown.lines().count(), 6);
        assert_eq!(notice, Some((true, 4)));
        let (shown, notice) = visible(&text, true, 0);
        assert_eq!(shown.lines().count(), 10);
        assert_eq!(notice, None);
        assert_eq!(visible("a", false, 10).1, Some((false, 0)));
    }
}
