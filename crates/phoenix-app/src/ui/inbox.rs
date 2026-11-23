//! Header popouts and small dialogs of the parity work:
//! - Inbox (header button): Mentions (with filters), Unreads, Saved.
//! - Threads list of the open channel (header button).
//! - Moderation entries of the user menu (Timeout / Kick / Ban, shown only
//!   with the matching permission) and their dialogs.
//! - "Select Friends" group DM picker (DM list "+").
//! - Guild folder / order sync after a drag in the guild rail.
//!
//! Popouts are 480-600px floating cards under the header (8px radius,
//! tertiary background, 48px title bar), like the official client.

use crate::theme::z;
use gpui::{AnyElement, App, Context, Entity, FontWeight, IntoElement, ParentElement, SharedString, Window, div, prelude::*};
use gpui_component::input::{Input, InputState};

use crate::{
    app::PhoenixApp,
    assets::Icon,
    commands::{
        FolderSync, ModAction, ParityReply, ParityRequest,
        state::{GroupDmPicker, InboxTab, ModDialog, ModKind},
    },
    model::{Id, Message},
    theme::{Palette, c},
    ui::{
        avatar,
        context_menu::MenuItem,
        home::data::RelationshipKind,
        icon, icon_button,
    },
};

pub const KICK_MEMBERS: u64 = 1 << 1;
pub const BAN_MEMBERS: u64 = 1 << 2;
pub const ADMINISTRATOR: u64 = 1 << 3;
pub const MODERATE_MEMBERS: u64 = 1 << 40;

/// Timeout choices of the official dialog (label, seconds).
pub const TIMEOUTS: [(&str, u64); 6] =
    [("60 secs", 60), ("5 mins", 300), ("10 mins", 600), ("1 hour", 3600), ("1 day", 86_400), ("1 week", 604_800)];

/// "Delete Message History" choices of the ban dialog.
pub const BAN_HISTORY: [(&str, u32); 7] = [
    ("Don't Delete Any", 0),
    ("Previous Hour", 3600),
    ("Previous 6 Hours", 21_600),
    ("Previous 12 Hours", 43_200),
    ("Previous 24 Hours", 86_400),
    ("Previous 3 Days", 259_200),
    ("Previous 7 Days", 604_800),
];

pub fn has_perm(perms: u64, bit: u64) -> bool {
    perms & ADMINISTRATOR != 0 || perms & bit != 0
}

// ------------------------------------------------------------ app actions

impl PhoenixApp {
    pub fn toggle_inbox(&mut self, cx: &mut Context<Self>) {
        let i = &mut self.parity.inbox;
        i.open = !i.open;
        self.parity.threads_open = false;
        if self.parity.inbox.open {
            self.load_inbox_tab(true, cx);
        }
        cx.notify();
    }

    pub fn toggle_threads_popout(&mut self, cx: &mut Context<Self>) {
        self.parity.threads_open = !self.parity.threads_open;
        self.parity.inbox.open = false;
        cx.notify();
    }

    fn set_inbox_tab(&mut self, tab: InboxTab, cx: &mut Context<Self>) {
        self.parity.inbox.tab = tab;
        self.parity.inbox.filter_menu = false;
        self.load_inbox_tab(true, cx);
        cx.notify();
    }

    fn load_inbox_tab(&mut self, fresh: bool, cx: &mut Context<Self>) {
        let i = &mut self.parity.inbox;
        if i.loading {
            return;
        }
        let req = match i.tab {
            InboxTab::Mentions => {
                if fresh {
                    i.mentions.clear();
                    i.reached_end = false;
                }
                if i.reached_end {
                    return;
                }
                ParityRequest::Mentions {
                    before: i.mentions.last().map(|m| m.id),
                    guild: if i.this_server { self.selected_guild } else { None },
                }
            }
            InboxTab::Saved => ParityRequest::SavedMessages,
            InboxTab::Unreads => return,
        };
        i.loading = true;
        let tab = i.tab;
        let (everyone, roles) = (i.everyone, i.roles);
        let me = self.me.id;
        self.parity_request(req, cx, move |a, res, cx| {
            let i = &mut a.parity.inbox;
            i.loading = false;
            match res {
                Ok(ParityReply::Messages(msgs)) => match tab {
                    InboxTab::Mentions => {
                        i.reached_end = msgs.len() < 25;
                        // The endpoint flags are applied locally too (cheap, and
                        // the toggles act at once on what is loaded).
                        i.mentions.extend(msgs.into_iter().filter(|m| keep_mention(m, me, everyone, roles)));
                    }
                    _ => i.saved = msgs,
                },
                Ok(_) => {}
                Err(e) => a.parity_toast(format!("Couldn't load the inbox: {}", e.0), cx),
            }
        });
    }

    fn dismiss_mention(&mut self, id: Id, cx: &mut Context<Self>) {
        self.parity.inbox.mentions.retain(|m| m.id != id);
        self.parity_request(ParityRequest::DismissMention(id), cx, |_, _, _| {});
        cx.notify();
    }

    fn unsave(&mut self, channel: Id, id: Id, cx: &mut Context<Self>) {
        self.parity.inbox.saved.retain(|m| m.id != id);
        self.parity_request(ParityRequest::SetSaved { channel, message: id, saved: false }, cx, |_, _, _| {});
        cx.notify();
    }

    /// "Save Message" from the message menu (chat agent can call this).
    pub fn save_message(&mut self, channel: Id, id: Id, cx: &mut Context<Self>) {
        self.parity_request(ParityRequest::SetSaved { channel, message: id, saved: true }, cx, |a, r, cx| {
            if r.is_ok() {
                a.parity_toast("Message saved", cx);
            }
        });
    }

    fn jump_from_inbox(&mut self, channel: Id, id: Id, cx: &mut Context<Self>) {
        self.parity.inbox.open = false;
        if self.selected_channel != Some(channel) {
            self.select_channel(channel, cx);
        }
        self.jump_to_message(channel, id, cx);
    }

    pub fn open_moderation(&mut self, guild: Id, user: Id, kind: ModKind, cx: &mut Context<Self>) {
        self.parity.moderation = Some(ModDialog { guild, user, kind });
        cx.notify();
    }

    fn confirm_moderation(&mut self, cx: &mut Context<Self>) {
        let Some(d) = self.parity.moderation.take() else { return };
        let action = match d.kind {
            ModKind::Timeout(s) => ModAction::Timeout(s),
            ModKind::Kick => ModAction::Kick,
            ModKind::Ban(s) => ModAction::Ban(s),
        };
        let name = self.users.get(&d.user).map(|u| u.display_name.to_string()).unwrap_or_default();
        self.parity_request(ParityRequest::Moderate { guild: d.guild, user: d.user, action }, cx, move |a, r, cx| match r {
            Ok(_) => a.parity_toast(
                match d.kind {
                    ModKind::Timeout(0) => format!("Removed {name}'s timeout"),
                    ModKind::Timeout(_) => format!("{name} was timed out"),
                    ModKind::Kick => format!("{name} was kicked"),
                    ModKind::Ban(_) => format!("{name} was banned"),
                },
                cx,
            ),
            Err(e) => a.parity_toast(format!("Failed: {}", e.0), cx),
        });
        cx.notify();
    }

    pub fn open_group_dm_picker(&mut self, cx: &mut Context<Self>) {
        self.parity.group_dm = Some(GroupDmPicker { search: None, picked: Vec::new(), sending: false });
        cx.notify();
    }

    fn create_group_dm(&mut self, cx: &mut Context<Self>) {
        let Some(p) = self.parity.group_dm.as_mut() else { return };
        if p.picked.is_empty() || p.sending {
            return;
        }
        if p.picked.len() == 1 {
            let u = p.picked[0];
            self.parity.group_dm = None;
            self.message_user(u, cx);
            return;
        }
        p.sending = true;
        let users = p.picked.clone();
        self.parity_request(ParityRequest::CreateGroupDm(users), cx, |a, r, cx| {
            a.parity.group_dm = None;
            match r {
                Ok(ParityReply::Channel(ch)) if a.channels.contains_key(&ch) => a.select_channel(ch, cx),
                // The DM arrives through CHANNEL_CREATE; select it then.
                Ok(ParityReply::Channel(ch)) => {
                    cx.spawn(async move |this, cx| {
                        cx.background_executor().timer(std::time::Duration::from_millis(800)).await;
                        let _ = this.update(cx, |a, cx| a.select_channel(ch, cx));
                    })
                    .detach();
                }
                Ok(_) => {}
                Err(e) => a.parity_toast(format!("Couldn't create the group: {}", e.0), cx),
            }
        });
        cx.notify();
    }

    /// Persist the rail order + folders (call after a drag-reorder).
    pub fn sync_guild_folders(&mut self, cx: &mut Context<Self>) {
        let folders = folder_sync(self);
        self.parity_request(ParityRequest::GuildFolders(folders), cx, |a, r, cx| {
            if let Err(e) = r {
                a.parity_toast(format!("Couldn't save the server order: {}", e.0), cx);
            }
        });
    }
}

fn keep_mention(m: &Message, me: Id, everyone: bool, roles: bool) -> bool {
    let direct = m.content.contains(&format!("<@{}>", me.0)) || m.content.contains(&format!("<@!{}>", me.0));
    if direct {
        return true;
    }
    if m.mention_everyone {
        return everyone;
    }
    if m.content.contains("<@&") {
        return roles;
    }
    true
}

/// Rail order as `guild_folders`: guilds outside folders first (one lone
/// entry each), then the folders; a lone entry that got several guilds by a
/// drag is split again.
pub fn folder_sync(app: &PhoenixApp) -> Vec<FolderSync> {
    let in_folders: std::collections::HashSet<Id> = app.nav.folders.iter().flat_map(|f| f.guilds.iter().copied()).collect();
    let lone = |g: Id| FolderSync { id: None, name: None, color: None, guilds: vec![g] };
    let mut out: Vec<FolderSync> = app.guilds.iter().filter(|g| !in_folders.contains(&g.id)).map(|g| lone(g.id)).collect();
    for f in &app.nav.folders {
        match f.id {
            Some(id) => out.push(FolderSync {
                id: Some(id),
                name: f.name.as_ref().map(|n| n.to_string()),
                color: f.color,
                guilds: f.guilds.clone(),
            }),
            None => out.extend(f.guilds.iter().copied().map(lone)),
        }
    }
    out
}

/// Moderation rows for the user context menu (empty without permissions).
pub fn moderation_items(app: &PhoenixApp, user: Id, guild: Option<Id>, entity: &Entity<PhoenixApp>) -> Vec<MenuItem> {
    let Some(g) = guild else { return Vec::new() };
    if user == app.me.id {
        return Vec::new();
    }
    let perms = app.backend.guild_permissions(g);
    let name = app.users.get(&user).map(|u| u.name.to_string()).unwrap_or_else(|| "user".into());
    let mut items = Vec::new();
    let item = |label: String, kind: ModKind| {
        let e = entity.clone();
        MenuItem::danger(label, None, move |_, _, cx: &mut App| e.update(cx, |a, cx| a.open_moderation(g, user, kind, cx)))
    };
    if has_perm(perms, MODERATE_MEMBERS) {
        items.push(item(format!("Timeout {name}"), ModKind::Timeout(TIMEOUTS[0].1)));
        let e = entity.clone();
        items.push(MenuItem::action("Remove Timeout", None, move |_, _, cx: &mut App| {
            e.update(cx, |a, cx| a.open_moderation(g, user, ModKind::Timeout(0), cx))
        }));
    }
    if has_perm(perms, KICK_MEMBERS) {
        items.push(item(format!("Kick {name}"), ModKind::Kick));
    }
    if has_perm(perms, BAN_MEMBERS) {
        items.push(item(format!("Ban {name}"), ModKind::Ban(0)));
    }
    if !items.is_empty() {
        items.insert(0, MenuItem::Separator);
    }
    items
}

// ------------------------------------------------------------- rendering

fn popout_frame(id: &'static str, title: &'static str, ic: Icon, header_extra: Option<AnyElement>, body: AnyElement) -> AnyElement {
    gpui::deferred(
        div()
            .id(id)
            .absolute()
            .top(z(52.))
            .right(z(16.))
            .w(z(600.))
            .max_h(z(640.))
            .rounded(z(8.))
            .bg(c(Palette::BG_TERTIARY))
            .shadow_lg()
            .occlude()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                div()
                    .px(z(16.))
                    .pt(z(12.))
                    .pb(z(8.))
                    .flex()
                    .flex_col()
                    .gap(z(8.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(z(8.))
                            .child(icon(ic, 24., c(Palette::INTERACTIVE)))
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(z(20.))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(c(Palette::TEXT_HEADER))
                                    .child(title),
                            ),
                    )
                    .children(header_extra),
            )
            .child(body),
    )
    .with_priority(10)
    .into_any_element()
}

fn tab(id: &'static str, label: &'static str, on: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .px(z(8.))
        .py(z(2.))
        .rounded(z(4.))
        .cursor_pointer()
        .text_size(z(14.))
        .font_weight(FontWeight::MEDIUM)
        .when(on, |d| d.bg(c(Palette::BG_MOD_SELECTED)).text_color(c(Palette::TEXT_HEADER)))
        .when(!on, |d| d.text_color(c(Palette::INTERACTIVE)).hover(|s| s.bg(c(Palette::BG_MOD_HOVER))))
        .child(label)
}

fn empty(text: &'static str, sub: &'static str) -> AnyElement {
    div()
        .p(z(32.))
        .flex()
        .flex_col()
        .items_center()
        .gap(z(8.))
        .child(icon(Icon::Inbox, 48., c(Palette::INTERACTIVE_MUTED)))
        .child(div().text_size(z(16.)).font_weight(FontWeight::SEMIBOLD).text_color(c(Palette::TEXT_HEADER)).child(text))
        .child(div().text_size(z(14.)).text_color(c(Palette::TEXT_MUTED)).child(sub))
        .into_any_element()
}

fn channel_label(app: &PhoenixApp, ch: Id) -> (SharedString, Option<SharedString>) {
    let c = app.channels.get(&ch);
    let name = c.map(|c| if c.guild.is_some() { format!("#{}", c.name) } else { c.name.to_string() }).unwrap_or_else(|| "Unknown channel".into());
    let guild = c.and_then(|c| c.guild).and_then(|g| app.guilds.iter().find(|x| x.id == g)).map(|g| g.name.clone());
    (name.into(), guild)
}

fn message_entry(
    app: &PhoenixApp,
    m: &Message,
    cx: &mut Context<PhoenixApp>,
    close: impl Fn(&mut PhoenixApp, &mut Context<PhoenixApp>) + 'static,
) -> AnyElement {
    let (name, guild) = channel_label(app, m.channel);
    let (ch, id) = (m.channel, m.id);
    let u = app.users.get(&m.author);
    div()
        .flex()
        .flex_col()
        .gap(z(4.))
        .child(
            div()
                .flex()
                .items_center()
                .gap(z(6.))
                .child(div().text_size(z(16.)).font_weight(FontWeight::SEMIBOLD).text_color(c(Palette::TEXT_HEADER)).child(name))
                .when_some(guild, |d, g| d.child(div().text_size(z(12.)).text_color(c(Palette::TEXT_MUTED)).child(g)))
                .child(div().flex_1())
                .child(
                    div()
                        .id(("inbox-jump", id.0))
                        .px(z(8.))
                        .py(z(2.))
                        .rounded(z(4.))
                        .bg(c(Palette::BG_MOD_HOVER))
                        .cursor_pointer()
                        .text_size(z(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(c(Palette::TEXT_NORMAL))
                        .hover(|s| s.bg(c(Palette::BG_MOD_SELECTED)))
                        .on_click(cx.listener(move |a, _, _, cx| a.jump_from_inbox(ch, id, cx)))
                        .child("Jump"),
                )
                .child(
                    icon_button("inbox-x", Icon::Close, c(Palette::INTERACTIVE), 16.)
                        .id(("inbox-x", id.0))
                        .on_click(cx.listener(move |a, _, _, cx| close(a, cx))),
                ),
        )
        .child(
            div()
                .p(z(12.))
                .rounded(z(8.))
                .bg(c(Palette::BG_PRIMARY))
                .flex()
                .gap(z(12.))
                .child(avatar(u, 40.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .flex()
                                .items_baseline()
                                .gap(z(8.))
                                .child(
                                    div()
                                        .text_size(z(16.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(crate::theme::role_color(app.author_color(m.author)))
                                        .child(u.map(|u| u.display_name.clone()).unwrap_or_else(|| "Unknown".into())),
                                )
                                .child(
                                    div()
                                        .text_size(z(12.))
                                        .text_color(c(Palette::TEXT_MUTED))
                                        .child(crate::app::format_time_full(m.timestamp)),
                                ),
                        )
                        .child(crate::ui::components::markdown(app, &m.content)),
                ),
        )
        .into_any_element()
}

fn check_row(id: &'static str, label: &'static str, on: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .px(z(8.))
        .py(z(6.))
        .rounded(z(2.))
        .cursor_pointer()
        .hover(|s| s.bg(c(Palette::BLURPLE)))
        .flex()
        .items_center()
        .justify_between()
        .gap(z(12.))
        .text_size(z(14.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(c(Palette::TEXT_NORMAL))
        .child(label)
        .child(
            div()
                .size(z(18.))
                .rounded(z(4.))
                .border_1()
                .border_color(c(Palette::INTERACTIVE))
                .when(on, |d| d.bg(c(Palette::BLURPLE)).border_color(c(Palette::BLURPLE)))
                .flex()
                .items_center()
                .justify_center()
                .text_size(z(12.))
                .text_color(gpui::white())
                .when(on, |d| d.child("✓")),
        )
}

fn inbox(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    let i = &app.parity.inbox;
    if !i.open {
        return None;
    }
    let tabs = div()
        .flex()
        .items_center()
        .gap(z(4.))
        .child(tab("inbox-mentions", "Mentions", i.tab == InboxTab::Mentions).on_click(cx.listener(|a, _, _, cx| a.set_inbox_tab(InboxTab::Mentions, cx))))
        .child(tab("inbox-unreads", "Unreads", i.tab == InboxTab::Unreads).on_click(cx.listener(|a, _, _, cx| a.set_inbox_tab(InboxTab::Unreads, cx))))
        .child(tab("inbox-saved", "Saved", i.tab == InboxTab::Saved).on_click(cx.listener(|a, _, _, cx| a.set_inbox_tab(InboxTab::Saved, cx))))
        .child(div().flex_1())
        .when(i.tab == InboxTab::Mentions, |d| {
            d.child(
                div()
                    .id("inbox-filter")
                    .px(z(8.))
                    .py(z(2.))
                    .rounded(z(4.))
                    .cursor_pointer()
                    .text_size(z(14.))
                    .text_color(c(Palette::INTERACTIVE))
                    .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
                    .on_click(cx.listener(|a, _, _, cx| {
                        a.parity.inbox.filter_menu = !a.parity.inbox.filter_menu;
                        cx.notify();
                    }))
                    .child("Filter ▾"),
            )
        });
    let mut header = div().flex().flex_col().gap(z(4.)).child(tabs);
    if i.filter_menu && i.tab == InboxTab::Mentions {
        let toggle = |f: fn(&mut crate::commands::state::InboxState)| {
            cx.listener(move |a: &mut PhoenixApp, _: &gpui::ClickEvent, _: &mut Window, cx: &mut Context<PhoenixApp>| {
                f(&mut a.parity.inbox);
                a.load_inbox_tab(true, cx);
                cx.notify();
            })
        };
        header = header.child(
            div()
                .p(z(8.))
                .rounded(z(4.))
                .bg(c(Palette::BG_FLOATING))
                .flex()
                .flex_col()
                .child(check_row("f-everyone", "Include @everyone and @here", i.everyone).on_click(toggle(|i| i.everyone = !i.everyone)))
                .child(check_row("f-roles", "Include all role mentions", i.roles).on_click(toggle(|i| i.roles = !i.roles)))
                .child(check_row("f-server", "This server only", i.this_server).on_click(toggle(|i| i.this_server = !i.this_server))),
        );
    }

    let mut list = div().id("inbox-list").flex_1().min_h_0().overflow_y_scroll().px(z(16.)).pb(z(16.)).flex().flex_col().gap(z(16.));
    match i.tab {
        InboxTab::Mentions => {
            if i.mentions.is_empty() && !i.loading {
                list = list.child(empty("You're all caught up!", "Mentions of you will show up here."));
            }
            for m in &i.mentions {
                let id = m.id;
                list = list.child(message_entry(app, m, cx, move |a, cx| a.dismiss_mention(id, cx)));
            }
            if !i.reached_end && !i.mentions.is_empty() && !i.loading {
                list = list.child(
                    div()
                        .id("inbox-more")
                        .py(z(8.))
                        .flex()
                        .justify_center()
                        .cursor_pointer()
                        .text_color(c(Palette::TEXT_LINK))
                        .on_click(cx.listener(|a, _, _, cx| a.load_inbox_tab(false, cx)))
                        .child("Load more"),
                );
            }
        }
        InboxTab::Saved => {
            if i.saved.is_empty() && !i.loading {
                list = list.child(empty("No saved messages", "Save messages to find them here later."));
            }
            for m in &i.saved {
                let (ch, id) = (m.channel, m.id);
                list = list.child(message_entry(app, m, cx, move |a, cx| a.unsave(ch, id, cx)));
            }
        }
        InboxTab::Unreads => {
            let mut unread: Vec<_> = app.channels.values().filter(|c| c.unread && !c.muted && c.guild.is_some()).collect();
            unread.sort_by_key(|c| (std::cmp::Reverse(c.mentions), c.guild, c.position));
            if unread.is_empty() {
                list = list.child(empty("You're all caught up!", "Unread channels will show up here."));
            }
            for ch in unread.into_iter().take(50) {
                let id = ch.id;
                let (name, guild) = channel_label(app, id);
                list = list.child(
                    div()
                        .id(("inbox-unread", id.0))
                        .p(z(12.))
                        .rounded(z(8.))
                        .bg(c(Palette::BG_PRIMARY))
                        .cursor_pointer()
                        .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
                        .flex()
                        .items_center()
                        .gap(z(8.))
                        .on_click(cx.listener(move |a, _, _, cx| {
                            a.parity.inbox.open = false;
                            a.select_channel(id, cx);
                        }))
                        .child(div().text_size(z(16.)).font_weight(FontWeight::SEMIBOLD).text_color(c(Palette::TEXT_HEADER)).child(name))
                        .when_some(guild, |d, g| d.child(div().text_size(z(12.)).text_color(c(Palette::TEXT_MUTED)).child(g)))
                        .child(div().flex_1())
                        .when(ch.mentions > 0, |d| d.child(crate::ui::badge(ch.mentions))),
                );
            }
        }
    }
    if i.loading {
        list = list.child(div().p(z(16.)).text_color(c(Palette::TEXT_MUTED)).child("Loading…"));
    }
    Some(popout_frame("inbox-popout", "Inbox", Icon::Inbox, Some(header.into_any_element()), list.into_any_element()))
}

fn threads(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    if !app.parity.threads_open {
        return None;
    }
    let ch = app.selected_channel?;
    let list_src = app.nav.threads.get(&ch).cloned().unwrap_or_default();
    let mut list = div().id("threads-list").flex_1().min_h_0().overflow_y_scroll().px(z(16.)).pb(z(16.)).flex().flex_col().gap(z(8.));
    if list_src.is_empty() {
        list = list.child(empty("There are no threads.", "Stay focused on a conversation with a thread."));
    } else {
        list = list.child(
            div()
                .text_size(z(12.))
                .font_weight(FontWeight::BOLD)
                .text_color(c(Palette::TEXT_MUTED))
                .child(format!("{} ACTIVE THREAD{}", list_src.len(), if list_src.len() == 1 { "" } else { "S" })),
        );
    }
    for t in list_src {
        let r = crate::model::ThreadRef { id: t.id, name: t.name.clone(), message_count: 0 };
        list = list.child(
            div()
                .id(("thread-row", t.id.0))
                .p(z(12.))
                .rounded(z(8.))
                .bg(c(Palette::BG_SECONDARY))
                .cursor_pointer()
                .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
                .flex()
                .items_center()
                .gap(z(8.))
                .on_click(cx.listener(move |a, _, w, cx| {
                    a.parity.threads_open = false;
                    a.open_thread(r.clone(), w, cx);
                }))
                .child(icon(Icon::Threads, 20., c(Palette::CHANNEL_ICON)))
                .child(div().text_size(z(16.)).font_weight(FontWeight::SEMIBOLD).text_color(c(Palette::TEXT_HEADER)).child(t.name.clone())),
        );
    }
    Some(popout_frame("threads-popout", "Threads", Icon::Threads, None, list.into_any_element()))
}

pub fn popouts(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Vec<AnyElement> {
    let mut out = Vec::new();
    out.extend(inbox(app, cx));
    out.extend(threads(app, cx));
    out
}

/// Build the inputs that need a `Window` (group DM search).
pub fn prepare(app: &mut PhoenixApp, window: &mut Window, cx: &mut Context<PhoenixApp>) {
    if let Some(p) = app.parity.group_dm.as_mut()
        && p.search.is_none()
    {
        let i = cx.new(|cx| InputState::new(window, cx).placeholder("Type the username of a friend"));
        i.update(cx, |s, cx| s.focus(window, cx));
        p.search = Some(i);
    }
}

fn radio_row(id: (&'static str, u64), label: &'static str, on: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .px(z(12.))
        .py(z(10.))
        .rounded(z(4.))
        .cursor_pointer()
        .bg(if on { c(Palette::BG_MOD_SELECTED) } else { c(Palette::BG_SECONDARY) })
        .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
        .flex()
        .items_center()
        .gap(z(10.))
        .child(
            div()
                .size(z(20.))
                .rounded_full()
                .border_2()
                .border_color(if on { c(Palette::BLURPLE) } else { c(Palette::INTERACTIVE) })
                .flex()
                .items_center()
                .justify_center()
                .when(on, |d| d.child(div().size(z(10.)).rounded_full().bg(c(Palette::BLURPLE)))),
        )
        .child(div().text_size(z(16.)).text_color(c(Palette::TEXT_NORMAL)).child(label))
}

fn moderation_dialog(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    let d = app.parity.moderation?;
    let name = app.users.get(&d.user).map(|u| u.name.to_string()).unwrap_or_else(|| "this user".into());
    let guild = app.guilds.iter().find(|g| g.id == d.guild).map(|g| g.name.to_string()).unwrap_or_default();
    let e = cx.entity();
    let set = move |kind: ModKind| {
        let e = e.clone();
        move |_: &gpui::ClickEvent, _: &mut Window, cx: &mut App| {
            e.update(cx, |a, cx| {
                if let Some(m) = a.parity.moderation.as_mut() {
                    m.kind = kind;
                }
                cx.notify();
            })
        }
    };
    let (title, confirm, body): (String, &str, Vec<AnyElement>) = match d.kind {
        ModKind::Timeout(0) => (
            format!("Remove timeout from {name}"),
            "Remove Timeout",
            vec![div().text_color(c(Palette::TEXT_NORMAL)).child(format!("{name} will be able to talk again.")).into_any_element()],
        ),
        ModKind::Timeout(secs) => {
            let mut col = div().flex().flex_col().gap(z(4.)).child(
                div().text_size(z(12.)).font_weight(FontWeight::BOLD).text_color(c(Palette::TEXT_MUTED)).child("DURATION"),
            );
            for (i, (label, s)) in TIMEOUTS.iter().enumerate() {
                col = col.child(radio_row(("timeout-opt", i as u64), label, secs == *s).on_click(set(ModKind::Timeout(*s))));
            }
            (
                format!("Timeout {name}"),
                "Timeout",
                vec![
                    div()
                        .text_color(c(Palette::TEXT_NORMAL))
                        .child("Members who are in timeout are temporarily not allowed to chat or react in text channels, or connect to voice and stage channels.")
                        .into_any_element(),
                    col.into_any_element(),
                ],
            )
        }
        ModKind::Kick => (
            format!("Kick '{name}' from Server"),
            "Kick",
            vec![
                div()
                    .text_color(c(Palette::TEXT_NORMAL))
                    .child(format!("Are you sure you want to kick @{name} from {guild}? They will be able to rejoin again with a new invite."))
                    .into_any_element(),
            ],
        ),
        ModKind::Ban(del) => {
            let mut col = div().flex().flex_col().gap(z(4.)).child(
                div().text_size(z(12.)).font_weight(FontWeight::BOLD).text_color(c(Palette::TEXT_MUTED)).child("DELETE MESSAGE HISTORY"),
            );
            for (i, (label, s)) in BAN_HISTORY.iter().enumerate() {
                col = col.child(radio_row(("ban-opt", i as u64), label, del == *s).on_click(set(ModKind::Ban(*s))));
            }
            (
                format!("Ban '{name}'"),
                "Ban",
                vec![
                    div()
                        .text_color(c(Palette::TEXT_NORMAL))
                        .child(format!("Are you sure you want to ban @{name} from {guild}?"))
                        .into_any_element(),
                    col.into_any_element(),
                ],
            )
        }
    };
    let (e1, e2) = (cx.entity(), cx.entity());
    Some(crate::ui::modal::confirm(crate::ui::modal::Confirm {
        title: title.into(),
        body,
        confirm_label: confirm.into(),
        danger: true,
        on_confirm: std::rc::Rc::new(move |_, cx| e1.update(cx, |a, cx| a.confirm_moderation(cx))),
        on_cancel: std::rc::Rc::new(move |_, cx| {
            e2.update(cx, |a, cx| {
                a.parity.moderation = None;
                cx.notify();
            })
        }),
    }))
}

/// Max other people in a group DM (10 members including us).
pub const GROUP_DM_MAX: usize = 9;

fn group_dm(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    let p = app.parity.group_dm.as_ref()?;
    let q = p.search.as_ref().map(|s| s.read(cx).value().to_lowercase()).unwrap_or_default();
    let friends: Vec<Id> = app
        .nav
        .relationships
        .iter()
        .filter(|r| r.kind == RelationshipKind::Friend)
        .map(|r| r.user)
        .filter(|u| {
            q.is_empty()
                || app.users.get(u).is_some_and(|u| u.name.to_lowercase().contains(&q) || u.display_name.to_lowercase().contains(&q))
        })
        .collect();
    let left = GROUP_DM_MAX.saturating_sub(p.picked.len());
    let mut list = div().id("gdm-list").h(z(240.)).overflow_y_scroll().flex().flex_col();
    if friends.is_empty() {
        list = list.child(div().p(z(16.)).text_color(c(Palette::TEXT_MUTED)).child("No friends found that are not already in this DM."));
    }
    for u in friends {
        let on = p.picked.contains(&u);
        let user = app.users.get(&u);
        list = list.child(
            div()
                .id(("gdm-friend", u.0))
                .px(z(8.))
                .py(z(6.))
                .rounded(z(4.))
                .cursor_pointer()
                .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
                .flex()
                .items_center()
                .gap(z(12.))
                .on_click(cx.listener(move |a, _, _, cx| {
                    if let Some(p) = a.parity.group_dm.as_mut() {
                        if let Some(i) = p.picked.iter().position(|x| *x == u) {
                            p.picked.remove(i);
                        } else if p.picked.len() < GROUP_DM_MAX {
                            p.picked.push(u);
                        }
                    }
                    cx.notify();
                }))
                .child(avatar(user, 32.))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .gap(z(6.))
                        .items_baseline()
                        .child(div().text_size(z(16.)).text_color(c(Palette::TEXT_NORMAL)).child(user.map(|u| u.display_name.clone()).unwrap_or_default()))
                        .child(div().text_size(z(14.)).text_color(c(Palette::TEXT_MUTED)).child(user.map(|u| u.name.clone()).unwrap_or_default())),
                )
                .child(
                    div()
                        .size(z(22.))
                        .rounded(z(4.))
                        .border_1()
                        .border_color(c(Palette::INTERACTIVE_MUTED))
                        .when(on, |d| d.bg(c(Palette::BLURPLE)).border_color(c(Palette::BLURPLE)))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(gpui::white())
                        .when(on, |d| d.child("✓")),
                ),
        );
    }
    let label = if p.picked.len() > 1 { "Create Group DM" } else { "Create DM" };
    let can = !p.picked.is_empty() && !p.sending;
    let e = cx.entity();
    let close: crate::ui::modal::Action = std::rc::Rc::new(move |_, cx| {
        e.update(cx, |a, cx| {
            a.parity.group_dm = None;
            cx.notify();
        })
    });
    let card = div()
        .w(z(440.))
        .rounded(z(8.))
        .overflow_hidden()
        .bg(c(Palette::BG_PRIMARY))
        .shadow_lg()
        .flex()
        .flex_col()
        .child(
            div()
                .p(z(16.))
                .flex()
                .flex_col()
                .gap(z(4.))
                .child(div().text_size(z(20.)).font_weight(FontWeight::SEMIBOLD).text_color(c(Palette::TEXT_HEADER)).child("Select Friends"))
                .child(div().text_size(z(12.)).text_color(c(Palette::TEXT_MUTED)).child(format!("You can add {left} more friend{}.", if left == 1 { "" } else { "s" })))
                .child(
                    div()
                        .mt(z(12.))
                        .h(z(40.))
                        .px(z(8.))
                        .rounded(z(4.))
                        .bg(c(Palette::INPUT_BG))
                        .flex()
                        .items_center()
                        .text_size(z(16.))
                        .when_some(p.search.clone(), |d, s| d.child(Input::new(&s).appearance(false).bordered(false))),
                ),
        )
        .child(div().px(z(8.)).child(list))
        .child(
            div().p(z(16.)).border_t_1().border_color(c(Palette::DIVIDER)).child(
                div()
                    .id("gdm-create")
                    .h(z(38.))
                    .rounded(z(8.))
                    .bg(c(Palette::BLURPLE))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(z(14.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(gpui::white())
                    .when(can, |d| d.cursor_pointer().on_click(cx.listener(|a, _, _, cx| a.create_group_dm(cx))))
                    .when(!can, |d| d.opacity(0.5))
                    .child(label),
            ),
        );
    Some(crate::ui::modal::frame("group-dm-modal", close, card))
}

pub fn dialogs(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Vec<AnyElement> {
    let mut out = Vec::new();
    out.extend(moderation_dialog(app, cx));
    out.extend(group_dm(app, cx));
    out.extend(crate::ui::forum::compose_modal(app, cx));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perms() {
        assert!(has_perm(ADMINISTRATOR, BAN_MEMBERS));
        assert!(has_perm(KICK_MEMBERS, KICK_MEMBERS));
        assert!(!has_perm(KICK_MEMBERS, MODERATE_MEMBERS));
    }

    #[test]
    fn mention_filters() {
        let me = Id(5);
        let m = |content: &str, everyone| Message { content: content.to_owned().into(), mention_everyone: everyone, ..Default::default() };
        assert!(keep_mention(&m("<@5> hi", true), me, false, false));
        assert!(!keep_mention(&m("@everyone", true), me, false, true));
        assert!(!keep_mention(&m("<@&9>", false), me, true, false));
        assert!(keep_mention(&m("<@&9>", false), me, true, true));
    }
}
