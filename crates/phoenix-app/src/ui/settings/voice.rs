//! Voice & Video settings page (Discord layout): devices, volumes, mic test
//! with live meter, input mode (voice activity / push-to-talk with keybind
//! recording), input sensitivity, voice processing, attenuation, video.
//!
//! Also hosts the continuous [`slider`] used by the voice context menu.

use crate::theme::z;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gpui::{
    AnyElement, Bounds, Context, DispatchPhase, Div, ElementId, FontWeight, IntoElement, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, SharedString, Stateful, Styled, canvas, deferred, div, prelude::*,
    relative,
};
use phoenix_voice::audio::device::MicTest;

use super::Page;
use crate::app::PhoenixApp;
use crate::assets::Icon;
use crate::theme::{Palette, c};
use crate::ui::icon;
use crate::voice::call::{PortalState, manager, portal_state};
use crate::voice::prefs::{InputMode, VoicePrefs, prefs, update};
use crate::voice::{VoiceStatus, ptt};

type Cx<'a> = Context<'a, PhoenixApp>;
pub(crate) type OnSlide = Rc<dyn Fn(&mut PhoenixApp, f32, bool, &mut Cx)>;

const TRACK: u32 = 0x4E5058;
const SELECT_BG: u32 = 0x1E1F22;
const METER_OFF: u32 = 0x4E5058;
const SENS_LOW: u32 = 0xF0B232;
const SENS_HIGH: u32 = 0x23A55A;
/// Segments of the mic test meter.
const METER_SEGMENTS: usize = 48;
/// Meter range in dBFS.
const DB_MIN: f32 = -100.;

thread_local! {
    static BOUNDS: RefCell<HashMap<&'static str, Bounds<Pixels>>> = RefCell::new(HashMap::new());
    static DRAG: Cell<Option<&'static str>> = const { Cell::new(None) };
    static OPEN_SELECT: Cell<Option<&'static str>> = const { Cell::new(None) };
    static MIC_TEST: RefCell<Option<MicTest>> = const { RefCell::new(None) };
    static MIC_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
    static DEVICES: RefCell<Option<(Vec<String>, Vec<String>)>> = const { RefCell::new(None) };
    static LOADING: Cell<bool> = const { Cell::new(false) };
    static TICKING: Cell<bool> = const { Cell::new(false) };
}

fn eid(s: impl Into<String>) -> ElementId {
    ElementId::Name(SharedString::from(s.into()))
}

fn h1(t: &'static str) -> Div {
    div()
        .mb(z(20.))
        .text_size(z(20.))
        .line_height(z(24.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(c(Palette::TEXT_HEADER))
        .child(t)
}

fn h5(t: impl Into<SharedString>) -> Div {
    div()
        .mb(z(8.))
        .text_size(z(12.))
        .line_height(z(16.))
        .font_weight(FontWeight::BOLD)
        .text_color(c(Palette::TEXT_MUTED))
        .child(t.into())
}

fn note(t: impl Into<SharedString>) -> Div {
    div().text_size(z(14.)).line_height(z(20.)).text_color(c(Palette::TEXT_MUTED)).child(t.into())
}

fn divider() -> Div {
    div().my(z(40.)).h(z(1.)).bg(c(Palette::DIVIDER))
}

fn knob(on: bool, disabled: bool) -> Div {
    div()
        .w(z(40.))
        .h(z(24.))
        .flex_none()
        .rounded(z(12.))
        .bg(c(if on { Palette::GREEN } else { 0x80848E }))
        .when(disabled, |d| d.opacity(0.5))
        .p(z(3.))
        .flex()
        .when(on, |d| d.justify_end())
        .child(div().size(z(18.)).rounded_full().bg(gpui::white()))
}

fn switch_row(
    id: &'static str,
    title: &'static str,
    desc: &'static str,
    on: bool,
    disabled: bool,
    f: impl Fn(&mut VoicePrefs) + 'static,
    cx: &mut Cx,
) -> Div {
    div()
        .mb(z(20.))
        .flex()
        .flex_col()
        .child(
            div()
                .id(eid(format!("vs-switch-{id}")))
                .flex()
                .items_center()
                .when(!disabled, |d| {
                    d.cursor_pointer().on_click(cx.listener(move |_, _, _, cx| {
                        update(&f, true);
                        cx.notify();
                    }))
                })
                .child(
                    div()
                        .flex_1()
                        .text_size(z(16.))
                        .line_height(z(24.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(c(if disabled { Palette::TEXT_MUTED } else { Palette::TEXT_NORMAL }))
                        .child(title),
                )
                .child(knob(on, disabled)),
        )
        .when(!desc.is_empty(), |d| d.child(div().mt(z(8.)).child(note(desc))))
        .child(div().mt(z(20.)).h(z(1.)).bg(c(Palette::DIVIDER)))
}

fn frac_at(id: &'static str, x: Pixels) -> Option<f32> {
    BOUNDS.with(|b| {
        let b = *b.borrow().get(id)?;
        let w = f32::from(b.size.width).max(1.);
        Some(((f32::from(x) - f32::from(b.origin.x)) / w).clamp(0., 1.))
    })
}

/// Continuous Discord slider: 8px track, blurple fill, 10x24 white grabber,
/// value bubble above the grabber while dragging. `on_change(app, frac,
/// done)`; `done` is true on release (save then).
pub(crate) fn slider(
    id: &'static str,
    frac: f32,
    bubble: SharedString,
    track: Option<(u32, u32)>,
    on_change: OnSlide,
    cx: &mut Cx,
) -> Stateful<Div> {
    let frac = frac.clamp(0., 1.);
    let dragging = DRAG.with(Cell::get) == Some(id);
    let weak = cx.entity().downgrade();
    let paint_cb = on_change.clone();
    let probe = canvas(
        move |bounds, _, _| {
            BOUNDS.with(|b| b.borrow_mut().insert(id, bounds));
        },
        move |_, _, window, _| {
            if DRAG.with(Cell::get) != Some(id) {
                return;
            }
            let (w1, f1) = (weak.clone(), paint_cb.clone());
            window.on_mouse_event(move |ev: &MouseMoveEvent, phase, _, cx| {
                if phase != DispatchPhase::Bubble || DRAG.with(Cell::get) != Some(id) {
                    return;
                }
                if let Some(f) = frac_at(id, ev.position.x) {
                    let _ = w1.update(cx, |a, cx| {
                        f1(a, f, false, cx);
                        cx.notify();
                    });
                }
            });
            let (w2, f2) = (weak.clone(), paint_cb.clone());
            window.on_mouse_event(move |ev: &MouseUpEvent, phase, _, cx| {
                if phase != DispatchPhase::Bubble || DRAG.with(Cell::get) != Some(id) {
                    return;
                }
                DRAG.with(|d| d.set(None));
                let f = frac_at(id, ev.position.x);
                let _ = w2.update(cx, |a, cx| {
                    if let Some(f) = f {
                        f2(a, f, true, cx);
                    }
                    cx.notify();
                });
            });
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full();
    let (lo, hi) = track.unwrap_or((Palette::BLURPLE, TRACK));
    div()
        .id(eid(format!("slider-{id}")))
        .relative()
        .w_full()
        .h(z(24.))
        .cursor_pointer()
        .child(probe)
        .child(
            div()
                .absolute()
                .top(z(8.))
                .left_0()
                .w_full()
                .h(z(8.))
                .rounded(z(4.))
                .bg(c(hi))
                .child(div().h_full().w(relative(frac)).rounded(z(4.)).bg(c(lo))),
        )
        .child(
            div().absolute().top_0().left(relative(frac)).w(z(0.)).flex().justify_center().child(
                div()
                    .relative()
                    .flex_none()
                    .w(z(10.))
                    .h(z(24.))
                    .rounded(z(3.))
                    .bg(gpui::white())
                    .border_1()
                    .border_color(c(0xDBDEE1))
                    .shadow_sm()
                    .when(dragging, |d| {
                        d.child(
                            div().absolute().bottom(z(30.)).left(z(-27.)).w(z(64.)).flex().justify_center().child(
                                div()
                                    .px(z(8.))
                                    .py(z(4.))
                                    .rounded(z(4.))
                                    .bg(c(Palette::BG_FLOATING))
                                    .text_size(z(14.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(c(Palette::TEXT_NORMAL))
                                    .whitespace_nowrap()
                                    .child(bubble),
                            ),
                        )
                    }),
            ),
        )
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |a, ev: &MouseDownEvent, _, cx| {
                DRAG.with(|d| d.set(Some(id)));
                if let Some(f) = frac_at(id, ev.position.x) {
                    on_change(a, f, false, cx);
                }
                cx.stop_propagation();
                cx.notify();
            }),
        )
}

fn select(
    id: &'static str,
    current: Option<String>,
    options: Vec<String>,
    f: impl Fn(&mut VoicePrefs, Option<String>) + 'static,
    cx: &mut Cx,
) -> Div {
    let open = OPEN_SELECT.with(Cell::get) == Some(id);
    let label: SharedString = current.clone().unwrap_or_else(|| "Default".into()).into();
    let f = Rc::new(f);
    let mut list = div()
        .id(eid(format!("{id}-list")))
        .absolute()
        .top(z(44.))
        .left_0()
        .w_full()
        .max_h(z(300.))
        .overflow_y_scroll()
        .rounded(z(4.))
        .border_1()
        .border_color(c(Palette::DIVIDER))
        .bg(c(Palette::BG_FLOATING))
        .shadow_lg()
        .occlude()
        .flex()
        .flex_col();
    let mut entries: Vec<Option<String>> = vec![None];
    entries.extend(options.into_iter().map(Some));
    for (ix, opt) in entries.into_iter().enumerate() {
        let selected = opt == current;
        let text: SharedString = opt.clone().unwrap_or_else(|| "Default".into()).into();
        let f = f.clone();
        list = list.child(
            div()
                .id(eid(format!("{id}-opt-{ix}")))
                .px(z(12.))
                .py(z(10.))
                .flex()
                .items_center()
                .cursor_pointer()
                .text_size(z(16.))
                .line_height(z(20.))
                .text_color(c(if selected { Palette::TEXT_HEADER } else { Palette::TEXT_NORMAL }))
                .when(selected, |d| d.bg(c(Palette::BG_MOD_SELECTED)))
                .hover(|s| s.bg(c(Palette::BG_MOD_HOVER)))
                .child(div().flex_1().min_w_0().text_ellipsis().child(text))
                .on_click(cx.listener(move |_, _, _, cx| {
                    OPEN_SELECT.with(|o| o.set(None));
                    let v = opt.clone();
                    let f = f.clone();
                    update(move |p| f(p, v), true);
                    restart_mic_test();
                    cx.notify();
                })),
        );
    }
    div()
        .relative()
        .child(
            div()
                .id(eid(format!("{id}-box")))
                .h(z(40.))
                .pl(z(12.))
                .pr(z(8.))
                .rounded(z(4.))
                .bg(c(SELECT_BG))
                .border_1()
                .border_color(c(if open { 0x00A8FC } else { SELECT_BG }))
                .flex()
                .items_center()
                .cursor_pointer()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_ellipsis()
                        .text_size(z(16.))
                        .text_color(c(Palette::TEXT_NORMAL))
                        .child(label),
                )
                .child(icon(Icon::Chevron, 20., c(Palette::INTERACTIVE)))
                .on_click(cx.listener(move |_, _, _, cx| {
                    OPEN_SELECT.with(|o| o.set(if open { None } else { Some(id) }));
                    cx.notify();
                })),
        )
        .when(open, |d| d.child(deferred(list).with_priority(2)))
}

fn button(id: &'static str, label: &'static str, bg: u32, hover: u32) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .h(z(32.))
        .min_w(z(96.))
        .px(z(16.))
        .flex_none()
        .rounded(z(4.))
        .bg(c(bg))
        .hover(move |s| s.bg(c(hover)))
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .text_size(z(14.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(gpui::white())
        .child(label)
}

/// Segmented level meter (mic test).
fn meter(level_db: f32, active: bool) -> Div {
    let frac = ((level_db - DB_MIN) / -DB_MIN).clamp(0., 1.);
    let lit = if active { (frac * METER_SEGMENTS as f32).round() as usize } else { 0 };
    div().flex_1().h(z(16.)).flex().items_center().gap(z(4.)).children(
        (0..METER_SEGMENTS)
            .map(|i| div().flex_1().h(z(16.)).rounded(z(2.)).bg(c(if i < lit { SENS_HIGH } else { METER_OFF }))),
    )
}

/// Live input level from the mic test, or from the call.
fn live_level() -> Option<(f32, bool)> {
    let t = MIC_TEST.with(|m| m.borrow().as_ref().map(|t| (t.level_db(), t.gate_open())));
    t.or_else(|| {
        let c = manager().control();
        (c.input_level_db() > DB_MIN).then(|| (c.input_level_db(), c.transmitting()))
    })
}

fn mic_testing() -> bool {
    MIC_TEST.with(|m| m.borrow().is_some())
}

fn stop_mic_test() {
    if let Some(t) = MIC_TEST.with(|m| m.borrow_mut().take()) {
        std::thread::spawn(move || t.stop());
    }
}

fn start_mic_test() {
    let p = prefs().lock().clone();
    match MicTest::start(Arc::clone(manager().control()), p.input_device, p.output_device, true) {
        Ok(t) => {
            MIC_ERROR.with(|e| *e.borrow_mut() = None);
            MIC_TEST.with(|m| *m.borrow_mut() = Some(t));
        }
        Err(e) => MIC_ERROR.with(|x| *x.borrow_mut() = Some(e.to_string())),
    }
}

fn restart_mic_test() {
    if mic_testing() {
        if let Some(t) = MIC_TEST.with(|m| m.borrow_mut().take()) {
            t.stop();
        }
        start_mic_test();
    }
}

/// Repaint ~20 fps while the page is open (meters); stop the test after.
fn ensure_ticker(cx: &mut Cx) {
    if TICKING.with(Cell::get) {
        return;
    }
    TICKING.with(|t| t.set(true));
    cx.spawn(async move |this, cx| {
        loop {
            cx.background_executor().timer(Duration::from_millis(50)).await;
            let keep = this
                .update(cx, |a, cx| {
                    let on = a.settings.as_ref().is_some_and(|s| s.page == Page::VoiceVideo);
                    if on {
                        cx.notify();
                    }
                    on
                })
                .unwrap_or(false);
            if !keep {
                break;
            }
        }
        TICKING.with(|t| t.set(false));
        OPEN_SELECT.with(|o| o.set(None));
        ptt::set_recording(false);
        stop_mic_test();
    })
    .detach();
}

fn ensure_devices(cx: &mut Cx) {
    if DEVICES.with(|d| d.borrow().is_some()) || LOADING.with(Cell::get) {
        return;
    }
    LOADING.with(|l| l.set(true));
    let task = cx.background_executor().spawn(async { crate::voice::call::devices() });
    cx.spawn(async move |this, cx| {
        let list = task.await;
        DEVICES.with(|d| *d.borrow_mut() = Some(list));
        LOADING.with(|l| l.set(false));
        let _ = this.update(cx, |_, cx| cx.notify());
    })
    .detach();
}

fn pct_slider(
    id: &'static str,
    value: u32,
    max: u32,
    f: impl Fn(&mut VoicePrefs, u32) + 'static,
    cx: &mut Cx,
) -> Stateful<Div> {
    let f = Rc::new(f);
    slider(
        id,
        value as f32 / max as f32,
        format!("{value}%").into(),
        None,
        Rc::new(move |_, frac, done, _| {
            let v = (frac * max as f32).round() as u32;
            let f = f.clone();
            update(move |p| f(p, v), done);
        }),
        cx,
    )
}

fn column(children: impl IntoIterator<Item = AnyElement>) -> Div {
    div().flex_1().min_w_0().flex().flex_col().children(children)
}

fn input_mode(p: &VoicePrefs, cx: &mut Cx) -> Div {
    let radio = |id: &'static str, label: &'static str, mode: InputMode, cx: &mut Cx| {
        let selected = p.input_mode == mode;
        div()
            .id(id)
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
                    .border_color(c(if selected { 0xFFFFFF } else { Palette::INTERACTIVE }))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(selected, |d| d.child(div().size(z(10.)).rounded_full().bg(gpui::white()))),
            )
            .child(
                div()
                    .text_size(z(16.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(c(Palette::TEXT_NORMAL))
                    .child(label),
            )
            .on_click(cx.listener(move |_, _, _, cx| {
                update(|p| p.input_mode = mode, true);
                cx.notify();
            }))
    };
    div()
        .flex()
        .flex_col()
        .child(h5("INPUT MODE"))
        .child(radio("vs-mode-vad", "Voice Activity", InputMode::VoiceActivity, cx))
        .child(radio("vs-mode-ptt", "Push to Talk", InputMode::PushToTalk, cx))
}

fn keybind(p: &VoicePrefs, cx: &mut Cx) -> Div {
    let recording = ptt::is_recording();
    let label: SharedString = match (&p.ptt_key, recording) {
        (_, true) => "Press a key…".into(),
        (Some(k), false) => ptt::label(k).into(),
        (None, false) => "No Keybind Set".into(),
    };
    let portal: Option<(SharedString, u32)> = match portal_state().lock().clone() {
        PortalState::Off => None,
        PortalState::Pending => Some(("Registering the global shortcut with your desktop…".into(), Palette::TEXT_MUTED)),
        PortalState::Bound(d) if d.is_empty() => Some(("Works everywhere, even when Phoenix is in the background.".into(), Palette::TEXT_MUTED)),
        PortalState::Bound(d) => Some((format!("Global shortcut active: {d}").into(), Palette::TEXT_MUTED)),
        PortalState::Unavailable(_) => Some(("Your desktop did not grant a global shortcut: push to talk only works while Phoenix is focused.".into(), Palette::YELLOW)),
    };
    let delay = p.ptt_delay_ms;
    div()
        .flex()
        .gap(z(16.))
        .child(column([
            h5("SHORTCUT").into_any_element(),
            div()
                .h(z(40.))
                .pl(z(10.))
                .pr(z(4.))
                .rounded(z(4.))
                .bg(c(SELECT_BG))
                .border_1()
                .border_color(c(if recording { Palette::RED } else { SELECT_BG }))
                .flex()
                .items_center()
                .child(
                    div()
                        .flex_1()
                        .text_size(z(14.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c(if p.ptt_key.is_some() || recording { Palette::TEXT_NORMAL } else { Palette::TEXT_MUTED }))
                        .child(label),
                )
                .child(
                    button(
                        "vs-record",
                        if recording { "Stop Recording" } else if p.ptt_key.is_some() { "Edit Keybind" } else { "Record Keybind" },
                        if recording { 0xDA373C } else { 0x4E5058 },
                        if recording { 0xA12D2F } else { 0x6D6F78 },
                    )
                    .min_w(z(0.))
                    .on_click(cx.listener(move |_, _, _, cx| {
                        ptt::set_recording(!recording);
                        cx.notify();
                    })),
                )
                .into_any_element(),
            div().when_some(portal, |d, (t, col)| d.mt(z(8.)).text_size(z(12.)).text_color(c(col)).child(t)).into_any_element(),
        ]))
        .child(column([
            h5("PUSH TO TALK RELEASE DELAY").into_any_element(),
            slider(
                "ptt-delay",
                delay as f32 / 2000.,
                format!("{delay}ms").into(),
                None,
                Rc::new(|_, f, done, _| {
                    let v = (f * 2000.).round() as u32;
                    update(move |p| p.ptt_delay_ms = v, done);
                }),
                cx,
            )
            .mt(z(8.))
            .into_any_element(),
            div()
                .mt(z(8.))
                .text_size(z(12.))
                .text_color(c(Palette::TEXT_MUTED))
                .child(SharedString::from(format!("{delay}ms")))
                .into_any_element(),
        ]))
}

fn sensitivity(p: &VoicePrefs, cx: &mut Cx) -> Div {
    let auto = p.auto_sensitivity;
    let level = live_level();
    let lvl_frac = level.map(|(db, _)| ((db - DB_MIN) / -DB_MIN).clamp(0., 1.)).unwrap_or(0.);
    let bar: AnyElement = if auto {
        // Discord: a single bar that lights green while you are detected as speaking.
        let speaking = level.is_some_and(|(_, open)| open);
        div()
            .mt(z(4.))
            .h(z(8.))
            .rounded(z(4.))
            .bg(c(TRACK))
            .child(div().h_full().w(relative(lvl_frac)).rounded(z(4.)).bg(c(if speaking { SENS_HIGH } else { SENS_LOW })))
            .into_any_element()
    } else {
        let t = p.sensitivity_db;
        let frac = (t - DB_MIN) / -DB_MIN;
        div()
            .relative()
            .child(slider(
                "sensitivity",
                frac,
                format!("{t:.0}dB").into(),
                Some((SENS_LOW, SENS_HIGH)),
                Rc::new(|_, f, done, _| {
                    let db = (DB_MIN + f * -DB_MIN).round();
                    update(move |p| p.sensitivity_db = db, done);
                }),
                cx,
            ))
            // Live level: dim the part of the bar above the current level.
            .child(
                div()
                    .absolute()
                    .top(z(8.))
                    .left(relative(lvl_frac))
                    .right_0()
                    .h(z(8.))
                    .rounded_r(z(4.))
                    .bg(gpui::black().opacity(0.45)),
            )
            .into_any_element()
    };
    div()
        .flex()
        .flex_col()
        .child(h5("INPUT SENSITIVITY"))
        .child(
            div()
                .id("vs-auto-sens")
                .mb(z(16.))
                .flex()
                .items_center()
                .cursor_pointer()
                .child(
                    div()
                        .flex_1()
                        .text_size(z(16.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(c(Palette::TEXT_NORMAL))
                        .child("Automatically determine input sensitivity"),
                )
                .child(knob(auto, false))
                .on_click(cx.listener(move |_, _, _, cx| {
                    update(|p| p.auto_sensitivity = !auto, true);
                    cx.notify();
                })),
        )
        .child(bar)
        .when(auto && level.is_none(), |d| {
            d.child(div().mt(z(8.)).child(note("Start a mic test or join a voice channel to see your input level.")))
        })
}

fn mic_test(app: &PhoenixApp, cx: &mut Cx) -> Div {
    let testing = mic_testing();
    let level = live_level().map(|(db, _)| db).unwrap_or(DB_MIN);
    let err = MIC_ERROR.with(|e| e.borrow().clone());
    let in_call = app.voice.status == VoiceStatus::Connected;
    div()
        .flex()
        .flex_col()
        .child(h5("MIC TEST"))
        .child(note("Having mic issues? Start a test and say something fun—we'll play your voice back to you.").mb(z(16.)))
        .child(
            div()
                .flex()
                .items_center()
                .gap(z(16.))
                .child(
                    button(
                        "vs-mic-test",
                        if testing { "Stop Testing" } else { "Let's Check" },
                        if testing { 0x4E5058 } else { Palette::BLURPLE },
                        if testing { 0x6D6F78 } else { 0x4752C4 },
                    )
                    .on_click(cx.listener(move |_, _, _, cx| {
                        if testing {
                            stop_mic_test();
                        } else {
                            start_mic_test();
                        }
                        cx.notify();
                    })),
                )
                .child(meter(level, testing || in_call)),
        )
        .when_some(err, |d, e| d.child(div().mt(z(8.)).text_size(z(12.)).text_color(c(Palette::RED)).child(SharedString::from(e))))
}

/// The whole page.
pub fn render(app: &PhoenixApp, cx: &mut Cx) -> Div {
    ensure_ticker(cx);
    ensure_devices(cx);
    let p = prefs().lock().clone();
    let (inputs, outputs) = DEVICES.with(|d| d.borrow().clone()).unwrap_or_default();
    div()
        .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| {
            if OPEN_SELECT.with(|o| o.take()).is_some() {
                cx.notify();
            }
        }))
        .child(h1("Voice Settings"))
        .child(
            div()
                .flex()
                .gap(z(16.))
                .child(column([
                    h5("INPUT DEVICE").into_any_element(),
                    select("input", p.input_device.clone(), inputs, |p, v| p.input_device = v, cx).into_any_element(),
                ]))
                .child(column([
                    h5("OUTPUT DEVICE").into_any_element(),
                    select("output", p.output_device.clone(), outputs, |p, v| p.output_device = v, cx).into_any_element(),
                ])),
        )
        .child(
            div()
                .mt(z(20.))
                .flex()
                .gap(z(16.))
                .child(column([
                    h5("INPUT VOLUME").into_any_element(),
                    pct_slider("input-volume", p.input_volume, 100, |p, v| p.input_volume = v, cx).into_any_element(),
                ]))
                .child(column([
                    h5("OUTPUT VOLUME").into_any_element(),
                    pct_slider("output-volume", p.output_volume, 200, |p, v| p.output_volume = v, cx).into_any_element(),
                ])),
        )
        .child(div().mt(z(20.)).child(mic_test(app, cx)))
        .child(divider())
        .child(input_mode(&p, cx))
        .child(div().mt(z(20.)).child(if p.input_mode == InputMode::PushToTalk { keybind(&p, cx) } else { sensitivity(&p, cx) }))
        .when(p.input_mode == InputMode::PushToTalk, |d| d.child(div().mt(z(20.)).child(sensitivity(&p, cx))))
        .child(divider())
        .child(h5("VOICE PROCESSING"))
        .child(switch_row(
            "echo",
            "Echo Cancellation",
            "Not available in Phoenix yet: use headphones to avoid echo.",
            false,
            true,
            |_| {},
            cx,
        ))
        .child(switch_row(
            "noise",
            "Noise Suppression",
            "Suppress background noise from your mic.",
            p.noise_suppression,
            false,
            |p| p.noise_suppression = !p.noise_suppression,
            cx,
        ))
        .child(switch_row(
            "agc",
            "Automatic Gain Control",
            "Keep your voice at a consistent volume.",
            p.agc,
            false,
            |p| p.agc = !p.agc,
            cx,
        ))
        .child(divider())
        .child(h5("ATTENUATION"))
        .child(note("Lower the volume of other applications by this percent when someone is speaking. Set to 0% to completely disable attenuation.").mb(z(16.)))
        .child(pct_slider("attenuation", p.attenuation, 100, |p, v| p.attenuation = v, cx).mb(z(20.)))
        .child(switch_row("att-self", "When I speak", "", p.attenuate_self, false, |p| p.attenuate_self = !p.attenuate_self, cx))
        .child(switch_row(
            "att-others",
            "When others speak",
            "",
            p.attenuate_others,
            false,
            |p| p.attenuate_others = !p.attenuate_others,
            cx,
        ))
        .child(divider())
        .child(h1("Video Settings"))
        .child(
            div()
                .h(z(200.))
                .rounded(z(8.))
                .bg(c(Palette::BG_TERTIARY))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(z(8.))
                .child(note("No video devices found")),
        )
        .child(div().mt(z(20.)).child(h5("CAMERA")))
        .child(
            div()
                .h(z(40.))
                .px(z(12.))
                .rounded(z(4.))
                .bg(c(SELECT_BG))
                .flex()
                .items_center()
                .text_size(z(16.))
                .text_color(c(Palette::TEXT_MUTED))
                .child("No video devices"),
        )
}
