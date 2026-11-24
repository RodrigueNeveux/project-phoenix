//! "Channels & Roles" page (sidebar row of onboarding guilds; client route
//! `/channels/{g}/customize-community`). Values from the live client (dump
//! vanilla-channels-roles): header container_ybUK7Z (icon 32/20, body-md/medium
//! title); tabBar_xTP3zy margin 32 32 0, gap 24, items padding-bottom 16 with a
//! --text-brand selected tab; scroller padding 32 32 0; pageBody_ padding-right
//! 16; prompt_ cards (1px --border-subtle, radius 8, --background-surface-high,
//! padding 16) with a 2-column promptOptions_ grid (gap 8) of optionButton_
//! (radius 12, padding 12 16, 22px emoji + margin 12, body-md/medium title,
//! body-xs/normal description). Read-only: options are shown, never answered.

use gpui::{AnyElement, App, Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, prelude::*, px};

use crate::{
    app::PhoenixApp,
    commands::{OnboardingOption, OnboardingPrompt, ParityReply, ParityRequest},
    model::Id,
    theme::{c, generated::idx, tok, z},
};

#[derive(Default)]
struct Slot(Option<Page>);
impl gpui::Global for Slot {}

struct Page {
    guild: Id,
    /// Channel selected when the page opened: selecting another one closes it.
    from: Option<Id>,
    prompts: Option<Vec<OnboardingPrompt>>,
    error: Option<SharedString>,
}

fn t(i: usize) -> gpui::Hsla {
    c(tok(i))
}

/// The page shown for the selected guild, if open.
pub fn is_open(app: &PhoenixApp, cx: &App) -> bool {
    cx.try_global::<Slot>().and_then(|s| s.0.as_ref()).is_some_and(|p| Some(p.guild) == app.selected_guild && p.from == app.selected_channel)
}

pub fn open(app: &mut PhoenixApp, guild: Id, cx: &mut Context<PhoenixApp>) {
    cx.set_global(Slot(Some(Page { guild, from: app.selected_channel, prompts: None, error: None })));
    app.parity_request(ParityRequest::Onboarding { guild }, cx, move |a, r, cx| {
        let from = a.selected_channel;
        let (prompts, error) = match r {
            Ok(ParityReply::Onboarding(p)) => (Some(p), None),
            Ok(_) => (Some(vec![]), None),
            Err(e) => (Some(vec![]), Some(SharedString::from(e.0))),
        };
        if cx.try_global::<Slot>().and_then(|s| s.0.as_ref()).is_some_and(|p| p.guild == guild) {
            cx.set_global(Slot(Some(Page { guild, from, prompts, error })));
        }
        cx.notify();
    });
    cx.notify();
}

fn emoji_token(o: &OnboardingOption) -> Option<String> {
    match (o.emoji_id, o.emoji_name.as_deref()) {
        (Some(id), n) => Some(format!("<{}:{}:{}>", if o.animated { "a" } else { "" }, n.unwrap_or("e"), id.0)),
        (None, Some(n)) if !n.is_empty() => Some(n.to_owned()),
        _ => None,
    }
}

fn option(o: &OnboardingOption) -> gpui::Div {
    // optionButtonWrapper_87WSWY: 1px --border-subtle, radius 12, rgba(151,151,159,.08)
    // (--control-secondary-background-default at 8%: --background-mod-subtle)
    div()
        .rounded(z(12.))
        .border_1()
        .border_color(t(idx::BORDER_SUBTLE))
        .bg(t(idx::BACKGROUND_MOD_SUBTLE))
        .hover(|s| s.bg(t(idx::BACKGROUND_MOD_NORMAL)))
        .cursor_pointer()
        // optionButton_87WSWY: padding 12 16
        .py(z(12.))
        .px(z(16.))
        .flex()
        .items_center()
        .when_some(emoji_token(o), |d, tk| d.child(div().flex_none().mr(z(12.)).child(crate::media::emoji::emoji_element(&tk, 22.))))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(z(16.))
                        .line_height(z(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(t(idx::TEXT_STRONG))
                        .child(o.title.clone()),
                )
                .when(!o.description.is_empty(), |d| {
                    d.child(div().mt(z(4.)).text_size(z(12.)).line_height(z(16.)).text_color(t(idx::TEXT_DEFAULT)).child(o.description.clone()))
                }),
        )
}

fn prompt(p: &OnboardingPrompt) -> gpui::Div {
    // promptOptions_pHGfKY: 2 equal columns, gap 8
    let mut grid = div().mt(z(16.)).flex().flex_col().gap(z(8.));
    for pair in p.options.chunks(2) {
        let mut row = div().flex().gap(z(8.));
        for o in pair {
            row = row.child(option(o).flex_1().min_w_0());
        }
        if pair.len() == 1 {
            row = row.child(div().flex_1());
        }
        grid = grid.child(row);
    }
    div()
        .mt(z(16.))
        .p(z(16.))
        .rounded(z(8.))
        .border_1()
        .border_color(t(idx::BORDER_SUBTLE))
        .bg(t(idx::BACKGROUND_SURFACE_HIGH))
        .flex()
        .flex_col()
        .child(div().text_size(z(16.)).line_height(z(20.)).font_weight(FontWeight::SEMIBOLD).text_color(t(idx::TEXT_STRONG)).child(p.title.clone()))
        .child(grid)
        .child(div().mt(z(16.)).text_size(z(12.)).line_height(z(16.)).text_color(t(idx::TEXT_MUTED)).child(if p.single_select {
            crate::t!("community.pick_one")
        } else {
            crate::t!("community.pick_all")
        }))
}

pub fn render(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    if !is_open(app, cx) {
        return None;
    }
    let page = cx.try_global::<Slot>()?.0.as_ref()?;
    let header = div()
        .h(z(48.))
        .flex_none()
        .px(z(16.))
        .flex()
        .items_center()
        .border_b_1()
        .border_color(t(idx::BORDER_SUBTLE))
        .child(div().size(z(32.)).flex().items_center().justify_center().child(
            gpui::svg().path(crate::icons::path("channel-browse")).size(z(20.)).flex_none().text_color(t(idx::ICON_MUTED)),
        ))
        .child(
            div()
                .mr(z(8.))
                .text_size(z(16.))
                .line_height(z(20.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(t(idx::TEXT_STRONG))
                .child(crate::t!("guild.top.channels_roles")),
        );
    let tab = |label: SharedString, on: bool| {
        div()
            .pb(z(16.))
            .mb(px(-1.))
            .text_size(z(16.))
            .line_height(z(14.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(if on { t(idx::TEXT_BRAND) } else { t(idx::TEXT_MUTED) })
            .when(on, |d| d.border_b_2().border_color(t(idx::TEXT_BRAND)))
            .child(label)
    };
    let tabs = div()
        .mt(z(32.))
        .mx(z(32.))
        .flex_none()
        .flex()
        .gap(z(24.))
        .border_b_1()
        .border_color(t(idx::BORDER_SUBTLE))
        .child(tab(crate::t!("community.customize"), true))
        .child(tab(crate::t!("community.browse"), false));
    let mut body = div()
        .pr(z(16.))
        .flex()
        .flex_col()
        .child(div().text_size(z(16.)).line_height(z(20.)).font_weight(FontWeight::SEMIBOLD).text_color(t(idx::TEXT_STRONG)).child(crate::t!("community.questions")))
        .child(div().text_size(z(12.)).line_height(z(16.)).text_color(t(idx::TEXT_MUTED)).child(crate::t!("community.questions_sub")));
    match &page.prompts {
        None => body = body.child(div().mt(z(16.)).text_color(t(idx::TEXT_MUTED)).child(crate::t!("common.loading"))),
        Some(list) => {
            for p in list {
                body = body.child(prompt(p));
            }
            if let Some(er) = page.error.clone() {
                body = body.child(div().mt(z(16.)).text_size(z(14.)).text_color(t(idx::TEXT_FEEDBACK_CRITICAL)).child(er));
            }
        }
    }
    Some(
        div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .bg(t(idx::BACKGROUND_BASE_LOWER))
            .child(header)
            .child(tabs)
            .child(div().id("community-scroll").flex_1().min_h_0().overflow_y_scroll().pt(z(32.)).px(z(32.)).pb(z(32.)).child(body.max_w(z(744.)))) // pageBody_ 727 + padding-right 16 beside the profile column
            .into_any_element(),
    )
}
