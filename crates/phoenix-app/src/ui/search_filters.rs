//! "More filters" modal of the search popout (live dump search-more-filters,
//! 2026-11-24): the official client opens a `container_dvN-MP size-md` dialog
//! titled "Filtres" with one labelled field per filter (De, Dans, Contient,
//! Mentions, Date, Type d'auteur, Épinglé) and an action bar
//! "Effacer les filtres" / "Annuler" / "Appliquer les filtres".
//! Applying turns the fields into `key:value` tokens appended to the query.

use gpui::{AnyElement, App, Context, Entity, FontWeight, ParentElement, SharedString, Styled, Window, div, prelude::*};
use gpui_component::input::{Input, InputState};

use crate::{
    app::PhoenixApp,
    theme::{c, generated::idx, tok, z},
    ui::controls::{BtnKind, button},
};

/// (search key, label, description) in dump order.
const FIELDS: [(&str, &str, &str); 6] = [
    ("from:", "search.m_from", "search.m_from_desc"),
    ("in:", "search.m_in", "search.m_in_desc"),
    ("has:", "search.m_has", "search.m_has_desc"),
    ("mentions:", "search.m_mentions", "search.m_mentions_desc"),
    ("authorType:", "search.m_author", "search.m_author_desc"),
    ("pinned:", "search.m_pinned", "search.m_pinned_desc"),
];

pub struct FiltersModal {
    fields: Vec<Entity<InputState>>,
    /// "Ajouter une date" turns into a `during:` date field.
    date: Option<Entity<InputState>>,
}

#[derive(Default)]
pub struct Filters(pub Option<FiltersModal>);

impl gpui::Global for Filters {}

pub fn is_open(cx: &App) -> bool {
    cx.try_global::<Filters>().is_some_and(|f| f.0.is_some())
}

fn placeholder(app: &PhoenixApp, key: &str) -> SharedString {
    // Dump placeholders: "p. ex. : <user>", "p.ex. <current channel>",
    // "N'importe quel contenu", "p. ex. : Vrai".
    match key {
        "from:" | "mentions:" => crate::t!("search.m_ph_user", u = app.me.name),
        "in:" => {
            let ch = app.selected_channel.and_then(|c| app.channels.get(&c)).map(|c| c.name.to_string()).unwrap_or_default();
            crate::t!("search.m_ph_channel", c = ch)
        }
        "has:" => crate::t!("search.m_ph_has"),
        "authorType:" => crate::t!("search.m_ph_author"),
        _ => crate::t!("search.m_ph_pinned"),
    }
}

pub fn open(app: &mut PhoenixApp, window: &mut Window, cx: &mut Context<PhoenixApp>) {
    let mut fields = Vec::with_capacity(FIELDS.len());
    for (k, _, _) in FIELDS {
        let ph = placeholder(app, k);
        fields.push(cx.new(|cx| InputState::new(window, cx).placeholder(ph)));
    }
    cx.default_global::<Filters>().0 = Some(FiltersModal { fields, date: None });
    cx.notify();
}

pub fn close(cx: &mut Context<PhoenixApp>) {
    cx.default_global::<Filters>().0 = None;
    cx.notify();
}

/// Normalise one field value for its key (French or English words accepted).
fn value_for(key: &str, v: &str) -> Option<String> {
    let l = v.trim().to_lowercase();
    if l.is_empty() {
        return None;
    }
    Some(match key {
        "pinned:" => match l.as_str() {
            "vrai" | "oui" | "true" | "yes" => "true".into(),
            "faux" | "non" | "false" | "no" => "false".into(),
            _ => return None,
        },
        "authorType:" => match l.as_str() {
            "utilisateur" | "user" => "user".into(),
            "bot" => "bot".into(),
            "webhook" => "webhook".into(),
            _ => return None,
        },
        _ if l.contains(' ') => format!("\"{}\"", v.trim()),
        _ => v.trim().to_string(),
    })
}

/// The tokens the modal adds to the query.
fn tokens(m: &FiltersModal, cx: &App) -> String {
    let mut out = Vec::new();
    for ((key, _, _), f) in FIELDS.iter().zip(&m.fields) {
        for part in f.read(cx).value().split(',') {
            if let Some(v) = value_for(key, part) {
                out.push(format!("{key}{v}"));
            }
        }
    }
    if let Some(d) = &m.date {
        let v = d.read(cx).value().trim().to_string();
        if !v.is_empty() {
            out.push(format!("during:{v}"));
        }
    }
    out.join(" ")
}

/// `container_VhFsFg`: label body-md/medium 16/20 --text-strong, description
/// body-sm/normal 14/18 --text-subtle (gap 4), control 8px below.
fn label(title: SharedString, desc: SharedString) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap(z(4.))
        .child(div().text_size(z(16.)).line_height(z(20.)).font_weight(FontWeight::MEDIUM).text_color(c(tok(idx::TEXT_STRONG))).child(title))
        .child(div().text_size(z(14.)).line_height(z(18.)).text_color(c(tok(idx::TEXT_SUBTLE))).child(desc))
}

/// `selectField_vWC4bk` combobox: 40px, 1px --input-border-default, radius 8,
/// --input-background-default; input padding 1 8 1 12; 36px chevron button.
fn field(input: &Entity<InputState>) -> gpui::Div {
    div()
        .mt(z(8.))
        .h(z(40.))
        .flex()
        .items_center()
        .rounded(z(8.))
        .border_1()
        .border_color(c(tok(idx::INPUT_BORDER_DEFAULT)))
        .bg(c(tok(idx::INPUT_BACKGROUND_DEFAULT)))
        .child(div().flex_1().min_w_0().pl(z(12.)).pr(z(8.)).text_size(z(16.)).child(Input::new(input).appearance(false).bordered(false)))
        .child(
            div().w(z(36.)).h_full().flex().items_center().justify_center().pr(z(4.)).child(
                gpui::svg().path("icons/chevron.svg").size(z(20.)).text_color(c(tok(idx::TEXT_STRONG))),
            ),
        )
}

pub fn render(_app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    let m = cx.try_global::<Filters>()?.0.as_ref()?;
    let fields = m.fields.clone();
    let date = m.date.clone();
    // filterSection_R726Px: padding-bottom 20 (first one also 12 on top, last 12 below).
    let mut body = div().flex().flex_col().pt(z(12.)).pb(z(12.));
    for (i, ((key, title, desc), f)) in FIELDS.iter().zip(&fields).enumerate() {
        if *key == "authorType:" {
            // Date section (labelContainer_R726Px + full-width md secondary button) sits before "Type d'auteur".
            let mut sec = div().pb(z(20.)).flex().flex_col().child(label(crate::t!("search.m_date"), crate::t!("search.m_date_desc")));
            sec = match &date {
                Some(d) => sec.child(field(d)),
                None => sec.child(
                    div().mt(z(8.)).child(
                        button("search-filters-date", crate::t!("search.m_add_date"), BtnKind::Secondary).w_full().on_click(cx.listener(|_, _, w, cx| {
                            let ph = crate::t!("search.hint.date");
                            let d = cx.new(|cx| InputState::new(w, cx).placeholder(ph));
                            d.update(cx, |s, cx| s.focus(w, cx));
                            if let Some(m) = cx.default_global::<Filters>().0.as_mut() {
                                m.date = Some(d);
                            }
                            cx.notify();
                        })),
                    ),
                ),
            };
            body = body.child(sec);
        }
        let last = i + 1 == FIELDS.len();
        body = body.child(div().pb(z(if last { 0. } else { 20. })).flex().flex_col().child(label(crate::t!(title), crate::t!(desc))).child(field(f)));
    }
    let card = div()
        .relative()
        .w(z(480.)) // size-md
        .max_h(z(728.))
        .rounded(z(12.))
        .bg(c(tok(idx::BACKGROUND_SURFACE_HIGH)))
        .border_1()
        .border_color(c(tok(idx::BORDER_SUBTLE)))
        .shadow(vec![gpui::BoxShadow {
            color: gpui::hsla(0., 0., 0., 0.24),
            offset: gpui::point(z(0.), z(12.)),
            blur_radius: z(24.),
            spread_radius: z(0.),
            inset: false,
        }])
        .pt(z(8.))
        .pb(z(16.))
        .flex()
        .flex_col()
        // header_dvN-MP: padding 16 24 8, heading-lg/semibold 20/24.
        .child(
            div()
                .pt(z(16.))
                .px(z(24.))
                .pb(z(8.))
                .text_size(z(20.))
                .line_height(z(24.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c(tok(idx::TEXT_STRONG)))
                .child(crate::t!("search.filters")),
        )
        // bodySpacerTopBorder: 16px + 1px --border-subtle line; bodyInner padding 12 8 12 24 (scrolls).
        .child(div().mt(z(16.)).h(z(1.)).bg(c(tok(idx::BORDER_SUBTLE))))
        .child(div().id("search-filters-body").flex_1().min_h_0().overflow_y_scroll().pl(z(24.)).pr(z(8.)).child(div().pr(z(16.)).child(body)))
        .child(div().h(z(1.)).bg(c(tok(idx::BORDER_SUBTLE))))
        // actionBar_dvN-MP: padding 16 24 8; leading text button (--text-brand,
        // body-md/medium) and trailing md Secondary + Primary, gap 8.
        .child(
            div()
                .pt(z(16.))
                .px(z(24.))
                .pb(z(8.))
                .flex()
                .items_center()
                .gap(z(8.))
                .child(
                    div()
                        .id("search-filters-clear")
                        .flex_1()
                        .cursor_pointer()
                        .text_size(z(16.))
                        .line_height(z(20.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(c(tok(idx::TEXT_BRAND)))
                        .hover(|s| s.underline())
                        .on_click(cx.listener(|_, _, w, cx| {
                            let Some(m) = cx.global::<Filters>().0.as_ref() else { return };
                            let inputs: Vec<_> = m.fields.iter().cloned().chain(m.date.clone()).collect();
                            for i in inputs {
                                i.update(cx, |s, cx| s.set_value("", w, cx));
                            }
                        }))
                        .child(crate::t!("search.m_clear")),
                )
                .child(button("search-filters-cancel", crate::t!("common.cancel"), BtnKind::Secondary).on_click(cx.listener(|_, _, _, cx| close(cx))))
                .child(button("search-filters-apply", crate::t!("search.m_apply"), BtnKind::Primary).on_click(cx.listener(|a, _, w, cx| {
                    let acx: &App = cx;
                    let t = acx.global::<Filters>().0.as_ref().map(|m| tokens(m, acx)).unwrap_or_default();
                    close(cx);
                    crate::ui::search::append_and_submit(a, &t, w, cx);
                }))),
        )
        // headerTrailing: 40px md icon-only close 8px from the top/right edges.
        .child(
            div()
                .id("search-filters-close")
                .absolute()
                .top(z(8.))
                .right(z(8.))
                .size(z(40.))
                .rounded(z(8.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(|s| s.bg(c(tok(idx::CONTROL_ICON_ONLY_BACKGROUND_HOVER))))
                .on_click(cx.listener(|_, _, _, cx| close(cx)))
                .child(gpui::svg().path(crate::icons::path("close")).size(z(20.)).text_color(c(tok(idx::CONTROL_ICON_ONLY_ICON_DEFAULT)))),
        );
    let weak = cx.entity().downgrade();
    Some(crate::ui::modal::frame(
        "search-filters-modal",
        std::rc::Rc::new(move |_, cx| {
            let _ = weak.update(cx, |_, cx| close(cx));
        }),
        card,
    ))
}

#[cfg(test)]
mod tests {
    use super::value_for;

    #[test]
    fn normalises_values() {
        assert_eq!(value_for("pinned:", "Vrai").as_deref(), Some("true"));
        assert_eq!(value_for("pinned:", "nope"), None);
        assert_eq!(value_for("authorType:", "Utilisateur").as_deref(), Some("user"));
        assert_eq!(value_for("from:", " Some Name ").as_deref(), Some("\"Some Name\""));
        assert_eq!(value_for("in:", ""), None);
    }
}
