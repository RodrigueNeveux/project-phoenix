use gpui::{AnyElement, Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, img, prelude::*};

use super::{Modal, card, close_action};
use crate::{
    actions::{close_modal, open_modal, with_modal},
    app::PhoenixApp,
    deeplink::Target,
    model::ImageRef,
    theme::{c, generated::idx, tok, z},
    ui::{
        controls::{BtnKind, button},
        home::data::{InviteKind, InvitePreview, NavRequest},
    },
};

pub struct State {
    pub code: String,
    pub invite: Option<Result<InvitePreview, ()>>,
    pub busy: bool,
    pub error: Option<SharedString>,
}

pub fn open_loading(code: &str) {
    open_modal(Modal::AcceptInvite(Box::new(State { code: code.to_owned(), invite: None, busy: false, error: None })), None);
}

fn is_for(m: &Option<Modal>, code: &str) -> bool {
    matches!(m, Some(Modal::AcceptInvite(s)) if s.code == code)
}

pub fn show(code: &str, invite: Result<InvitePreview, ()>) {
    with_modal(|m| {
        if let Some(Modal::AcceptInvite(s)) = m
            && s.code == code
        {
            s.invite = Some(invite);
        }
    });
}

pub fn close_if(code: &str) {
    if with_modal(|m| is_for(m, code)) {
        close_modal();
    }
}

pub fn member_route(app: &PhoenixApp, p: &InvitePreview) -> Option<Target> {
    let channel = p.channel_id.filter(|c| app.channels.contains_key(c));
    match p.guild {
        Some(g) if app.guilds.iter().any(|x| x.id == g) => Some(Target::Channel { guild: Some(g), channel, message: None }),
        Some(_) => None,
        None => channel.map(|c| Target::Channel { guild: None, channel: Some(c), message: None }),
    }
}

pub fn joined_route(p: &InvitePreview) -> Option<Target> {
    match (p.guild, p.channel_id) {
        (Some(g), ch) => Some(Target::Channel { guild: Some(g), channel: ch, message: None }),
        (None, Some(ch)) => Some(Target::Channel { guild: None, channel: Some(ch), message: None }),
        (None, None) => None,
    }
}

fn accept(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) {
    let taken = with_modal(|m| {
        let Some(Modal::AcceptInvite(s)) = m else { return None };
        let Some(Ok(p)) = s.invite.clone().filter(|_| !s.busy) else { return None };
        s.busy = true;
        s.error = None;
        Some((s.code.clone(), p))
    });
    let Some((code, preview)) = taken else { return };
    if let Some(t) = member_route(app, &preview) {
        close_modal();
        app.route_when_ready(t, cx);
        cx.notify();
        return;
    }
    app.nav_request(NavRequest::AcceptInvite(code.clone()), cx, move |this, res, cx| match res {
        Ok(_) => {
            close_if(&code);
            if let Some(t) = joined_route(&preview) {
                this.route_when_ready(t, cx);
            }
        }
        Err(e) => with_modal(|m| {
            if let Some(Modal::AcceptInvite(s)) = m
                && s.code == code
            {
                s.busy = false;
                s.error = Some(e.0.into());
            }
        }),
    });
    cx.notify();
}

fn text(size: f32, lh: f32, weight: FontWeight, col: usize, s: impl Into<SharedString>) -> gpui::Div {
    div().text_size(z(size)).line_height(z(lh)).font_weight(weight).text_color(c(tok(col))).child(s.into())
}

fn icon(p: &InvitePreview) -> AnyElement {
    let src = match p.kind {
        InviteKind::Guild => p.guild_icon.as_ref(),
        _ => p.channel_icon.as_ref().or(p.inviter_avatar.as_ref()),
    };
    match src {
        Some(ImageRef::Url(u)) => img(gpui::ImageSource::from(u.clone())).size(z(64.)).rounded(z(16.)).flex_none().into_any_element(),
        _ => crate::ui::invite::guild_icon(None, &p.guild_name, 64.),
    }
}

fn pill(dot: usize, label: SharedString) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .child(div().size(z(10.)).mr(z(4.)).rounded_full().bg(c(tok(dot))))
        .child(text(16., 20., FontWeight::NORMAL, idx::TEXT_DEFAULT, label).whitespace_nowrap())
}

fn counts(p: &InvitePreview) -> gpui::Div {
    div()
        .flex()
        .justify_center()
        .when(p.online > 0, |d| {
            d.child(pill(idx::ICON_FEEDBACK_POSITIVE, crate::t!("invite.online", n = p.online)).mr(z(16.)))
        })
        .child(pill(idx::TEXT_DEFAULT, crate::t!("invite.members", n = p.members)))
}

fn name(p: &InvitePreview) -> SharedString {
    match p.kind {
        InviteKind::Guild => p.guild_name.clone(),
        _ => p.channel_name.clone().filter(|n| !n.is_empty()).or_else(|| p.inviter.clone()).unwrap_or_default(),
    }
}

fn lead(p: &InvitePreview) -> SharedString {
    let unnamed_group = p.kind == InviteKind::GroupDm && p.channel_name.as_ref().is_none_or(|n| n.is_empty());
    match &p.inviter {
        _ if unnamed_group => crate::t!("invite.accept.invited_group"),
        Some(i) => crate::t!("invite.invited_you", i = i),
        None => crate::t!("invite.accept.invited_to_join"),
    }
}

fn invalid(cx: &mut Context<PhoenixApp>) -> AnyElement {
    let close = close_action(cx);
    let footer = button("accept-invite-got-it", crate::t!("invite.accept.got_it"), BtnKind::Primary).on_click(move |_, w, cx| close(w, cx));
    card(
        440.,
        crate::t!("invite.accept.invalid"),
        Some(crate::t!("invite.accept.invalid_sub")),
        div().into_any_element(),
        Some(footer.into_any_element()),
        cx,
    )
}

fn root(body: gpui::Div, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let el = div()
        .relative()
        .w(z(440.))
        .rounded(z(12.))
        .bg(c(tok(idx::BACKGROUND_SURFACE_HIGH)))
        .border_1()
        .border_color(c(tok(idx::BORDER_SUBTLE)))
        .shadow(vec![gpui::BoxShadow {
            color: gpui::hsla(0., 0., 0., 0.24),
            offset: gpui::point(z(0.), z(12.)),
            blur_radius: z(24.),
            spread_radius: z(0.),
            inset: false,
        }])
        .child(body);
    crate::ui::modal::frame("accept-invite-backdrop", close_action(cx), el)
}

pub fn render(app: &PhoenixApp, s: &State, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let p = match &s.invite {
        None => return root(div().h(z(200.)).flex().items_center().justify_center().child(crate::ui::spinner::spinner()), cx),
        Some(Err(())) => return invalid(cx),
        Some(Ok(p)) => p,
    };
    let me = app.me.display_name.clone();
    let me = if me.is_empty() { app.me.name.clone() } else { me };
    let header = div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(z(8.))
        .w_full()
        .child(icon(p))
        .child(text(16., 20., FontWeight::NORMAL, idx::TEXT_DEFAULT, lead(p)))
        .child(text(24., 30., FontWeight::SEMIBOLD, idx::TEXT_STRONG, name(p)).flex().items_center().justify_center().text_center())
        .when(p.kind == InviteKind::Guild, |d| d.child(counts(p)))
        .when_some(s.error.clone(), |d, e| d.child(text(14., 18., FontWeight::NORMAL, idx::TEXT_FEEDBACK_CRITICAL, e)));
    let label = if p.kind == InviteKind::Guild { crate::t!("invite.accept.accept_as", name = me) } else { crate::t!("invite.accept.accept") };
    let actions = div()
        .flex()
        .flex_col()
        .items_center()
        .gap(z(8.))
        .w_full()
        .child(
            button("accept-invite-accept", if s.busy { crate::t!("invite.joining") } else { label }, BtnKind::Primary)
                .w_full()
                .on_click(cx.listener(|a, _, _, cx| accept(a, cx))),
        )
        .child(
            div()
                .id("accept-invite-no-thanks")
                .cursor_pointer()
                .px(z(8.))
                .py(z(2.))
                .text_size(z(14.))
                .line_height(z(18.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(c(tok(idx::TEXT_MUTED)))
                .hover(|d| d.text_color(c(tok(idx::TEXT_DEFAULT))).underline())
                .child(crate::t!("invite.accept.no_thanks"))
                .on_click(cx.listener(|_, _, _, cx| {
                    close_modal();
                    cx.notify();
                })),
        );
    root(div().p(z(24.)).flex().flex_col().gap(z(24.)).child(header).child(actions), cx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Id;

    fn preview(guild: Option<u64>, channel: Option<u64>) -> InvitePreview {
        InvitePreview { code: "abc".into(), guild: guild.map(Id), channel_id: channel.map(Id), ..Default::default() }
    }

    #[test]
    fn joining_routes_to_the_invite_channel() {
        assert_eq!(joined_route(&preview(Some(1), Some(2))), Some(Target::Channel { guild: Some(Id(1)), channel: Some(Id(2)), message: None }));
        assert_eq!(joined_route(&preview(Some(1), None)), Some(Target::Channel { guild: Some(Id(1)), channel: None, message: None }));
        assert_eq!(joined_route(&preview(None, Some(5))), Some(Target::Channel { guild: None, channel: Some(Id(5)), message: None }));
        assert_eq!(joined_route(&preview(None, None)), None);
    }

    #[test]
    fn modal_state_follows_its_own_code() {
        open_loading("abc");
        show("other", Err(()));
        assert!(with_modal(|m| matches!(m, Some(Modal::AcceptInvite(s)) if s.invite.is_none())));
        show("abc", Err(()));
        assert!(with_modal(|m| matches!(m, Some(Modal::AcceptInvite(s)) if matches!(s.invite, Some(Err(()))))));
        close_if("other");
        assert!(with_modal(|m| m.is_some()));
        close_if("abc");
        assert!(with_modal(|m| m.is_none()));
    }
}
