//! Thread header "More" (…) menu (vanilla-thread-channel: `Plus` 32×32 at the end
//! of the toolbar, after the member list toggle). The client's menu (navId
//! `thread-context`, "Actions du fil") has six groups: join/voice | jump to top,
//! edit, pin, tags, open, summarize | favorite | invitable, archive, lock,
//! delete | mute, notifications | Copy Link, Copy Thread ID. It has no Mark As
//! Read. Only the last group (read-only) is offered here, in the client's order
//! and without a separator between its two items (client string key for Copy
//! Thread ID = `DQ797g`).

use std::cell::RefCell;

use gpui::{AnyElement, App, ClipboardItem, Context, Pixels, Point, Window};

use crate::{
    app::PhoenixApp,
    model::Id,
    ui::{
        context_menu::{self, MenuItem},
    },
};

thread_local! {
    static OPEN: RefCell<Option<(Id, Point<Pixels>)>> = const { RefCell::new(None) };
}

pub fn open(thread: Id, pos: Point<Pixels>, window: &mut Window) {
    OPEN.with(|o| *o.borrow_mut() = Some((thread, pos)));
    window.refresh();
}

fn close(w: &mut Window) {
    OPEN.with(|o| *o.borrow_mut() = None);
    w.refresh();
}

fn copy(label: gpui::SharedString, text: String) -> MenuItem {
    MenuItem::action(label, None, move |_, _, cx: &mut App| cx.write_to_clipboard(ClipboardItem::new_string(text.clone())))
}

pub fn render(app: &PhoenixApp, _cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    let (t, pos) = OPEN.with(|o| *o.borrow())?;
    let guild = app.channels.get(&t).and_then(|c| c.guild).or(app.selected_guild);
    let mut items = Vec::new();
    if let Some(g) = guild {
        items.push(copy(crate::t!("menu.copy_link"), format!("https://discord.com/channels/{}/{}", g.0, t.0)));
    }
    if app.prefs.synced.developer_mode {
        items.push(copy(crate::t!("menu.copy_thread_id"), t.0.to_string()).with_icon("id"));
    }
    if items.is_empty() {
        return None;
    }
    Some(context_menu::render(pos, items, std::rc::Rc::new(|w: &mut Window, _: &mut App| close(w))))
}
