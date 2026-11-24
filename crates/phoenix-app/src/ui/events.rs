//! Server "Events" modal (sidebar row "Événements"): `GET
//! /guilds/{id}/scheduled-events`, read-only. Frame, header and empty state
//! from the live client (dump vanilla-events-modal): root__KoJTP medium 602x400,
//! --radius-md 12, 1px border, --modal-background; header padding 16 24 8 with
//! the 24px calendar icon (margin-right 8, --icon-muted), heading-sm/semibold
//! title, a 16px divider (margin 0 16) and the "Create Event" sm primary button;
//! close icon 24 on the right. Content padding 8 16 16 24.

use gpui::{AnyElement, App, Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, Window, div, prelude::*, px};

use crate::{
    app::PhoenixApp,
    commands::{GuildEvent, ParityReply, ParityRequest},
    model::Id,
    theme::{c, generated::idx, tok, z},
};

#[derive(Default)]
struct Slot(Option<Events>);
impl gpui::Global for Slot {}

struct Events {
    guild: Id,
    list: Option<Vec<GuildEvent>>,
    error: Option<SharedString>,
}

fn t(i: usize) -> gpui::Hsla {
    c(tok(i))
}

/// CREATE_EVENTS (1 << 44) or MANAGE_EVENTS (1 << 33).
fn can_create(perms: u64) -> bool {
    crate::ui::inbox::has_perm(perms, (1 << 44) | (1 << 33))
}

pub fn is_open(cx: &App) -> bool {
    cx.try_global::<Slot>().is_some_and(|s| s.0.is_some())
}

pub fn close(cx: &mut App) {
    cx.set_global(Slot(None));
}

/// Opens the modal for `guild` and loads its events.
pub fn open(app: &mut PhoenixApp, guild: Id, cx: &mut Context<PhoenixApp>) {
    cx.set_global(Slot(Some(Events { guild, list: None, error: None })));
    app.parity_request(ParityRequest::ScheduledEvents { guild }, cx, move |_, r, cx| {
        if let Some(Some(e)) = cx.try_global::<Slot>().map(|s| s.0.as_ref().map(|e| e.guild)) {
            if e != guild {
                return;
            }
        } else {
            return;
        }
        let (list, error) = match r {
            Ok(ParityReply::Events(mut v)) => {
                // Upcoming and live first, by start time; completed/canceled hidden.
                v.retain(|e| e.status <= 2);
                v.sort_by_key(|e| (e.status != 2, e.start));
                (Some(v), None)
            }
            Ok(_) => (Some(vec![]), None),
            Err(e) => (Some(vec![]), Some(SharedString::from(e.0))),
        };
        cx.set_global(Slot(Some(Events { guild, list, error })));
        cx.notify();
    });
    cx.notify();
}

fn when_label(e: &GuildEvent) -> String {
    let Some(d) = chrono::DateTime::from_timestamp(e.start, 0) else { return String::new() };
    let local = d.with_timezone(&chrono::Local);
    format!("{} {}", crate::i18n::long_date(local.date_naive()), local.format("%H:%M"))
}

/// One event card (not measured on the live client yet: tokens only).
fn card(app: &PhoenixApp, e: &GuildEvent) -> gpui::Div {
    let live = e.status == 2;
    let place = e
        .location
        .clone()
        .or_else(|| e.channel.and_then(|ch| app.channels.get(&ch)).map(|ch| ch.name.to_string()))
        .unwrap_or_default();
    div()
        .rounded(z(8.))
        .border_1()
        .border_color(t(idx::BORDER_SUBTLE))
        .bg(t(idx::BACKGROUND_BASE_LOW))
        .p(z(16.))
        .flex()
        .flex_col()
        .gap(z(8.))
        .child(
            div()
                .flex()
                .items_center()
                .gap(z(8.))
                .text_size(z(14.))
                .line_height(z(18.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(if live { t(idx::TEXT_FEEDBACK_POSITIVE) } else { t(idx::TEXT_MUTED) })
                .child(gpui::svg().path(crate::icons::path("calendar")).size(z(16.)).flex_none().text_color(if live {
                    t(idx::TEXT_FEEDBACK_POSITIVE)
                } else {
                    t(idx::ICON_MUTED)
                }))
                .child(if live { crate::t!("events.live").to_string() } else { when_label(e) })
                .child(div().flex_1())
                .when(e.user_count > 0, |d| {
                    d.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(z(4.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(t(idx::TEXT_MUTED))
                            .child(gpui::svg().path(crate::icons::path("members")).size(z(16.)).flex_none().text_color(t(idx::ICON_MUTED)))
                            .child(e.user_count.to_string()),
                    )
                }),
        )
        .child(
            div()
                .text_size(z(20.))
                .line_height(z(24.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(t(idx::TEXT_STRONG))
                .child(e.name.clone()),
        )
        .when(!e.description.is_empty(), |d| {
            d.child(div().text_size(z(14.)).line_height(z(18.)).text_color(t(idx::TEXT_DEFAULT)).child(e.description.clone()))
        })
        .when(!place.is_empty(), |d| {
            d.child(div().pt(z(8.)).border_t_1().border_color(t(idx::BORDER_SUBTLE)).text_size(z(14.)).text_color(t(idx::TEXT_MUTED)).child(place))
        })
}

pub fn modal(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    let s = cx.try_global::<Slot>()?.0.as_ref()?;
    let guild = s.guild;
    let perms = app.backend.guild_permissions(guild);
    let e = cx.entity();
    let on_close: crate::ui::modal::Action = std::rc::Rc::new(move |_: &mut Window, cx: &mut App| {
        close(cx);
        e.update(cx, |_, cx| cx.notify());
    });
    let close2 = on_close.clone();
    let header = div()
        // flex_hkRWAH header: padding 16 24 8, radius 12 12 0 0
        .pt(z(16.))
        .px(z(24.))
        .pb(z(8.))
        .flex()
        .items_center()
        .child(gpui::svg().path(crate::icons::path("calendar")).size(z(24.)).mr(z(8.)).flex_none().text_color(t(idx::ICON_MUTED)))
        .child(
            // heading-sm/semibold 16/20 --text-strong
            div()
                .text_size(z(16.))
                .line_height(z(20.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(t(idx::TEXT_STRONG))
                .child(crate::t!("events.title")),
        )
        .when(can_create(perms), |d| {
            // divider_JG2DGL: 1px x 16, margin 0 16, --border-subtle
            d.child(div().w(px(1.)).h(z(16.)).mx(z(16.)).bg(t(idx::BORDER_SUBTLE))).child(
                // button_F7qvhY sm primary: h 32, padding 3 11, radius 8, body-sm/medium
                div()
                    .id("events-create")
                    .h(z(32.))
                    .px(z(12.))
                    .rounded(z(8.))
                    .bg(t(idx::CONTROL_PRIMARY_BACKGROUND_DEFAULT))
                    .hover(|s| s.bg(t(idx::CONTROL_PRIMARY_BACKGROUND_HOVER)))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .text_size(z(14.))
                    .line_height(z(18.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(gpui::white())
                    .on_click(cx.listener(move |a, _, _, cx| {
                        close(cx);
                        a.open_create_event(guild, cx);
                    }))
                    .child(crate::t!("events.create")),
            )
        })
        .child(div().flex_1())
        .child(
            // iconButton_JG2DGL: 24px close icon, --icon-muted
            div()
                .id("events-close")
                .cursor_pointer()
                .on_click(move |_, w, cx| close2(w, cx))
                .child(gpui::svg().path(crate::icons::path("close")).size(z(24.)).flex_none().text_color(t(idx::ICON_MUTED))),
        );
    let body: AnyElement = match &s.list {
        None => div().size_full().flex().items_center().justify_center().text_color(t(idx::TEXT_MUTED)).child(crate::t!("common.loading")).into_any_element(),
        Some(list) if list.is_empty() => {
            // container_NkbMg8: centred column; title heading-xl/semibold 24/32 margin-bottom 8;
            // subtitle / roleTip_ (margin-top 4) body-sm/normal 14/18 --text-default.
            div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .text_center()
                .child(
                    div()
                        .mb(z(8.))
                        .text_size(z(24.))
                        .line_height(z(32.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(t(idx::TEXT_STRONG))
                        .child(crate::t!("events.empty_title")),
                )
                .child(div().text_size(z(14.)).line_height(z(18.)).text_color(t(idx::TEXT_DEFAULT)).child(crate::t!("events.empty_sub")))
                .when(can_create(perms), |d| {
                    d.child(
                        div()
                            .mt(z(4.))
                            .text_size(z(14.))
                            .line_height(z(18.))
                            .text_color(t(idx::TEXT_DEFAULT))
                            .child(crate::t!("events.role_tip"))
                            .child(div().font_weight(FontWeight::SEMIBOLD).text_color(t(idx::TEXT_LINK)).child(crate::t!("events.role_tip_link"))),
                    )
                })
                .when_some(s.error.clone(), |d, er| d.child(div().mt(z(8.)).text_size(z(14.)).text_color(t(idx::TEXT_FEEDBACK_CRITICAL)).child(er)))
                .into_any_element()
        }
        Some(list) => {
            let mut col = div().id("events-list").size_full().overflow_y_scroll().flex().flex_col().gap(z(16.));
            for ev in list {
                col = col.child(card(app, ev));
            }
            col.into_any_element()
        }
    };
    let card = div()
        // root__KoJTP medium: 602x400, radius 12, 1px border, shadow 0 4 8 black 16%
        .w(z(602.))
        .h(z(400.))
        .rounded(z(12.))
        .border_1()
        .border_color(t(idx::BORDER_SUBTLE))
        .bg(t(idx::MODAL_BACKGROUND))
        .shadow(vec![gpui::BoxShadow {
            color: gpui::hsla(0., 0., 0., 0.16),
            offset: gpui::point(px(0.), px(4.)),
            blur_radius: px(8.),
            spread_radius: px(0.),
            inset: false,
        }])
        .flex()
        .flex_col()
        .child(header)
        // content__KoJTP: padding 8 16 16 24
        .child(div().flex_1().min_h_0().pt(z(8.)).pr(z(16.)).pb(z(16.)).pl(z(24.)).child(body));
    Some(crate::ui::modal::frame("events-modal", on_close, card))
}
