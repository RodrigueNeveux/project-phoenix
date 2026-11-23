//! Expression pickers (emoji, GIF, sticker popover), plus the shared
//! keyboard routing for the composer autocomplete, quick switcher and
//! message search.
//!
//! State lives in the [`Pickers`] global so the root view only needs three
//! hooks: [`install`] (in `PhoenixApp::new`), [`render_overlays`] (in
//! `render`) and the composer buttons calling [`toggle`].
//!
//! Other views open the emoji picker for reactions with
//! [`open_emoji_picker`]`(app, EmojiTarget::Reaction { .. }, window, cx)`.

pub mod emoji;
#[rustfmt::skip]
pub mod emoji_data;
mod gif;
pub mod grid;
mod prefs;
mod sticker;
pub mod types;

use std::time::Duration;

use gpui::{
    AnyElement, App, Context, Entity, FontWeight, Keystroke, ListState, ParentElement, Pixels,
    Point, SharedString, Styled, Subscription, WeakEntity, Window, div, prelude::*, px,
};
use gpui_component::input::{Input, InputEvent, InputState};

use crate::{
    app::PhoenixApp,
    assets::Icon,
    model::{Id, Reaction},
    theme::{Palette, c},
    ui::{self, icon},
};

pub use grid::{Grid, PickItem};
pub use prefs::Prefs;
use types::{GifHome, GifItem, StickerPackData};

/// What a picked emoji is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmojiTarget {
    /// Insert into the message composer.
    Composer,
    /// Add a reaction to a message.
    Reaction { channel: Id, message: Id },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Gif,
    Sticker,
    Emoji,
}

pub struct OpenPicker {
    pub tab: Tab,
    pub target: EmojiTarget,
    pub input: Entity<InputState>,
    pub grid: Grid,
    pub list: ListState,
    /// Keyboard selection (index into `grid.items`).
    pub selected: usize,
    pub hovered: Option<usize>,
    pub tone_open: bool,
    /// Where to anchor the popover (reactions); `None` = above the composer.
    pub anchor: Option<Point<Pixels>>,
    _sub: Subscription,
}

#[derive(Default)]
pub struct GifState {
    pub home: Option<GifHome>,
    pub home_loading: bool,
    /// Query whose results are in `results` ("" = trending GIFs).
    pub shown: Option<String>,
    pub results: Vec<GifItem>,
    pub loading: bool,
    pub error: Option<SharedString>,
    seq: u64,
}

#[derive(Default)]
pub struct StickerState {
    pub packs: Option<Vec<StickerPackData>>,
    pub loading: bool,
}

pub struct Pickers {
    pub app: WeakEntity<PhoenixApp>,
    pub open: Option<OpenPicker>,
    pub prefs: Prefs,
    pub gifs: GifState,
    pub stickers: StickerState,
}

impl gpui::Global for Pickers {}

/// Register the picker state, the keystroke interceptor and the composer
/// autocomplete. Call once from `PhoenixApp::new`.
pub fn install(composer: &Entity<InputState>, window: &mut Window, cx: &mut Context<PhoenixApp>) -> Vec<Subscription> {
    let weak = cx.entity().downgrade();
    cx.set_global(Pickers {
        app: weak.clone(),
        open: None,
        prefs: Prefs::load(),
        gifs: GifState::default(),
        stickers: StickerState::default(),
    });
    let keys = cx.intercept_keystrokes(move |ev, window, cx| {
        let Some(app) = weak.upgrade() else { return };
        app.update(cx, |app, cx| {
            if route_key(app, &ev.keystroke, window, cx) {
                cx.stop_propagation();
            }
        });
    });
    let ac = cx.subscribe_in(composer, window, |app, _, ev: &InputEvent, window, cx| {
        if let InputEvent::Change = ev {
            ui::autocomplete::refresh(app, window, cx);
        }
    });
    vec![keys, ac]
}

/// Keyboard handling for every popup we own. Returns true when consumed.
fn route_key(app: &mut PhoenixApp, ks: &Keystroke, window: &mut Window, cx: &mut Context<PhoenixApp>) -> bool {
    let m = &ks.modifiers;
    let plain = !m.control && !m.alt && !m.platform;
    if m.control && !m.shift && ks.key == "f" && app.screen == crate::app::Screen::Main {
        ui::search::open(app, window, cx);
        return true;
    }
    if m.control && !m.shift && ks.key == "e" && app.screen == crate::app::Screen::Main {
        toggle(app, Tab::Emoji, EmojiTarget::Composer, window, cx);
        return true;
    }
    if m.control && !m.shift && ks.key == "g" && app.screen == crate::app::Screen::Main {
        toggle(app, Tab::Gif, EmojiTarget::Composer, window, cx);
        return true;
    }
    if is_open(cx) {
        return plain && picker_key(app, &ks.key, m.shift, window, cx);
    }
    if app.switcher.is_some() {
        return plain && ui::switcher::on_key(app, &ks.key, window, cx);
    }
    if ui::search::is_focused(window, cx) {
        return plain && ui::search::on_key(app, &ks.key, window, cx);
    }
    plain && ui::autocomplete::on_key(app, &ks.key, m.shift, window, cx)
}

pub fn is_open(cx: &App) -> bool {
    cx.try_global::<Pickers>().is_some_and(|p| p.open.is_some())
}

pub fn open_tab(cx: &App) -> Option<(Tab, EmojiTarget)> {
    cx.try_global::<Pickers>()?.open.as_ref().map(|o| (o.tab, o.target))
}

/// Open the emoji picker, e.g. for a reaction (anchored at the mouse).
pub fn open_emoji_picker(app: &mut PhoenixApp, target: EmojiTarget, window: &mut Window, cx: &mut Context<PhoenixApp>) {
    open(app, Tab::Emoji, target, window, cx);
}

/// Composer buttons: open `tab`, switch to it, or close when already shown.
pub fn toggle(app: &mut PhoenixApp, tab: Tab, target: EmojiTarget, window: &mut Window, cx: &mut Context<PhoenixApp>) {
    match open_tab(cx) {
        Some((t, tg)) if t == tab && tg == target => close(app, window, cx),
        Some((_, tg)) if tg == target => switch_tab(app, tab, window, cx),
        _ => open(app, tab, target, window, cx),
    }
}

pub fn open(app: &mut PhoenixApp, tab: Tab, target: EmojiTarget, window: &mut Window, cx: &mut Context<PhoenixApp>) {
    if app.screen != crate::app::Screen::Main {
        return;
    }
    ui::autocomplete::dismiss(cx);
    let tab = if matches!(target, EmojiTarget::Reaction { .. }) { Tab::Emoji } else { tab };
    let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder(tab)));
    let sub = cx.subscribe_in(&input, window, |app, _, ev: &InputEvent, window, cx| match ev {
        InputEvent::Change => on_query(app, cx),
        InputEvent::PressEnter { shift, .. } => pick_selected(app, *shift, window, cx),
        _ => {}
    });
    input.update(cx, |s, cx| s.focus(window, cx));
    let anchor = matches!(target, EmojiTarget::Reaction { .. }).then(|| window.mouse_position());
    let grid = grid::build(app, "", cx);
    let list = ListState::new(grid.rows.len(), gpui::ListAlignment::Top, px(400.));
    cx.global_mut::<Pickers>().open = Some(OpenPicker {
        tab,
        target,
        input,
        grid,
        list,
        selected: 0,
        hovered: None,
        tone_open: false,
        anchor,
        _sub: sub,
    });
    ensure_tab_data(app, tab, cx);
    cx.notify();
}

fn placeholder(tab: Tab) -> SharedString {
    match tab {
        Tab::Gif => crate::t!("picker.search_tenor"),
        Tab::Sticker => crate::t!("picker.search_stickers"),
        Tab::Emoji => crate::t!("picker.search_emoji"),
    }
}

fn switch_tab(app: &mut PhoenixApp, tab: Tab, window: &mut Window, cx: &mut Context<PhoenixApp>) {
    let Some(op) = cx.global_mut::<Pickers>().open.as_mut() else { return };
    op.tab = tab;
    op.selected = 0;
    op.tone_open = false;
    let input = op.input.clone();
    input.update(cx, |s, cx| {
        s.set_value("", window, cx);
        s.set_placeholder(placeholder(tab), window, cx);
        s.focus(window, cx);
    });
    ensure_tab_data(app, tab, cx);
    cx.notify();
}

pub fn close(app: &mut PhoenixApp, window: &mut Window, cx: &mut Context<PhoenixApp>) {
    if cx.global_mut::<Pickers>().open.take().is_some() {
        app.composer.update(cx, |s, cx| s.focus(window, cx));
        cx.notify();
    }
}

fn query(cx: &App) -> String {
    cx.global::<Pickers>().open.as_ref().map(|o| o.input.read(cx).value().to_string()).unwrap_or_default()
}

fn on_query(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) {
    let Some((tab, _)) = open_tab(cx) else { return };
    let q = query(cx);
    match tab {
        Tab::Emoji => {
            let grid = grid::build(app, &q, cx);
            if let Some(op) = cx.global_mut::<Pickers>().open.as_mut() {
                op.list.reset(grid.rows.len());
                op.grid = grid;
                op.selected = 0;
                op.hovered = None;
            }
        }
        Tab::Gif => gif::search_debounced(app, q, cx),
        Tab::Sticker => {}
    }
    cx.notify();
}

fn ensure_tab_data(app: &mut PhoenixApp, tab: Tab, cx: &mut Context<PhoenixApp>) {
    match tab {
        Tab::Gif => gif::load_home(app, cx),
        Tab::Sticker => sticker::load_packs(app, cx),
        Tab::Emoji => {}
    }
}

fn picker_key(app: &mut PhoenixApp, key: &str, shift: bool, window: &mut Window, cx: &mut Context<PhoenixApp>) -> bool {
    if key == "escape" {
        close(app, window, cx);
        return true;
    }
    let Some(op) = cx.global_mut::<Pickers>().open.as_mut() else { return false };
    if op.tab != Tab::Emoji {
        return false;
    }
    let n = op.grid.items.len();
    if n == 0 {
        return false;
    }
    let next = match key {
        "left" => op.selected.saturating_sub(1),
        "right" => (op.selected + 1).min(n - 1),
        "up" => op.grid.vertical(op.selected, false),
        "down" => op.grid.vertical(op.selected, true),
        "enter" => {
            pick_selected(app, shift, window, cx);
            return true;
        }
        _ => return false,
    };
    op.selected = next;
    op.hovered = None;
    if let Some(row) = op.grid.row_of(next) {
        op.list.scroll_to_reveal_item(row);
    }
    cx.notify();
    true
}

fn pick_selected(app: &mut PhoenixApp, keep_open: bool, window: &mut Window, cx: &mut Context<PhoenixApp>) {
    let Some(op) = cx.global::<Pickers>().open.as_ref() else { return };
    if op.tab != Tab::Emoji {
        return;
    }
    if let Some(item) = op.grid.items.get(op.selected).cloned() {
        pick(app, item, keep_open, window, cx);
    }
}

/// Use an emoji: insert it in the composer or react with it.
pub fn pick(app: &mut PhoenixApp, item: PickItem, keep_open: bool, window: &mut Window, cx: &mut Context<PhoenixApp>) {
    let Some((_, target)) = open_tab(cx) else { return };
    let tone = cx.global::<Pickers>().prefs.tone;
    cx.global_mut::<Pickers>().prefs.record(&item);
    match target {
        EmojiTarget::Composer => {
            let text = match &item {
                PickItem::Unicode(ix) => emoji::toned(*ix, tone),
                PickItem::Custom(e) => e.token(),
            };
            app.composer.update(cx, |s, cx| s.insert(text, window, cx));
            if !keep_open {
                close(app, window, cx);
            }
        }
        EmojiTarget::Reaction { channel, message } => {
            let reaction = match &item {
                PickItem::Unicode(ix) => {
                    Reaction { emoji: emoji::toned(*ix, tone).into(), count: 1, me: true, emoji_id: None }
                }
                PickItem::Custom(e) => Reaction { emoji: e.name.clone(), count: 1, me: true, emoji_id: Some(e.id) },
            };
            detach(app.backend.set_reaction(channel, message, reaction, true), cx);
            if !keep_open {
                close(app, window, cx);
            }
        }
    }
    cx.notify();
}

/// Fire-and-forget a backend call.
pub(crate) fn detach<T: Send + 'static>(fut: crate::backend::BoxFuture<T>, cx: &mut App) {
    cx.background_spawn(async move {
        let _ = fut.await;
    })
    .detach();
}

/// Run `f` on the app from a plain element callback.
pub(crate) fn with_app(cx: &mut App, window: &mut Window, f: impl FnOnce(&mut PhoenixApp, &mut Window, &mut Context<PhoenixApp>)) {
    let Some(app) = cx.try_global::<Pickers>().and_then(|p| p.app.upgrade()) else { return };
    app.update(cx, |a, cx| f(a, window, cx));
}

pub(crate) fn debounce(cx: &mut Context<PhoenixApp>, ms: u64, f: impl FnOnce(&mut PhoenixApp, &mut Context<PhoenixApp>) + 'static) {
    cx.spawn(async move |this, cx| {
        cx.background_executor().timer(Duration::from_millis(ms)).await;
        let _ = this.update(cx, f);
    })
    .detach();
}

// ------------------------------------------------------------------ render

/// Popups drawn above the whole window, in stacking order.
pub fn render_overlays(app: &PhoenixApp, window: &mut Window, cx: &mut Context<PhoenixApp>) -> Vec<AnyElement> {
    let mut out = Vec::new();
    if let Some(el) = ui::autocomplete::render(app, cx) {
        out.push(el);
    }
    if let Some(el) = ui::search::render(app, window, cx) {
        out.push(el);
    }
    if let Some(el) = render_picker(app, window, cx) {
        out.push(el);
    }
    out
}

pub const PICKER_W: f32 = 498.;
pub const PICKER_H: f32 = 440.;

fn render_picker(app: &PhoenixApp, window: &mut Window, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    let op = cx.global::<Pickers>().open.as_ref()?;
    let (tab, target, anchor, input) = (op.tab, op.target, op.anchor, op.input.clone());
    let vp = window.viewport_size();
    let (vw, vh) = (f32::from(vp.width), f32::from(vp.height));
    let (left, top) = match anchor {
        Some(p) => {
            let x = (f32::from(p.x) - PICKER_W - 8.).clamp(8., (vw - PICKER_W - 8.).max(8.));
            let y = (f32::from(p.y) - 40.).clamp(8., (vh - PICKER_H - 8.).max(8.));
            (x, y)
        }
        None => {
            let right = 16. + if app.show_members && app.selected_guild.is_some() { 240. } else { 0. };
            ((vw - right - PICKER_W).max(8.), (vh - 84. - PICKER_H).max(8.))
        }
    };
    let tabs = match target {
        EmojiTarget::Composer => div()
            .flex()
            .gap(px(8.))
            .px(px(16.))
            .pt(px(16.))
            .child(tab_button(crate::t!("picker.gifs"), Tab::Gif, tab, cx))
            .child(tab_button(crate::t!("picker.stickers"), Tab::Sticker, tab, cx))
            .child(tab_button(crate::t!("picker.emoji"), Tab::Emoji, tab, cx))
            .into_any_element(),
        EmojiTarget::Reaction { .. } => div().into_any_element(),
    };
    let header = div()
        .flex_none()
        .flex()
        .flex_col()
        .child(tabs)
        .child(
            div()
                .p(px(16.))
                .pb(px(8.))
                .flex()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .flex_1()
                        .h(px(30.))
                        .px(px(8.))
                        .rounded(px(4.))
                        .bg(c(Palette::BG_TERTIARY))
                        .flex()
                        .items_center()
                        .text_size(px(16.))
                        .child(div().flex_1().child(Input::new(&input).appearance(false).bordered(false)))
                        .child(icon(Icon::Search, 18., c(Palette::INTERACTIVE))),
                )
                .when(tab == Tab::Emoji, |d| d.child(grid::tone_button(cx))),
        );
    let body = match tab {
        Tab::Emoji => grid::render_body(app, cx),
        Tab::Gif => gif::render_body(app, cx),
        Tab::Sticker => sticker::render_body(app, cx),
    };
    Some(
        div()
            .id("picker-layer")
            .absolute()
            .inset_0()
            .occlude()
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|a, _, w, cx| close(a, w, cx)))
            .child(
                div()
                    .id("picker")
                    .absolute()
                    .left(px(left))
                    .top(px(top))
                    .w(px(PICKER_W))
                    .h(px(PICKER_H))
                    .rounded(px(8.))
                    .bg(c(Palette::BG_SECONDARY))
                    .border_1()
                    .border_color(c(Palette::BG_TERTIARY))
                    .shadow_lg()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(header)
                    .child(div().flex_1().min_h_0().flex().child(body)),
            )
            .into_any_element(),
    )
}

fn tab_button(label: SharedString, tab: Tab, current: Tab, cx: &mut Context<PhoenixApp>) -> impl IntoElement {
    let active = tab == current;
    div()
        .id(label.clone())
        .px(px(8.))
        .py(px(2.))
        .rounded(px(4.))
        .cursor_pointer()
        .text_size(px(14.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(c(if active { Palette::TEXT_HEADER } else { Palette::INTERACTIVE }))
        .when(active, |d| d.bg(c(Palette::BG_MOD_SELECTED)))
        .when(!active, |d| d.hover(|s| s.bg(c(Palette::BG_MOD_HOVER)).text_color(c(Palette::INTERACTIVE_HOVER))))
        .on_click(cx.listener(move |a, _, w, cx| switch_tab(a, tab, w, cx)))
        .child(label)
}

/// Section title in a picker body ("FREQUENTLY USED", guild names...).
pub(crate) fn section_title(title: SharedString) -> gpui::Div {
    div()
        .h(px(28.))
        .px(px(8.))
        .flex()
        .items_center()
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(c(Palette::TEXT_MUTED))
        .child(title.to_uppercase())
}

pub(crate) fn empty_state(text: SharedString) -> AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(14.))
        .text_color(c(Palette::TEXT_MUTED))
        .child(text)
        .into_any_element()
}
