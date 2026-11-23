//! Right-click menu on a user (member list row, DM row, message author).
//!
//! Item order observed in the official client (guild member, not self):
//! Profile, Message, (Call in DMs) | Add Note | Invite to Server >,
//! Add Friend | Ignore, Block | Copy User ID (developer mode).

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
    let act = |label: &'static str, f: Act| {
        let e = e.clone();
        MenuItem::action(label, None, move |ev, w, cx: &mut App| e.update(cx, |a, cx| f(a, ev, w, cx)))
    };
    let (u, g) = (o.user, o.guild);
    let mut items = vec![
        act("Profile", Rc::new(move |a, ev, w, cx| a.open_profile(u, g, ev.position(), w, cx))),
    ];
    if app.selected_channel.is_some() {
        items.push(act(
            "Mention",
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
        items.push(act("Message", Rc::new(move |a, _, _, cx| a.message_user(u, cx))));
    }
    items.push(MenuItem::Separator);
    items.push(act("Add Note", Rc::new(move |a, ev, w, cx| a.open_profile(u, g, ev.position(), w, cx))));
    let in_call = app.voice_channel.is_some_and(|v| app.channels.get(&v).is_some_and(|c| c.voice_users.contains(&u)));
    if in_call {
        items.push(MenuItem::Separator);
        items.extend(crate::ui::voice_panel::user_menu_items(app, u, cx));
    }
    if !me {
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
            items.push(MenuItem::Separator);
            items.push(MenuItem::submenu("Invite to Server", invites));
        }
        items.push(MenuItem::Separator);
        if app.nav.relationship(u).is_none() {
            items.push(act("Add Friend", Rc::new(move |a, _, _, cx| a.relationship_action(u, RelAction::Accept, cx))));
            items.push(MenuItem::Separator);
        }
        items.push(MenuItem::danger("Block", None, {
            let e = e.clone();
            move |_, _, cx: &mut App| e.update(cx, |a, cx| a.relationship_action(u, RelAction::Block, cx))
        }));
    }
    if app.prefs.synced.developer_mode {
        items.push(MenuItem::Separator);
        items.push(MenuItem::action("Copy User ID", None, move |_, _, cx: &mut App| {
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
