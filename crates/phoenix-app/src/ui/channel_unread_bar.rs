use gpui::{FontWeight, IntoElement, ParentElement, Styled, div, prelude::*, px};

use crate::app::{PhoenixApp, SidebarRow};
use crate::theme::{c, generated::idx, tok, z};

#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    Unread,
    Mentions(u32),
}

#[derive(Default)]
struct Side {
    unread: Option<usize>,
    mention: Option<usize>,
    mentions: u32,
}

type Bar = Option<(usize, Mode)>;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Place {
    Above,
    Visible,
    Below,
}

#[derive(Clone, Copy, Debug)]
struct Item {
    ix: usize,
    place: Place,
    unread: bool,
    mentions: u32,
}

fn counts_as_unread(unread: bool, muted: bool, text: bool, unreads_all: bool) -> bool {
    unread && !muted && text && unreads_all
}

fn bars(app: &PhoenixApp) -> (Bar, Bar) {
    let Some(g) = app.selected_guild.and_then(|g| app.guilds.iter().find(|x| x.id == g)) else { return (None, None) };
    if !g.community {
        return (None, None);
    }
    let list = &app.sidebar_list;
    let laid_out = list.item_is_above_viewport(0).is_some();
    let mut items = Vec::new();
    for (r, row) in app.sidebar_rows.iter().enumerate() {
        let (SidebarRow::Channel(id) | SidebarRow::Thread(id)) = *row else { continue };
        let Some(ch) = app.channels.get(&id) else { continue };
        let text = !matches!(ch.kind, crate::model::ChannelKind::Voice | crate::model::ChannelKind::Stage);
        if !(ch.unread && !ch.muted && text) && ch.mentions == 0 {
            continue;
        }
        let unreads_all = app.backend.unreads_all(id);
        let unread = counts_as_unread(ch.unread, ch.muted, text, unreads_all);
        let ix = r + 1;
        let place = match (list.item_is_above_viewport(ix), list.item_is_below_viewport(ix)) {
            (Some(true), _) => Some(Place::Above),
            (_, Some(true)) => Some(Place::Below),
            (Some(false), Some(false)) => Some(Place::Visible),
            (None, None) if laid_out => Some(Place::Below),
            _ => None,
        };
        tracing::debug!(
            target: "phoenix::unread_bar",
            channel = id.0,
            name = %ch.name,
            unread = ch.unread,
            muted = ch.muted,
            unreads_all,
            mention_count = ch.mentions,
            last_read = app.backend.last_read(id).map_or(0, |m| m.0),
            place = ?place,
            counted = unread,
        );
        if let Some(place) = place
            && (unread || ch.mentions > 0)
        {
            items.push(Item { ix, place, unread, mentions: ch.mentions });
        }
    }
    let out = pick(&items);
    tracing::debug!(target: "phoenix::unread_bar", top = ?out.0, bottom = ?out.1);
    out
}

fn pick(items: &[Item]) -> (Bar, Bar) {
    let (mut above, mut below) = (Side::default(), Side::default());
    let (mut unread_visible, mut mention_visible) = (false, false);
    for it in items {
        let (side, is_above) = match it.place {
            Place::Above => (&mut above, true),
            Place::Below => (&mut below, false),
            Place::Visible => {
                unread_visible |= it.unread;
                mention_visible |= it.mentions > 0;
                continue;
            }
        };
        if it.unread && (is_above || side.unread.is_none()) {
            side.unread = Some(it.ix);
        }
        if it.mentions > 0 {
            if is_above || side.mention.is_none() {
                side.mention = Some(it.ix);
            }
            side.mentions += it.mentions;
        }
    }
    let pick = |s: &Side| -> Bar {
        if !mention_visible && s.mentions > 0 {
            return s.mention.map(|ix| (ix, Mode::Mentions(s.mentions)));
        }
        if !unread_visible { s.unread.map(|ix| (ix, Mode::Unread)) } else { None }
    };
    let bottom = pick(&below);
    let top = pick(&above);
    let is_m = |b: &Bar| matches!(b, Some((_, Mode::Mentions(_))));
    let show_top = top.is_some() && (bottom.is_none() || (!is_m(&bottom) && is_m(&top)));
    let show_bottom = bottom.is_some() && (is_m(&bottom) || !show_top);
    (top.filter(|_| show_top), bottom.filter(|_| show_bottom))
}

pub fn render(app: &PhoenixApp) -> Vec<gpui::AnyElement> {
    let (top, bottom) = bars(app);
    let mut out = Vec::new();
    for (bar, is_top) in [(top, true), (bottom, false)] {
        if let Some((ix, mode)) = bar {
            out.push(bar_el(app, ix, mode, is_top));
        }
    }
    out
}

fn bar_el(app: &PhoenixApp, ix: usize, mode: Mode, top: bool) -> gpui::AnyElement {
    let list = app.sidebar_list.clone();
    let mention = matches!(mode, Mode::Mentions(_));
    let label = match mode {
        Mode::Unread => crate::t!("channels.unread_bar"),
        Mode::Mentions(1) => crate::t!("channels.mention_bar_one"),
        Mode::Mentions(n) => crate::t!("channels.mention_bar", count = n),
    };
    div()
        .absolute()
        .left_0()
        .right_0()
        .p(z(8.))
        .flex()
        .justify_center()
        .when(top, |d| d.top_0())
        .when(!top, |d| d.bottom_0())
        .child(
            div()
                .id(if top { "channels-unread-top" } else { "channels-unread-bottom" })
                .px(z(8.))
                .py(z(4.))
                .rounded(z(20.))
                .flex()
                .items_center()
                .justify_center()
                .gap(z(4.))
                .cursor_pointer()
                .border_1()
                .map(|d| {
                    if mention {
                        d.bg(c(tok(idx::BACKGROUND_FEEDBACK_NOTIFICATION)))
                            .border_color(c(tok(idx::BORDER_SUBTLE)))
                            .shadow(crate::ui::inbox::shadow_medium())
                    } else {
                        d.bg(c(tok(idx::BACKGROUND_SURFACE_HIGHEST))).border_color(c(tok(idx::BORDER_NORMAL))).shadow(vec![
                            gpui::BoxShadow {
                                color: c(tok(idx::OPACITY_BLACK_24)),
                                offset: gpui::point(px(0.), z(12.)),
                                blur_radius: z(24.),
                                spread_radius: px(0.),
                                inset: false,
                            },
                        ])
                    }
                })
                .text_size(z(12.))
                .line_height(z(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c(tok(if mention { idx::TEXT_OVERLAY_LIGHT } else { idx::INTERACTIVE_TEXT_DEFAULT })))
                .when(!mention, |d| {
                    d.child(
                        gpui::svg()
                            .path(crate::icons::path(if top { "chevron-up" } else { "chevron" }))
                            .size(z(14.))
                            .flex_none()
                            .text_color(c(tok(idx::INTERACTIVE_TEXT_DEFAULT))),
                    )
                })
                .child(label.to_uppercase())
                .on_click(move |_, w, _| {
                    list.scroll_to_reveal_item(ix);
                    w.refresh();
                }),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn it(ix: usize, place: Place, unread: bool, mentions: u32) -> Item {
        Item { ix, place, unread, mentions }
    }

    #[test]
    fn only_mentions_channels_do_not_count() {
        assert!(counts_as_unread(true, false, true, true));
        assert!(!counts_as_unread(true, false, true, false));
        assert!(!counts_as_unread(true, true, true, true));
        assert!(!counts_as_unread(true, false, false, true));
    }

    #[test]
    fn top_bar_needs_no_unread_visible_or_below() {
        let above = it(2, Place::Above, true, 0);
        assert_eq!(pick(&[above]), (Some((2, Mode::Unread)), None));
        assert_eq!(pick(&[above, it(5, Place::Visible, true, 0)]), (None, None));
        assert_eq!(pick(&[above, it(9, Place::Below, true, 0)]), (None, Some((9, Mode::Unread))));
        assert_eq!(pick(&[above, it(5, Place::Visible, false, 0)]), (Some((2, Mode::Unread)), None));
    }

    #[test]
    fn mentions_win_over_unreads() {
        let r = pick(&[it(2, Place::Above, true, 2), it(9, Place::Below, true, 0)]);
        assert_eq!(r, (Some((2, Mode::Mentions(2))), None));
        let r = pick(&[it(2, Place::Above, true, 0), it(9, Place::Below, false, 1)]);
        assert_eq!(r, (None, Some((9, Mode::Mentions(1)))));
    }
}
