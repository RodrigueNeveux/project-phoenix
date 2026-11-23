//! Rendering of the User Settings overlay.

use crate::theme::z;
use gpui::{
    AnyElement, Context, Div, ElementId, FontWeight, IntoElement, ParentElement, SharedString, Stateful, Styled,
    Window, div, prelude::*,
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

fn h1(t: impl Into<SharedString>) -> Div {
    div()
        .mb(z(20.))
        .text_size(z(20.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(c(Palette::TEXT_HEADER))
        .child(t.into())
}

fn h5(t: impl Into<SharedString>) -> Div {
    div()
        .mb(z(8.))
        .text_size(z(12.))
        .font_weight(FontWeight::BOLD)
        .text_color(c(Palette::TEXT_MUTED))
        .child(t.into())
}

fn note(t: impl Into<SharedString>) -> Div {
    div().text_size(z(14.)).text_color(c(Palette::TEXT_MUTED)).child(t.into())
}

fn divider() -> Div {
    div().my(z(24.)).h(z(1.)).bg(c(Palette::DIVIDER))
}

fn button(id: impl Into<String>, label: impl Into<SharedString>, bg: u32, hover: u32) -> Stateful<Div> {
    div()
        .id(eid(id))
        .h(z(32.))
        .px(z(16.))
        .flex_none()
        .rounded(z(3.))
        .bg(c(bg))
        .hover(move |s| s.bg(c(hover)))
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .text_size(z(14.))
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
    title: SharedString,
    desc: SharedString,
    on: bool,
    f: impl Fn(&mut PhoenixApp, &mut Cx) + 'static,
    cx: &mut Cx,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .mb(z(20.))
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
                        .text_size(z(16.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(c(Palette::TEXT_NORMAL))
                        .child(title),
                )
                .child(toggle_knob(on)),
        )
        .when(!desc.is_empty(), |d| d.child(div().mt(z(8.)).child(note(desc))))
        .child(div().mt(z(20.)).h(z(1.)).bg(c(Palette::DIVIDER)))
        .into_any_element()
}

fn radio(id: impl Into<String>, label: impl Into<SharedString>, selected: bool) -> Stateful<Div> {
    div()
        .id(eid(id))
        .mb(z(8.))
        .p(z(10.))
        .rounded(z(4.))
        .bg(c(if selected { Palette::BG_MOD_SELECTED } else { Palette::BG_SECONDARY }))
        .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(z(12.))
        .child(
            div()
                .size(z(20.))
                .rounded_full()
                .border_2()
                .border_color(c(if selected { Palette::BLURPLE } else { Palette::INTERACTIVE }))
                .flex()
                .items_center()
                .justify_center()
                .when(selected, |d| d.child(div().size(z(10.)).rounded_full().bg(c(Palette::BLURPLE)))),
        )
        .child(
            div()
                .text_size(z(16.))
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

fn field_row(label: impl Into<SharedString>, value: SharedString, action: Option<AnyElement>) -> Div {
    div()
        .mb(z(24.))
        .flex()
        .items_center()
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .child(h5(label.into()).mb(z(4.)))
                .child(div().text_size(z(16.)).text_color(c(Palette::TEXT_NORMAL)).child(value)),
        )
        .children(action)
}

fn my_account(app: &PhoenixApp, cx: &mut Cx) -> Div {
    let me = &app.me;
    let reveal = app.settings.as_ref().is_some_and(|s| s.reveal_email);
    let email: SharedString = if reveal { crate::t!("settings.account.email_hidden") } else { "**********@*****.com".into() };
    let card = div()
        .rounded(z(8.))
        .bg(c(Palette::BG_TERTIARY))
        .overflow_hidden()
        .child(div().h(z(100.)).bg(c(Palette::BLURPLE)))
        .child(
            div()
                .relative()
                .h(z(72.))
                .px(z(16.))
                .flex()
                .items_start()
                .child(
                    div()
                        .absolute()
                        .left(z(16.))
                        .top(z(-38.))
                        .p(z(7.))
                        .rounded_full()
                        .bg(c(Palette::BG_TERTIARY))
                        .child(avatar(Some(me), 80.)),
                )
                .child(
                    div()
                        .ml(z(104.))
                        .mt(z(14.))
                        .flex_1()
                        .text_size(z(20.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(Palette::TEXT_HEADER))
                        .child(me.display_name.clone()),
                )
                .child(
                    button("edit-profile", crate::t!("settings.account.edit_profile"), Palette::BLURPLE, 0x4752C4)
                        .mt(z(16.))
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
                .m(z(16.))
                .mt_0()
                .p(z(16.))
                .rounded(z(8.))
                .bg(c(Palette::BG_SECONDARY))
                .child(field_row(crate::t!("settings.account.display_name"), me.display_name.clone(), Some(secondary("edit-dn", crate::t!("common.edit")).into_any_element())))
                .child(field_row(crate::t!("settings.account.username"), me.name.clone(), Some(secondary("edit-un", crate::t!("common.edit")).into_any_element())))
                .child(field_row(
                    crate::t!("settings.account.email"),
                    email,
                    Some(
                        div()
                            .id("reveal-email")
                            .mr(z(16.))
                            .text_size(z(14.))
                            .text_color(c(Palette::TEXT_LINK))
                            .cursor_pointer()
                            .on_click(cx.listener(|a, _, _, cx| {
                                if let Some(s) = st(a) {
                                    s.reveal_email = !s.reveal_email;
                                }
                                cx.notify();
                            }))
                            .child(if reveal { crate::t!("common.hide") } else { crate::t!("common.reveal") })
                            .into_any_element(),
                    ),
                ))
                .child(field_row(crate::t!("settings.account.phone"), crate::t!("settings.account.no_phone"), Some(secondary("edit-ph", crate::t!("common.add")).into_any_element()))),
        );
    div()
        .child(h1(crate::t!("settings.page.my_account")))
        .child(card)
        .child(divider())
        .child(h1(crate::t!("settings.account.password_auth")))
        .child(secondary("change-pw", crate::t!("settings.account.change_password")).w(z(160.)))
        .child(div().mt(z(20.)).child(h5(crate::t!("settings.account.authenticator"))))
        .child(note(crate::t!("settings.account.authenticator_note")))
        .child(divider())
        .child(h5(crate::t!("settings.account.removal")))
        .child(note(crate::t!("settings.account.removal_note")))
}

fn profiles(app: &PhoenixApp) -> Div {
    div()
        .child(h1(crate::t!("settings.page.profiles")))
        .child(h5(crate::t!("settings.account.display_name")))
        .child(
            div()
                .mb(z(20.))
                .h(z(40.))
                .px(z(10.))
                .rounded(z(3.))
                .bg(c(Palette::BG_TERTIARY))
                .flex()
                .items_center()
                .text_color(c(Palette::TEXT_NORMAL))
                .child(app.me.display_name.clone()),
        )
        .child(h5(crate::t!("settings.profiles.preview")))
        .child(
            div()
                .w(z(300.))
                .rounded(z(8.))
                .bg(c(Palette::BG_FLOATING))
                .overflow_hidden()
                .child(div().h(z(60.)).bg(c(Palette::BLURPLE)))
                .child(div().p(z(16.)).flex().gap(z(12.)).items_center().child(avatar(Some(&app.me), 48.)).child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(z(18.))
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
        .child(h1(crate::t!("settings.page.privacy")))
        .child(h5(crate::t!("settings.privacy.server_defaults")))
        .child(switch_row(
            "server-dms",
            crate::t!("settings.privacy.allow_dms"),
            crate::t!("settings.privacy.allow_dms_note"),
            allow,
            move |a, cx| save(a, SettingsPatch::AllowServerDms(!allow), cx),
            cx,
        ))
        .child(h5(crate::t!("settings.privacy.data")))
        .child(note(crate::t!("settings.privacy.data_note")))
}

fn devices() -> Div {
    div()
        .child(h1(crate::t!("settings.page.devices")))
        .child(note(crate::t!("settings.devices.note")))
        .child(div().mt(z(20.)).child(h5(crate::t!("settings.devices.current"))))
        .child(
            div()
                .p(z(16.))
                .rounded(z(8.))
                .bg(c(Palette::BG_SECONDARY))
                .child(
                    div()
                        .text_size(z(16.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(Palette::TEXT_HEADER))
                        .child("LINUX · DISCORD PHOENIX"),
                )
                .child(note(crate::t!("settings.devices.this"))),
        )
}

fn connections() -> Div {
    let tile = |name: &'static str| {
        div()
            .size(z(48.))
            .rounded(z(8.))
            .bg(c(Palette::BG_TERTIARY))
            .flex()
            .items_center()
            .justify_center()
            .text_size(z(11.))
            .text_color(c(Palette::INTERACTIVE))
            .child(name)
    };
    div()
        .child(h1(crate::t!("settings.page.connections")))
        .child(
            div()
                .p(z(16.))
                .rounded(z(8.))
                .bg(c(Palette::BG_SECONDARY))
                .child(h5(crate::t!("settings.connections.add")))
                .child(note(crate::t!("settings.connections.note")))
                .child(
                    div()
                        .mt(z(16.))
                        .flex()
                        .flex_wrap()
                        .gap(z(8.))
                        .children(["GitHub", "Steam", "Spotify", "Twitch", "YouTube", "Xbox", "Reddit"].map(tile)),
                ),
        )
}

fn notifications(app: &PhoenixApp, cx: &mut Cx) -> Div {
    let l = &app.prefs.local;
    let (desk, sounds) = (l.desktop_notifications, l.notification_sounds);
    div()
        .child(h1(crate::t!("settings.page.notifications")))
        .child(switch_row(
            "desktop-notif",
            crate::t!("settings.notif.desktop"),
            crate::t!("settings.notif.desktop_note"),
            desk,
            move |a, cx| update_local(a, |p| p.desktop_notifications = !desk, cx),
            cx,
        ))
        .child(switch_row(
            "notif-sounds",
            crate::t!("settings.notif.sounds"),
            "".into(),
            sounds,
            move |a, cx| update_local(a, |p| p.notification_sounds = !sounds, cx),
            cx,
        ))
}

fn keybinds() -> Div {
    let row = |action: SharedString, keys: &'static str| {
        div()
            .mb(z(8.))
            .p(z(16.))
            .rounded(z(8.))
            .bg(c(Palette::BG_SECONDARY))
            .flex()
            .items_center()
            .child(div().flex_1().text_size(z(16.)).text_color(c(Palette::TEXT_NORMAL)).child(action))
            .child(
                div()
                    .px(z(8.))
                    .py(z(2.))
                    .rounded(z(4.))
                    .bg(c(Palette::BG_TERTIARY))
                    .text_size(z(14.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(c(Palette::INTERACTIVE))
                    .child(keys),
            )
    };
    div()
        .child(h1(crate::t!("settings.page.keybinds")))
        .child(h5(crate::t!("settings.keybinds.default")))
        .child(row(crate::t!("settings.keybinds.quick_switcher"), "CTRL + K"))
        .child(row(crate::t!("settings.keybinds.member_list"), "CTRL + U"))
        .child(row(crate::t!("settings.keybinds.user_settings"), "CTRL + ,"))
        .child(row(crate::t!("settings.keybinds.edit_last"), "UP"))
        .child(row(crate::t!("settings.keybinds.close"), "ESC"))
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
    div().child(h1(crate::t!("settings.page.language"))).child(h5(crate::t!("settings.language.select"))).children(LOCALES.iter().map(|(code, name)| {
        let code: SharedString = (*code).into();
        let c2 = code.clone();
        radio(format!("lang-{code}"), *name, cur == code)
            .on_click(cx.listener(move |a, _, _, cx| save(a, SettingsPatch::Locale(c2.clone()), cx)))
    }))
}

/// A theme choice drawn as a miniature chat window in that theme's own colours
/// (rail, channel list, two messages and a composer), with its name below.
fn theme_card(t: AppTheme, selected: bool) -> Stateful<Div> {
    let tc = |tok: u32| crate::theme::c_for(tok, t);
    let line = |w: f32, tok: u32| div().h(z(6.)).w(z(w)).rounded(z(3.)).bg(tc(tok));
    let msg = |w1: f32, w2: f32, av: u32| {
        div()
            .flex()
            .gap(z(6.))
            .items_center()
            .child(div().size(z(14.)).flex_none().rounded_full().bg(c(av)))
            .child(div().flex().flex_col().gap(z(3.)).child(line(w1, Palette::TEXT_HEADER)).child(line(w2, Palette::TEXT_MUTED)))
    };
    let preview = div()
        .w(z(128.))
        .h(z(84.))
        .rounded(z(8.))
        .overflow_hidden()
        .flex()
        .bg(tc(Palette::BG_TERTIARY))
        .border_2()
        .border_color(c(if selected { Palette::BLURPLE } else { Palette::DIVIDER }))
        .child(
            div()
                .w(z(18.))
                .flex_none()
                .pt(z(6.))
                .flex()
                .flex_col()
                .items_center()
                .gap(z(4.))
                .children((0..3).map(|i| div().size(z(10.)).rounded(z(3.)).bg(tc(if i == 0 { Palette::BLURPLE } else { Palette::BG_SECONDARY })))),
        )
        .child(
            div()
                .w(z(32.))
                .flex_none()
                .bg(tc(Palette::BG_SECONDARY))
                .pt(z(8.))
                .px(z(4.))
                .flex()
                .flex_col()
                .gap(z(5.))
                .children([20., 16., 22.].map(|w| line(w, Palette::CHANNEL_ICON))),
        )
        .child(
            div()
                .flex_1()
                .bg(tc(Palette::BG_PRIMARY))
                .p(z(8.))
                .flex()
                .flex_col()
                .gap(z(8.))
                .child(msg(30., 42., 0x5865F2))
                .child(msg(24., 36., 0x3BA55C))
                .child(div().mt_auto().h(z(12.)).rounded(z(4.)).bg(tc(Palette::INPUT_BG))),
        );
    div()
        .id(eid(format!("theme-{}", t.label())))
        .flex()
        .flex_col()
        .gap(z(6.))
        .cursor_pointer()
        .child(
            div()
                .relative()
                .rounded(z(10.))
                .when(!selected, |d| d.hover(|s| s.opacity(0.85)))
                .child(preview)
                .when(selected, |d| {
                    d.child(
                        div()
                            .absolute()
                            .top(z(-6.))
                            .right(z(-6.))
                            .size(z(22.))
                            .rounded_full()
                            .bg(c(Palette::BLURPLE))
                            .border_2()
                            .border_color(c(Palette::BG_PRIMARY))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(gpui::svg().path("icons/check.svg").size(z(14.)).text_color(gpui::white())),
                    )
                }),
        )
        .child(
            div()
                .text_size(z(14.))
                .font_weight(if selected { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                .text_color(c(if selected { Palette::TEXT_HEADER } else { Palette::TEXT_MUTED }))
                .child(t.label()),
        )
}

fn appearance(app: &PhoenixApp, cx: &mut Cx) -> Div {
    let s = &app.prefs.synced;
    let theme = s.theme;
    let compact = s.compact;
    let l = &app.prefs.local;
    let swatches = div().mb(z(24.)).flex().gap(z(12.)).children(AppTheme::ALL.map(|t| {
        theme_card(t, t == theme).on_click(cx.listener(move |a, _, _, cx| save(a, SettingsPatch::Theme(t), cx)))
    }));
    let (fs, zoom) = (l.font_size, l.zoom);
    div()
        .child(h1(crate::t!("settings.page.appearance")))
        .child(h5(crate::t!("settings.appearance.theme")))
        .child(swatches)
        .child(divider())
        .child(h5(crate::t!("settings.appearance.display")))
        .child(
            radio("display-cozy", crate::t!("settings.appearance.cozy"), !compact)
                .on_click(cx.listener(|a, _, _, cx| save(a, SettingsPatch::Compact(false), cx))),
        )
        .child(
            radio("display-compact", crate::t!("settings.appearance.compact"), compact)
                .on_click(cx.listener(|a, _, _, cx| save(a, SettingsPatch::Compact(true), cx))),
        )
        .child(divider())
        .child(h5(crate::t!("settings.appearance.font")))
        .child(slider(
            "font",
            &[12., 14., 15., 16., 18., 20., 24.],
            "px",
            fs,
            16.,
            |a, v, cx| update_local(a, |p| p.font_size = v, cx),
            cx,
        ))
        .child(h5(crate::t!("settings.appearance.zoom")))
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
        .child(h1(crate::t!("settings.page.accessibility")))
        .child(h5(crate::t!("settings.a11y.saturation")))
        .child(slider(
            "sat",
            &[0., 10., 20., 30., 40., 50., 60., 70., 80., 90., 100.],
            "%",
            sat as f32,
            100.,
            |a, v, cx| update_local(a, |p| p.saturation = v as _, cx),
            cx,
        ))
        .child(h5(crate::t!("settings.a11y.reduced_motion")))
        .child(switch_row(
            "reduced-motion",
            crate::t!("settings.a11y.reduced_motion_enable"),
            crate::t!("settings.a11y.reduced_motion_note"),
            rm,
            move |a, cx| update_local(a, |p| p.reduced_motion = !rm, cx),
            cx,
        ))
}

fn advanced(app: &PhoenixApp, cx: &mut Cx) -> Div {
    let dev = app.prefs.synced.developer_mode;
    let my_id = app.me.id.0.to_string();
    div()
        .child(h1(crate::t!("settings.page.advanced")))
        .child(switch_row(
            "dev-mode",
            crate::t!("settings.advanced.dev_mode"),
            crate::t!("settings.advanced.dev_mode_note"),
            dev,
            move |a, cx| save(a, SettingsPatch::DeveloperMode(!dev), cx),
            cx,
        ))
        .when(dev, |d| {
            d.child(h5(crate::t!("settings.advanced.user_id"))).child(
                div()
                    .flex()
                    .items_center()
                    .gap(z(12.))
                    .child(div().text_size(z(16.)).text_color(c(Palette::TEXT_NORMAL)).child(my_id.clone()))
                    .child(secondary("copy-my-id", crate::t!("common.copy_user_id")).on_click(cx.listener(move |_, _, _, cx| {
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

fn nav_item(id: impl Into<String>, label: impl Into<SharedString>, selected: bool, color: u32) -> Stateful<Div> {
    div()
        .id(eid(id))
        .mb(z(2.))
        .h(z(32.))
        .px(z(10.))
        .rounded(z(4.))
        .flex()
        .items_center()
        .cursor_pointer()
        .text_size(z(16.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(c(if selected { Palette::TEXT_HEADER } else { color }))
        .when(selected, |d| d.bg(c(Palette::BG_MOD_SELECTED)))
        .when(!selected, |d| d.hover(|s| s.bg(c(Palette::BG_MOD_HOVER)).text_color(c(Palette::INTERACTIVE_HOVER))))
        .child(label.into())
}

fn nav(app: &PhoenixApp, cx: &mut Cx) -> Div {
    let cur = app.settings.as_ref().map(|s| s.page).unwrap_or_default();
    let mut col = div().w(z(218.)).pt(z(60.)).pb(z(20.)).pr(z(6.)).pl(z(20.)).flex().flex_col();
    for (ix, (title, pages)) in NAV.iter().enumerate() {
        if ix > 0 {
            col = col.child(div().mx(z(10.)).my(z(8.)).h(z(1.)).bg(c(Palette::DIVIDER)));
        }
        col = col.child(div().px(z(10.)).pb(z(6.)).child(h5(crate::t!(title)).mb_0()));
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
    col.child(div().mx(z(10.)).my(z(8.)).h(z(1.)).bg(c(Palette::DIVIDER)))
        .child(nav_item("nav-logout", crate::t!("settings.logout"), false, 0xF23F43).on_click(cx.listener(|a, _, _, cx| {
            if let Some(s) = st(a) {
                s.confirm_logout = true;
            }
            cx.notify();
        })))
        .child(div().mx(z(10.)).my(z(8.)).h(z(1.)).bg(c(Palette::DIVIDER)))
        .child(div().px(z(10.)).text_size(z(12.)).text_color(c(Palette::TEXT_MUTED)).child(concat!(
            "Discord Phoenix ",
            env!("CARGO_PKG_VERSION"),
            " (Linux)"
        )))
}

fn close_button(cx: &mut Cx) -> Div {
    div()
        .ml(z(20.))
        .pt(z(60.))
        .flex()
        .flex_col()
        .items_center()
        .gap(z(8.))
        .child(
            div()
                .id("settings-close")
                .size(z(36.))
                .rounded_full()
                .border_2()
                .border_color(c(Palette::INTERACTIVE))
                .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
                .cursor_pointer()
                .flex()
                .items_center()
                .justify_center()
                .text_size(z(18.))
                .text_color(c(Palette::INTERACTIVE))
                .on_click(cx.listener(|a, _, _, cx| {
                    a.settings = None;
                    cx.notify();
                }))
                .child("✕"),
        )
        .child(div().text_size(z(13.)).font_weight(FontWeight::SEMIBOLD).text_color(c(Palette::INTERACTIVE)).child("ESC"))
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
                .w(z(440.))
                .rounded(z(8.))
                .bg(c(Palette::BG_PRIMARY))
                .overflow_hidden()
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .p(z(16.))
                        .child(
                            div()
                                .mb(z(12.))
                                .text_size(z(20.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(c(Palette::TEXT_HEADER))
                                .child(crate::t!("settings.logout")),
                        )
                        .child(div().text_size(z(16.)).text_color(c(Palette::TEXT_NORMAL)).child(crate::t!("settings.logout_confirm"))),
                )
                .child(
                    div()
                        .p(z(16.))
                        .bg(c(Palette::BG_SECONDARY))
                        .flex()
                        .justify_end()
                        .gap(z(8.))
                        .child(
                            div()
                                .id("logout-cancel")
                                .h(z(38.))
                                .px(z(16.))
                                .flex()
                                .items_center()
                                .cursor_pointer()
                                .text_size(z(14.))
                                .text_color(c(Palette::TEXT_NORMAL))
                                .hover(|s| s.underline())
                                .on_click(cx.listener(|a, _, _, cx| {
                                    if let Some(s) = st(a) {
                                        s.confirm_logout = false;
                                    }
                                    cx.notify();
                                }))
                                .child(crate::t!("common.cancel")),
                        )
                        .child(button("logout-confirm", crate::t!("settings.logout"), Palette::RED, 0xDA373C).h(z(38.)).on_click(
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
                    .min_w(z(218.))
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
                    .min_w(z(460.))
                    .flex()
                    .overflow_y_scroll()
                    .child(
                        div()
                            .w(z(740.))
                            .min_w(z(460.))
                            .pt(z(60.))
                            .px(z(40.))
                            .pb(z(80.))
                            .flex()
                            .flex_col()
                            .when_some(error, |d, e| {
                                d.child(
                                    div()
                                        .mb(z(16.))
                                        .p(z(10.))
                                        .rounded(z(4.))
                                        .bg(gpui::rgba(0xF23F4333))
                                        .text_color(c(Palette::TEXT_NORMAL))
                                        .text_size(z(14.))
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
