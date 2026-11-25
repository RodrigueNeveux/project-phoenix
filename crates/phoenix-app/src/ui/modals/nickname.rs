use gpui::{AnyElement, Context, Entity, IntoElement, ParentElement, SharedString, Styled, Window, div, prelude::*};
use gpui_component::input::{InputEvent, InputState};

use crate::{
    actions::{ActionRequest, close_modal, open_modal, with_modal},
    app::PhoenixApp,
    model::Id,
    theme::{c, generated::idx, tok, z},
    ui::{
        controls::{BtnKind, button},
        modals::{Modal, card, clamp_insert, close_action, field_label, input_box},
    },
};

pub const MAX_LEN: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Member { guild: Id, user: Id },
    Friend { user: Id },
}

pub struct Nickname {
    pub target: Target,
    pub initial: Option<String>,
    pub placeholder: SharedString,
    pub disabled: bool,
    pub input: Option<Entity<InputState>>,
    pub error: Option<SharedString>,
    pub saving: bool,
    _limit: Option<gpui::Subscription>,
}

impl Nickname {
    pub fn new(target: Target, initial: Option<String>, placeholder: SharedString, disabled: bool) -> Self {
        Self { target, initial, placeholder, disabled, input: None, error: None, saving: false, _limit: None }
    }

    pub fn title(&self) -> SharedString {
        match self.target {
            Target::Member { .. } => crate::t!("menu.change_nickname"),
            Target::Friend { .. } => friend_title(self.initial.is_some()),
        }
    }
}

pub fn friend_title(has_nick: bool) -> SharedString {
    if has_nick { crate::t!("menu.friend_nickname_edit") } else { crate::t!("menu.friend_nickname") }
}

pub fn request(target: Target, initial: Option<&str>, value: &str) -> Option<ActionRequest> {
    match target {
        Target::Member { guild, user } => (value != initial.unwrap_or("")).then(|| ActionRequest::SetNick { guild, user: Some(user), nick: value.to_owned() }),
        Target::Friend { user } => Some(ActionRequest::FriendNick { user, nick: value.to_owned() }),
    }
}

pub fn prepare(s: &mut Nickname, window: &mut Window, cx: &mut Context<PhoenixApp>) {
    if s.input.is_some() {
        return;
    }
    let initial: String = s.initial.clone().unwrap_or_default().chars().take(MAX_LEN).collect();
    let input = cx.new(|cx| InputState::new(window, cx).placeholder(s.placeholder.clone()));
    let disabled = s.disabled;
    input.update(cx, |st, cx| {
        if !initial.is_empty() {
            st.set_value(initial.clone(), window, cx);
        }
        if !disabled {
            st.focus(window, cx);
        }
    });
    let last = std::rc::Rc::new(std::cell::RefCell::new(initial));
    s._limit = Some(cx.subscribe_in(&input, window, move |a, st, ev: &InputEvent, window, cx| match ev {
        InputEvent::Change => {
            let now = st.read(cx).value().to_string();
            match clamp_insert(&last.borrow(), &now, MAX_LEN) {
                Some((text, cursor)) => {
                    st.update(cx, |s, cx| {
                        s.set_value(text.clone(), window, cx);
                        s.set_selected_range(cursor..cursor, cx);
                    });
                    *last.borrow_mut() = text;
                }
                None => *last.borrow_mut() = now,
            }
        }
        InputEvent::PressEnter { .. } => a.save_nickname(cx),
        _ => {}
    }));
    s.input = Some(input);
}

fn text_button(id: &'static str, text: SharedString, weight: gpui::FontWeight) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex_none()
        .cursor_pointer()
        .text_size(z(14.))
        .line_height(z(18.))
        .font_weight(weight)
        .text_color(c(tok(idx::TEXT_BRAND)))
        .hover(|s| s.underline())
        .child(text)
}

fn reset(id: &'static str, text: SharedString, weight: gpui::FontWeight, cx: &mut Context<PhoenixApp>) -> AnyElement {
    div()
        .flex()
        .child(text_button(id, text, weight).on_click(cx.listener(|_, _, window, cx| {
            let input = with_modal(|m| match m {
                Some(Modal::Nickname(s)) => s.input.clone(),
                _ => None,
            });
            if let Some(i) = input {
                i.update(cx, |s, cx| {
                    s.set_value("", window, cx);
                    s.focus(window, cx);
                });
            }
            cx.notify();
        })))
        .into_any_element()
}

fn warning() -> AnyElement {
    div()
        .mb(z(20.))
        .p(z(10.))
        .rounded(z(8.))
        .border_1()
        .border_color(c(tok(idx::BACKGROUND_FEEDBACK_MOD_WARNING)))
        .bg(c(tok(idx::BACKGROUND_FEEDBACK_MOD_WARNING)))
        .text_size(z(16.))
        .line_height(z(20.))
        .text_color(c(tok(idx::TEXT_FEEDBACK_WARNING)))
        .child(SharedString::from(crate::t!("nick.warning").replace('\n', " ")))
        .into_any_element()
}

pub fn render(s: &Nickname, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let input = s.input.as_ref().map(|i| input_box(i).when(s.disabled, |d| d.opacity(0.5)));
    let body = match s.target {
        Target::Member { .. } => {
            let label = if s.disabled { crate::t!("nick.disabled_label") } else { crate::t!("nick.label") };
            div()
                .flex()
                .flex_col()
                .when(!s.disabled, |d| d.child(warning()))
                .child(field_label(label, false))
                .children(input)
                .when_some(s.error.clone(), |d, e| {
                    d.child(div().mt(z(8.)).text_size(z(12.)).line_height(z(16.)).text_color(c(tok(idx::TEXT_FEEDBACK_CRITICAL))).child(e))
                })
                .when(s.initial.is_some() && !s.disabled, |d| d.child(div().mt(z(8.)).child(reset("nick-reset", crate::t!("nick.reset"), gpui::FontWeight::NORMAL, cx))))
        }
        Target::Friend { .. } => div()
            .flex()
            .flex_col()
            .gap(z(8.))
            .child(div().flex().flex_col().child(field_label(crate::t!("nick.friend_label"), false)).children(input))
            .child(reset("friend-nick-reset", crate::t!("nick.friend_reset"), gpui::FontWeight::MEDIUM, cx)),
    };
    let subtitle = matches!(s.target, Target::Friend { .. }).then(|| crate::t!("nick.friend_sub"));
    let close = close_action(cx);
    let saving = s.saving;
    let footer = div()
        .w_full()
        .flex()
        .gap(z(8.))
        .child(button("nick-cancel", crate::t!("common.cancel"), BtnKind::Secondary).flex_1().on_click(move |_, w, cx| close(w, cx)))
        .child(
            button("nick-save", crate::t!("common.save"), BtnKind::Primary)
                .flex_1()
                .when(saving, |d| d.opacity(0.5).cursor_default())
                .on_click(cx.listener(|a, _, _, cx| a.save_nickname(cx))),
        );
    card(480., s.title(), subtitle, body.pb(z(16.)).into_any_element(), Some(footer.into_any_element()), cx)
}

fn set_state(f: impl FnOnce(&mut Nickname)) {
    with_modal(|m| {
        if let Some(Modal::Nickname(s)) = m {
            f(s);
        }
    });
}

impl PhoenixApp {
    pub fn open_nickname(&mut self, guild: Id, user: Option<Id>, cx: &mut Context<Self>) {
        let who = user.unwrap_or(self.me.id);
        if who == self.me.id {
            self.open_server_profile(guild, cx);
            return;
        }
        let initial = self.members.get(&guild).and_then(|m| m.iter().find(|m| m.user == who)).and_then(|m| m.nick.as_ref()).map(|n| n.to_string());
        let placeholder = self.users.get(&who).map(|u| u.display_name.clone()).unwrap_or_default();
        let manage = crate::ui::inbox::has_perm(self.backend.guild_permissions(guild), crate::actions::MANAGE_NICKNAMES);
        open_modal(Modal::Nickname(Nickname::new(Target::Member { guild, user: who }, initial, placeholder, !manage)), None);
        cx.notify();
    }

    pub fn open_friend_nickname(&mut self, user: Id, cx: &mut Context<Self>) {
        let initial = self.nav.relationship(user).and_then(|r| r.nickname.clone()).map(|s| s.to_string());
        let placeholder = self.users.get(&user).map(|u| u.display_name.clone()).unwrap_or_default();
        open_modal(Modal::Nickname(Nickname::new(Target::Friend { user }, initial, placeholder, false)), None);
        cx.notify();
    }

    pub fn save_nickname(&mut self, cx: &mut Context<Self>) {
        let Some((target, initial, value, saving, disabled)) = with_modal(|m| match m {
            Some(Modal::Nickname(s)) => Some((s.target, s.initial.clone(), s.input.as_ref().map(|i| i.read(cx).value().to_string()).unwrap_or_default(), s.saving, s.disabled)),
            _ => None,
        }) else {
            return;
        };
        if saving {
            return;
        }
        let Some(req) = request(target, initial.as_deref(), &value).filter(|_| !disabled) else {
            close_modal();
            cx.notify();
            return;
        };
        set_state(|s| {
            s.saving = true;
            s.error = None;
        });
        self.action(req, cx, move |a, r, cx| {
            match r {
                Ok(_) => {
                    if let Target::Friend { user } = target
                        && let Some(r) = a.nav.relationships.iter_mut().find(|r| r.user == user)
                    {
                        r.nickname = (!value.trim().is_empty()).then(|| value.trim().to_owned().into());
                    }
                    if let Target::Member { guild, user } = target
                        && let Some(m) = a.members.get_mut(&guild).and_then(|m| m.iter_mut().find(|m| m.user == user))
                    {
                        m.nick = (!value.trim().is_empty()).then(|| value.trim().to_owned().into());
                    }
                    close_modal();
                }
                Err(e) => match target {
                    Target::Member { .. } => set_state(|s| {
                        s.saving = false;
                        s.error = Some(if e.0.is_empty() { crate::t!("nick.unknown_error") } else { SharedString::from(e.0.clone()) });
                    }),
                    Target::Friend { .. } => set_state(|s| s.saving = false),
                },
            }
            cx.notify();
        });
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn member_save_matches_the_client() {
        let t = Target::Member { guild: Id(1), user: Id(2) };
        assert!(request(t, Some("Bob"), "Bob").is_none());
        assert!(request(t, None, "").is_none());
        assert!(matches!(request(t, Some("Bob"), ""), Some(ActionRequest::SetNick { guild: Id(1), user: Some(Id(2)), ref nick }) if nick.is_empty()));
        assert!(matches!(request(t, None, "Al"), Some(ActionRequest::SetNick { ref nick, .. }) if nick == "Al"));
    }

    #[test]
    fn friend_save_always_patches() {
        let t = Target::Friend { user: Id(3) };
        assert!(matches!(request(t, Some("x"), "x"), Some(ActionRequest::FriendNick { user: Id(3), .. })));
    }
}
