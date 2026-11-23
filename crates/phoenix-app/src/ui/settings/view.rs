//! Rendering of the User Settings overlay.

use gpui::{
    AnyElement, Context, Div, ElementId, FontWeight, IntoElement, ParentElement, SharedString, Stateful, Styled,
    Window, div, prelude::*, px,
};

use super::{AppTheme, NAV, Page, SettingsPatch, SettingsState, save, update_local};
use crate::{
    app::PhoenixApp,
    theme::{Palette, c},
    ui::avatar,
};

type Cx<'a> = Context<'a, PhoenixApp>;

fn eid(s: impl Into<String>) -> ElementId {
    ElementId::Name(SharedString::from(s.into()))
}

fn st(app: &mut PhoenixApp) -> Option<&mut SettingsState> {
    app.settings.as_mut()
}

fn h1(t: &'static str) -> Div {
    div()
        .mb(px(20.))
        .text_size(px(20.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(c(Palette::TEXT_HEADER))
        .child(t)
}

fn h5(t: impl Into<SharedString>) -> Div {
    div()
        .mb(px(8.))
        .text_size(px(12.))
        .font_weight(FontWeight::BOLD)
        .text_color(c(Palette::TEXT_MUTED))
        .child(t.into())
}

fn note(t: impl Into<SharedString>) -> Div {
    div().text_size(px(14.)).text_color(c(Palette::TEXT_MUTED)).child(t.into())
}

fn divider() -> Div {
    div().my(px(24.)).h(px(1.)).bg(c(Palette::DIVIDER))
}

fn button(id: impl Into<String>, label: impl Into<SharedString>, bg: u32, hover: u32) -> Stateful<Div> {
    div()
        .id(eid(id))
        .h(px(32.))
        .px(px(16.))
        .flex_none()
        .rounded(px(3.))
        .bg(c(bg))
        .hover(move |s| s.bg(c(hover)))
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(14.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(gpui::white())
        .child(label.into())
}

fn secondary(id: impl Into<String>, label: impl Into<SharedString>) -> Stateful<Div> {
    button(id, label, 0x4E5058, 0x6D6F78)
}

fn toggle_knob(on: bool) -> Div {
    crate::ui::controls::switch(on)
}

/// Discord switch row: title + description on the left, toggle on the right.
fn switch_row(
    id: &str,
    title: &'static str,
    desc: &'static str,
    on: bool,
    f: impl Fn(&mut PhoenixApp, &mut Cx) + 'static,
    cx: &mut Cx,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .mb(px(20.))
        .child(
            div()
                .id(eid(format!("switch-{id}")))
                .flex()
                .items_center()
                .cursor_pointer()
                .on_click(cx.listener(move |a, _, _, cx| f(a, cx)))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(16.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(c(Palette::TEXT_NORMAL))
                        .child(title),
                )
                .child(toggle_knob(on)),
        )
        .when(!desc.is_empty(), |d| d.child(div().mt(px(8.)).child(note(desc))))
        .child(div().mt(px(20.)).h(px(1.)).bg(c(Palette::DIVIDER)))
        .into_any_element()
}

fn radio(id: impl Into<String>, label: impl Into<SharedString>, selected: bool) -> Stateful<Div> {
    div()
        .id(eid(id))
        .mb(px(8.))
        .p(px(10.))
        .rounded(px(4.))
        .bg(c(if selected { Palette::BG_MOD_SELECTED } else { Palette::BG_SECONDARY }))
        .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(12.))
        .child(
            div()
                .size(px(20.))
                .rounded_full()
                .border_2()
                .border_color(c(if selected { 0xFFFFFF } else { Palette::INTERACTIVE }))
                .flex()
                .items_center()
                .justify_center()
                .when(selected, |d| d.child(div().size(px(10.)).rounded_full().bg(gpui::white()))),
        )
        .child(
            div()
                .text_size(px(16.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(c(Palette::TEXT_NORMAL))
                .child(label.into()),
        )
}

/// Discord marker slider bound to a local preference.
fn slider(
    id: &str,
    values: &[f32],
    unit: &'static str,
    current: f32,
    default: f32,
    set: impl Fn(&mut PhoenixApp, f32, &mut Cx) + 'static,
    cx: &mut Cx,
) -> Div {
    use crate::ui::controls;
    let labels = values.iter().map(|v| SharedString::from(format!("{v}{unit}"))).collect();
    let vals: Vec<f32> = values.to_vec();
    let ent = cx.entity();
    controls::marker_slider(
        SharedString::from(format!("slider-{id}")),
        labels,
        controls::nearest(values, current),
        Some(controls::nearest(values, default)),
        std::rc::Rc::new(move |i, _, cx| {
            let v = vals[i];
            ent.update(cx, |a, cx| set(a, v, cx));
        }),
    )
}

fn field_row(label: &'static str, value: SharedString, action: Option<AnyElement>) -> Div {
    div()
        .mb(px(24.))
        .flex()
        .items_center()
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .child(h5(label).mb(px(4.)))
                .child(div().text_size(px(16.)).text_color(c(Palette::TEXT_NORMAL)).child(value)),
        )
        .children(action)
}

fn my_account(app: &PhoenixApp, cx: &mut Cx) -> Div {
    let me = &app.me;
    let reveal = app.settings.as_ref().is_some_and(|s| s.reveal_email);
    let email: SharedString = if reveal { "Not shared with Phoenix".into() } else { "**********@*****.com".into() };
    let card = div()
        .rounded(px(8.))
        .bg(c(Palette::BG_TERTIARY))
        .overflow_hidden()
        .child(div().h(px(100.)).bg(c(Palette::BLURPLE)))
        .child(
            div()
                .relative()
                .h(px(72.))
                .px(px(16.))
                .flex()
                .items_start()
                .child(
                    div()
                        .absolute()
                        .left(px(16.))
                        .top(px(-38.))
                        .p(px(7.))
                        .rounded_full()
                        .bg(c(Palette::BG_TERTIARY))
                        .child(avatar(Some(me), 80.)),
                )
                .child(
                    div()
                        .ml(px(104.))
                        .mt(px(14.))
                        .flex_1()
                        .text_size(px(20.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(Palette::TEXT_HEADER))
                        .child(me.display_name.clone()),
                )
                .child(
                    button("edit-profile", "Edit User Profile", Palette::BLURPLE, 0x4752C4)
                        .mt(px(16.))
                        .on_click(cx.listener(|a, _, _, cx| {
                            if let Some(s) = st(a) {
                                s.page = Page::Profiles;
                            }
                            cx.notify();
                        })),
                ),
        )
        .child(
            div()
                .m(px(16.))
                .mt_0()
                .p(px(16.))
                .rounded(px(8.))
                .bg(c(Palette::BG_SECONDARY))
                .child(field_row("DISPLAY NAME", me.display_name.clone(), Some(secondary("edit-dn", "Edit").into_any_element())))
                .child(field_row("USERNAME", me.name.clone(), Some(secondary("edit-un", "Edit").into_any_element())))
                .child(field_row(
                    "EMAIL",
                    email,
                    Some(
                        div()
                            .id("reveal-email")
                            .mr(px(16.))
                            .text_size(px(14.))
                            .text_color(c(Palette::TEXT_LINK))
                            .cursor_pointer()
                            .on_click(cx.listener(|a, _, _, cx| {
                                if let Some(s) = st(a) {
                                    s.reveal_email = !s.reveal_email;
                                }
                                cx.notify();
                            }))
                            .child(if reveal { "Hide" } else { "Reveal" })
                            .into_any_element(),
                    ),
                ))
                .child(field_row("PHONE NUMBER", "You haven't added a phone number yet.".into(), Some(secondary("edit-ph", "Add").into_any_element()))),
        );
    div()
        .child(h1("My Account"))
        .child(card)
        .child(divider())
        .child(h1("Password and Authentication"))
        .child(secondary("change-pw", "Change Password").w(px(160.)))
        .child(div().mt(px(20.)).child(h5("AUTHENTICATOR APP")))
        .child(note("Protect your account with an extra layer of security. Manage two-factor authentication from the official client."))
        .child(divider())
        .child(h5("ACCOUNT REMOVAL"))
        .child(note("Disabling your account means you can recover it at any time after taking this action."))
}

fn profiles(app: &PhoenixApp) -> Div {
    div()
        .child(h1("Profiles"))
        .child(h5("DISPLAY NAME"))
        .child(
            div()
                .mb(px(20.))
                .h(px(40.))
                .px(px(10.))
                .rounded(px(3.))
                .bg(c(Palette::BG_TERTIARY))
                .flex()
                .items_center()
                .text_color(c(Palette::TEXT_NORMAL))
                .child(app.me.display_name.clone()),
        )
        .child(h5("PREVIEW"))
        .child(
            div()
                .w(px(300.))
                .rounded(px(8.))
                .bg(c(Palette::BG_FLOATING))
                .overflow_hidden()
                .child(div().h(px(60.)).bg(c(Palette::BLURPLE)))
                .child(div().p(px(16.)).flex().gap(px(12.)).items_center().child(avatar(Some(&app.me), 48.)).child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(18.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(c(Palette::TEXT_HEADER))
                                .child(app.me.display_name.clone()),
                        )
                        .child(note(app.me.name.clone())),
                )),
        )
}

fn privacy(app: &PhoenixApp, cx: &mut Cx) -> Div {
    let allow = app.prefs.synced.allow_server_dms;
    div()
        .child(h1("Privacy & Safety"))
        .child(h5("SERVER PRIVACY DEFAULTS"))
        .child(switch_row(
            "server-dms",
            "Allow DMs from other members in servers you join",
            "This setting is applied when you join a new server. It does not apply retroactively to your existing servers.",
            allow,
            move |a, cx| save(a, SettingsPatch::AllowServerDms(!allow), cx),
            cx,
        ))
        .child(h5("HOW WE USE YOUR DATA"))
        .child(note("Phoenix stores your token only in the system keyring and talks to Discord directly."))
}

fn devices() -> Div {
    div()
        .child(h1("Devices"))
        .child(note("Here are all the devices that are currently logged in with your Discord account."))
        .child(div().mt(px(20.)).child(h5("CURRENT DEVICE")))
        .child(
            div()
                .p(px(16.))
                .rounded(px(8.))
                .bg(c(Palette::BG_SECONDARY))
                .child(
                    div()
                        .text_size(px(16.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(Palette::TEXT_HEADER))
                        .child("LINUX · DISCORD PHOENIX"),
                )
                .child(note("This device")),
        )
}

fn connections() -> Div {
    let tile = |name: &'static str| {
        div()
            .size(px(48.))
            .rounded(px(8.))
            .bg(c(Palette::BG_TERTIARY))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(11.))
            .text_color(c(Palette::INTERACTIVE))
            .child(name)
    };
    div()
        .child(h1("Connections"))
        .child(
            div()
                .p(px(16.))
                .rounded(px(8.))
                .bg(c(Palette::BG_SECONDARY))
                .child(h5("ADD ACCOUNTS TO YOUR PROFILE"))
                .child(note("Connecting accounts needs the official client's browser flow."))
                .child(
                    div()
                        .mt(px(16.))
                        .flex()
                        .flex_wrap()
                        .gap(px(8.))
                        .children(["GitHub", "Steam", "Spotify", "Twitch", "YouTube", "Xbox", "Reddit"].map(tile)),
                ),
        )
}

fn notifications(app: &PhoenixApp, cx: &mut Cx) -> Div {
    let l = &app.prefs.local;
    let (desk, sounds) = (l.desktop_notifications, l.notification_sounds);
    div()
        .child(h1("Notifications"))
        .child(switch_row(
            "desktop-notif",
            "Enable Desktop Notifications",
            "Direct messages and mentions show a system notification, following each server's and channel's notification settings and mutes.",
            desk,
            move |a, cx| update_local(a, |p| p.desktop_notifications = !desk, cx),
            cx,
        ))
        .child(switch_row(
            "notif-sounds",
            "Enable Notification Sounds",
            "",
            sounds,
            move |a, cx| update_local(a, |p| p.notification_sounds = !sounds, cx),
            cx,
        ))
}

fn keybinds() -> Div {
    let row = |action: &'static str, keys: &'static str| {
        div()
            .mb(px(8.))
            .p(px(16.))
            .rounded(px(8.))
            .bg(c(Palette::BG_SECONDARY))
            .flex()
            .items_center()
            .child(div().flex_1().text_size(px(16.)).text_color(c(Palette::TEXT_NORMAL)).child(action))
            .child(
                div()
                    .px(px(8.))
                    .py(px(2.))
                    .rounded(px(4.))
                    .bg(c(Palette::BG_TERTIARY))
                    .text_size(px(14.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(c(Palette::INTERACTIVE))
                    .child(keys),
            )
    };
    div()
        .child(h1("Keybinds"))
        .child(h5("DEFAULT KEYBINDS"))
        .child(row("Quick Switcher", "CTRL + K"))
        .child(row("Toggle Member List", "CTRL + U"))
        .child(row("Open User Settings", "CTRL + ,"))
        .child(row("Edit Last Message", "UP"))
        .child(row("Close / Cancel", "ESC"))
}

pub const LOCALES: &[(&str, &str)] = &[
    ("en-US", "English, US"),
    ("en-GB", "English, UK"),
    ("fr", "Français"),
    ("de", "Deutsch"),
    ("es-ES", "Español"),
    ("it", "Italiano"),
    ("pt-BR", "Português do Brasil"),
    ("nl", "Nederlands"),
    ("pl", "Polski"),
    ("ru", "Русский"),
    ("ja", "日本語"),
    ("ko", "한국어"),
    ("zh-CN", "中文"),
];

fn language(app: &PhoenixApp, cx: &mut Cx) -> Div {
    let cur = app.prefs.synced.locale.clone();
    div().child(h1("Language")).child(h5("SELECT A LANGUAGE")).children(LOCALES.iter().map(|(code, name)| {
        let code: SharedString = (*code).into();
        let c2 = code.clone();
        radio(format!("lang-{code}"), *name, cur == code)
            .on_click(cx.listener(move |a, _, _, cx| save(a, SettingsPatch::Locale(c2.clone()), cx)))
    }))
}

fn appearance(app: &PhoenixApp, cx: &mut Cx) -> Div {
    let s = &app.prefs.synced;
    let theme = s.theme;
    let compact = s.compact;
    let l = &app.prefs.local;
    let swatches = div().mb(px(24.)).flex().gap(px(16.)).children(AppTheme::ALL.map(|t| {
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(6.))
            .child(
                div()
                    .id(eid(format!("theme-{}", t.label())))
                    .size(px(60.))
                    .rounded_full()
                    .bg(c(t.swatch()))
                    .border_2()
                    .border_color(c(if t == theme { Palette::BLURPLE } else { Palette::DIVIDER }))
                    .cursor_pointer()
                    .on_click(cx.listener(move |a, _, _, cx| save(a, SettingsPatch::Theme(t), cx))),
            )
            .child(note(t.label()))
    }));
    let (fs, zoom) = (l.font_size, l.zoom);
    div()
        .child(h1("Appearance"))
        .child(h5("THEME"))
        .child(swatches)
        .child(divider())
        .child(h5("MESSAGE DISPLAY"))
        .child(
            radio("display-cozy", "Cozy: Modern, beautiful, and easy on your eyes.", !compact)
                .on_click(cx.listener(|a, _, _, cx| save(a, SettingsPatch::Compact(false), cx))),
        )
        .child(
            radio("display-compact", "Compact: Fit more messages on screen at one time. #IRC", compact)
                .on_click(cx.listener(|a, _, _, cx| save(a, SettingsPatch::Compact(true), cx))),
        )
        .child(divider())
        .child(h5("CHAT FONT SCALING"))
        .child(slider(
            "font",
            &[12., 14., 15., 16., 18., 20., 24.],
            "px",
            fs,
            16.,
            |a, v, cx| update_local(a, |p| p.font_size = v, cx),
            cx,
        ))
        .child(h5("ZOOM LEVEL"))
        .child(slider(
            "zoom",
            &[50., 67., 75., 80., 90., 100., 110., 125., 150., 175., 200.],
            "",
            zoom as f32,
            100.,
            |a, v, cx| update_local(a, |p| p.zoom = v as _, cx),
            cx,
        ))
}

fn accessibility(app: &PhoenixApp, cx: &mut Cx) -> Div {
    let l = &app.prefs.local;
    let (rm, sat) = (l.reduced_motion, l.saturation);
    div()
        .child(h1("Accessibility"))
        .child(h5("SATURATION"))
        .child(slider(
            "sat",
            &[0., 10., 20., 30., 40., 50., 60., 70., 80., 90., 100.],
            "%",
            sat as f32,
            100.,
            |a, v, cx| update_local(a, |p| p.saturation = v as _, cx),
            cx,
        ))
        .child(h5("REDUCED MOTION"))
        .child(switch_row(
            "reduced-motion",
            "Enable Reduced Motion",
            "Reduce the amount of animations and hover effects.",
            rm,
            move |a, cx| update_local(a, |p| p.reduced_motion = !rm, cx),
            cx,
        ))
}

fn advanced(app: &PhoenixApp, cx: &mut Cx) -> Div {
    let dev = app.prefs.synced.developer_mode;
    let my_id = app.me.id.0.to_string();
    div()
        .child(h1("Advanced"))
        .child(switch_row(
            "dev-mode",
            "Developer Mode",
            "Developer Mode exposes context menu items helpful for people writing bots using the Discord API.",
            dev,
            move |a, cx| save(a, SettingsPatch::DeveloperMode(!dev), cx),
            cx,
        ))
        .when(dev, |d| {
            d.child(h5("YOUR USER ID")).child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .child(div().text_size(px(16.)).text_color(c(Palette::TEXT_NORMAL)).child(my_id.clone()))
                    .child(secondary("copy-my-id", "Copy User ID").on_click(cx.listener(move |_, _, _, cx| {
                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(my_id.clone()));
                    }))),
            )
        })
}

fn page(app: &PhoenixApp, cx: &mut Cx) -> Div {
    match app.settings.as_ref().map(|s| s.page).unwrap_or_default() {
        Page::MyAccount => my_account(app, cx),
        Page::Profiles => profiles(app),
        Page::PrivacySafety => privacy(app, cx),
        Page::Devices => devices(),
        Page::Connections => connections(),
        Page::Notifications => notifications(app, cx),
        Page::Keybinds => keybinds(),
        Page::Language => language(app, cx),
        Page::Appearance => appearance(app, cx),
        Page::Accessibility => accessibility(app, cx),
        Page::VoiceVideo => super::voice::render(app, cx),
        Page::Advanced => advanced(app, cx),
    }
}

fn nav_item(id: impl Into<String>, label: &'static str, selected: bool, color: u32) -> Stateful<Div> {
    div()
        .id(eid(id))
        .mb(px(2.))
        .h(px(32.))
        .px(px(10.))
        .rounded(px(4.))
        .flex()
        .items_center()
        .cursor_pointer()
        .text_size(px(16.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(c(if selected { Palette::TEXT_HEADER } else { color }))
        .when(selected, |d| d.bg(c(Palette::BG_MOD_SELECTED)))
        .when(!selected, |d| d.hover(|s| s.bg(c(Palette::BG_MOD_HOVER)).text_color(c(Palette::INTERACTIVE_HOVER))))
        .child(label)
}

fn nav(app: &PhoenixApp, cx: &mut Cx) -> Div {
    let cur = app.settings.as_ref().map(|s| s.page).unwrap_or_default();
    let mut col = div().w(px(218.)).pt(px(60.)).pb(px(20.)).pr(px(6.)).pl(px(20.)).flex().flex_col();
    for (ix, (title, pages)) in NAV.iter().enumerate() {
        if ix > 0 {
            col = col.child(div().mx(px(10.)).my(px(8.)).h(px(1.)).bg(c(Palette::DIVIDER)));
        }
        col = col.child(div().px(px(10.)).pb(px(6.)).child(h5(*title).mb_0()));
        for p in pages.iter().copied() {
            col = col.child(nav_item(format!("nav-{}", p.label()), p.label(), p == cur, Palette::INTERACTIVE).on_click(
                cx.listener(move |a, _, _, cx| {
                    if let Some(s) = st(a) {
                        s.page = p;
                        s.error = None;
                    }
                    cx.notify();
                }),
            ));
        }
    }
    col.child(div().mx(px(10.)).my(px(8.)).h(px(1.)).bg(c(Palette::DIVIDER)))
        .child(nav_item("nav-logout", "Log Out", false, 0xF23F43).on_click(cx.listener(|a, _, _, cx| {
            if let Some(s) = st(a) {
                s.confirm_logout = true;
            }
            cx.notify();
        })))
        .child(div().mx(px(10.)).my(px(8.)).h(px(1.)).bg(c(Palette::DIVIDER)))
        .child(div().px(px(10.)).text_size(px(12.)).text_color(c(Palette::TEXT_MUTED)).child(concat!(
            "Discord Phoenix ",
            env!("CARGO_PKG_VERSION"),
            " (Linux)"
        )))
}

fn close_button(cx: &mut Cx) -> Div {
    div()
        .ml(px(20.))
        .pt(px(60.))
        .flex()
        .flex_col()
        .items_center()
        .gap(px(8.))
        .child(
            div()
                .id("settings-close")
                .size(px(36.))
                .rounded_full()
                .border_2()
                .border_color(c(Palette::INTERACTIVE))
                .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
                .cursor_pointer()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(18.))
                .text_color(c(Palette::INTERACTIVE))
                .on_click(cx.listener(|a, _, _, cx| {
                    a.settings = None;
                    cx.notify();
                }))
                .child("✕"),
        )
        .child(div().text_size(px(13.)).font_weight(FontWeight::SEMIBOLD).text_color(c(Palette::INTERACTIVE)).child("ESC"))
}

fn logout_modal(cx: &mut Cx) -> Div {
    div()
        .absolute()
        .inset_0()
        .bg(gpui::rgba(0x000000B3))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .id("logout-modal")
                .w(px(440.))
                .rounded(px(8.))
                .bg(c(Palette::BG_PRIMARY))
                .overflow_hidden()
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .p(px(16.))
                        .child(
                            div()
                                .mb(px(12.))
                                .text_size(px(20.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(c(Palette::TEXT_HEADER))
                                .child("Log Out"),
                        )
                        .child(div().text_size(px(16.)).text_color(c(Palette::TEXT_NORMAL)).child("Are you sure you want to logout?")),
                )
                .child(
                    div()
                        .p(px(16.))
                        .bg(c(Palette::BG_SECONDARY))
                        .flex()
                        .justify_end()
                        .gap(px(8.))
                        .child(
                            div()
                                .id("logout-cancel")
                                .h(px(38.))
                                .px(px(16.))
                                .flex()
                                .items_center()
                                .cursor_pointer()
                                .text_size(px(14.))
                                .text_color(c(Palette::TEXT_NORMAL))
                                .hover(|s| s.underline())
                                .on_click(cx.listener(|a, _, _, cx| {
                                    if let Some(s) = st(a) {
                                        s.confirm_logout = false;
                                    }
                                    cx.notify();
                                }))
                                .child("Cancel"),
                        )
                        .child(button("logout-confirm", "Log Out", Palette::RED, 0xDA373C).h(px(38.)).on_click(
                            cx.listener(|a, _, _, cx| {
                                a.settings = None;
                                a.logout(cx);
                            }),
                        )),
                ),
        )
}

/// The overlay, when open.
pub fn render(app: &PhoenixApp, _window: &mut Window, cx: &mut Cx) -> Option<AnyElement> {
    let s = app.settings.as_ref()?;
    let error = s.error.clone();
    let confirm = s.confirm_logout;
    Some(
        div()
            .id("user-settings")
            .absolute()
            .inset_0()
            .flex()
            .occlude()
            .bg(c(Palette::BG_PRIMARY))
            .child(
                div()
                    .id("settings-nav")
                    .flex_1()
                    .min_w(px(218.))
                    .bg(c(Palette::BG_SECONDARY))
                    .flex()
                    .justify_end()
                    .overflow_y_scroll()
                    .child(nav(app, cx)),
            )
            .child(
                div()
                    .id("settings-content")
                    .flex_1()
                    .min_w(px(460.))
                    .flex()
                    .overflow_y_scroll()
                    .child(
                        div()
                            .w(px(740.))
                            .min_w(px(460.))
                            .pt(px(60.))
                            .px(px(40.))
                            .pb(px(80.))
                            .flex()
                            .flex_col()
                            .when_some(error, |d, e| {
                                d.child(
                                    div()
                                        .mb(px(16.))
                                        .p(px(10.))
                                        .rounded(px(4.))
                                        .bg(gpui::rgba(0xF23F4333))
                                        .text_color(c(Palette::TEXT_NORMAL))
                                        .text_size(px(14.))
                                        .child(e),
                                )
                            })
                            .child(page(app, cx)),
                    )
                    .child(close_button(cx)),
            )
            .when(confirm, |d| d.child(logout_modal(cx)))
            .into_any_element(),
    )
}
