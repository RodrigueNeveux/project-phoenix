//! Account popout, opened from the avatar in the bottom user panel (live
//! "Manage profile and status" popout, account-popout / status-submenu dumps
//! 2026-11-23): banner, avatar, custom-status bubble, name, then two menu
//! groups (Edit Profile, Status >; Switch Accounts, Copy User ID). The status
//! submenu lists Online / Idle / Do Not Disturb / Invisible.
//! Applied through op 3 and `PATCH /users/@me/settings`.

use crate::theme::z;
use gpui::{AnyElement, Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, Window, div, prelude::*, px, svg};
use gpui_component::input::{Input, InputEvent, InputState};

use crate::{
    app::PhoenixApp,
    model::Presence,
    presence::status_label,
    theme::{c, generated::idx, m, tok},
    ui::{avatar, status_dot},
};

impl PhoenixApp {
    pub fn toggle_status_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let open = !self.people.status_picker;
        self.people.close_overlays();
        if open {
            let cur = self.people.custom_status.clone().unwrap_or_default();
            let input = cx.new(|cx| InputState::new(window, cx).placeholder(crate::t!("status.custom_placeholder")).default_value(cur));
            let sub = cx.subscribe_in(&input, window, |this, input, ev: &InputEvent, _, cx| {
                if let InputEvent::PressEnter { .. } = ev {
                    let t = input.read(cx).value().trim().to_owned();
                    this.set_custom_status((!t.is_empty()).then_some(t), cx);
                    this.people.close_overlays();
                }
            });
            self.people.custom_input = Some(input);
            self.people.custom_sub = Some(sub);
            self.people.status_picker = true;
        }
        cx.notify();
    }

    pub fn set_own_status(&mut self, p: Presence, cx: &mut Context<Self>) {
        self.presence = p;
        let custom = self.people.custom_status.as_ref().map(|s| s.to_string());
        cx.background_spawn(self.backend.set_status(p, custom)).detach();
        self.people.close_overlays();
        cx.notify();
    }

    pub fn set_custom_status(&mut self, text: Option<String>, cx: &mut Context<Self>) {
        self.people.custom_status = text.clone().map(Into::into);
        cx.background_spawn(self.backend.set_status(self.presence, text)).detach();
        cx.notify();
    }
}

const ITEMS: [(Presence, Option<&str>); 4] = [
    (Presence::Online, None),
    (Presence::Idle, None),
    (Presence::Dnd, Some("status.dnd_desc")),
    (Presence::Offline, Some("status.invisible_desc")),
];

thread_local! {
    /// Status submenu shown next to the popout.
    static STATUS_SUB: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn set_sub(v: bool, w: &mut Window) {
    if STATUS_SUB.with(|c| c.replace(v)) != v {
        w.refresh();
    }
}

/// menuItemInner_nitlyF row: radius 8, padding 8, gap 8, 16px icon,
/// body-sm/medium 14/16 label, --interactive-text-default; hover
/// --user-profile-overlay-background-hover (--background-mod-subtle here).
fn pop_row(id: &'static str, icon: Option<AnyElement>, label: SharedString, sub: Option<SharedString>, caret: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .rounded(z(8.))
        .p(z(8.))
        .flex()
        .items_center()
        .gap(z(8.))
        .cursor_pointer()
        .text_color(c(tok(idx::INTERACTIVE_TEXT_DEFAULT)))
        .hover(|s| s.bg(c(tok(idx::BACKGROUND_MOD_SUBTLE))).text_color(c(tok(idx::INTERACTIVE_TEXT_HOVER))))
        .child(div().size(z(16.)).flex_none().flex().items_center().justify_center().children(icon))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().text_size(z(14.)).line_height(z(16.)).font_weight(FontWeight::MEDIUM).child(label))
                .children(sub.map(|t| div().text_size(z(12.)).line_height(z(16.)).font_weight(FontWeight::MEDIUM).child(t))),
        )
        .when(caret, |d| d.child(svg().path(crate::icons::path("chevron-right")).size(z(16.)).flex_none()))
}

/// menuItem_:after separator: 1px --user-profile-border, margin 8 4.
fn pop_sep() -> gpui::Div {
    div().h(z(1.)).mx(z(4.)).my(z(8.)).bg(c(tok(idx::BORDER_SUBTLE)))
}

/// overlay_P1YEjQ group: radius 8, padding 8, rgba(151,151,159,.16) wash.
fn pop_group(rows: Vec<AnyElement>) -> gpui::Div {
    div().rounded(z(8.)).p(z(8.)).bg(c(tok(idx::BACKGROUND_MOD_SUBTLE))).flex().flex_col().children(rows)
}

pub fn render(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> AnyElement {
    let surface = c(tok(idx::BACKGROUND_SURFACE_HIGH));
    let me = &app.me;
    // header_: 105px banner, 80px avatar (6px ring) at left 14 / top 59.
    let header = div()
        .relative()
        .h(z(151.))
        .child(div().h(z(105.)).bg(c(tok(idx::BACKGROUND_BRAND))))
        .child(
            div()
                .absolute()
                .left(z(8.))
                .top(z(53.))
                .p(z(6.))
                .rounded_full()
                .bg(surface)
                .child(div().relative().child(avatar(Some(me), 80.)).child(
                    div().absolute().right(z(-2.)).bottom(z(-2.)).child(status_dot(app.presence, 20., surface)),
                )),
        );
    // Custom status bubble (outer_rmCN9_): radius 16, 1px --border-normal,
    // padding 8 12, 14/18 text; doubles as the editor.
    let bubble = app.people.custom_input.as_ref().map(|input| {
        let has = app.people.custom_status.is_some();
        div()
            .absolute()
            .left(z(109.))
            .top(z(98.))
            .right(z(14.))
            .px(z(12.))
            .py(z(4.))
            .rounded(z(16.))
            .border_1()
            .border_color(c(tok(idx::BORDER_NORMAL)))
            .bg(surface)
            .shadow_sm()
            .flex()
            .items_center()
            .gap(z(4.))
            .text_size(z(14.))
            .child(div().flex_1().min_w_0().child(Input::new(input).appearance(false).bordered(false)))
            .when(has, |d| {
                d.child(
                    div()
                        .id("custom-clear")
                        .size(z(20.))
                        .rounded_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .hover(|s| s.bg(c(tok(idx::BACKGROUND_MOD_SUBTLE))))
                        .on_click(cx.listener(|a, _, _, cx| {
                            a.set_custom_status(None, cx);
                            a.people.close_overlays();
                        }))
                        .child(svg().path(crate::icons::path("close")).size(z(14.)).text_color(c(tok(idx::INTERACTIVE_TEXT_DEFAULT)))),
                )
            })
    });
    // userInfo_: display name heading 20/600/24, username 14/18.
    let info = div()
        .flex()
        .flex_col()
        .child(
            div()
                .text_size(z(20.))
                .line_height(z(24.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c(tok(idx::TEXT_STRONG)))
                .child(me.display_name.clone()),
        )
        .child(div().h(z(24.)).flex().items_center().text_size(z(14.)).line_height(z(18.)).text_color(c(tok(idx::TEXT_DEFAULT))).child(me.name.clone()));

    let e = cx.entity();
    let status_row = pop_row(
        "acct-status",
        Some(status_dot(app.presence, 12., surface).into_any_element()),
        status_label(app.presence),
        None,
        true,
    )
    .on_hover(|h, w, _| {
        if *h {
            set_sub(true, w)
        }
    })
    .on_click(|_, w, _| set_sub(!STATUS_SUB.with(|c| c.get()), w));
    let edit = pop_row(
        "acct-edit",
        Some(svg().path(crate::icons::path("pen")).size(z(16.)).into_any_element()),
        crate::t!("status.edit_profile"),
        None,
        false,
    )
    .on_hover(|h, w, _| {
        if *h {
            set_sub(false, w)
        }
    })
    .on_click(cx.listener(|a, _, _, cx| {
        a.people.close_overlays();
        crate::ui::settings::open(a, cx);
        if let Some(s) = a.settings.as_mut() {
            s.page = crate::ui::settings::Page::Profiles;
        }
    }));
    let my_id = me.id.0.to_string();
    let copy = pop_row(
        "acct-copy-id",
        Some(svg().path(crate::icons::path("id")).size(z(16.)).into_any_element()),
        crate::t!("common.copy_user_id"),
        None,
        false,
    )
    .on_hover(|h, w, _| {
        if *h {
            set_sub(false, w)
        }
    })
    .on_click(cx.listener(move |a, _, _, cx| {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(my_id.clone()));
        a.people.close_overlays();
        cx.notify();
    }));
    let menus = div()
        .flex()
        .flex_col()
        .gap(z(12.))
        .child(pop_group(vec![edit.into_any_element(), pop_sep().into_any_element(), status_row.into_any_element()]))
        .child(pop_group(vec![copy.into_any_element()]));
    // body_: padding 4 8 16 16, gap 12
    let body = div().pt(z(4.)).pr(z(8.)).pb(z(16.)).pl(z(16.)).flex().flex_col().gap(z(12.)).child(info).child(menus);

    // outer_P1YEjQ: 300 + 2px padding, radius 8, 1px white/8 ring + --shadow-high.
    let card = div()
        .id("status-picker")
        .w(z(304.))
        .p(z(2.))
        .rounded(z(8.))
        .bg(surface)
        .border_1()
        .border_color(c(tok(idx::BORDER_SUBTLE)))
        .shadow_lg()
        .occlude()
        .child(
            div()
                .rounded(z(6.))
                .overflow_hidden()
                .pb(z(4.))
                .flex()
                .flex_col()
                .gap(z(8.))
                .child(header.children(bubble))
                .child(body),
        );

    // Status submenu (menu_M3qk4j, 300 wide) to the right of the popout.
    let sub_open = STATUS_SUB.with(|c| c.get());
    let submenu = sub_open.then(|| {
        let mut items = Vec::new();
        for (i, (p, desc)) in ITEMS.into_iter().enumerate() {
            let e = e.clone();
            if i == 1 {
                items.push(crate::ui::context_menu::MenuItem::Separator);
            }
            let mut it = crate::ui::context_menu::MenuItem::action(status_label(p), None, move |_, _, cx| {
                e.update(cx, |a, cx| a.set_own_status(p, cx))
            });
            if let Some(d) = desc {
                it = it.with_subtext(crate::t!(d));
            }
            items.push(it);
        }
        let dismiss: crate::ui::context_menu::Dismiss = std::rc::Rc::new(|w, _| set_sub(false, w));
        crate::ui::context_menu::panel("status-sub", items, dismiss).w(z(300.))
    });
    div()
        .absolute()
        .left(px(m::SERVER_BAR_W - 54.))
        .bottom(px(m::USER_PANEL_H + 4.))
        .flex()
        .items_end()
        .gap(z(4.))
        .child(card)
        .children(submenu)
        .into_any_element()
}
