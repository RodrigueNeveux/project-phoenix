//! Rendering of the User Settings overlay.

use crate::theme::z;
use gpui::{
    AnyElement, Context, Div, ElementId, FontWeight, IntoElement, ParentElement, SharedString, Stateful, Styled,
    Window, div, prelude::*,
};

use super::{AppTheme, NAV, Page, SettingsPatch, SettingsState, save, update_local};
use crate::{
    app::PhoenixApp,
    theme::{Palette, c, generated::idx, tok},
    ui::{
        avatar,
        controls::{self, BtnKind},
    },
};

type Cx<'a> = Context<'a, PhoenixApp>;

fn eid(s: impl Into<String>) -> ElementId {
    ElementId::Name(SharedString::from(s.into()))
}

fn st(app: &mut PhoenixApp) -> Option<&mut SettingsState> {
    app.settings.as_mut()
}

// Live 2026 settings modal (dumps settings-home / set-* via CDP, 2026-11-23).
// Content panel_nNq2Vb: max-width 696 + padding-inline 16, margin-block 64.
// Each category: heading-xl/medium title + 24px gap, then stacked
// baseControlItem_FiiQ6I rows (gap 10), categories split by a divider_
// with margin 50 (--space-xxl + --space-xs).

fn h1(t: impl Into<SharedString>) -> Div {
    controls::section_title(t)
}

/// The 2026 client labels groups in sentence case ("Thèmes par défaut"):
/// older all-caps strings are folded to that.
fn sentence(t: SharedString) -> SharedString {
    if t.chars().any(|ch| ch.is_lowercase()) {
        return t;
    }
    let lower = t.to_lowercase();
    let mut it = lower.chars();
    it.next().map(|f| f.to_uppercase().chain(it).collect::<String>().into()).unwrap_or(t)
}

fn h5(t: impl Into<SharedString>) -> Div {
    let t = sentence(t.into());
    // heading-sm/semibold group label inside a category
    div()
        .pt(z(8.))
        .pb(z(12.))
        .text_size(z(16.))
        .line_height(z(20.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(c(tok(idx::TEXT_STRONG)))
        .child(t)
}

fn note(t: impl Into<SharedString>) -> Div {
    div().text_size(z(14.)).line_height(z(18.)).text_color(c(tok(idx::TEXT_SUBTLE))).child(t.into())
}

fn divider() -> Div {
    // divider_ZaYWLb: 1px --border-subtle, margin 50px 0
    div().my(z(50.)).h(z(1.)).bg(c(tok(idx::BORDER_SUBTLE)))
}

fn category(title: impl Into<SharedString>, rows: Vec<AnyElement>) -> Div {
    div().flex().flex_col().child(h1(title)).child(div().flex().flex_col().gap(z(10.)).children(rows))
}

fn secondary(id: impl Into<String>, label: impl Into<SharedString>) -> Stateful<Div> {
    controls::button(eid(id), label, BtnKind::Secondary)
}

/// Switch row: baseControlItem with the 48x24 switch trailing.
fn switch_row(
    id: &str,
    title: SharedString,
    desc: SharedString,
    on: bool,
    f: impl Fn(&mut PhoenixApp, &mut Cx) + 'static,
    cx: &mut Cx,
) -> AnyElement {
    div()
        .id(eid(format!("switch-{id}")))
        .cursor_pointer()
        .on_click(cx.listener(move |a, _, _, cx| f(a, cx)))
        .child(controls::control_item(title, Some(desc), Some(controls::switch(on).into_any_element())))
        .into_any_element()
}

fn radio(id: impl Into<String>, label: impl Into<SharedString>, selected: bool) -> Stateful<Div> {
    controls::radio_option(eid(id), label, selected).py(z(6.))
}

/// Marker slider bound to a local preference.
fn slider(
    id: &str,
    values: &[f32],
    unit: &'static str,
    current: f32,
    default: f32,
    set: impl Fn(&mut PhoenixApp, f32, &mut Cx) + 'static,
    cx: &mut Cx,
) -> Div {
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

/// "Username   kasiopy  [Edit]" row: value (16/500 --text-default) and the
/// trailing button, 20px apart.
fn field_row(label: impl Into<SharedString>, value: SharedString, action: Option<AnyElement>) -> AnyElement {
    let trailing = div()
        .flex()
        .items_center()
        .gap(z(20.))
        .child(div().text_size(z(16.)).line_height(z(20.)).font_weight(FontWeight::MEDIUM).text_color(c(tok(idx::TEXT_DEFAULT))).child(value))
        .children(action);
    controls::control_item(label, None, Some(trailing.into_any_element())).into_any_element()
}

fn my_account(app: &PhoenixApp, cx: &mut Cx) -> Div {
    let me = &app.me;
    let reveal = app.settings.as_ref().is_some_and(|s| s.reveal_email);
    let email: SharedString = if reveal { crate::t!("settings.account.email_hidden") } else { "**********@*****.com".into() };
    // textButton_nJgowo: 16/500 --text-link
    let reveal_btn = div()
        .id("reveal-email")
        .text_size(z(16.))
        .line_height(z(20.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(c(tok(idx::TEXT_LINK)))
        .cursor_pointer()
        .on_click(cx.listener(|a, _, _, cx| {
            if let Some(s) = st(a) {
                s.reveal_email = !s.reveal_email;
            }
            cx.notify();
        }))
        .child(if reveal { crate::t!("common.hide") } else { crate::t!("common.reveal") });
    let email_val = div()
        .flex()
        .items_center()
        .gap(z(4.))
        .child(div().text_size(z(16.)).line_height(z(20.)).font_weight(FontWeight::MEDIUM).text_color(c(tok(idx::TEXT_DEFAULT))).child(email))
        .child(reveal_btn);
    let email_row = controls::control_item(
        crate::t!("settings.account.email"),
        None,
        Some(div().flex().items_center().gap(z(20.)).child(email_val).child(secondary("edit-em", crate::t!("common.edit"))).into_any_element()),
    )
    .into_any_element();
    div()
        .child(category(
            crate::t!("settings.account.info"),
            vec![
                field_row(crate::t!("settings.account.display_name"), me.display_name.clone(), Some(secondary("edit-dn", crate::t!("common.edit")).into_any_element())),
                field_row(crate::t!("settings.account.username"), me.name.clone(), Some(secondary("edit-un", crate::t!("common.edit")).into_any_element())),
                email_row,
                field_row(crate::t!("settings.account.phone"), crate::t!("settings.account.no_phone"), Some(secondary("edit-ph", crate::t!("common.add")).into_any_element())),
            ],
        ))
        .child(divider())
        .child(category(
            crate::t!("settings.account.security"),
            vec![
                controls::control_item(
                    crate::t!("settings.account.password"),
                    None,
                    Some(secondary("change-pw", crate::t!("common.edit")).into_any_element()),
                )
                .into_any_element(),
                controls::control_item(crate::t!("settings.account.authenticator"), Some(crate::t!("settings.account.authenticator_note")), None).into_any_element(),
            ],
        ))
        .child(divider())
        .child(category(
            crate::t!("settings.account.removal"),
            vec![controls::control_item(crate::t!("settings.account.removal"), Some(crate::t!("settings.account.removal_note")), None).into_any_element()],
        ))
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
                .bg(c(tok(idx::BACKGROUND_BASE_LOWER)))
                .flex()
                .items_center()
                .text_color(c(tok(idx::TEXT_DEFAULT)))
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
                                .text_color(c(tok(idx::TEXT_STRONG)))
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
                .bg(c(tok(idx::BACKGROUND_MOD_SUBTLE)))
                .child(
                    div()
                        .text_size(z(16.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(tok(idx::TEXT_STRONG)))
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
            .bg(c(tok(idx::BACKGROUND_BASE_LOWER)))
            .flex()
            .items_center()
            .justify_center()
            .text_size(z(11.))
            .text_color(c(tok(idx::INTERACTIVE_TEXT_DEFAULT)))
            .child(name)
    };
    div()
        .child(h1(crate::t!("settings.page.connections")))
        .child(
            div()
                .p(z(16.))
                .rounded(z(8.))
                .bg(c(tok(idx::BACKGROUND_MOD_SUBTLE)))
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
            .bg(c(tok(idx::BACKGROUND_MOD_SUBTLE)))
            .flex()
            .items_center()
            .child(div().flex_1().text_size(z(16.)).text_color(c(tok(idx::TEXT_DEFAULT))).child(action))
            .child(
                div()
                    .px(z(8.))
                    .py(z(2.))
                    .rounded(z(4.))
                    .bg(c(tok(idx::BACKGROUND_BASE_LOWER)))
                    .text_size(z(14.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(c(tok(idx::INTERACTIVE_TEXT_DEFAULT)))
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

/// themeSelection_IhyaN_ (live Appearance dump): 48x48 swatch, radius 8,
/// 8px apart; the selected one gets the 52px selectionCircle_ ring in
/// --control-brand-foreground-new with a check badge.
fn theme_card(t: AppTheme, selected: bool) -> Stateful<Div> {
    div()
        .id(eid(format!("theme-{}", t.label())))
        .relative()
        .size(z(48.))
        .flex_none()
        .cursor_pointer()
        .child(
            div()
                .size_full()
                .rounded(z(8.))
                .bg(c(t.swatch()))
                .border_1()
                .border_color(c(tok(idx::BORDER_SUBTLE))),
        )
        .when(selected, |d| {
            d.child(
                div()
                    .absolute()
                    .top(z(-2.))
                    .left(z(-2.))
                    .size(z(52.))
                    .rounded(z(10.))
                    .border_2()
                    .border_color(c(tok(idx::CONTROL_BRAND_FOREGROUND_NEW))),
            )
            .child(
                div()
                    .absolute()
                    .top(z(-6.))
                    .right(z(-6.))
                    .size(z(20.))
                    .rounded_full()
                    .bg(c(tok(idx::CONTROL_BRAND_FOREGROUND_NEW)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(gpui::svg().path(crate::icons::path("check")).size(z(14.)).text_color(gpui::white())),
            )
        })
        .hover(|s| s.opacity(0.9))
}

fn appearance(app: &PhoenixApp, cx: &mut Cx) -> Div {
    let s = &app.prefs.synced;
    let theme = s.theme;
    let compact = s.compact;
    let l = &app.prefs.local;
    let swatches = div().mb(z(24.)).flex().gap(z(8.)).children(AppTheme::ALL.map(|t| {
        theme_card(t, t == theme).on_click(cx.listener(move |a, _, _, cx| save(a, SettingsPatch::Theme(t), cx)))
    }));
    let (fs, zoom, density) = (l.font_size, l.zoom, l.density);
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
        .child(h5(crate::t!("settings.appearance.density")))
        .children(crate::theme::Density::ALL.map(|d| {
            radio(format!("density-{}", d.key()), crate::t!(&format!("settings.density.{}", d.key())), density == d)
                .on_click(cx.listener(move |a, _, _, cx| update_local(a, |p| p.density = d, cx)))
        }))
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
                    .child(div().text_size(z(16.)).text_color(c(tok(idx::TEXT_DEFAULT))).child(my_id.clone()))
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

/// Sidebar page icon (icons are requested by name; the provider swaps in the
/// client's own glyphs when available).
fn page_icon(p: Page) -> &'static str {
    match p {
        Page::MyAccount | Page::Profiles => "friends",
        Page::PrivacySafety => "lock",
        Page::Devices => "image",
        Page::Connections => "plus-circle",
        Page::Notifications => "bell",
        Page::Keybinds => "file",
        Page::Language => "compass",
        Page::Appearance => "emoji",
        Page::Accessibility => "help",
        Page::VoiceVideo => "mic",
        Page::Advanced => "gear",
    }
}

/// item_xyJHva: padding 10, radius 8 (--radius-sm), --text-muted; hover
/// --background-mod-muted, active --background-mod-normal + --text-strong.
/// itemContent gap 10: 20px icon + body-md/medium 16/20 label.
fn nav_item(id: impl Into<String>, icon: &str, label: impl Into<SharedString>, selected: bool, destructive: bool) -> Stateful<Div> {
    let fg = if selected { idx::TEXT_STRONG } else { idx::TEXT_MUTED };
    let label_fg = if destructive { idx::TEXT_FEEDBACK_CRITICAL } else { fg };
    div()
        .id(eid(id))
        .p(z(10.))
        .rounded(z(8.))
        .flex()
        .items_center()
        .gap(z(10.))
        .cursor_pointer()
        .text_color(c(tok(fg)))
        .when(selected, |d| d.bg(c(tok(idx::BACKGROUND_MOD_NORMAL))))
        .when(!selected && !destructive, |d| {
            d.hover(|s| s.bg(c(tok(idx::BACKGROUND_MOD_MUTED))).text_color(c(tok(idx::TEXT_STRONG))))
        })
        .when(destructive, |d| d.hover(|s| s.bg(c(tok(idx::BACKGROUND_FEEDBACK_MOD_CRITICAL)))))
        .child(gpui::svg().path(crate::icons::path(icon)).size(z(20.)).flex_none().text_color(c(tok(fg))))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(z(16.))
                .line_height(z(20.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(c(tok(label_fg)))
                .child(label.into()),
        )
}

/// subnav_8qMUr2 under the active "Account" item: 2px --background-mod-subtle
/// track (19px in, padding 10 0) with the --interactive-text-active thumb on
/// the current row; rows body-md/normal 16/20, 20px apart.
fn account_subnav() -> Div {
    let rows = [
        crate::t!("settings.account.info"),
        crate::t!("settings.account.security"),
        crate::t!("settings.account.status"),
    ];
    let n = rows.len() as f32;
    div()
        .py(z(10.))
        .ml(z(19.))
        .flex()
        .gap(z(19.))
        .child(
            div()
                .flex_none()
                .w(z(2.))
                .h(z(20. * n + 20. * (n - 1.)))
                .rounded(z(1.))
                .bg(c(tok(idx::BACKGROUND_MOD_SUBTLE)))
                .child(div().w_full().h(z(20.)).rounded_full().bg(c(tok(idx::INTERACTIVE_TEXT_ACTIVE)))),
        )
        .child(div().flex_1().flex().flex_col().gap(z(20.)).children(rows.into_iter().enumerate().map(|(i, t)| {
            div()
                .text_size(z(16.))
                .line_height(z(20.))
                .text_color(c(tok(if i == 0 { idx::INTERACTIVE_TEXT_ACTIVE } else { idx::TEXT_MUTED })))
                .child(t)
        })))
}

fn select_page(p: Page, cx: &mut Cx) -> impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static {
    cx.listener(move |a, _, _, cx| {
        if let Some(s) = st(a) {
            s.page = p;
            s.error = None;
        }
        cx.notify();
    })
}

fn nav(app: &PhoenixApp, cx: &mut Cx) -> Div {
    let cur = app.settings.as_ref().map(|s| s.page).unwrap_or_default();
    // fixedContent_: padding-inline 4 16. Profile tab: 48px avatar, gap 12,
    // name 16/500 --text-strong over "Edit Profiles" 14/18 --text-muted.
    let profile = div()
        .id("nav-profiles")
        .p(z(10.))
        .rounded(z(8.))
        .flex()
        .items_center()
        .gap(z(12.))
        .cursor_pointer()
        .when(cur == Page::Profiles, |d| d.bg(c(tok(idx::BACKGROUND_MOD_NORMAL))))
        .when(cur != Page::Profiles, |d| d.hover(|s| s.bg(c(tok(idx::BACKGROUND_MOD_MUTED)))))
        .on_click(select_page(Page::Profiles, cx))
        .child(avatar(Some(&app.me), 48.))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(z(2.))
                .child(
                    div()
                        .text_size(z(16.))
                        .line_height(z(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(c(tok(idx::TEXT_STRONG)))
                        .whitespace_nowrap()
                        .overflow_hidden()
                        .child(app.me.display_name.clone()),
                )
                .child(div().text_size(z(14.)).line_height(z(18.)).text_color(c(tok(idx::TEXT_MUTED))).child(crate::t!("settings.edit_profiles"))),
        );
    // searchBar: 40px text input (--input-*), 30px icon box + 16px glyph.
    let search = div()
        .h(z(40.))
        .pl(z(4.))
        .flex()
        .items_center()
        .gap(z(4.))
        .rounded(z(8.))
        .border_1()
        .border_color(c(tok(idx::INPUT_BORDER_DEFAULT)))
        .bg(c(tok(idx::INPUT_BACKGROUND_DEFAULT)))
        .child(
            div().size(z(30.)).flex().items_center().justify_center().child(
                gpui::svg().path(crate::icons::path("search")).size(z(16.)).text_color(c(tok(idx::INPUT_ICON_DEFAULT))),
            ),
        )
        .child(div().text_size(z(16.)).text_color(c(tok(idx::INPUT_PLACEHOLDER_TEXT_DEFAULT))).child(crate::t!("settings.search")));
    let fixed = div()
        .pl(z(4.))
        .pr(z(16.))
        .flex_none()
        .child(div().pb(z(10.)).child(profile))
        .child(search);

    // navScroller_: padding 12 12 16 4, margin-end 4; sublist gap 10;
    // section_: border-bottom 1 --border-subtle (none on the last), pb 10,
    // gap 4; label_ heading-xs/medium 14/18 --text-muted, margin 8 8 4.
    let mut list = div().flex().flex_col().gap(z(10.));
    let last = NAV.len() - 1;
    for (ix, (title, pages)) in NAV.iter().enumerate() {
        let mut sec = div().pb(z(10.)).flex().flex_col().gap(z(4.));
        if ix != last {
            sec = sec.border_b_1().border_color(c(tok(idx::BORDER_SUBTLE)));
        }
        if ix > 0 {
            sec = sec.child(
                div()
                    .mt(z(8.))
                    .mx(z(8.))
                    .mb(z(4.))
                    .text_size(z(14.))
                    .line_height(z(18.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c(tok(idx::TEXT_MUTED)))
                    .child(crate::t!(title)),
            );
        }
        for p in pages.iter().copied() {
            sec = sec.child(nav_item(format!("nav-{}", p.label()), page_icon(p), p.label(), p == cur, false).on_click(select_page(p, cx)));
            if p == Page::MyAccount && cur == Page::MyAccount {
                sec = sec.child(account_subnav());
            }
        }
        if ix == last {
            sec = sec.child(nav_item("nav-logout", "close", crate::t!("settings.logout"), false, true).on_click(cx.listener(|a, _, _, cx| {
                if let Some(s) = st(a) {
                    s.confirm_logout = true;
                }
                cx.notify();
            })));
            sec = sec.child(div().px(z(10.)).pt(z(8.)).text_size(z(12.)).line_height(z(16.)).text_color(c(tok(idx::TEXT_MUTED))).child(concat!(
                "Discord Phoenix ",
                env!("CARGO_PKG_VERSION"),
                " (Linux)"
            )));
        }
        list = list.child(sec);
    }
    // sidebar_: flex 0 0 240 (+12 padding-start), --background-base-lower,
    // border-inline-end 1 --border-muted, padding-top 16.
    div()
        .w(z(253.))
        .flex_none()
        .h_full()
        .pt(z(16.))
        .pl(z(12.))
        .bg(c(tok(idx::BACKGROUND_BASE_LOWER)))
        .border_r_1()
        .border_color(c(tok(idx::BORDER_MUTED)))
        .flex()
        .flex_col()
        .child(fixed)
        .child(
            div()
                .id("settings-nav")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .pt(z(12.))
                .pb(z(16.))
                .pl(z(4.))
                .pr(z(12.))
                .mr(z(4.))
                .child(list),
        )
}

/// contentHeader_: 48px, --background-base-low, border-bottom 1
/// --border-muted, padding-inline 16 8; breadcrumb body-md/medium 16/20
/// --text-default; sm icon-only close button (32px, 16px glyph).
fn content_header(label: SharedString, cx: &mut Cx) -> Div {
    div()
        .h(z(48.))
        .flex_none()
        .pl(z(16.))
        .pr(z(8.))
        .flex()
        .items_center()
        .justify_between()
        .border_b_1()
        .border_color(c(tok(idx::BORDER_MUTED)))
        .child(div().text_size(z(16.)).line_height(z(20.)).font_weight(FontWeight::MEDIUM).text_color(c(tok(idx::TEXT_DEFAULT))).child(label))
        .child(
            div()
                .id("settings-close")
                .size(z(32.))
                .rounded(z(8.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(|s| s.bg(c(tok(idx::CONTROL_ICON_ONLY_BACKGROUND_HOVER))))
                .on_click(cx.listener(|a, _, _, cx| {
                    a.settings = None;
                    cx.notify();
                }))
                .child(gpui::svg().path(crate::icons::path("close")).size(z(16.)).text_color(c(tok(idx::CONTROL_ICON_ONLY_ICON_DEFAULT)))),
        )
}

fn logout_modal(cx: &mut Cx) -> AnyElement {
    let (e1, e2) = (cx.entity(), cx.entity());
    crate::ui::modal::confirm(crate::ui::modal::Confirm {
        title: crate::t!("settings.logout"),
        body: vec![crate::t!("settings.logout_confirm").into_any_element()],
        confirm_label: crate::t!("settings.logout"),
        danger: true,
        on_confirm: std::rc::Rc::new(move |_, cx| {
            e1.update(cx, |a, cx| {
                a.settings = None;
                a.logout(cx);
            })
        }),
        on_cancel: std::rc::Rc::new(move |_, cx| {
            e2.update(cx, |a, cx| {
                if let Some(s) = st(a) {
                    s.confirm_logout = false;
                }
                cx.notify();
            })
        }),
    })
}

/// The overlay, when open: the 2026 settings modal (modal__F9ieB) inset by
/// --custom-viewport-padding (40) over the scrim, max-width 1400, radius 12,
/// 1px --border-subtle; sidebar + content (--background-base-low).
pub fn render(app: &PhoenixApp, _window: &mut Window, cx: &mut Cx) -> Option<AnyElement> {
    let s = app.settings.as_ref()?;
    let error = s.error.clone();
    let confirm = s.confirm_logout;
    let page_label = s.page.label();
    let body = div()
        .id("settings-content")
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .child(
            // panel_nNq2Vb: margin-block 64 auto, max-width 696, padding 0 16
            div()
                .mx_auto()
                .my(z(64.))
                .w_full()
                .max_w(z(728.))
                .px(z(16.))
                .flex()
                .flex_col()
                .when_some(error, |d, e| {
                    d.child(
                        div()
                            .mb(z(16.))
                            .p(z(12.))
                            .rounded(z(8.))
                            .bg(c(tok(idx::BACKGROUND_FEEDBACK_CRITICAL)))
                            .border_1()
                            .border_color(c(tok(idx::BORDER_FEEDBACK_CRITICAL)))
                            .text_color(c(tok(idx::TEXT_DEFAULT)))
                            .text_size(z(14.))
                            .child(e),
                    )
                })
                .child(page(app, cx)),
        );
    let card = div()
        .id("user-settings-card")
        .size_full()
        .max_w(z(1400.))
        .rounded(z(12.))
        .border_1()
        .border_color(c(tok(idx::BORDER_SUBTLE)))
        .bg(c(tok(idx::BACKGROUND_BASE_LOWER)))
        .overflow_hidden()
        .occlude()
        .flex()
        .child(nav(app, cx))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .flex_col()
                .bg(c(tok(idx::BACKGROUND_BASE_LOW)))
                .child(content_header(page_label, cx))
                .child(body),
        );
    Some(
        div()
            .id("user-settings")
            .absolute()
            .inset_0()
            .occlude()
            .bg(c(tok(idx::BACKGROUND_SCRIM)))
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|a, _, _, cx| {
                a.settings = None;
                cx.notify();
            }))
            .child(
                div()
                    .absolute()
                    .inset(z(40.))
                    .flex()
                    .justify_center()
                    .child(card.on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())),
            )
            .when(confirm, |d| d.child(logout_modal(cx)))
            .into_any_element(),
    )
}
