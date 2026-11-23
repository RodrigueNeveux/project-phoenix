//! Declarative content of the settings pages, reproduced from the live 2026
//! client (CDP outlines of each page, 2026-11-23): every page is a list of
//! [`B`] blocks rendered with the client's control layout.
//!
//! Layout values (panel_nNq2Vb children):
//! * category: heading-xl/medium 24/32 title, optional body-sm lead 4px
//!   under it, then container_Gl6U0S rows (padding --space-xs/--space-sm,
//!   margin negated) holding baseControlItem_FiiQ6I (padding/gap 12,
//!   min-height 48, radius --radius-lg; `.clickable` hover
//!   --interactive-background-hover).
//! * categories are split by divider_ZaYWLb (1px, margin 50px 0).
//! * legends: body-md/semibold 16/20 (30px box); keybind groups body-lg/medium 20/24.
//! * key caps (key_dMUH0f): --background-mod-muted, 1px --border-strong,
//!   radius 4, height 23, padding 3 6 4, 12/600 text, 3px apart.
//!
//! Settings with no backing preference are kept for the session only
//! (`SettingsState::toggles` / `picks`) so the page behaves like the client
//! without pretending to sync anything.

use gpui::{AnyElement, Context, Div, ElementId, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, prelude::*};

use super::{AppTheme, LocalPrefs, Page, SettingsPatch, SyncedSettings, save, update_local};
use crate::{
    app::PhoenixApp,
    theme::{c, generated::idx, tok, z},
    ui::controls::{self, BtnKind},
};

type Cx<'a> = Context<'a, PhoenixApp>;
type Render = fn(&PhoenixApp, &mut Context<'_, PhoenixApp>) -> AnyElement;

/// Where a switch reads/writes its value.
#[derive(Clone, Copy)]
pub enum Bind {
    /// Session-only, with the client's default.
    Ui(bool),
    Local(fn(&LocalPrefs) -> bool, fn(&mut LocalPrefs, bool)),
    Synced(fn(&SyncedSettings) -> bool, fn(bool) -> SettingsPatch),
}

/// Where a radio group / select reads/writes its index.
#[derive(Clone, Copy)]
pub enum Pick {
    Ui(usize),
    Local(fn(&LocalPrefs) -> usize, fn(&mut LocalPrefs, usize)),
    Synced(fn(&SyncedSettings) -> usize, fn(usize) -> SettingsPatch),
}

/// One block of a settings page. Strings are i18n keys ("" = none).
pub enum B {
    Title(&'static str),
    Lead(&'static str),
    Legend(&'static str),
    Switch(&'static str, &'static str, &'static str, Bind),
    /// Switch whose description is the "Sound preview" link.
    Sound(&'static str, &'static str),
    Radios(&'static str, &'static str, &'static str, &'static [(&'static str, &'static str)], Pick),
    Select(&'static str, &'static str, &'static str, &'static [&'static str], Pick),
    Action(&'static str, &'static str, &'static str, bool),
    Related(&'static str, &'static str, &'static str, Page),
    /// Collapsed "Show advanced ..." row revealing more blocks.
    Expand(&'static str, &'static str, &'static str, &'static [B]),
    KeyGroup(&'static str, &'static str, &'static [(&'static str, &'static [&'static str])]),
    Note(&'static str),
    Divider,
    /// Custom control; the key is used for search.
    Custom(&'static str, Render),
}

fn eid(s: impl Into<String>) -> ElementId {
    ElementId::Name(SharedString::from(s.into()))
}

fn tr(k: &str) -> SharedString {
    crate::t!(k)
}

// ---------------------------------------------------------------- rendering

fn title_text(t: SharedString) -> Div {
    div().text_size(z(16.)).line_height(z(20.)).font_weight(FontWeight::MEDIUM).text_color(c(tok(idx::TEXT_STRONG))).child(t)
}

fn desc_text(t: SharedString) -> Div {
    // body-sm/normal 14/18 --text-subtle
    div().text_size(z(14.)).line_height(z(18.)).text_color(c(tok(idx::TEXT_SUBTLE))).child(t)
}

/// baseControlItem row with the trailing control aligned to the title line.
fn item(title: SharedString, desc: Option<AnyElement>, trailing: Option<AnyElement>, top: bool) -> Div {
    div()
        .flex()
        .gap(z(12.))
        .mx(z(-12.))
        .px(z(12.))
        // vanilla-settings-13 #2688 baseSettingWrapper_4Z: switch rows pad
        // 10px 0 (no 48px floor); other rows keep baseControlItem's 12 / 48.
        .when(top, |d| d.py(z(10.)))
        .when(!top, |d| d.py(z(12.)).min_h(z(48.)))
        .rounded(z(16.)) // --radius-lg
        // vanilla-settings-02 #2700 description_VhFsFg: switch rows wrap
        // their description at 572px, other rows use the full 696.
        .child(div().flex_1().min_w_0().flex().flex_col().justify_center().gap(z(4.)).child(title_text(title)).children(desc.map(|d| div().when(top, |d| d.max_w(z(572.))).child(d))))
        .children(trailing.map(|t| {
            let d = div().flex_none().flex();
            if top { d.items_start().child(t) } else { d.items_center().child(t) }
        }))
}

fn get_bool(app: &PhoenixApp, id: &str, b: Bind) -> bool {
    match b {
        Bind::Ui(d) => app.settings.as_ref().and_then(|s| s.toggles.get(id).copied()).unwrap_or(d),
        Bind::Local(g, _) => g(&app.prefs.local),
        Bind::Synced(g, _) => g(&app.prefs.synced),
    }
}

fn set_bool(app: &mut PhoenixApp, id: &'static str, b: Bind, v: bool, cx: &mut Cx) {
    match b {
        Bind::Ui(_) => {
            if let Some(s) = app.settings.as_mut() {
                s.toggles.insert(id, v);
            }
            cx.notify();
        }
        Bind::Local(_, s) => update_local(app, |p| s(p, v), cx),
        Bind::Synced(_, p) => save(app, p(v), cx),
    }
}

fn get_pick(app: &PhoenixApp, id: &str, p: Pick) -> usize {
    match p {
        Pick::Ui(d) => app.settings.as_ref().and_then(|s| s.picks.get(id).copied()).unwrap_or(d),
        Pick::Local(g, _) => g(&app.prefs.local),
        Pick::Synced(g, _) => g(&app.prefs.synced),
    }
}

fn set_pick(app: &mut PhoenixApp, id: &'static str, p: Pick, v: usize, cx: &mut Cx) {
    if let Some(s) = app.settings.as_mut() {
        s.open_select = None;
    }
    match p {
        Pick::Ui(_) => {
            if let Some(s) = app.settings.as_mut() {
                s.picks.insert(id, v);
            }
            cx.notify();
        }
        Pick::Local(_, s) => update_local(app, |pr| s(pr, v), cx),
        Pick::Synced(_, f) => save(app, f(v), cx),
    }
}

fn switch_row(app: &PhoenixApp, id: &'static str, title: SharedString, desc: Option<AnyElement>, b: Bind, cx: &mut Cx) -> AnyElement {
    let on = get_bool(app, id, b);
    item(title, desc, Some(controls::switch(on).into_any_element()), true)
        .id(eid(format!("sw-{id}")))
        .cursor_pointer()
        .on_click(cx.listener(move |a, _, _, cx| set_bool(a, id, b, !on, cx)))
        .into_any_element()
}

/// Dropdown field + its listBox (listBox_b5PEJH: padding 4, 40px options,
/// radius 4) floating under it while open.
pub(super) fn dropdown(
    app: &PhoenixApp,
    id: &'static str,
    opts: Vec<SharedString>,
    cur: usize,
    width: f32,
    on_pick: impl Fn(&mut PhoenixApp, usize, &mut Cx) + Clone + 'static,
    cx: &mut Cx,
) -> AnyElement {
    let open = app.settings.as_ref().is_some_and(|s| s.open_select == Some(id));
    let field = controls::select_field(eid(format!("sel-{id}")), opts.get(cur).cloned().unwrap_or_default())
        .w(z(width))
        .when(open, |d| d.border_color(c(tok(idx::INPUT_BORDER_ACTIVE))))
        .on_click(cx.listener(move |a, _, _, cx| {
            if let Some(s) = a.settings.as_mut() {
                s.open_select = if s.open_select == Some(id) { None } else { Some(id) };
            }
            cx.notify();
        }));
    let mut wrap = div().relative().child(field);
    if open {
        let list = div()
            .id(eid(format!("sel-{id}-list")))
            .absolute()
            .top(z(44.))
            .left_0()
            .w(z(width))
            .max_h(z(320.))
            .overflow_y_scroll()
            .p(z(4.))
            .rounded(z(8.))
            .border_1()
            .border_color(c(tok(idx::BORDER_SUBTLE)))
            .bg(c(tok(idx::BACKGROUND_SURFACE_HIGHER)))
            .shadow_lg()
            .occlude()
            .children(opts.into_iter().enumerate().map(|(i, o)| {
                let f = on_pick.clone();
                div()
                    .id(eid(format!("sel-{id}-{i}")))
                    .h(z(40.))
                    .px(z(8.))
                    .rounded(z(4.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .when(i == cur, |d| d.bg(c(tok(idx::INTERACTIVE_BACKGROUND_SELECTED))))
                    .hover(|s| s.bg(c(tok(idx::INTERACTIVE_BACKGROUND_HOVER))))
                    .on_click(cx.listener(move |a, _, _, cx| f(a, i, cx)))
                    .child(div().text_size(z(16.)).line_height(z(20.)).text_color(c(tok(idx::TEXT_STRONG))).child(o))
                    .when(i == cur, |d| {
                        d.child(gpui::svg().path(crate::icons::path("check")).size(z(16.)).text_color(c(tok(idx::CONTROL_BRAND_FOREGROUND_NEW))))
                    })
            }));
        wrap = wrap.child(gpui::deferred(list).with_priority(2));
    }
    wrap.into_any_element()
}

fn radio_group(app: &PhoenixApp, id: &'static str, opts: &'static [(&'static str, &'static str)], p: Pick, cx: &mut Cx) -> Div {
    let cur = get_pick(app, id, p);
    // group_oJuRbG: options 36px apart (indicator 20, text at 32px)
    div().flex().flex_col().children(opts.iter().enumerate().map(|(i, (label, desc))| {
        div()
            .id(eid(format!("rg-{id}-{i}")))
            .py(z(6.))
            .flex()
            .gap(z(12.))
            .cursor_pointer()
            .on_click(cx.listener(move |a, _, _, cx| set_pick(a, id, p, i, cx)))
            .child(div().pt(z(1.)).child(controls::radio_indicator(i == cur)))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(z(4.))
                    .child(div().text_size(z(16.)).line_height(z(20.)).text_color(c(tok(idx::TEXT_DEFAULT))).child(tr(label)))
                    .when(!desc.is_empty(), |d| d.child(desc_text(tr(desc)))),
            )
    }))
}

fn key_cap(k: &'static str) -> Div {
    div()
        .h(z(23.))
        .min_w(z(14.))
        .pt(z(3.))
        .pb(z(4.))
        .px(z(6.))
        .rounded(z(4.))
        .border_1()
        .border_color(c(tok(idx::BORDER_STRONG)))
        .bg(c(tok(idx::BACKGROUND_MOD_MUTED)))
        .flex()
        .items_center()
        .justify_center()
        .text_size(z(12.))
        .line_height(z(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(c(tok(idx::TEXT_STRONG)))
        .child(k)
}

fn key_group(legend: &'static str, lead: &'static str, rows: &'static [(&'static str, &'static [&'static str])]) -> Div {
    let n = rows.len();
    div()
        .flex()
        .flex_col()
        .when(!legend.is_empty(), |d| {
            d.child(
                div()
                    .pt(z(16.))
                    .text_size(z(20.))
                    .line_height(z(24.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c(tok(idx::TEXT_STRONG)))
                    .child(tr(legend)),
            )
        })
        .when(!lead.is_empty(), |d| d.child(div().pt(z(4.)).child(desc_text(tr(lead)))))
        .child(div().mt(z(16.)).rounded(z(8.)).border_1().border_color(c(tok(idx::BORDER_SUBTLE))).children(rows.iter().enumerate().map(
            |(i, (action, keys))| {
                div()
                    .px(z(16.))
                    .min_h(z(56.))
                    .flex()
                    .items_center()
                    .gap(z(6.))
                    .when(i + 1 < n, |d| d.border_b_1().border_color(c(tok(idx::BORDER_SUBTLE))))
                    .child(div().flex_1().text_size(z(16.)).line_height(z(20.)).text_color(c(tok(idx::TEXT_DEFAULT))).child(tr(action)))
                    .child(div().flex().gap(z(3.)).children(keys.iter().map(|k| key_cap(k))))
            },
        )))
}

fn related(icon: &'static str, title: &'static str, desc: &'static str, page: Page, cx: &mut Cx) -> AnyElement {
    let lead = div()
        .w(z(48.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        // vanilla-settings-13 #2733 icon_W6yko_: 48px tile, --radius-sm,
        // --background-mod-muted, 24px glyph.
        .child(
            div()
                .size(z(48.))
                .rounded(z(8.))
                .bg(c(tok(idx::BACKGROUND_MOD_MUTED)))
                .flex()
                .items_center()
                .justify_center()
                .child(gpui::svg().path(crate::icons::path(icon)).size(z(24.)).text_color(c(tok(idx::ICON_DEFAULT)))),
        );
    div()
        .id(eid(format!("rel-{title}")))
        .flex()
        .gap(z(12.))
        .mx(z(-12.))
        .p(z(12.))
        .min_h(z(48.))
        .rounded(z(16.))
        .cursor_pointer()
        .hover(|s| s.bg(c(tok(idx::INTERACTIVE_BACKGROUND_HOVER))))
        .on_click(cx.listener(move |a, _, _, cx| {
            if let Some(s) = a.settings.as_mut() {
                s.page = page;
                s.open_select = None;
            }
            cx.notify();
        }))
        .child(lead)
        .child(div().flex_1().min_w_0().flex().flex_col().gap(z(4.)).child(title_text(tr(title))).child(desc_text(tr(desc))))
        .child(
            div().flex_none().flex().items_center().child(
                gpui::svg().path(crate::icons::path("chevron-right")).size(z(24.)).text_color(c(tok(idx::ICON_DEFAULT))),
            ),
        )
        .into_any_element()
}

fn render_block(app: &PhoenixApp, b: &'static B, out: &mut Vec<AnyElement>, cx: &mut Cx) {
    match b {
        B::Title(t) => out.push(controls::section_title(tr(t)).into_any_element()),
        B::Lead(t) => out.push(div().mt(z(-20.)).pb(z(12.)).child(desc_text(tr(t))).into_any_element()),
        B::Legend(t) => out.push(
            div()
                .pt(z(10.))
                .pb(z(4.))
                .text_size(z(16.))
                .line_height(z(20.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c(tok(idx::TEXT_STRONG)))
                .child(tr(t))
                .into_any_element(),
        ),
        B::Switch(id, t, d, bind) => {
            let desc = (!d.is_empty()).then(|| desc_text(tr(d)).into_any_element());
            out.push(switch_row(app, id, tr(t), desc, *bind, cx));
        }
        B::Sound(id, t) => {
            // the preview link (anchor_R6ReNl, 14/18 --text-link)
            let link = div().text_size(z(14.)).line_height(z(18.)).text_color(c(tok(idx::TEXT_LINK))).child(tr("sp.sound_preview"));
            out.push(switch_row(app, id, tr(t), Some(link.into_any_element()), Bind::Ui(true), cx));
        }
        B::Radios(id, t, d, opts, p) => {
            let mut col = div().flex().flex_col().py(z(12.));
            if !t.is_empty() {
                col = col.child(title_text(tr(t)).pb(z(if d.is_empty() { 8. } else { 4. })));
            }
            if !d.is_empty() {
                col = col.child(desc_text(tr(d)).pb(z(8.)));
            }
            out.push(col.child(radio_group(app, id, opts, *p, cx)).into_any_element());
        }
        B::Select(id, t, d, opts, p) => {
            let cur = get_pick(app, id, *p);
            let (id, p) = (*id, *p);
            let opts: Vec<SharedString> = opts.iter().map(|o| tr(o)).collect();
            let dd = dropdown(app, id, opts, cur, 280., move |a, i, cx| set_pick(a, id, p, i, cx), cx);
            let desc = (!d.is_empty()).then(|| desc_text(tr(d)).into_any_element());
            out.push(item(tr(t), desc, Some(dd), false).into_any_element());
        }
        B::Action(t, d, label, danger) => {
            let kind = BtnKind::Secondary;
            let btn = controls::button(eid(format!("act-{t}")), tr(label), kind)
                .when(*danger, |b| b.text_color(c(tok(idx::TEXT_FEEDBACK_CRITICAL))))
                .into_any_element();
            let desc = (!d.is_empty()).then(|| desc_text(tr(d)).into_any_element());
            out.push(item(tr(t), desc, Some(btn), false).into_any_element());
        }
        B::Related(icon, t, d, page) => {
            {
                out.push(
                    div()
                        .pt(z(12.))
                        .pb(z(2.))
                        .text_size(z(16.))
                        .line_height(z(20.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(tok(idx::TEXT_MUTED)))
                        .child(tr("sp.related"))
                        .into_any_element(),
                );
            }
            out.push(related(icon, t, d, *page, cx));
        }
        B::Expand(id, t, d, inner) => {
            let open = app.settings.as_ref().is_some_and(|s| s.expanded.contains(id));
            let id = *id;
            out.push(
                item(
                    tr(t),
                    Some(desc_text(tr(d)).into_any_element()),
                    Some(
                        gpui::svg()
                            .path(crate::icons::path("chevron"))
                            .size(z(24.))
                            .text_color(c(tok(idx::ICON_DEFAULT)))
                            .when(open, |s| s.with_transformation(gpui::Transformation::rotate(gpui::radians(std::f32::consts::PI))))
                            .into_any_element(),
                    ),
                    false,
                )
                .id(eid(format!("exp-{id}")))
                .cursor_pointer()
                .hover(|s| s.bg(c(tok(idx::INTERACTIVE_BACKGROUND_HOVER))))
                .on_click(cx.listener(move |a, _, _, cx| {
                    if let Some(s) = a.settings.as_mut()
                        && !s.expanded.remove(id)
                    {
                        s.expanded.insert(id);
                    }
                    cx.notify();
                }))
                .into_any_element(),
            );
            if open {
                for b in inner.iter() {
                    render_block(app, b, out, cx);
                }
            }
        }
        B::KeyGroup(l, d, rows) => out.push(key_group(l, d, rows).into_any_element()),
        B::Note(t) => out.push(div().py(z(8.)).child(desc_text(tr(t))).into_any_element()),
        B::Divider => out.push(div().my(z(50.)).h(z(1.)).bg(c(tok(idx::BORDER_SUBTLE))).into_any_element()),
        B::Custom(_, f) => out.push(f(app, cx)),
    }
}

/// Renders a page's blocks (rows 10px apart like the stacked containers).
pub fn render(app: &PhoenixApp, blocks: &'static [B], cx: &mut Cx) -> Div {
    let mut out = Vec::new();
    for b in blocks {
        render_block(app, b, &mut out, cx);
    }
    div().flex().flex_col().gap(z(10.)).children(out)
}

// ------------------------------------------------------------------ search

fn push_texts(blocks: &'static [B], out: &mut Vec<&'static str>) {
    for b in blocks {
        match b {
            B::Title(t) | B::Lead(t) | B::Legend(t) | B::Note(t) | B::Sound(_, t) | B::Custom(t, _) => out.push(t),
            B::Switch(_, t, d, _) | B::Action(t, d, _, _) | B::Related(_, t, d, _) => out.extend([*t, *d]),
            B::Radios(_, t, d, o, _) => {
                out.extend([*t, *d]);
                out.extend(o.iter().flat_map(|(a, b)| [*a, *b]));
            }
            B::Select(_, t, d, o, _) => {
                out.extend([*t, *d]);
                out.extend(o.iter().copied());
            }
            B::Expand(_, t, d, inner) => {
                out.extend([*t, *d]);
                push_texts(inner, out);
            }
            B::KeyGroup(l, d, rows) => {
                out.extend([*l, *d]);
                out.extend(rows.iter().map(|r| r.0));
            }
            B::Divider => {}
        }
    }
}

/// Category titles of a page (sidebar sub-navigation under the active item).
pub fn titles(p: Page) -> Vec<SharedString> {
    blocks(p).iter().filter_map(|b| if let B::Title(t) = b { Some(tr(t)) } else { None }).collect()
}

/// True when the page label or any of its texts contains `q` (lowercase).
pub fn matches(p: Page, q: &str) -> bool {
    if q.is_empty() || p.label().to_lowercase().contains(q) {
        return true;
    }
    let mut keys = Vec::new();
    push_texts(blocks(p), &mut keys);
    keys.extend(extra_search(p).iter().copied());
    keys.iter().filter(|k| !k.is_empty()).any(|k| tr(k).to_lowercase().contains(q))
}

fn extra_search(p: Page) -> &'static [&'static str] {
    match p {
        Page::MyAccount => &[
            "settings.account.info",
            "settings.account.display_name",
            "settings.account.username",
            "settings.account.email",
            "settings.account.phone",
            "settings.account.security",
            "settings.account.password",
            "settings.account.status",
        ],
        Page::Profiles => &["sp.pf.nameplate", "sp.pf.avatar", "sp.pf.name_style", "sp.pf.theme", "sp.pf.effect"],
        Page::VoiceVideo => &[
            "sp.vo.t1",
            "sp.vo.t2",
            "voice.input_device",
            "voice.output_device",
            "voice.input_volume",
            "voice.output_volume",
            "voice.mic_test",
            "voice.input_mode",
            "voice.sensitivity",
            "voice.echo",
            "voice.noise",
            "voice.agc",
            "voice.attenuation",
        ],
        Page::Devices => &["settings.devices.current"],
        _ => &[],
    }
}

// ------------------------------------------------------------------- pages

const NONE: &[B] = &[];

pub fn blocks(p: Page) -> &'static [B] {
    match p {
        Page::DataPrivacy => DATA,
        Page::PrivacySafety => CONTENT,
        Page::Notifications => NOTIF,
        Page::Appearance => APPEARANCE,
        Page::Accessibility => A11Y,
        Page::System | Page::Keybinds => SYSTEM,
        Page::Language => LANGUAGE,
        Page::ActivityPrivacy => ACTIVITY,
        Page::Connections => CONNECTIONS,
        Page::Advanced => DEVELOPER,
        Page::Profiles => PROFILES,
        Page::RegisteredGames => GAMES,
        Page::Nitro => NITRO,
        Page::ServerBoost => BOOST,
        Page::Subscriptions => SUBSCRIPTIONS,
        Page::GiftInventory => GIFTS,
        Page::Billing => BILLING,
        _ => NONE,
    }
}

// "Jeux enregistrés" (vanilla-settings-12 dump): Current Game / Added Games
// with the IGDB note, then Related Settings → Activity Privacy.
static GAMES: &[B] = &[
    B::Title("sp.rg.current"),
    B::Note("sp.rg.none"),
    B::Action("sp.rg.cant_find", "", "sp.rg.add", false),
    B::Divider,
    B::Title("sp.rg.added"),
    B::Lead("sp.rg.igdb"),
    B::Divider,
    B::Related("user-profile", "settings.page.activity", "sp.rg.related_d", Page::ActivityPrivacy),
];

// Billing pages: Phoenix doesn't handle payments, each page points to the web.
static NITRO: &[B] = &[B::Title("settings.page.nitro"), B::Lead("sp.billing.web")];
static BOOST: &[B] = &[B::Title("settings.page.server_boost"), B::Lead("sp.billing.web")];
static SUBSCRIPTIONS: &[B] = &[B::Title("settings.page.subscriptions"), B::Lead("sp.billing.web")];
static GIFTS: &[B] = &[B::Title("settings.page.gift_inventory"), B::Lead("sp.billing.web")];
static BILLING: &[B] = &[B::Title("settings.page.billing"), B::Lead("sp.billing.web")];

static DATA: &[B] = &[
    B::Title("sp.data.t1"),
    B::Custom("sp.data.core", |_, _| {
        item(tr("sp.data.core"), Some(desc_text(tr("sp.data.core_d")).into_any_element()), None, false).into_any_element()
    }),
    B::Switch("data-improve", "sp.data.improve", "sp.data.improve_d", Bind::Ui(true)),
    B::Switch("data-personalize", "sp.data.personalize", "sp.data.personalize_d", Bind::Ui(true)),
    B::Switch("data-sponsored", "sp.data.sponsored", "sp.data.sponsored_d", Bind::Ui(true)),
    B::Switch("data-third", "sp.data.third", "sp.data.third_d", Bind::Ui(false)),
    B::Switch("data-discovery", "sp.data.discovery", "sp.data.discovery_d", Bind::Ui(true)),
    B::Switch("data-clips", "sp.data.clips", "sp.data.clips_d", Bind::Ui(true)),
    B::Action("sp.data.request", "sp.data.request_d", "sp.data.request_b", false),
    B::Related("apps", "sp.games", "sp.games_d", Page::ActivityPrivacy),
    B::Divider,
    B::Title("sp.data.t2"),
    B::Lead("sp.data.t2_d"),
    B::Radios(
        "share-profile",
        "sp.data.share_profile",
        "",
        &[("sp.data.sp_all", "sp.data.sp_all_d"), ("sp.data.sp_small", "sp.data.sp_small_d"), ("sp.data.sp_friends", "sp.data.sp_friends_d")],
        Pick::Ui(0),
    ),
    B::Switch("share-update", "sp.data.share_update", "sp.data.share_update_d", Bind::Ui(true)),
    B::Related("user-profile", "settings.page.activity", "sp.activity_related_d", Page::ActivityPrivacy),
    B::Divider,
    B::Title("sp.data.t3"),
    B::Switch("e2ee-codes", "sp.data.codes", "sp.data.codes_d", Bind::Ui(false)),
];

const SHOW_BLOCK: &[&str] = &["sp.cs.show", "sp.cs.blur", "sp.cs.block"];

static CONTENT: &[B] = &[
    B::Title("sp.cs.t1"),
    B::Lead("sp.cs.sensitive_d"),
    B::Select("cf-friends", "sp.cs.f_friends", "", SHOW_BLOCK, Pick::Ui(0)),
    B::Select("cf-others", "sp.cs.f_others", "", SHOW_BLOCK, Pick::Ui(2)),
    B::Select("cf-servers", "sp.cs.f_servers", "", SHOW_BLOCK, Pick::Ui(0)),
    B::Switch("age-cmds", "sp.cs.age_cmds", "sp.cs.age_cmds_d", Bind::Ui(false)),
    B::Switch("age-ios", "sp.cs.age_ios", "sp.cs.age_ios_d", Bind::Ui(false)),
    B::Related("image", "settings.page.appearance", "sp.cs.appearance_d", Page::Appearance),
    B::Divider,
    B::Title("sp.cs.t2"),
    B::Radios("spam", "sp.cs.spam", "sp.cs.spam_d", &[("sp.cs.spam_all", ""), ("sp.cs.spam_nf", ""), ("sp.cs.spam_none", "")], Pick::Ui(1)),
    B::Divider,
    B::Title("sp.cs.t3"),
    B::Switch(
        "server-dms",
        "sp.cs.dms",
        "sp.cs.dms_d",
        Bind::Synced(|s| s.allow_server_dms, SettingsPatch::AllowServerDms),
    ),
    B::Switch("msg-requests", "sp.cs.requests", "sp.cs.requests_d", Bind::Ui(true)),
    B::Divider,
    B::Title("sp.cs.t4"),
    B::Lead("sp.cs.t4_d"),
    B::Legend("sp.cs.fr_from"),
    B::Switch("fr-everyone", "sp.cs.fr_everyone", "", Bind::Ui(true)),
    B::Switch("fr-fof", "sp.cs.fr_fof", "", Bind::Ui(true)),
    B::Switch("fr-members", "sp.cs.fr_members", "sp.cs.fr_members_d", Bind::Ui(true)),
    B::Switch("fr-msg", "sp.cs.fr_msg", "sp.cs.fr_msg_d", Bind::Ui(true)),
    B::Divider,
    B::Title("sp.cs.t5"),
    B::Lead("sp.cs.t5_d"),
    B::Switch("game-dms", "sp.cs.game_dms", "sp.cs.game_dms_d", Bind::Ui(true)),
    B::Radios("ingame", "sp.cs.ingame", "sp.cs.ingame_d", &[("sp.cs.ig_all", ""), ("sp.cs.ig_players", ""), ("sp.cs.ig_none", "")], Pick::Ui(0)),
    B::Related("apps", "sp.cs.authorized", "sp.cs.authorized_d", Page::Connections),
    B::Divider,
    B::Title("sp.cs.t6"),
    B::Lead("sp.cs.t6_d"),
    B::Custom("sp.cs.blocked", |_, _| {
        item(tr("sp.cs.blocked"), Some(desc_text(tr("sp.cs.blocked_d")).into_any_element()), None, false).into_any_element()
    }),
];

static NOTIF: &[B] = &[
    B::Title("sp.no.t1"),
    B::Switch(
        "desktop-notif",
        "sp.no.desktop",
        "sp.no.desktop_d",
        Bind::Local(|p| p.desktop_notifications, |p, v| p.desktop_notifications = v),
    ),
    B::Legend("sp.no.when"),
    B::Switch("n-stream", "sp.no.stream", "", Bind::Ui(true)),
    B::Switch("n-anniv", "sp.no.anniv", "", Bind::Ui(true)),
    B::Switch("n-online", "sp.no.online", "", Bind::Ui(false)),
    B::Switch("n-event", "sp.no.event", "", Bind::Ui(true)),
    B::Switch("n-profile", "sp.no.profile", "", Bind::Ui(true)),
    B::Select("n-react", "sp.no.react", "", &["sp.no.r_all", "sp.no.r_dm", "sp.no.r_never"], Pick::Ui(0)),
    B::Divider,
    B::Title("sp.no.t2"),
    B::Sound("snd-msg", "sp.no.msg"),
    B::Sound("snd-msg-cur", "sp.no.msg_cur"),
    B::Sound("snd-ring", "sp.no.ring"),
    B::Switch(
        "notif-sounds",
        "sp.no.mute_all",
        "sp.no.mute_all_d",
        Bind::Local(|p| !p.notification_sounds, |p, v| p.notification_sounds = !v),
    ),
    B::Related("speaker", "settings.page.voice_video", "sp.no.voice_d", Page::VoiceVideo),
    B::Divider,
    B::Title("sp.no.t3"),
    B::Switch("badge", "sp.no.badge", "sp.no.badge_d", Bind::Ui(true)),
    B::Divider,
    B::Title("sp.no.t4"),
    B::Switch("e-comm", "sp.no.e_comm", "sp.no.e_comm_d", Bind::Ui(true)),
    B::Switch("e-social", "sp.no.e_social", "sp.no.e_social_d", Bind::Ui(true)),
    B::Switch("e-news", "sp.no.e_news", "sp.no.e_news_d", Bind::Ui(false)),
    B::Switch("e-tips", "sp.no.e_tips", "sp.no.e_tips_d", Bind::Ui(false)),
    B::Switch("e-recs", "sp.no.e_recs", "sp.no.e_recs_d", Bind::Ui(false)),
    B::Action("sp.no.unsub", "", "sp.no.unsub_b", true),
    B::Divider,
    B::Title("sp.no.t5"),
    B::Expand(
        "notif-adv",
        "sp.no.adv",
        "sp.no.adv_d",
        &[
            B::Select("push-delay", "sp.no.push", "sp.no.push_d", &["sp.no.min1", "sp.no.min5", "sp.no.min10"], Pick::Ui(0)),
            B::Switch("tts", "sp.no.tts", "sp.no.tts_d", Bind::Ui(true)),
            B::Radios("read-all", "sp.no.read_all", "sp.no.read_all_d", &[("sp.no.ra_all", ""), ("sp.no.ra_cur", ""), ("sp.no.r_never", "")], Pick::Ui(2)),
        ],
    ),
];

fn theme_row(app: &PhoenixApp, cx: &mut Context<'_, PhoenixApp>) -> AnyElement {
    let theme = app.prefs.synced.theme;
    div()
        .flex()
        .flex_col()
        .pt(z(8.))
        // vanilla-settings-08 #2721 heading-sm/semibold 16/20, swatches 24px under it.
        .child(title_text(tr("sp.ap.defaults")).font_weight(FontWeight::SEMIBOLD).pb(z(24.)))
        .child(div().flex().gap(z(8.)).children(AppTheme::ALL.map(|t| {
            super::view::theme_card(t, t == theme).on_click(cx.listener(move |a, _, _, cx| save(a, SettingsPatch::Theme(t), cx)))
        })))
        .into_any_element()
}

static APPEARANCE: &[B] = &[
    B::Title("sp.ap.t1"),
    B::Switch("theme-sync", "sp.ap.sync", "sp.ap.sync_d", Bind::Ui(false)),
    B::Custom("sp.ap.defaults", theme_row),
    B::Switch("theme-devices", "sp.ap.sync_devices", "", Bind::Ui(true)),
    B::Switch("theme-profiles", "sp.ap.apply_profiles", "", Bind::Ui(true)),
    B::Related("help", "settings.page.accessibility", "sp.ap.a11y_d", Page::Accessibility),
    B::Divider,
    B::Title("sp.ap.t2"),
    B::Legend("sp.ap.media"),
    B::Switch("media-links", "sp.ap.links", "", Bind::Ui(true)),
    B::Switch("media-uploads", "sp.ap.uploads", "", Bind::Ui(true)),
    B::Switch("embeds", "sp.ap.embeds", "", Bind::Ui(true)),
    B::Switch("reactions", "sp.ap.reactions", "", Bind::Ui(true)),
    B::Select("spoilers", "sp.ap.spoilers", "", &["sp.ap.sp_click", "sp.ap.sp_always", "sp.ap.sp_mod"], Pick::Ui(0)),
    B::Switch("split-threads", "sp.ap.split", "", Bind::Ui(true)),
    B::Switch("avatars", "sp.ap.avatars", "sp.ap.avatars_d", Bind::Ui(true)),
    B::Switch("favs", "sp.ap.favs", "sp.ap.favs_d", Bind::Ui(true)),
    B::Related("help", "settings.page.accessibility", "sp.ap.density_d", Page::Accessibility),
    B::Divider,
    B::Title("sp.ap.t3"),
    B::Switch("md-preview", "sp.ap.preview", "", Bind::Ui(true)),
    B::Switch("emoticons", "sp.ap.emoticons", "sp.ap.emoticons_d", Bind::Ui(true)),
    B::Switch("ac-stickers", "sp.ap.stickers", "", Bind::Ui(true)),
    B::Switch("ac-games", "sp.ap.games_ac", "", Bind::Ui(true)),
    B::Switch("send-btn", "sp.ap.send_btn", "", Bind::Ui(false)),
    B::Divider,
    B::Title("sp.ap.t4"),
    B::Radios("dm-search", "sp.ap.dm_search", "", &[("sp.ap.dm_sel", ""), ("sp.ap.dm_all", "")], Pick::Ui(0)),
    B::Divider,
    B::Title("sp.ap.t5"),
    B::Switch("streamer", "sp.ap.streamer", "sp.ap.streamer_d", Bind::Ui(false)),
    B::Switch("streamer-auto", "sp.ap.streamer_auto", "", Bind::Ui(true)),
    B::Legend("sp.ap.streamer_when"),
    B::Switch("st-info", "sp.ap.hide_info", "", Bind::Ui(true)),
    B::Switch("st-invites", "sp.ap.hide_invites", "", Bind::Ui(true)),
    B::Switch("st-sounds", "sp.ap.no_sounds", "", Bind::Ui(true)),
    B::Switch("st-notifs", "sp.ap.no_notifs", "", Bind::Ui(true)),
];

fn slider_row(title: &'static str, desc: &'static str, s: Div) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .py(z(12.))
        .gap(z(4.))
        .child(title_text(tr(title)))
        .child(desc_text(tr(desc)))
        .child(div().pt(z(8.)).child(s))
        .into_any_element()
}

fn density_idx(p: &LocalPrefs) -> usize {
    crate::theme::Density::ALL.iter().position(|d| *d == p.density).unwrap_or(0)
}

static A11Y: &[B] = &[
    B::Title("sp.ac.t1"),
    B::Custom("sp.ac.font", |app, cx| {
        let fs = app.prefs.local.font_size;
        slider_row(
            "sp.ac.font",
            "sp.ac.font_d",
            super::view::slider("font", &[12., 14., 15., 16., 18., 20., 24.], "px", fs, 16., |a, v, cx| update_local(a, |p| p.font_size = v, cx), cx),
        )
    }),
    B::Switch("underline", "sp.ac.underline", "sp.ac.underline_d", Bind::Ui(false)),
    B::Switch("name-styles", "sp.ac.name_styles", "sp.ac.name_styles_d", Bind::Ui(true)),
    B::Divider,
    B::Title("sp.ac.t2"),
    // Compacte / Par défaut / Spacieuse = Density::ALL order
    B::Radios(
        "density",
        "settings.appearance.density",
        "sp.ac.density_d",
        &[("settings.density.compact", ""), ("settings.density.default", ""), ("settings.density.cozy", "")],
        Pick::Local(density_idx, |p, i| p.density = crate::theme::Density::ALL[i.min(2)]),
    ),
    B::Radios(
        "display",
        "sp.ac.display",
        "sp.ac.display_d",
        &[("sp.ac.d_default", ""), ("sp.ac.d_compact", "")],
        Pick::Synced(|s| s.compact as usize, |i| SettingsPatch::Compact(i == 1)),
    ),
    B::Custom("sp.ac.zoom", |app, cx| {
        let zoom = app.prefs.local.zoom as f32;
        slider_row(
            "sp.ac.zoom",
            "sp.ac.zoom_d",
            super::view::slider(
                "zoom",
                &[50., 67., 75., 80., 90., 100., 110., 125., 150., 175., 200.],
                "",
                zoom,
                100.,
                |a, v, cx| update_local(a, |p| p.zoom = v as _, cx),
                cx,
            ),
        )
    }),
    B::Divider,
    B::Title("sp.ac.t3"),
    B::Custom("sp.ac.sat", |app, cx| {
        let sat = app.prefs.local.saturation as f32;
        slider_row(
            "sp.ac.sat",
            "sp.ac.sat_d",
            super::view::slider(
                "sat",
                &[0., 10., 20., 30., 40., 50., 60., 70., 80., 90., 100.],
                "%",
                sat,
                100.,
                |a, v, cx| update_local(a, |p| p.saturation = v as _, cx),
                cx,
            ),
        )
    }),
    B::Switch("sat-custom", "sp.ac.sat_custom", "sp.ac.sat_custom_d", Bind::Ui(false)),
    B::Switch("contrast", "sp.ac.contrast", "sp.ac.contrast_d", Bind::Ui(false)),
    B::Select("roles", "sp.ac.roles", "sp.ac.roles_d", &["sp.ac.r_names", "sp.ac.r_next", "sp.ac.r_none"], Pick::Ui(0)),
    B::Switch("indicators", "sp.ac.indicators", "sp.ac.indicators_d", Bind::Ui(false)),
    B::Related("emoji", "settings.page.appearance", "sp.ac.appearance_d", Page::Appearance),
    B::Divider,
    B::Title("sp.ac.t4"),
    B::Switch("reduced-motion", "sp.ac.motion", "sp.ac.motion_d", Bind::Local(|p| p.reduced_motion, |p, v| p.reduced_motion = v)),
    B::Switch("motion-sync", "sp.ac.motion_sync", "", Bind::Ui(true)),
    B::Switch("gifs", "sp.ac.gifs", "", Bind::Ui(true)),
    B::Switch("anim-emoji", "sp.ac.anim_emoji", "", Bind::Ui(true)),
    B::Radios(
        "stickers",
        "sp.ac.stickers",
        "",
        &[("sp.ac.st_always", ""), ("sp.ac.st_inter", "sp.ac.st_inter_d"), ("sp.ac.st_never", "")],
        Pick::Ui(0),
    ),
    B::Divider,
    B::Title("sp.ac.t5"),
    B::Switch("alt-text", "sp.ac.alt", "sp.ac.alt_d", Bind::Ui(false)),
    B::Switch("legacy-input", "sp.ac.legacy", "sp.ac.legacy_d", Bind::Ui(false)),
];

static SYSTEM: &[B] = &[
    B::Title("sp.sy.t1"),
    B::Switch("startup", "sp.sy.startup", "", Bind::Ui(false)),
    B::Switch("tray", "sp.sy.tray", "sp.sy.tray_d", Bind::Ui(true)),
    B::Switch("hwaccel", "sp.sy.hwaccel", "sp.sy.hwaccel_d", Bind::Ui(true)),
    B::Divider,
    B::Title("sp.sy.t2"),
    B::Action("sp.sy.t2", "sp.sy.t2_d", "sp.sy.add_kb", false),
    B::Note("sp.sy.no_kb"),
    B::Divider,
    B::Title("sp.sy.t3"),
    B::KeyGroup("", "", &[("sp.kb.show", &["ctrl", "/"])]),
    B::KeyGroup(
        "sp.kb.g_msg",
        "sp.kb.g_msg_d",
        &[
            ("sp.kb.edit", &["e"]),
            ("sp.kb.delete", &["backspace"]),
            ("sp.kb.pin", &["p"]),
            ("sp.kb.react", &["+"]),
            ("sp.kb.reply", &["r"]),
            ("sp.kb.forward", &["f"]),
            ("sp.kb.copy", &["ctrl", "c"]),
            ("sp.kb.unread", &["alt", "enter"]),
            ("sp.kb.focus", &["esc"]),
        ],
    ),
    B::KeyGroup(
        "sp.kb.g_nav",
        "",
        &[
            ("sp.kb.servers", &["ctrl", "alt", "↑ ↓"]),
            ("sp.kb.channels", &["alt", "↑ ↓"]),
            ("sp.kb.history", &["alt", "← →"]),
            ("sp.kb.unreads", &["alt", "shift", "↑ ↓"]),
            ("sp.kb.mentions", &["ctrl", "shift", "alt", "↑ ↓"]),
            ("sp.kb.call", &["ctrl", "shift", "alt", "v"]),
            ("sp.kb.last", &["ctrl", "alt", "→"]),
            ("sp.kb.switcher", &["ctrl", "k"]),
            ("sp.kb.create", &["ctrl", "shift", "n"]),
        ],
    ),
    B::KeyGroup(
        "sp.kb.g_chat",
        "",
        &[
            ("sp.kb.server_read", &["shift", "esc"]),
            ("sp.kb.channel_read", &["esc"]),
            ("sp.kb.group", &["ctrl", "shift", "t"]),
            ("sp.kb.pins", &["ctrl", "p"]),
            ("sp.kb.inbox", &["ctrl", "i"]),
            ("sp.kb.members", &["ctrl", "u"]),
            ("sp.kb.emoji", &["ctrl", "e"]),
            ("sp.kb.gif", &["ctrl", "g"]),
            ("sp.kb.sticker", &["ctrl", "s"]),
            ("sp.kb.scroll", &["page up", "page down"]),
            ("sp.kb.oldest", &["shift", "page up"]),
            ("sp.kb.upload", &["ctrl", "shift", "u"]),
        ],
    ),
    B::KeyGroup(
        "sp.kb.g_voice",
        "",
        &[
            ("sp.kb.mute", &["ctrl", "shift", "m"]),
            ("sp.kb.deafen", &["ctrl", "shift", "d"]),
            ("sp.kb.answer", &["ctrl", "enter"]),
            ("sp.kb.decline", &["esc"]),
        ],
    ),
    B::KeyGroup(
        "sp.kb.g_misc",
        "",
        &[("sp.kb.help", &["ctrl", "shift", "h"]), ("sp.kb.search", &["ctrl", "f"]), ("sp.kb.settings", &["ctrl", ","])],
    ),
    B::Divider,
    B::Title("sp.sy.t4"),
    B::Action("sp.sy.perf", "sp.sy.perf_d", "sp.sy.capture", false),
];

fn language_row(app: &PhoenixApp, cx: &mut Context<'_, PhoenixApp>) -> AnyElement {
    let cur = app.prefs.synced.locale.clone();
    let ix = super::view::LOCALES.iter().position(|(c, _)| *c == cur.as_ref()).unwrap_or(0);
    let opts = super::view::LOCALES.iter().map(|(_, n)| SharedString::from(*n)).collect();
    let dd = dropdown(
        app,
        "language",
        opts,
        ix,
        696.,
        |a, i, cx| {
            if let Some(s) = a.settings.as_mut() {
                s.open_select = None;
            }
            save(a, SettingsPatch::Locale(super::view::LOCALES[i].0.into()), cx)
        },
        cx,
    );
    div()
        .flex()
        .flex_col()
        .py(z(12.))
        .gap(z(4.))
        .child(title_text(tr("sp.la.select")))
        .child(desc_text(tr("sp.la.select_d")))
        .child(div().pt(z(8.)).child(dd))
        .into_any_element()
}

static LANGUAGE: &[B] = &[
    B::Custom("sp.la.select", language_row),
    B::Radios("time-format", "sp.la.time", "", &[("sp.la.auto", ""), ("sp.la.h12", ""), ("sp.la.h24", "")], Pick::Ui(0)),
];

static ACTIVITY: &[B] = &[
    B::Title("sp.at.t1"),
    B::Switch("act-share", "sp.at.share", "sp.at.share_d", Bind::Ui(true)),
    B::Switch("act-online", "sp.at.online", "sp.at.online_d", Bind::Ui(true)),
    B::Switch("act-stream", "sp.at.stream", "sp.at.stream_d", Bind::Ui(true)),
    B::Related("apps", "sp.games", "sp.games_d", Page::ActivityPrivacy),
    B::Divider,
    B::Title("sp.at.t2"),
    B::Radios("act-where", "sp.at.auto", "sp.at.auto_d", &[("sp.at.a_all", ""), ("sp.at.a_small", ""), ("sp.at.a_none", "")], Pick::Ui(0)),
    B::Related("lock", "settings.page.privacy", "sp.data.t2_d", Page::DataPrivacy),
    B::Divider,
    B::Title("sp.at.t3"),
    B::Lead("sp.at.t3_d"),
    B::Switch("act-friends", "sp.at.friends", "sp.at.friends_d", Bind::Ui(true)),
    B::Switch("act-voice", "sp.at.voice", "sp.at.voice_d", Bind::Ui(true)),
];

fn connection_tiles(_: &PhoenixApp, _: &mut Context<'_, PhoenixApp>) -> AnyElement {
    let tile = |name: &'static str| {
        div()
            .h(z(40.))
            .px(z(12.))
            .rounded(z(8.))
            .border_1()
            .border_color(c(tok(idx::BORDER_SUBTLE)))
            .bg(c(tok(idx::BACKGROUND_MOD_SUBTLE)))
            .flex()
            .items_center()
            .text_size(z(14.))
            .line_height(z(18.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(c(tok(idx::TEXT_DEFAULT)))
            .child(name)
    };
    div()
        .flex()
        .flex_col()
        .py(z(12.))
        .gap(z(12.))
        .child(title_text(tr("sp.cn.add")))
        .child(div().flex().flex_wrap().gap(z(8.)).children(
            ["Battle.net", "Bluesky", "Crunchyroll", "Epic Games", "GitHub", "PlayStation", "Reddit", "Riot Games", "Spotify", "Steam", "Twitch", "X", "Xbox", "YouTube"]
                .map(tile),
        ))
        .into_any_element()
}

static CONNECTIONS: &[B] = &[
    B::Title("sp.cn.t1"),
    B::Lead("sp.cn.t1_d"),
    B::Custom("sp.cn.add", connection_tiles),
    B::Divider,
    B::Title("sp.cn.t2"),
    B::Lead("sp.cn.t2_d"),
    B::Note("sp.cn.none"),
];


// Vanilla Developer page (vanilla-settings-15): Developer Mode and
// Application Test Mode switches only.
static DEVELOPER: &[B] = &[
    B::Switch(
        "dev-mode",
        "settings.advanced.dev_mode",
        "sp.dv.mode_d",
        Bind::Synced(|s| s.developer_mode, SettingsPatch::DeveloperMode),
    ),
    B::Switch("app-test", "sp.dv.test", "sp.dv.test_d", Bind::Ui(false)),
];

static PROFILES: &[B] = &[B::Custom("sp.pf.main", |app, _| super::view::profiles(app).into_any_element())];
