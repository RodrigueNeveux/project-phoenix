//! Bot message components: buttons, select menus, Components V2 layout
//! (containers, sections, text, thumbnails, media galleries, separators),
//! polls, and the interaction modal a bot can open.
//!
//! Measurements observed in the official client (dark theme): buttons 32px
//! high, 16px side padding, 8px radius, 14px medium text, 8px gap; selects
//! 40px high with a 1px border; containers 8px radius with a 4px accent bar
//! and 16px padding; polls in a 8px-radius card, 16px padding, 600px max.
//!
//! The chat row calls [`render`] (components) and [`render_poll`] (poll)
//! under the embeds; [`overlays`] draws the modal / dialogs / toast.

use std::collections::HashSet;

use gpui::{
    AnyElement, App, Entity, FontWeight, IntoElement, MouseButton, ParentElement, SharedString, StyledText, Window,
    div, img, prelude::*, px,
};
use gpui_component::input::{Input, InputState, Textarea, TextareaState};

use crate::{
    app::PhoenixApp,
    commands::{
        ParityRequest,
        state::{ModalSpec, ModalView},
        store::{self, Comp, Emoji, Media, PollView, SelectOption},
    },
    markdown::MentionResolver,
    model::{Id, Message},
    richtext::{self, RBlock},
    theme::{Palette, c, ca},
};

/// Discord button colours (style 1..=6).
fn button_colors(style: u8) -> (u32, u32) {
    match style {
        1 => (0x5865F2, 0x4752C4),
        3 => (0x248046, 0x1A6334),
        4 => (0xDA373C, 0xA12828),
        // 2 secondary, 5 link, 6 premium
        _ => (0x4E5058, 0x6D6F78),
    }
}

struct Names<'a>(&'a PhoenixApp);

impl MentionResolver for Names<'_> {
    fn user(&self, id: u64) -> Option<String> {
        self.0.users.get(&Id(id)).map(|u| u.display_name.to_string())
    }
    fn channel(&self, id: u64) -> Option<String> {
        self.0.channels.get(&Id(id)).map(|c| c.name.to_string())
    }
    fn role(&self, id: u64) -> Option<String> {
        self.0.roles.values().flatten().find(|r| r.id == Id(id)).map(|r| r.name.to_string())
    }
}

/// Markdown text (text displays, section texts) without inline images.
pub fn markdown(app: &PhoenixApp, text: &str) -> AnyElement {
    let (_, blocks, _, _) = richtext::build(text, &Names(app), app.me.id, &|_| true);
    let mut col = div().flex().flex_col().min_w_0().text_size(px(16.)).line_height(px(22.)).text_color(c(Palette::TEXT_NORMAL));
    for b in blocks {
        col = col.child(match b {
            RBlock::Code { code, .. } => div()
                .my(px(4.))
                .p(px(8.))
                .rounded(px(4.))
                .bg(c(Palette::CODE_BG))
                .border_1()
                .border_color(c(Palette::CODE_BORDER))
                .font_family(crate::theme::FONT_MONO)
                .text_size(px(14.))
                .child(code)
                .into_any_element(),
            RBlock::Quote(t) => div()
                .flex()
                .gap(px(8.))
                .child(div().w(px(4.)).rounded(px(4.)).bg(c(Palette::INTERACTIVE_MUTED)))
                .child(StyledText::new(t.text).with_runs(t.runs))
                .into_any_element(),
            RBlock::Heading(l, t) => div()
                .mt(px(8.))
                .text_size(px(match l {
                    1 => 24.,
                    2 => 20.,
                    _ => 16.,
                }))
                .line_height(gpui::relative(1.375))
                .child(StyledText::new(t.text).with_runs(t.runs))
                .into_any_element(),
            RBlock::Subtext(t) => div().text_size(px(13.)).child(StyledText::new(t.text).with_runs(t.runs)).into_any_element(),
            RBlock::ListItem { ordered, depth, text } => div()
                .flex()
                .pl(px(16. * (depth as f32 + 1.)))
                .gap(px(4.))
                .child(match ordered {
                    Some(n) => format!("{n}."),
                    None => "•".into(),
                })
                .child(StyledText::new(text.text).with_runs(text.runs))
                .into_any_element(),
            RBlock::Text(t) => div().child(StyledText::new(t.text).with_runs(t.runs)).into_any_element(),
        });
    }
    col.into_any_element()
}

fn emoji_el(e: &Emoji, size: f32) -> AnyElement {
    crate::media::emoji::emoji_element(&e.token(), size)
}

/// Everything a click handler needs to send an interaction.
#[derive(Clone)]
struct Ctx {
    entity: Entity<PhoenixApp>,
    guild: Option<Id>,
    channel: Id,
    message: Id,
    app: Id,
    flags: u64,
}

/// Components of a bot message, or `None` when it has none.
pub fn render(app: &PhoenixApp, entity: &Entity<PhoenixApp>, msg: &Message) -> Option<AnyElement> {
    let parts = store::get(msg.id)?;
    if parts.components.is_empty() {
        return None;
    }
    let k = Ctx {
        entity: entity.clone(),
        guild: app.channels.get(&msg.channel).and_then(|c| c.guild),
        channel: msg.channel,
        message: msg.id,
        app: parts.app,
        flags: parts.flags,
    };
    let mut col = div().mt(px(4.)).flex().flex_col().gap(px(4.)).max_w(px(600.));
    for (i, comp) in parts.components.iter().enumerate() {
        col = col.child(comp_el(app, &k, comp, i));
    }
    Some(col.into_any_element())
}

fn comp_el(app: &PhoenixApp, k: &Ctx, comp: &Comp, ix: usize) -> AnyElement {
    match comp {
        Comp::Row(items) => {
            let mut row = div().flex().flex_wrap().gap(px(8.)).py(px(2.));
            for (j, it) in items.iter().enumerate() {
                row = row.child(comp_el(app, k, it, ix * 8 + j));
            }
            row.into_any_element()
        }
        Comp::Button { style, label, emoji, custom_id, url, disabled } => {
            button(app, k, *style, label.as_deref(), emoji.as_ref(), custom_id.clone(), url.clone(), *disabled, ix)
        }
        Comp::Select { kind, custom_id, placeholder, options, min, max, disabled } => {
            select(app, k, *kind, custom_id, placeholder.as_deref(), options, *min, *max, *disabled)
        }
        Comp::Text(t) => markdown(app, t),
        Comp::Section { texts, accessory } => {
            let mut left = div().flex_1().min_w_0().flex().flex_col().gap(px(4.));
            for (j, t) in texts.iter().enumerate() {
                left = left.child(comp_el(app, k, t, ix * 8 + j));
            }
            div()
                .flex()
                .gap(px(12.))
                .child(left)
                .when_some(accessory.as_deref(), |d, a| d.child(div().flex_none().child(comp_el(app, k, a, ix * 8 + 7))))
                .into_any_element()
        }
        Comp::Thumbnail(m) => img(SharedString::from(m.url.clone()))
            .size(px(85.))
            .rounded(px(8.))
            .object_fit(gpui::ObjectFit::Cover)
            .into_any_element(),
        Comp::Gallery(items) => gallery(items, k.message, ix),
        Comp::File { url, .. } => {
            let name = url.rsplit('/').next().unwrap_or("file").to_owned();
            div()
                .p(px(16.))
                .rounded(px(8.))
                .bg(c(Palette::BG_SECONDARY))
                .border_1()
                .border_color(c(Palette::BG_TERTIARY))
                .text_color(c(Palette::TEXT_LINK))
                .child(name)
                .into_any_element()
        }
        Comp::Separator { divider, spacing } => {
            let m = if *spacing >= 2 { 16. } else { 8. };
            div()
                .my(px(m / 2.))
                .h(px(if *divider { 1. } else { 0. }))
                .bg(c(Palette::DIVIDER))
                .into_any_element()
        }
        Comp::Container { accent, spoiler, children } => {
            let mut inner = div().flex_1().min_w_0().p(px(16.)).flex().flex_col().gap(px(8.));
            for (j, ch) in children.iter().enumerate() {
                inner = inner.child(comp_el(app, k, ch, ix * 16 + j));
            }
            let mut card = div()
                .flex()
                .rounded(px(8.))
                .overflow_hidden()
                .bg(c(Palette::EMBED_BG))
                .border_1()
                .border_color(ca(0xFFFFFF0F))
                .when_some(*accent, |d, a| d.child(div().w(px(4.)).flex_none().bg(c(a))))
                .child(inner);
            if *spoiler {
                card = card.opacity(0.6);
            }
            card.into_any_element()
        }
        Comp::Unknown => div().into_any_element(),
    }
}

#[allow(clippy::too_many_arguments)]
fn button(
    app: &PhoenixApp,
    k: &Ctx,
    style: u8,
    label: Option<&str>,
    emoji: Option<&Emoji>,
    custom_id: Option<String>,
    url: Option<String>,
    disabled: bool,
    ix: usize,
) -> AnyElement {
    let (bg, hover) = button_colors(style);
    let pending = custom_id.as_ref().is_some_and(|id| app.parity.pending.contains(&(k.message, id.clone())));
    let mut b = div()
        .id(("component-button", (k.message.0 << 8) ^ ix as u64))
        .h(px(32.))
        .min_w(px(60.))
        .px(px(16.))
        .rounded(px(8.))
        .bg(c(bg))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(6.))
        .text_size(px(14.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(gpui::white())
        .when_some(emoji, |d, e| d.child(emoji_el(e, 18.)))
        .when_some(label, |d, l| d.child(SharedString::from(l.to_owned())))
        .when(style == 5, |d| d.child(div().text_size(px(12.)).child("↗")))
        .when(pending, |d| d.child(div().text_size(px(12.)).child("…")));
    if disabled || pending {
        return b.opacity(0.5).cursor_not_allowed().into_any_element();
    }
    b = b.cursor_pointer().hover(move |s| s.bg(c(hover)));
    let k = k.clone();
    b.on_click(move |_, _, cx: &mut App| {
        if let Some(u) = &url {
            cx.open_url(u);
            return;
        }
        let Some(id) = custom_id.clone() else { return };
        let k = k.clone();
        k.entity.clone().update(cx, |a, cx| a.send_component(&k, id, 2, None, cx));
    })
    .into_any_element()
}

/// Options shown by user / role / channel selects (from local state).
fn auto_options(app: &PhoenixApp, kind: u8, guild: Option<Id>) -> Vec<SelectOption> {
    let opt = |label: String, value: u64| SelectOption {
        label,
        value: value.to_string(),
        description: None,
        emoji: None,
        default: false,
    };
    let mut out = Vec::new();
    if matches!(kind, 5 | 7) {
        let members: Vec<Id> = match guild {
            Some(g) => app.members.get(&g).map(|m| m.iter().map(|m| m.user).collect()).unwrap_or_default(),
            None => app.users.keys().copied().collect(),
        };
        out.extend(members.iter().filter_map(|u| app.users.get(u)).map(|u| opt(u.display_name.to_string(), u.id.0)));
    }
    if matches!(kind, 6 | 7)
        && let Some(g) = guild
    {
        out.extend(app.roles.get(&g).into_iter().flatten().map(|r| opt(format!("@{}", r.name), r.id.0)));
    }
    if kind == 8
        && let Some(g) = guild
    {
        out.extend(
            app.guild_channels
                .get(&g)
                .into_iter()
                .flatten()
                .filter_map(|c| app.channels.get(c))
                .filter(|c| c.kind != crate::model::ChannelKind::Category)
                .map(|c| opt(format!("#{}", c.name), c.id.0)),
        );
    }
    out.truncate(25);
    out
}

#[allow(clippy::too_many_arguments)]
fn select(
    app: &PhoenixApp,
    k: &Ctx,
    kind: u8,
    custom_id: &str,
    placeholder: Option<&str>,
    options: &[SelectOption],
    _min: u32,
    max: u32,
    disabled: bool,
) -> AnyElement {
    let key = (k.message, custom_id.to_owned());
    let open = app.parity.open_select.as_ref() == Some(&key);
    let options: Vec<SelectOption> = if kind == 3 { options.to_vec() } else { auto_options(app, kind, k.guild) };
    let picked: Vec<String> = app
        .parity
        .select_values
        .get(&key)
        .cloned()
        .unwrap_or_else(|| options.iter().filter(|o| o.default).map(|o| o.value.clone()).collect());
    let shown: Vec<&SelectOption> = options.iter().filter(|o| picked.contains(&o.value)).collect();
    let label: SharedString = if shown.is_empty() {
        placeholder.unwrap_or("Make a selection").to_owned().into()
    } else {
        shown.iter().map(|o| o.label.as_str()).collect::<Vec<_>>().join(", ").into()
    };
    let e = k.entity.clone();
    let key2 = key.clone();
    let kk = k.clone();
    let header = div()
        .id(("select", k.message.0 ^ hash(custom_id)))
        .h(px(40.))
        .w(px(400.))
        .max_w_full()
        .px(px(12.))
        .rounded(px(8.))
        .bg(c(Palette::INPUT_BG))
        .border_1()
        .border_color(if open { c(Palette::BLURPLE) } else { c(Palette::BG_TERTIARY) })
        .flex()
        .items_center()
        .justify_between()
        .text_size(px(16.))
        .text_color(if shown.is_empty() { c(Palette::TEXT_MUTED) } else { c(Palette::TEXT_NORMAL) })
        .child(div().flex_1().min_w_0().text_ellipsis().child(label))
        .child(div().text_color(c(Palette::INTERACTIVE)).child(if open { "▴" } else { "▾" }));
    let header = if disabled {
        header.opacity(0.5).cursor_not_allowed()
    } else {
        header.cursor_pointer().on_click(move |_, _, cx: &mut App| {
            let (key, kk) = (key2.clone(), kk.clone());
            e.update(cx, |a, cx| a.toggle_select(&kk, key, kind, max, cx));
        })
    };
    let mut col = div().flex().flex_col().child(header);
    if open {
        let mut menu = div()
            .id(("select-menu", k.message.0 ^ hash(custom_id)))
            .mt(px(4.))
            .w(px(400.))
            .max_w_full()
            .max_h(px(320.))
            .overflow_y_scroll()
            .rounded(px(8.))
            .bg(c(Palette::BG_FLOATING))
            .border_1()
            .border_color(c(Palette::BG_TERTIARY))
            .py(px(4.));
        if options.is_empty() {
            menu = menu.child(div().p(px(12.)).text_color(c(Palette::TEXT_MUTED)).child("No options"));
        }
        for (i, o) in options.into_iter().enumerate() {
            let on = picked.contains(&o.value);
            let (e, kk, key, value) = (k.entity.clone(), k.clone(), key.clone(), o.value.clone());
            menu = menu.child(
                div()
                    .id(("select-opt", i as u64))
                    .px(px(12.))
                    .py(px(8.))
                    .mx(px(4.))
                    .rounded(px(4.))
                    .cursor_pointer()
                    .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .when_some(o.emoji.as_ref(), |d, em| d.child(emoji_el(em, 20.)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().text_size(px(16.)).text_color(c(Palette::TEXT_NORMAL)).child(o.label.clone()))
                            .when_some(o.description.clone(), |d, ds| {
                                d.child(div().text_size(px(12.)).text_color(c(Palette::TEXT_MUTED)).child(ds))
                            }),
                    )
                    .when(max > 1, |d| d.child(checkbox(on)))
                    .when(max <= 1 && on, |d| d.child(div().text_color(c(Palette::BLURPLE)).child("✓")))
                    .on_click(move |_, _, cx: &mut App| {
                        let (kk, key, value) = (kk.clone(), key.clone(), value.clone());
                        e.update(cx, |a, cx| a.pick_option(&kk, key, kind, value, max, cx));
                    }),
            );
        }
        col = col.child(menu);
    }
    col.into_any_element()
}

fn checkbox(on: bool) -> AnyElement {
    div()
        .size(px(20.))
        .flex_none()
        .rounded(px(4.))
        .border_1()
        .border_color(if on { c(Palette::BLURPLE) } else { c(Palette::INTERACTIVE_MUTED) })
        .bg(if on { c(Palette::BLURPLE) } else { gpui::transparent_black() })
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(14.))
        .text_color(gpui::white())
        .when(on, |d| d.child("✓"))
        .into_any_element()
}

fn radio(on: bool) -> AnyElement {
    div()
        .size(px(20.))
        .flex_none()
        .rounded_full()
        .border_2()
        .border_color(if on { c(Palette::BLURPLE) } else { c(Palette::INTERACTIVE_MUTED) })
        .flex()
        .items_center()
        .justify_center()
        .when(on, |d| d.child(div().size(px(10.)).rounded_full().bg(c(Palette::BLURPLE))))
        .into_any_element()
}

fn hash(s: &str) -> u64 {
    s.bytes().fold(1469598103934665603u64, |h, b| (h ^ b as u64).wrapping_mul(1099511628211))
}

fn gallery(items: &[Media], msg: Id, ix: usize) -> AnyElement {
    let n = items.len();
    let mut grid = div().flex().flex_wrap().gap(px(4.)).max_w(px(520.)).rounded(px(8.)).overflow_hidden();
    for (i, m) in items.iter().take(10).enumerate() {
        let (w, h) = match n {
            1 => (520., 300.),
            2 | 4 => (258., 258.),
            _ => (170., 170.),
        };
        let url: SharedString = m.url.clone().into();
        let original = url.clone();
        grid = grid.child(
            div()
                .id(("gallery", (msg.0 << 8) ^ ((ix * 16 + i) as u64)))
                .w(px(w))
                .h(px(h))
                .cursor_pointer()
                .on_click(move |_, _, cx: &mut App| cx.open_url(&original))
                .child(img(url).size_full().object_fit(gpui::ObjectFit::Cover))
                .when(m.spoiler, |d| d.opacity(0.3)),
        );
    }
    grid.into_any_element()
}

// ------------------------------------------------------------------ polls

fn time_left(secs: i64) -> String {
    match secs {
        s if s >= 86_400 => format!("{}d left", s / 86_400),
        s if s >= 3_600 => format!("{}h left", s / 3_600),
        s if s >= 60 => format!("{}m left", s / 60),
        _ => "Less than a minute left".into(),
    }
}

/// Percent of `count` over `total`, rounded like the client (0 when empty).
pub fn percent(count: u32, total: u32) -> u32 {
    if total == 0 { 0 } else { ((count as f64 * 100.) / total as f64).round() as u32 }
}

/// Poll card of a message (question, answers with bars once voted, Vote /
/// Remove Vote, "N votes · 3h left"). `None` when the message has no poll.
pub fn render_poll(app: &PhoenixApp, entity: &Entity<PhoenixApp>, msg: &Message) -> Option<AnyElement> {
    let poll = store::get(msg.id)?.poll.clone()?;
    let now = chrono::Utc::now().timestamp();
    let ended = poll.ended(now);
    let voted = poll.voted();
    let show_results = ended || voted;
    let total = poll.total();
    let choice = app.parity.poll_choice.get(&msg.id).cloned().unwrap_or_default();
    let (ch, id) = (msg.channel, msg.id);
    let winner = poll.answers.iter().map(|a| a.count).max().unwrap_or(0);

    let mut answers = div().flex().flex_col().gap(px(8.));
    for a in &poll.answers {
        let pct = percent(a.count, total);
        let ticked = choice.contains(&a.id);
        let e = entity.clone();
        let aid = a.id;
        let multi = poll.multi;
        let mut row = div()
            .id(("poll-answer", (id.0 << 6) ^ a.id as u64))
            .relative()
            .min_h(px(48.))
            .rounded(px(8.))
            .overflow_hidden()
            .bg(c(Palette::BG_MOD_HOVER))
            .border_1()
            .border_color(if ticked || (show_results && a.me) { c(Palette::BLURPLE) } else { gpui::transparent_black() });
        if show_results {
            let fill = if ended && a.count == winner && winner > 0 { ca(0x23A55A66) } else { ca(0x5865F24D) };
            row = row.child(div().absolute().top_0().left_0().h_full().w(gpui::relative(pct as f32 / 100.)).bg(fill));
        } else {
            row = row.cursor_pointer().hover(|s| s.bg(c(Palette::BG_MOD_SELECTED))).on_click(move |_, _, cx: &mut App| {
                e.update(cx, |a, cx| a.tick_poll_answer(id, aid, multi, cx));
            });
        }
        row = row.child(
            div()
                .relative()
                .px(px(16.))
                .py(px(12.))
                .flex()
                .items_center()
                .gap(px(12.))
                .when_some(a.emoji.as_ref(), |d, em| d.child(emoji_el(em, 24.)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(16.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(c(Palette::TEXT_NORMAL))
                        .child(SharedString::from(a.text.clone())),
                )
                .when(show_results, |d| {
                    d.child(
                        div()
                            .flex()
                            .flex_col()
                            .items_end()
                            .text_size(px(14.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(c(Palette::TEXT_HEADER))
                            .child(format!("{pct}%"))
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight::NORMAL)
                                    .text_color(c(Palette::TEXT_MUTED))
                                    .child(format!("{} vote{}", a.count, if a.count == 1 { "" } else { "s" })),
                            ),
                    )
                })
                .when(!show_results, |d| d.child(if poll.multi { checkbox(ticked) } else { radio(ticked) })),
        );
        answers = answers.child(row);
    }

    let mut footer_text = format!("{total} vote{}", if total == 1 { "" } else { "s" });
    footer_text.push_str(" • ");
    footer_text.push_str(&if ended {
        "Poll closed".to_owned()
    } else {
        poll.expiry.map(|e| time_left(e - now)).unwrap_or_default()
    });
    let e = entity.clone();
    let can_vote = !choice.is_empty();
    let action: Option<AnyElement> = if ended {
        None
    } else if voted {
        Some(
            pill_button("poll-remove", "Remove Vote", false, true)
                .on_click(move |_, _, cx: &mut App| e.update(cx, |a, cx| a.vote_poll(ch, id, Vec::new(), cx)))
                .into_any_element(),
        )
    } else {
        Some(
            pill_button("poll-vote", "Vote", true, can_vote)
                .when(can_vote, |b| {
                    b.on_click(move |_, _, cx: &mut App| {
                        e.update(cx, |a, cx| {
                            let mut picks: Vec<u32> = a.parity.poll_choice.get(&id).into_iter().flatten().copied().collect();
                            picks.sort_unstable();
                            a.vote_poll(ch, id, picks, cx)
                        })
                    })
                })
                .into_any_element(),
        )
    };

    Some(
        div()
            .mt(px(8.))
            .w_full()
            .max_w(px(600.))
            .p(px(16.))
            .rounded(px(8.))
            .bg(c(Palette::BG_SECONDARY))
            .border_1()
            .border_color(c(Palette::BG_TERTIARY))
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .text_size(px(17.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(c(Palette::TEXT_HEADER))
                    .child(SharedString::from(poll.question.clone())),
            )
            .child(div().text_size(px(14.)).text_color(c(Palette::TEXT_MUTED)).child(if poll.multi {
                "Select one or more answers"
            } else {
                "Select one answer"
            }))
            .child(answers)
            .child(
                div()
                    .mt(px(4.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_size(px(14.)).text_color(c(Palette::TEXT_MUTED)).child(footer_text))
                    .children(action),
            )
            .into_any_element(),
    )
}

fn pill_button(id: &'static str, label: &'static str, primary: bool, enabled: bool) -> gpui::Stateful<gpui::Div> {
    let (bg, hover) = if primary { (0x5865F2, 0x4752C4) } else { (0x4E5058, 0x6D6F78) };
    div()
        .id(id)
        .h(px(32.))
        .px(px(16.))
        .rounded(px(8.))
        .bg(c(bg))
        .flex()
        .items_center()
        .text_size(px(14.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(gpui::white())
        .child(label)
        .when(enabled, move |d| d.cursor_pointer().hover(move |s| s.bg(c(hover))))
        .when(!enabled, |d| d.opacity(0.5).cursor_not_allowed())
}

impl PhoenixApp {
    fn send_component(&mut self, k: &Ctx, custom_id: String, kind: u8, values: Option<Vec<String>>, cx: &mut gpui::Context<Self>) {
        self.parity.pending.insert((k.message, custom_id.clone()));
        self.parity.last_target = Some((k.guild, k.channel));
        let key = (k.message, custom_id.clone());
        let req = ParityRequest::Component {
            guild: k.guild,
            channel: k.channel,
            message: k.message,
            app: k.app,
            flags: k.flags,
            custom_id,
            kind,
            values,
        };
        let (ch, msg) = (k.channel, k.message);
        self.refresh_message(ch, msg);
        self.parity_request(req, cx, move |a, res, cx| {
            a.parity.pending.remove(&key);
            a.refresh_message(ch, msg);
            if let Err(e) = res {
                a.parity_toast(format!("Interaction failed: {}", e.0), cx);
            }
        });
    }

    fn toggle_select(&mut self, k: &Ctx, key: (Id, String), kind: u8, max: u32, cx: &mut gpui::Context<Self>) {
        let was_open = self.parity.open_select.as_ref() == Some(&key);
        self.parity.open_select = if was_open { None } else { Some(key.clone()) };
        // Multi-selects send when the menu closes.
        if was_open
            && max > 1
            && let Some(v) = self.parity.select_values.get(&key).cloned()
        {
            self.send_component(k, key.1.clone(), kind, Some(v), cx);
        }
        self.refresh_message(k.channel, k.message);
        cx.notify();
    }

    fn pick_option(&mut self, k: &Ctx, key: (Id, String), kind: u8, value: String, max: u32, cx: &mut gpui::Context<Self>) {
        if max <= 1 {
            self.parity.select_values.insert(key.clone(), vec![value.clone()]);
            self.parity.open_select = None;
            self.send_component(k, key.1, kind, Some(vec![value]), cx);
        } else {
            let v = self.parity.select_values.entry(key).or_default();
            if let Some(p) = v.iter().position(|x| *x == value) {
                v.remove(p);
            } else if (v.len() as u32) < max {
                v.push(value);
            }
        }
        self.refresh_message(k.channel, k.message);
        cx.notify();
    }

    fn tick_poll_answer(&mut self, msg: Id, answer: u32, multi: bool, cx: &mut gpui::Context<Self>) {
        let set = self.parity.poll_choice.entry(msg).or_default();
        if multi {
            if !set.remove(&answer) {
                set.insert(answer);
            }
        } else {
            *set = HashSet::from([answer]);
        }
        if let Some(ch) = self.selected_channel {
            self.refresh_message(ch, msg);
        }
        cx.notify();
    }

    /// Vote (or remove our vote with an empty list), optimistically.
    pub fn vote_poll(&mut self, channel: Id, msg: Id, answers: Vec<u32>, cx: &mut gpui::Context<Self>) {
        let before = store::get(msg).and_then(|p| p.poll.clone());
        store::update_poll(msg, |p| apply_own_vote(p, &answers));
        self.parity.poll_choice.remove(&msg);
        self.refresh_message(channel, msg);
        self.parity_request(ParityRequest::VotePoll { channel, message: msg, answers }, cx, move |a, res, cx| {
            if let Err(e) = res {
                if let Some(old) = before {
                    store::update_poll(msg, |p| *p = old);
                }
                a.refresh_message(channel, msg);
                a.parity_toast(format!("Couldn't vote: {}", e.0), cx);
            }
        });
        cx.notify();
    }
}

/// Replace our previous votes with `answers` in the counts.
pub fn apply_own_vote(p: &mut PollView, answers: &[u32]) {
    let old: Vec<u32> = p.answers.iter().filter(|a| a.me).map(|a| a.id).collect();
    for id in old {
        p.apply_vote(id, false, true);
    }
    for id in answers {
        p.apply_vote(*id, true, true);
    }
}

// ------------------------------------------------------------- modal

pub enum Field {
    Line(Entity<InputState>),
    Area(Entity<TextareaState>),
}

impl Field {
    pub fn value(&self, cx: &App) -> String {
        match self {
            Field::Line(i) => i.read(cx).value().to_string(),
            Field::Area(i) => i.read(cx).value().to_string(),
        }
    }
}

fn build_modal(spec: ModalSpec, window: &mut Window, cx: &mut gpui::Context<PhoenixApp>) -> ModalView {
    let inputs = spec
        .inputs
        .iter()
        .map(|f| {
            let ph = f.placeholder.clone().unwrap_or_default();
            let val = f.value.clone().unwrap_or_default();
            if f.style == 2 {
                Field::Area(cx.new(|cx| TextareaState::new(window, cx).auto_grow(3, 8).placeholder(ph).default_value(val)))
            } else {
                Field::Line(cx.new(|cx| InputState::new(window, cx).placeholder(ph).default_value(val)))
            }
        })
        .collect::<Vec<_>>();
    if let Some(Field::Line(i)) = inputs.first() {
        i.update(cx, |s, cx| s.focus(window, cx));
    }
    ModalView { spec, inputs, sending: false, error: None }
}

impl PhoenixApp {
    fn submit_modal(&mut self, cx: &mut gpui::Context<Self>) {
        let Some(m) = self.parity.modal.as_mut() else { return };
        let mut fields = Vec::new();
        for (spec, f) in m.spec.inputs.iter().zip(&m.inputs) {
            let v = f.value(cx);
            if spec.required && v.trim().is_empty() {
                m.error = Some(format!("{} is required.", spec.label).into());
                cx.notify();
                return;
            }
            if let Some(min) = spec.min
                && !v.is_empty()
                && (v.chars().count() as u32) < min
            {
                m.error = Some(format!("{} must be at least {min} characters.", spec.label).into());
                cx.notify();
                return;
            }
            fields.push((spec.custom_id.clone(), v));
        }
        let target = m.spec.channel.map(|c| (m.spec.guild, c)).or(self.parity.last_target);
        let Some((guild, channel)) = target else {
            m.error = Some("No channel for this form.".into());
            cx.notify();
            return;
        };
        let guild = guild.or_else(|| self.channels.get(&channel).and_then(|c| c.guild));
        m.sending = true;
        m.error = None;
        let req = ParityRequest::ModalSubmit {
            guild,
            channel,
            app: m.spec.app,
            modal: m.spec.id,
            custom_id: m.spec.custom_id.clone(),
            fields,
        };
        self.parity_request(req, cx, |a, res, _| match res {
            Ok(_) => a.parity.modal = None,
            Err(e) => {
                if let Some(m) = a.parity.modal.as_mut() {
                    m.sending = false;
                    m.error = Some(e.0.into());
                }
            }
        });
        cx.notify();
    }
}

fn modal_el(app: &PhoenixApp, cx: &mut gpui::Context<PhoenixApp>) -> Option<AnyElement> {
    let m = app.parity.modal.as_ref()?;
    let close: crate::ui::modal::Action = {
        let e = cx.entity();
        std::rc::Rc::new(move |_, cx| {
            e.update(cx, |a, cx| {
                a.parity.modal = None;
                cx.notify();
            })
        })
    };
    let mut body = div().px(px(16.)).pb(px(16.)).flex().flex_col().gap(px(16.));
    for (spec, f) in m.spec.inputs.iter().zip(&m.inputs) {
        let field = match f {
            Field::Line(i) => Input::new(i).appearance(false).bordered(false).into_any_element(),
            Field::Area(i) => Textarea::new(i).appearance(false).bordered(false).into_any_element(),
        };
        body = body.child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(
                    div()
                        .flex()
                        .gap(px(4.))
                        .text_size(px(12.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(c(Palette::TEXT_MUTED))
                        .child(SharedString::from(spec.label.to_uppercase()))
                        .when(spec.required, |d| d.child(div().text_color(c(Palette::RED)).child("*"))),
                )
                .when_some(spec.description.clone(), |d, ds| {
                    d.child(div().text_size(px(14.)).text_color(c(Palette::TEXT_MUTED)).child(ds))
                })
                .child(
                    div()
                        .min_h(px(if spec.style == 2 { 80. } else { 40. }))
                        .px(px(10.))
                        .py(px(8.))
                        .rounded(px(8.))
                        .bg(c(Palette::INPUT_BG))
                        .border_1()
                        .border_color(c(Palette::BG_TERTIARY))
                        .text_size(px(16.))
                        .child(field),
                ),
        );
    }
    if let Some(err) = &m.error {
        body = body.child(div().text_size(px(14.)).text_color(c(Palette::TEXT_CRITICAL)).child(err.clone()));
    }
    let e = cx.entity();
    let close2 = close.clone();
    let card = div()
        .w(px(440.))
        .max_h(px(720.))
        .rounded(px(8.))
        .overflow_hidden()
        .bg(c(Palette::BG_PRIMARY))
        .shadow_lg()
        .flex()
        .flex_col()
        .child(
            div()
                .p(px(16.))
                .flex()
                .flex_col()
                .gap(px(4.))
                .when_some(m.spec.app_name.clone(), |d, n| {
                    d.child(div().text_size(px(12.)).text_color(c(Palette::TEXT_MUTED)).child(n))
                })
                .child(
                    div()
                        .text_size(px(20.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(Palette::TEXT_HEADER))
                        .child(SharedString::from(m.spec.title.clone())),
                ),
        )
        .child(div().id("modal-body").flex_1().min_h_0().overflow_y_scroll().child(body))
        .child(
            div()
                .p(px(16.))
                .bg(c(Palette::BG_SECONDARY))
                .flex()
                .justify_end()
                .items_center()
                .gap(px(16.))
                .child(
                    div()
                        .id("modal-cancel")
                        .cursor_pointer()
                        .text_size(px(14.))
                        .text_color(c(Palette::TEXT_NORMAL))
                        .hover(|s| s.underline())
                        .on_click(move |_, w, cx| close2(w, cx))
                        .child("Cancel"),
                )
                .child(
                    pill_button("modal-submit", if m.sending { "Submitting…" } else { "Submit" }, true, !m.sending)
                        .on_click(move |_, _, cx: &mut App| e.update(cx, |a, cx| a.submit_modal(cx))),
                ),
        );
    Some(crate::ui::modal::frame("interaction-modal", close, card))
}

fn toast(app: &PhoenixApp) -> Option<AnyElement> {
    let t = app.parity.toast.clone()?;
    Some(
        gpui::deferred(
            div().absolute().bottom(px(88.)).left_0().w_full().flex().justify_center().child(
                div()
                    .px(px(16.))
                    .py(px(10.))
                    .rounded(px(8.))
                    .bg(c(Palette::BG_FLOATING))
                    .shadow_lg()
                    .text_size(px(14.))
                    .text_color(c(Palette::TEXT_NORMAL))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(t),
            ),
        )
        .with_priority(30)
        .into_any_element(),
    )
}

/// Modals, dialogs, popouts and toast of the parity screens.
pub fn overlays(app: &mut PhoenixApp, window: &mut Window, cx: &mut gpui::Context<PhoenixApp>) -> Vec<AnyElement> {
    if let Some(spec) = app.parity.pending_modal.take() {
        app.parity.modal = Some(build_modal(spec, window, cx));
    }
    crate::ui::forum::prepare(app, window, cx);
    crate::ui::inbox::prepare(app, window, cx);
    let mut out = Vec::new();
    out.extend(crate::ui::inbox::popouts(app, cx));
    out.extend(crate::ui::inbox::dialogs(app, cx));
    out.extend(modal_el(app, cx));
    out.extend(toast(app));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::store::PollAnswer;

    #[test]
    fn percents() {
        assert_eq!(percent(1, 3), 33);
        assert_eq!(percent(2, 3), 67);
        assert_eq!(percent(0, 0), 0);
    }

    #[test]
    fn own_vote_replaces_previous() {
        let a = |id, count, me| PollAnswer { id, text: String::new(), emoji: None, count, me };
        let mut p = PollView { question: "q".into(), answers: vec![a(1, 2, true), a(2, 0, false)], multi: false, expiry: None, finalized: false };
        apply_own_vote(&mut p, &[2]);
        assert_eq!((p.answers[0].count, p.answers[0].me), (1, false));
        assert_eq!((p.answers[1].count, p.answers[1].me), (1, true));
        apply_own_vote(&mut p, &[]);
        assert_eq!(p.total(), 1);
        assert!(!p.voted());
    }

    #[test]
    fn hours_left() {
        assert_eq!(time_left(7_200), "2h left");
        assert_eq!(time_left(90_000), "1d left");
    }
}
