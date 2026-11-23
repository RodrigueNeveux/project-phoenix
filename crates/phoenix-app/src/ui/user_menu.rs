//! Right-click menu on a user (member list row, DM row, message author).
//!
//! Item order from the live client (member-menu / author-menu dumps,
//! 2026-11-23, guild member, not self): Profile, Mention, Message, Add Note
//! ("Only visible to you" subtext), Add Friend | Invite to Server >,
//! Remove Friend, Block | voice / moderation items | Copy User ID
//! (developer mode).

use std::{cell::RefCell, rc::Rc};

use gpui::{AnyElement, App, ClickEvent, Context, Pixels, Point, Window};

use crate::{
    app::PhoenixApp,
    model::Id,
    ui::{
        context_menu::{self, MenuItem},
        home::data::{NavRequest, RelAction},
    },
};

type Act = Rc<dyn Fn(&mut PhoenixApp, &ClickEvent, &mut Window, &mut Context<PhoenixApp>)>;

#[derive(Clone, Copy)]
struct Open {
    user: Id,
    guild: Option<Id>,
    pos: Point<Pixels>,
}

thread_local! {
    static OPEN: RefCell<Option<Open>> = const { RefCell::new(None) };
}

/// Open the user menu at `pos` (call from a right-click handler).
pub fn open(user: Id, guild: Option<Id>, pos: Point<Pixels>, window: &mut Window) {
    OPEN.with(|o| *o.borrow_mut() = Some(Open { user, guild, pos }));
    window.refresh();
}

pub fn render(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    let o = OPEN.with(|o| *o.borrow())?;
    let e = cx.entity();
    let me = o.user == app.me.id;
    let act = |label: gpui::SharedString, f: Act| {
        let e = e.clone();
        MenuItem::action(label, None, move |ev, w, cx: &mut App| e.update(cx, |a, cx| f(a, ev, w, cx)))
    };
    let (u, g) = (o.user, o.guild);
    let mut items = vec![
        act(crate::t!("menu.profile"), Rc::new(move |a, ev, w, cx| a.open_profile(u, g, ev.position(), w, cx))),
    ];
    if app.selected_channel.is_some() {
        items.push(act(
            crate::t!("menu.mention"),
            Rc::new(move |a, _, w, cx| {
                a.composer.update(cx, |s, cx| {
                    let v = s.value().to_string();
                    let sep = if v.is_empty() || v.ends_with(' ') { "" } else { " " };
                    s.set_value(format!("{v}{sep}<@{}> ", u.0), w, cx);
                    s.focus(w, cx);
                })
            }),
        ));
    }
    if !me {
        items.push(act(crate::t!("profile.message"), Rc::new(move |a, _, _, cx| a.message_user(u, cx))));
    }
    items.push(
        act(crate::t!("menu.add_note"), Rc::new(move |a, ev, w, cx| a.open_profile(u, g, ev.position(), w, cx)))
            .with_subtext(crate::t!("menu.note_subtext")),
    );
    let rel = app.nav.relationship(u).map(|r| r.kind);
    let friend = rel == Some(crate::ui::home::data::RelationshipKind::Friend);
    if !me && rel.is_none() {
        items.push(act(crate::t!("friends.add"), Rc::new(move |a, _, _, cx| a.relationship_action(u, RelAction::Accept, cx))));
    }
    if !me {
        items.push(MenuItem::Separator);
        let invites: Vec<MenuItem> = app
            .guilds
            .iter()
            .filter(|x| Some(x.id) != g)
            .map(|x| {
                let gid = x.id;
                act_owned(&e, x.name.clone(), Rc::new(move |a, _, _, cx| a.open_invite_people(gid, cx)))
            })
            .collect();
        if !invites.is_empty() {
            items.push(MenuItem::submenu(crate::t!("menu.invite_to_server"), invites));
        }
        if friend {
            items.push(act(crate::t!("menu.remove_friend"), Rc::new(move |a, _, _, cx| a.relationship_action(u, RelAction::Remove, cx))));
        }
        items.push(MenuItem::danger(crate::t!("menu.block"), None, {
            let e = e.clone();
            move |_, _, cx: &mut App| e.update(cx, |a, cx| a.relationship_action(u, RelAction::Block, cx))
        }));
    }
    let in_call = app.voice_channel.is_some_and(|v| app.channels.get(&v).is_some_and(|c| c.voice_users.contains(&u)));
    if in_call {
        items.push(MenuItem::Separator);
        items.extend(crate::ui::voice_panel::user_menu_items(app, u, cx));
    }
    items.extend(crate::ui::inbox::moderation_items(app, u, g, &e));
    if app.prefs.synced.developer_mode {
        items.push(MenuItem::Separator);
        items.push(MenuItem::action(crate::t!("common.copy_user_id"), None, move |_, _, cx: &mut App| {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(u.0.to_string()))
        }));
    }
    let dismiss: context_menu::Dismiss = Rc::new(|w, _| {
        OPEN.with(|o| *o.borrow_mut() = None);
        w.refresh();
    });
    Some(context_menu::render(o.pos, items, dismiss))
}

fn act_owned(e: &gpui::Entity<PhoenixApp>, label: gpui::SharedString, f: Act) -> MenuItem {
    let e = e.clone();
    MenuItem::action(label, None, move |ev, w, cx: &mut App| e.update(cx, |a, cx| f(a, ev, w, cx)))
}

impl PhoenixApp {
    fn relationship_action(&mut self, user: Id, action: RelAction, cx: &mut Context<Self>) {
        self.nav_request(NavRequest::Relationship { user, action }, cx, |_, _, _| {});
        cx.notify();
    }
}
