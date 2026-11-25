use std::rc::Rc;

use gpui::{AnyElement, App, Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, prelude::*};
use gpui_component::input::Textarea;
use serde_json::Value;

use super::{
    channel_settings::{AUTO_ARCHIVE, REGIONS, REQUIRE_TAG, SLOWMODE, State, slowmode_label, with_state},
    input_box,
    settings_form::{OnPick, description, dropdown, field, format_toolbar, helper, info_notice, radio_group, switch_field, text, value_slider, wrap_selection},
};
use crate::{
    app::PhoenixApp,
    model::ChannelKind,
    theme::{c, generated::idx, tok, z},
    ui::controls::{BtnKind, button_sm, checkbox_option},
};

fn refresh(e: &gpui::Entity<PhoenixApp>, cx: &mut App) {
    e.update(cx, |_, cx| cx.notify());
}

fn toggle(which: u8) -> Rc<dyn Fn(&mut App)> {
    Rc::new(move |_| with_state(|s| s.open_select = if s.open_select == Some(which) { None } else { Some(which) }))
}

fn picker(e: &gpui::Entity<PhoenixApp>, f: impl Fn(&mut State, usize) + 'static) -> OnPick {
    let e = e.clone();
    Rc::new(move |k, _, cx| {
        with_state(|s| {
            f(s, k);
            s.open_select = None;
        });
        refresh(&e, cx);
    })
}

pub fn max_bitrate(app: &PhoenixApp, s: &State) -> u32 {
    if s.kind == ChannelKind::Stage {
        return 64_000;
    }
    let g = s.guild.and_then(|g| app.guilds.iter().find(|x| x.id == g));
    let tier = match g.map_or(0, |g| g.boost_tier) {
        1 => 128_000,
        2 => 256_000,
        3 => 384_000,
        _ => 96_000,
    };
    let feats = s.guild.map(|g| app.backend.guild_features(g)).unwrap_or_default();
    let feat = feats
        .iter()
        .filter_map(|f| match f.as_str() {
            "AUDIO_BITRATE_128_KBPS" => Some(128_000),
            "AUDIO_BITRATE_256_KBPS" => Some(256_000),
            "AUDIO_BITRATE_384_KBPS" | "VIP_REGIONS" => Some(384_000),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    tier.max(feat)
}

fn slow_select(id: &'static str, which: u8, value: u32, open: bool, e: &gpui::Entity<PhoenixApp>, set: fn(&mut State, u32), cx: &mut Context<PhoenixApp>) -> AnyElement {
    let sel = SLOWMODE.iter().position(|x| *x == value).unwrap_or(0);
    dropdown(id, SLOWMODE.iter().map(|x| slowmode_label(*x)).collect(), sel, open, toggle(which), picker(e, move |s, k| set(s, SLOWMODE[k.min(SLOWMODE.len() - 1)])), cx)
}

fn topic_block(s: &State, forum: bool, cx: &App) -> Option<AnyElement> {
    let area = s.topic.clone()?;
    let max: i64 = if forum { 4096 } else { 1024 };
    let left = max - area.read(cx).value().encode_utf16().count() as i64;
    let a2 = area.clone();
    let wrap: Rc<dyn Fn(&'static str, &mut gpui::Window, &mut App)> = Rc::new(move |mark, w, cx| {
        a2.update(cx, |st, cx| {
            let (t, r) = wrap_selection(&st.value(), st.selected_range(), mark);
            st.set_value(t, w, cx);
            st.set_selected_range(r, cx);
            st.focus(w, cx);
        });
    });
    Some(
        div()
            .flex()
            .flex_col()
            .child(format_toolbar("cs-topic-tools", forum, wrap))
            .child(
                div()
                    .relative()
                    .h(z(202.))
                    .rounded_b(z(8.))
                    .border_1()
                    .border_color(c(tok(idx::BORDER_SUBTLE)))
                    .bg(c(tok(idx::INPUT_BACKGROUND_DEFAULT)))
                    .pl(z(15.))
                    .pr(z(12.))
                    .py(z(17.))
                    .text_size(z(16.))
                    .line_height(z(22.))
                    .child(Textarea::new(&area).appearance(false).bordered(false))
                    .child(div().absolute().right(z(13.)).bottom(z(10.)).child(text(left.to_string(), 14., 18., if left < 0 { idx::TEXT_FEEDBACK_CRITICAL } else { idx::TEXT_DEFAULT }).font_weight(FontWeight::SEMIBOLD))),
            )
            .into_any_element(),
    )
}

fn divider() -> gpui::Div {
    div().h(z(1.)).bg(c(tok(idx::BORDER_SUBTLE)))
}

pub fn render(app: &PhoenixApp, s: &State, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let e = cx.entity();
    let v = &s.v;
    let kind = s.kind;
    let mut d = div().flex().flex_col().gap(z(24.));
    if let Some(i) = &s.name {
        let key = if kind == ChannelKind::Category { "admin.category_name" } else { "admin.channel_name" };
        d = d.child(field(crate::t!(key), None, input_box(i), None));
    }
    if kind == ChannelKind::Category {
        return d.into_any_element();
    }
    let forum = kind == ChannelKind::Forum;
    if matches!(kind, ChannelKind::Text | ChannelKind::Announcement | ChannelKind::Forum)
        && let Some(t) = topic_block(s, forum, cx)
    {
        d = d.child(field(crate::t!(if forum { "cs.post_guidelines" } else { "admin.channel_topic" }), None, t, None));
    }
    if forum {
        let tags = {
            let mut row = div().flex().flex_wrap().gap(z(8.));
            for t in &s.tags {
                let name = t.get("name").and_then(Value::as_str).unwrap_or_default().to_owned();
                let emoji = t.get("emoji_name").and_then(Value::as_str).unwrap_or_default().to_owned();
                row = row.child(
                    div()
                        .h(z(32.))
                        .px(z(12.))
                        .rounded_full()
                        .bg(c(tok(idx::BACKGROUND_MOD_NORMAL)))
                        .flex()
                        .items_center()
                        .gap(z(4.))
                        .child(text(format!("{emoji} {name}").trim().to_owned(), 14., 18., idx::TEXT_DEFAULT).font_weight(FontWeight::MEDIUM)),
                );
            }
            row = row.child(button_sm("cs-new-tag", crate::t!("cs.create_tag"), BtnKind::Primary));
            let req = v.flags & REQUIRE_TAG != 0;
            div().flex().flex_col().gap(z(16.)).child(row).child(checkbox_option("cs-req-tags", crate::t!("cs.require_tags"), req).on_click(cx.listener(|_, _, _, cx| {
                with_state(|s| s.v.flags ^= REQUIRE_TAG);
                cx.notify();
            })))
        };
        d = d.child(field(crate::t!("cs.tags"), Some(crate::t!("cs.tags_desc")), tags, None));
        let reaction = div()
            .flex()
            .gap(z(8.))
            .child(button_sm("cs-react", crate::t!("cs.select_emoji"), BtnKind::Primary))
            .when(s.default_reaction.is_some(), |d| d.child(button_sm("cs-react-remove", crate::t!("cs.remove"), BtnKind::Secondary)));
        d = d.child(field(crate::t!("cs.default_reaction"), Some(crate::t!("cs.default_reaction_desc")), reaction, None));
        d = d.child(
            div()
                .flex()
                .flex_col()
                .gap(z(16.))
                .child(text(crate::t!("admin.slowmode"), 20., 24., idx::TEXT_STRONG).font_weight(FontWeight::MEDIUM))
                .child(field(crate::t!("cs.posts"), None, slow_select("cs-slow", 1, v.slow, s.open_select == Some(1), &e, |s, x| s.v.slow = x, cx), Some(crate::t!("cs.slow_posts_help"))))
                .child(field(
                    crate::t!("cs.messages"),
                    None,
                    slow_select("cs-tslow", 2, v.thread_slow, s.open_select == Some(2), &e, |s, x| s.v.thread_slow = x, cx),
                    Some(crate::t!("cs.slow_messages_help")),
                )),
        );
        let layouts = vec![crate::t!("cs.layout_list"), crate::t!("cs.layout_gallery")];
        let lsel = if v.layout == 2 { 1 } else { 0 };
        d = d.child(field(
            crate::t!("cs.default_layout"),
            Some(crate::t!("cs.default_layout_desc")),
            div().w(z(312.)).child(dropdown("cs-layout", layouts, lsel, s.open_select == Some(3), toggle(3), picker(&e, |s, k| s.v.layout = if k == 1 { 2 } else { 1 }), cx)),
            None,
        ));
        let sorts = vec![crate::t!("cs.sort_activity"), crate::t!("cs.sort_creation")];
        d = d.child(field(
            crate::t!("cs.sort_order"),
            Some(crate::t!("cs.sort_order_desc")),
            dropdown("cs-sort", sorts, v.sort.unwrap_or(0) as usize, s.open_select == Some(4), toggle(4), picker(&e, |s, k| s.v.sort = Some(k as u8)), cx),
            None,
        ));
        let matching = vec![crate::t!("cs.match_some"), crate::t!("cs.match_all")];
        let msel = usize::from(v.tag_setting.as_deref() == Some("match_all"));
        d = d.child(field(
            crate::t!("cs.tag_matching"),
            Some(crate::t!("cs.tag_matching_desc")),
            dropdown("cs-match", matching, msel, s.open_select == Some(5), toggle(5), picker(&e, |s, k| s.v.tag_setting = Some(if k == 1 { "match_all" } else { "match_some" }.into())), cx),
            None,
        ));
    } else if kind != ChannelKind::Announcement {
        let help = if matches!(kind, ChannelKind::Text | ChannelKind::Voice | ChannelKind::Stage) { "cs.slow_text_help" } else { "cs.slow_plain_help" };
        d = d.child(field(crate::t!("admin.slowmode"), None, slow_select("cs-slow", 1, v.slow, s.open_select == Some(1), &e, |s, x| s.v.slow = x, cx), Some(crate::t!(help))));
    }
    let vis = vec![
        (crate::t!("cs.vis_default"), Some(crate::t!("cs.vis_default_desc"))),
        (crate::t!("cs.vis_spoiler"), Some(crate::t!("cs.vis_spoiler_desc"))),
        (crate::t!("admin.nsfw"), Some(crate::t!("cs.vis_age_desc"))),
    ];
    d = d.child(div().py(z(8.)).child(field(crate::t!("cs.visibility"), None, radio_group("cs-vis", vis, v.visibility(), picker(&e, |s, k| s.v.set_visibility(k))), None)));
    let community = s.guild.and_then(|g| app.guilds.iter().find(|x| x.id == g)).is_some_and(|g| g.community);
    if community && matches!(kind, ChannelKind::Text | ChannelKind::Announcement) {
        let on = v.kind == 5;
        let desc = description(crate::t!("cs.announcement_desc")).into_any_element();
        let mut block = div().flex().flex_col().gap(z(4.)).child(switch_field("cs-news", crate::t!("cs.announcement"), Some(desc), on, Rc::new(|_| with_state(|s| s.v.kind = if s.v.kind == 5 { 0 } else { 5 })), cx));
        if !on {
            block = block.child(info_notice(crate::t!("cs.announcement_notice")));
        }
        d = d.child(block);
    }
    if matches!(kind, ChannelKind::Text | ChannelKind::Announcement | ChannelKind::Forum) {
        let sel = AUTO_ARCHIVE.iter().position(|x| x.0 == v.archive).unwrap_or(2);
        let help = if forum { "cs.hide_posts_help" } else { "cs.hide_threads_help" };
        d = d.child(div().mt(z(16.)).child(field(
            crate::t!("thread.hide_after"),
            None,
            dropdown("cs-archive", AUTO_ARCHIVE.iter().map(|(_, k)| crate::t!(k)).collect(), sel, s.open_select == Some(6), toggle(6), picker(&e, |s, k| s.v.archive = AUTO_ARCHIVE[k.min(3)].0), cx),
            Some(crate::t!(help)),
        )));
    }
    if matches!(kind, ChannelKind::Voice | ChannelKind::Stage) {
        let max = max_bitrate(app, s) / 1000;
        let mut marks = vec![(8, SharedString::from("8kbps")), (64, SharedString::from("64kbps"))];
        if max > 64 {
            marks.push((max, SharedString::from(format!("{max}kbps"))));
        }
        let e1 = e.clone();
        d = d.child(divider().my(z(16.))).child(field(
            crate::t!("cs.bitrate"),
            None,
            value_slider(
                "cs-bitrate",
                8,
                max,
                v.bitrate / 1000,
                marks,
                Rc::new(move |k, _, cx| {
                    with_state(|s| s.v.bitrate = k * 1000);
                    refresh(&e1, cx);
                }),
            ),
            Some(crate::t!("cs.bitrate_help", bitrate = 64)),
        ));
        let quality = vec![(crate::t!("cs.auto"), None), (SharedString::from("720p"), None)];
        d = d.child(field(
            crate::t!("cs.video_quality"),
            None,
            div().flex().flex_col().child(radio_group("cs-video", quality, if v.video == 2 { 1 } else { 0 }, picker(&e, |s, k| s.v.video = if k == 1 { 2 } else { 1 }))).child(helper(crate::t!("cs.video_quality_help")).mt(z(4.))),
            None,
        ));
        let lim_max = if kind == ChannelKind::Stage { 10_000 } else { 99 };
        let e2 = e.clone();
        d = d.child(field(
            crate::t!("cs.user_limit"),
            None,
            value_slider(
                "cs-limit",
                0,
                lim_max,
                v.user_limit,
                vec![(0, "∞".into()), (lim_max, lim_max.to_string().into())],
                Rc::new(move |k, _, cx| {
                    with_state(|s| s.v.user_limit = k);
                    refresh(&e2, cx);
                }),
            ),
            Some(crate::t!(if kind == ChannelKind::Stage { "cs.user_limit_stage_help" } else { "cs.user_limit_help" })),
        ));
        if kind == ChannelKind::Voice {
            let mut opts = vec![crate::t!("cs.auto")];
            opts.extend(REGIONS.iter().map(|(_, n)| SharedString::from(*n)));
            let sel = v.region.as_deref().and_then(|r| REGIONS.iter().position(|x| x.0 == r)).map_or(0, |i| i + 1);
            d = d.child(field(
                crate::t!("cs.region"),
                Some(crate::t!("cs.region_desc")),
                dropdown("cs-region", opts, sel, s.open_select == Some(7), toggle(7), picker(&e, |s, k| s.v.region = k.checked_sub(1).map(|i| REGIONS[i].0.to_owned())), cx),
                None,
            ));
        }
    }
    d.into_any_element()
}

#[cfg(test)]
mod tests {
    #[test]
    fn regions_and_slowmode() {
        assert_eq!(super::REGIONS.len(), 12);
        assert_eq!(super::SLOWMODE.last(), Some(&21_600));
    }
}
