//! Dropdown under the guild header (click the server name).

use gpui::{AnyElement, Context, FontWeight, IntoElement, ParentElement, Styled, div, prelude::*, px};

use crate::{
    app::PhoenixApp,
    model::Id,
    theme::{Palette, c},
    ui::home::data::Overlay,
};

#[derive(Clone, Copy)]
enum Item {
    Invite,
    Settings,
    CreateChannel,
    CreateCategory,
    Notifications,
    Privacy,
    HideMuted,
    Leave,
}

impl Item {
    fn label(self) -> &'static str {
        match self {
            Item::Invite => "Invite People",
            Item::Settings => "Server Settings",
            Item::CreateChannel => "Create Channel",
            Item::CreateCategory => "Create Category",
            Item::Notifications => "Notification Settings",
            Item::Privacy => "Privacy Settings",
            Item::HideMuted => "Hide Muted Channels",
            Item::Leave => "Leave Server",
        }
    }
}

fn run(a: &mut PhoenixApp, guild: Id, item: Item, cx: &mut Context<PhoenixApp>) {
    a.nav.guild_menu = false;
    match item {
        Item::Invite => a.open_invite_people(guild, cx),
        Item::HideMuted => {
            if !a.nav.hide_muted.remove(&guild) {
                a.nav.hide_muted.insert(guild);
            }
            a.rebuild_sidebar();
            a.save_nav_prefs();
        }
        Item::Leave => a.show_overlay(Overlay::LeaveGuild(guild), cx),
        other => a.stub(other.label(), cx),
    }
    cx.notify();
}

pub fn render(app: &PhoenixApp, guild: Id, top: f32, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let hide_muted = app.nav.hide_muted.contains(&guild);
    let sep = || div().mx(px(8.)).my(px(4.)).h(px(1.)).bg(c(Palette::DIVIDER));
    let row = |item: Item, color: u32, hover_bg: u32, cx: &mut Context<PhoenixApp>| {
        div()
            .id(item.label())
            .h(px(32.))
            .px(px(8.))
            .rounded(px(2.))
            .flex()
            .items_center()
            .justify_between()
            .cursor_pointer()
            .text_size(px(14.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(c(color))
            .hover(move |s| s.bg(c(hover_bg)).text_color(gpui::white()))
            .on_click(cx.listener(move |a, _, _, cx| run(a, guild, item, cx)))
            .child(item.label())
    };
    let normal = Palette::INTERACTIVE;
    div()
        .id("guild-menu-backdrop")
        .absolute()
        .inset_0()
        .on_click(cx.listener(|a, _, _, cx| {
            a.nav.guild_menu = false;
            cx.notify();
        }))
        .child(
            div()
                .id("guild-menu")
                .absolute()
                .top(px(top))
                .left(px(10.))
                .w(px(220.))
                .p(px(6.))
                .rounded(px(4.))
                .bg(c(Palette::BG_FLOATING))
                .shadow_lg()
                .flex()
                .flex_col()
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(row(Item::Invite, 0x949CF7, Palette::BLURPLE, cx))
                .child(sep())
                .child(row(Item::Settings, normal, Palette::BLURPLE, cx))
                .child(row(Item::CreateChannel, normal, Palette::BLURPLE, cx))
                .child(row(Item::CreateCategory, normal, Palette::BLURPLE, cx))
                .child(sep())
                .child(row(Item::Notifications, normal, Palette::BLURPLE, cx))
                .child(row(Item::Privacy, normal, Palette::BLURPLE, cx))
                .child(sep())
                .child(row(Item::HideMuted, normal, Palette::BLURPLE, cx).child(
                    div()
                        .size(px(18.))
                        .rounded(px(3.))
                        .border_1()
                        .border_color(c(Palette::INTERACTIVE))
                        .when(hide_muted, |d| d.bg(c(Palette::BLURPLE)).border_color(c(Palette::BLURPLE)))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.))
                        .text_color(gpui::white())
                        .when(hide_muted, |d| d.child("✓")),
                ))
                .child(sep())
                .child(row(Item::Leave, Palette::RED, Palette::RED, cx)),
        )
        .into_any_element()
}
