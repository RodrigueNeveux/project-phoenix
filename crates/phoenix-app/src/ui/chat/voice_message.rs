use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::Engine as _;
use gpui::{
    AnyElement, App, Bounds, Context, FontWeight, Hsla, InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*, px, svg,
};
use phoenix_voice::audio::effects::{self, Handle, Pcm};
use phoenix_voice::audio::voice_message::{self as vm, Recorder, Recording, Source};

use crate::app::PhoenixApp;
use crate::model::{Id, VoiceMeta};
use crate::theme::{c, generated::idx, tok, z};

enum Phase {
    Requesting,
    Recording(Recorder),
    Finalizing,
    Recorded(Arc<Recording>),
}

struct Draft {
    channel: Id,
    phase: Phase,
    started: Instant,
    token: u64,
}

#[derive(Clone)]
enum Audio {
    Loading,
    Ready(Arc<Pcm>),
    Failed,
}

struct Playing {
    key: SharedString,
    handle: Handle,
    from: f32,
    at: Instant,
    rate: f32,
    duration: f32,
}

impl Playing {
    fn position(&self) -> f32 {
        (self.from + self.at.elapsed().as_secs_f32() * self.rate).min(self.duration)
    }
}

struct Store {
    draft: Option<Draft>,
    playing: Option<Playing>,
    positions: HashMap<SharedString, f32>,
    audio: HashMap<SharedString, Audio>,
    stretched: HashMap<(SharedString, usize), Arc<Pcm>>,
    want: Option<SharedString>,
    rate_ix: usize,
    muted: bool,
    ticking: bool,
    next_token: u64,
    bounds: HashMap<SharedString, Bounds<Pixels>>,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            draft: None,
            playing: None,
            positions: HashMap::new(),
            audio: HashMap::new(),
            stretched: HashMap::new(),
            want: None,
            rate_ix: 1,
            muted: false,
            ticking: false,
            next_token: 0,
            bounds: HashMap::new(),
        }
    }
}

impl gpui::Global for Store {}

fn store(cx: &mut App) -> &mut Store {
    cx.default_global::<Store>()
}

pub fn format_secs(secs: u64) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

pub fn waveform_b64(samples: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(vm::waveform(samples))
}

pub fn decode_waveform(b64: &str) -> Vec<f32> {
    base64::engine::general_purpose::STANDARD.decode(b64.trim()).map(|v| v.iter().map(|&b| f32::from(b) / 255.).collect()).unwrap_or_default()
}

pub fn canvas_width(duration: f64) -> f32 {
    let w = if duration <= 0.5 {
        40.
    } else if duration >= 45. {
        294.
    } else {
        (duration.min(45.) - 0.5) / 44.5 * 254. + 40.
    };
    6. * ((w as f32 + 4.) / 6.).floor() - 4.
}

pub fn fit_waveform(v: &[f32], n: usize, fluid: bool) -> Vec<f32> {
    if n == 0 {
        return Vec::new();
    }
    if v.len() >= n {
        return vm::downsample(v, n);
    }
    if !fluid {
        let mut out = v.to_vec();
        out.resize(n, 0.);
        return out;
    }
    match v.len() {
        0 => vec![0.; n],
        1 => vec![v[0]; n],
        len => (0..n)
            .map(|i| {
                let r = i as f32 * (len - 1) as f32 / (n - 1).max(1) as f32;
                let a = r.floor() as usize;
                let b = (a + 1).min(len - 1);
                let t = r - a as f32;
                v[a] * (1. - t) + v[b] * t
            })
            .collect(),
    }
}

fn source() -> Source {
    if let Some(secs) = std::env::var("PHOENIX_VOICE_MESSAGE_TONE").ok().and_then(|v| v.parse::<f32>().ok()) {
        return Source::Pcm { rate: 48_000, samples: vm::tone(48_000, secs.clamp(0.5, 60.), 440., 0.4) };
    }
    Source::Microphone(crate::voice::prefs::prefs().lock().input_device.clone())
}

pub fn active(cx: &App, channel: Id) -> bool {
    cx.try_global::<Store>().and_then(|s| s.draft.as_ref()).is_some_and(|d| d.channel == channel)
}

pub fn start(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) {
    let Some(channel) = app.selected_channel else { return };
    if app.nav.extras.get(&channel).is_some_and(|x| x.send_blocked) {
        return;
    }
    if let Some(d) = cx.try_global::<Store>().and_then(|s| s.draft.as_ref()) {
        if d.channel != channel && matches!(d.phase, Phase::Recording(_)) {
            let other = d.channel;
            stop_recording(app, other, true, cx);
        }
        return;
    }
    let st = store(cx);
    st.next_token += 1;
    let token = st.next_token;
    st.draft = Some(Draft { channel, phase: Phase::Requesting, started: Instant::now(), token });
    let control = crate::voice::call::manager().control().clone();
    let src = source();
    ensure_ticker(cx);
    cx.spawn(async move |this, cx| {
        let res = cx.background_spawn(async move { Recorder::start(control, src) }).await;
        let _ = this.update(cx, |app, cx| {
            let st = store(cx);
            let ours = st.draft.as_ref().is_some_and(|d| d.token == token && matches!(d.phase, Phase::Requesting));
            match res {
                Ok(rec) if ours => {
                    if let Some(d) = st.draft.as_mut() {
                        d.phase = Phase::Recording(rec);
                        d.started = Instant::now();
                    }
                }
                Ok(rec) => {
                    cx.background_spawn(async move { rec.cancel() }).detach();
                }
                Err(e) => {
                    eprintln!("discord-phoenix: voice message recording: {e}");
                    if ours {
                        st.draft = None;
                        app.parity_toast(crate::t!("voice_msg.failed_start"), cx);
                    }
                }
            }
            cx.notify();
        });
    })
    .detach();
    cx.notify();
}

fn take_recorder(cx: &mut App, channel: Id) -> Option<(Recorder, u64)> {
    let d = store(cx).draft.as_mut().filter(|d| d.channel == channel)?;
    match std::mem::replace(&mut d.phase, Phase::Finalizing) {
        Phase::Recording(r) => Some((r, d.token)),
        other => {
            d.phase = other;
            None
        }
    }
}

fn stop_recording(app: &mut PhoenixApp, channel: Id, interrupted: bool, cx: &mut Context<PhoenixApp>) {
    let Some((rec, token)) = take_recorder(cx, channel) else { return };
    if interrupted {
        if !crate::sounds::is_disabled("voice_message_recording_stopped") {
            crate::sounds::play("voice_message_recording_stopped", 0.4);
        }
        app.parity_toast(crate::t!("voice_msg.stopped"), cx);
    }
    cx.spawn(async move |this, cx| {
        let res = cx.background_spawn(async move { rec.stop() }).await;
        let _ = this.update(cx, |app, cx| {
            let st = store(cx);
            let Some(d) = st.draft.as_mut().filter(|d| d.token == token) else { return };
            match res {
                Ok(r) => {
                    let r = Arc::new(r);
                    let key = draft_key(channel);
                    d.phase = Phase::Recorded(r.clone());
                    st.audio.insert(key.clone(), Audio::Loading);
                    let ogg = r.ogg.clone();
                    cx.spawn(async move |this, cx| {
                        let pcm = cx.background_spawn(async move { vm::decode_ogg(&ogg).ok() }).await;
                        let _ = this.update(cx, |_, cx| {
                            store(cx).audio.insert(key, pcm.map_or(Audio::Failed, |p| Audio::Ready(Arc::new(p))));
                            cx.notify();
                        });
                    })
                    .detach();
                }
                Err(e) => {
                    eprintln!("discord-phoenix: voice message finalize: {e}");
                    st.draft = None;
                    app.parity_toast(crate::t!("voice_msg.failed_send"), cx);
                }
            }
            cx.notify();
        });
    })
    .detach();
    cx.notify();
}

fn draft_key(channel: Id) -> SharedString {
    format!("voice-draft:{}", channel.0).into()
}

pub fn discard(cx: &mut App, channel: Id) {
    let key = draft_key(channel);
    let st = store(cx);
    if let Some(p) = st.playing.take_if(|p| p.key == key) {
        p.handle.stop();
    }
    st.audio.remove(&key);
    st.positions.remove(&key);
    st.stretched.retain(|(k, _), _| *k != key);
    let Some(d) = st.draft.take_if(|d| d.channel == channel) else { return };
    if let Phase::Recording(r) = d.phase {
        cx.background_spawn(async move { r.cancel() }).detach();
    }
}

pub fn send(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) -> bool {
    let Some(channel) = app.selected_channel else { return false };
    if !active(cx, channel) {
        return false;
    }
    if let Some((rec, token)) = take_recorder(cx, channel) {
        cx.spawn(async move |this, cx| {
            let res = cx.background_spawn(async move { rec.stop() }).await;
            let _ = this.update(cx, |app, cx| {
                if !store(cx).draft.as_ref().is_some_and(|d| d.token == token) {
                    return;
                }
                discard(cx, channel);
                match res {
                    Ok(r) => upload(app, channel, Arc::new(r), cx),
                    Err(_) => app.parity_toast(crate::t!("voice_msg.failed_send"), cx),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
        return true;
    }
    let rec = match cx.try_global::<Store>().and_then(|s| s.draft.as_ref()).map(|d| &d.phase) {
        Some(Phase::Recorded(r)) => r.clone(),
        _ => return true,
    };
    discard(cx, channel);
    upload(app, channel, rec, cx);
    cx.notify();
    true
}

fn upload(app: &mut PhoenixApp, channel: Id, rec: Arc<Recording>, cx: &mut Context<PhoenixApp>) {
    if rec.duration_secs < vm::MIN_DURATION_SECS {
        return;
    }
    let voice = crate::backend::VoiceUpload { ogg: rec.ogg.clone(), duration_secs: rec.duration_secs, waveform: waveform_b64(&rec.samples) };
    let reply = app.replying_to.take().map(|r| (r, app.chat.reply_mention));
    app.chat.reply_mention = true;
    let nonce = super::pending::make_nonce();
    let fut = app.backend.send_voice_message(channel, voice, reply, nonce);
    cx.spawn(async move |this, cx| {
        let res = fut.await;
        let _ = this.update(cx, |app, cx| match res {
            Ok(mut m) => {
                m.nonce.get_or_insert(nonce);
                app.append_message(m, cx);
                if app.selected_channel == Some(channel) {
                    app.message_list.scroll_to_end();
                }
            }
            Err(e) => {
                eprintln!("discord-phoenix: voice message upload: {}", e.0);
                app.parity_toast(crate::t!("voice_msg.failed_send"), cx);
            }
        });
    })
    .detach();
}

pub fn on_enter(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) -> bool {
    let Some(channel) = app.selected_channel else { return false };
    let ready = cx
        .try_global::<Store>()
        .and_then(|s| s.draft.as_ref())
        .is_some_and(|d| d.channel == channel && matches!(d.phase, Phase::Recording(_) | Phase::Recorded(_)));
    if ready { send(app, cx) } else { active(cx, channel) }
}

pub fn on_channel_change(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) {
    let Some(d) = cx.try_global::<Store>().and_then(|s| s.draft.as_ref()) else { return };
    if Some(d.channel) == app.selected_channel {
        return;
    }
    let ch = d.channel;
    let (recording, requesting) = (matches!(d.phase, Phase::Recording(_)), matches!(d.phase, Phase::Requesting));
    if recording {
        stop_recording(app, ch, true, cx);
    } else if requesting {
        discard(cx, ch);
    }
}

fn ensure_ticker(cx: &mut App) {
    let st = store(cx);
    if st.ticking {
        return;
    }
    st.ticking = true;
    cx.spawn(async move |cx| {
        loop {
            cx.background_executor().timer(Duration::from_millis(16)).await;
            if !cx.update(tick) {
                break;
            }
        }
    })
    .detach();
}

fn tick(cx: &mut App) -> bool {
    let st = store(cx);
    if let Some(p) = &st.playing
        && (p.handle.is_stopped() || p.position() >= p.duration - 0.01)
    {
        let key = p.key.clone();
        st.playing = None;
        st.positions.remove(&key);
    }
    let recording = st.draft.as_ref().is_some_and(|d| matches!(d.phase, Phase::Requesting | Phase::Recording(_) | Phase::Finalizing));
    let busy = st.playing.is_some() || recording || st.want.is_some();
    if !busy {
        st.ticking = false;
    }
    cx.refresh_windows();
    busy
}

fn audio_for(cx: &App, key: &SharedString) -> Option<Audio> {
    cx.try_global::<Store>().and_then(|s| s.audio.get(key).cloned())
}

fn ensure_loaded(key: &SharedString, url: &SharedString, cx: &mut App) {
    let st = store(cx);
    if st.audio.contains_key(key) {
        return;
    }
    st.audio.insert(key.clone(), Audio::Loading);
    let http = cx.http_client();
    let (k, u) = (key.clone(), url.to_string());
    cx.spawn(async move |cx| {
        use futures::AsyncReadExt;
        let mut bytes = Vec::new();
        let ok = match http.get(&u, Default::default(), true).await {
            Ok(mut r) => r.body_mut().read_to_end(&mut bytes).await.is_ok(),
            Err(_) => false,
        };
        let pcm = if ok { cx.background_spawn(async move { vm::decode_ogg(&bytes).ok() }).await } else { None };
        cx.update(|cx| {
            let failed = pcm.is_none();
            let st = store(cx);
            st.audio.insert(k.clone(), pcm.map_or(Audio::Failed, |p| Audio::Ready(Arc::new(p))));
            if st.want.as_ref() == Some(&k) {
                st.want = None;
                if !failed {
                    start_playback(&k, cx);
                }
            }
            cx.refresh_windows();
        });
    })
    .detach();
}

fn pause(cx: &mut App) {
    let st = store(cx);
    if let Some(p) = st.playing.take() {
        let pos = p.position();
        p.handle.stop();
        st.positions.insert(p.key, pos);
    }
}

fn stretched(key: &SharedString, ix: usize, cx: &mut App) -> Option<Arc<Pcm>> {
    let st = store(cx);
    let Some(Audio::Ready(pcm)) = st.audio.get(key).cloned() else { return None };
    let rate = vm::PLAYBACK_RATES[ix];
    if (rate - 1.).abs() < 1e-3 {
        return Some(pcm);
    }
    if let Some(p) = st.stretched.get(&(key.clone(), ix)) {
        return Some(p.clone());
    }
    st.want = Some(key.clone());
    let k = key.clone();
    cx.spawn(async move |cx| {
        let src = pcm.clone();
        let out = cx.background_spawn(async move { Pcm { rate: src.rate, channels: 1, samples: vm::time_stretch(&src.samples, rate, src.rate) } }).await;
        cx.update(|cx| {
            let st = store(cx);
            st.stretched.insert((k.clone(), ix), Arc::new(out));
            if st.want.as_ref() == Some(&k) {
                st.want = None;
                start_playback(&k, cx);
            }
        });
    })
    .detach();
    None
}

fn start_playback(key: &SharedString, cx: &mut App) {
    pause(cx);
    let Some(Audio::Ready(pcm)) = audio_for(cx, key) else { return };
    let duration = pcm.duration().as_secs_f32();
    let ix = store(cx).rate_ix;
    let Some(src) = stretched(key, ix, cx) else { return };
    let st = store(cx);
    let mut from = st.positions.get(key).copied().unwrap_or(0.);
    if from >= duration - 0.05 {
        from = 0.;
    }
    let rate = vm::PLAYBACK_RATES[ix];
    let off = ((from / rate) * src.rate as f32) as usize;
    let samples = src.samples.get(off..).map(<[f32]>::to_vec).unwrap_or_default();
    let volume = if st.muted { 0. } else { crate::voice::prefs::prefs().lock().output_volume as f32 / 100. };
    let handle = effects::play(Arc::new(Pcm { rate: src.rate, channels: 1, samples }), volume.clamp(0., 1.), false);
    let st = store(cx);
    st.positions.insert(key.clone(), from);
    st.playing = Some(Playing { key: key.clone(), handle, from, at: Instant::now(), rate, duration });
    ensure_ticker(cx);
}

fn toggle(key: &SharedString, url: Option<&SharedString>, cx: &mut App) {
    if cx.try_global::<Store>().and_then(|s| s.playing.as_ref()).is_some_and(|p| &p.key == key) {
        pause(cx);
        return;
    }
    match audio_for(cx, key) {
        Some(Audio::Ready(_)) => start_playback(key, cx),
        Some(Audio::Loading) => store(cx).want = Some(key.clone()),
        Some(Audio::Failed) => {}
        None => {
            if let Some(u) = url {
                store(cx).want = Some(key.clone());
                ensure_loaded(key, u, cx);
            }
        }
    }
    ensure_ticker(cx);
}

fn seek(key: &SharedString, frac: f32, duration: f32, cx: &mut App) {
    let playing = cx.try_global::<Store>().and_then(|s| s.playing.as_ref()).is_some_and(|p| &p.key == key);
    if playing {
        pause(cx);
    }
    store(cx).positions.insert(key.clone(), frac.clamp(0., 1.) * duration);
    if playing {
        start_playback(key, cx);
    }
}

fn cycle_rate(cx: &mut App) {
    let playing = cx.try_global::<Store>().and_then(|s| s.playing.as_ref().map(|p| p.key.clone()));
    if playing.is_some() {
        pause(cx);
    }
    let st = store(cx);
    st.rate_ix = (st.rate_ix + 1) % vm::PLAYBACK_RATES.len();
    if let Some(k) = playing {
        start_playback(&k, cx);
    }
}

fn toggle_mute(cx: &mut App) {
    let st = store(cx);
    st.muted = !st.muted;
    let v = if st.muted { 0. } else { (crate::voice::prefs::prefs().lock().output_volume as f32 / 100.).clamp(0., 1.) };
    if let Some(p) = &st.playing {
        p.handle.set_volume(v);
    }
}

pub fn rate_label(rate: f32) -> String {
    let s = format!("{rate}");
    format!("{}X", s.strip_prefix('0').unwrap_or(&s))
}

struct View {
    current: f32,
    playing: bool,
    played: bool,
}

fn view(cx: &App, key: &SharedString) -> View {
    let Some(st) = cx.try_global::<Store>() else { return View { current: 0., playing: false, played: false } };
    if let Some(p) = st.playing.as_ref().filter(|p| &p.key == key) {
        return View { current: p.position(), playing: true, played: true };
    }
    match st.positions.get(key) {
        Some(&pos) => View { current: pos, playing: false, played: true },
        None => View { current: 0., playing: false, played: false },
    }
}

#[derive(Clone, Copy)]
struct WaveColors {
    bg: Hsla,
    active: Hsla,
    inactive: Hsla,
}

fn wave_colors(played: bool, playing: bool) -> WaveColors {
    let muted = c(tok(idx::BACKGROUND_MOD_MUTED));
    let bg = if playing { c(tok(idx::BRAND_430)) } else { muted };
    let active = if playing {
        c(tok(idx::WHITE))
    } else if played {
        c(tok(idx::INTERACTIVE_TEXT_ACTIVE))
    } else {
        c(tok(idx::INTERACTIVE_TEXT_DEFAULT))
    };
    let inactive = if played { bg } else { c(tok(idx::INTERACTIVE_TEXT_DEFAULT)) };
    WaveColors { bg, active, inactive }
}

fn bar_quad(window: &mut Window, b: Bounds<Pixels>, x: f32, h: f32, color: Hsla) {
    if h <= 0. {
        return;
    }
    let bw = f32::from(z(2.));
    let top = (f32::from(b.size.height) - h) / 2.;
    let o = gpui::point(b.origin.x + px(x), b.origin.y + px(top));
    window.paint_quad(gpui::fill(Bounds::new(o, gpui::size(px(bw), px(h))), color).corner_radii(z(1.)));
}

fn static_wave(key: SharedString, values: Vec<f32>, fluid: bool, played_upto: Option<f32>, colors: WaveColors) -> gpui::Canvas<()> {
    gpui::canvas(
        move |b, _, cx| {
            store(cx).bounds.insert(key, b);
        },
        move |b, _, window, _| {
            let pitch = f32::from(z(6.));
            let w = f32::from(b.size.width);
            let n = ((w + f32::from(z(4.))) / pitch).floor().max(0.) as usize;
            let vals = fit_waveform(&values, n, fluid);
            let upto = played_upto.map_or(n, |f| (f * n as f32).round().max(0.) as usize);
            let (k22, k24, k2) = (f32::from(z(22.)), f32::from(z(24.)), f32::from(z(2.)));
            for (i, v) in vals.iter().enumerate() {
                let x = i as f32 * pitch;
                bar_quad(window, b, x, k22 * v + k2, colors.bg);
                if i < upto {
                    bar_quad(window, b, x, k22 * v + k2, colors.active);
                } else {
                    bar_quad(window, b, x, k24 * (v - 0.1).max(0.), colors.inactive);
                }
            }
        },
    )
}

fn live_wave(levels: Vec<f32>, elapsed_ms: f32, moving: bool, color: Hsla) -> gpui::Canvas<()> {
    gpui::canvas(
        |_, _, _| (),
        move |b, _, window, _| {
            let pitch = f32::from(z(6.));
            let w = f32::from(b.size.width);
            let n = (w / pitch).ceil() as usize + 2;
            let ticks = (elapsed_ms / 48.).floor().max(0.) as usize;
            let mut d = 0f32;
            let mut vals = Vec::with_capacity(ticks);
            for j in 0..ticks {
                let si = ((j as f32 * 48. / 20.) as usize).min(levels.len().saturating_sub(1));
                let r = levels.get(si).copied().unwrap_or(0.);
                d += (r - d) * 0.45;
                vals.push(d);
            }
            let tail = &vals[vals.len().saturating_sub(n)..];
            let mut shown = vec![0f32; n - tail.len()];
            shown.extend_from_slice(tail);
            let frac = (elapsed_ms % 48.) / 48.;
            let offset = if moving { pitch * (1. - frac) } else { 0. };
            let x0 = w - n as f32 * pitch + offset;
            let (k22, k2) = (f32::from(z(22.)), f32::from(z(2.)));
            for (i, v) in shown.iter().enumerate() {
                let x = x0 + i as f32 * pitch;
                if x < 0. || x > w {
                    continue;
                }
                bar_quad(window, b, x, v * k22 + k2, color);
            }
        },
    )
}

fn chat_button(id: impl Into<SharedString>, icon: &'static str, tip: SharedString) -> gpui::Stateful<gpui::Div> {
    let id: SharedString = id.into();
    let group: SharedString = format!("vm-{id}").into();
    div()
        .id(gpui::ElementId::Name(id))
        .group(group.clone())
        .size(z(32.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(z(8.))
        .cursor_pointer()
        .hover(|s| s.bg(c(tok(idx::INTERACTIVE_BACKGROUND_SELECTED))))
        .tooltip(crate::ui::tooltip::tip(tip))
        .child(
            svg()
                .path(crate::icons::path(icon))
                .size(z(20.))
                .flex_none()
                .text_color(c(tok(idx::INTERACTIVE_TEXT_DEFAULT)))
                .group_hover(group, |s| s.text_color(c(tok(idx::INTERACTIVE_TEXT_HOVER)))),
        )
}

fn timer_text(text: String, color: Hsla) -> gpui::Div {
    div().flex_none().min_w(z(32.)).text_size(z(14.)).line_height(z(18.)).text_color(color).child(SharedString::from(text))
}

pub fn bar(app: &PhoenixApp, cx: &mut Context<PhoenixApp>) -> Option<AnyElement> {
    let channel = app.selected_channel?;
    let st = cx.try_global::<Store>()?;
    let d = st.draft.as_ref().filter(|d| d.channel == channel)?;
    let key = draft_key(channel);
    let recording = matches!(d.phase, Phase::Recording(_));
    let can_send = matches!(d.phase, Phase::Recording(_) | Phase::Recorded(_));
    let elapsed = d.started.elapsed();
    if recording && elapsed >= vm::MAX_DURATION {
        cx.spawn(async move |this, cx| {
            let _ = this.update(cx, |a, cx| stop_recording(a, channel, false, cx));
        })
        .detach();
    }
    let capture: AnyElement = match &d.phase {
        Phase::Recorded(r) => {
            let dur = r.duration_secs as f32;
            let v = view(cx, &key);
            let values: Vec<f32> = vm::waveform(&r.samples).iter().map(|&b| f32::from(b) / 255.).collect();
            let shown = if v.played { (dur - v.current).max(0.).ceil() } else { dur.ceil() };
            let (k1, k2, k3) = (key.clone(), key.clone(), key.clone());
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap(z(16.))
                .child(
                    chat_button("vm-preview-play", if v.playing { "pause" } else { "play" }, if v.playing { crate::t!("voice_msg.pause") } else { crate::t!("voice_msg.play") })
                        .on_click(move |_, _, cx| toggle(&k1, None, cx)),
                )
                .child(
                    div()
                        .id("vm-preview-wave")
                        .flex_1()
                        .min_w_0()
                        .h(z(32.))
                        .my(z(-4.))
                        .on_mouse_down(MouseButton::Left, move |e, _, cx| {
                            if let Some(b) = cx.try_global::<Store>().and_then(|s| s.bounds.get(&k2).copied()) {
                                let f = f32::from(e.position.x - b.origin.x) / f32::from(b.size.width).max(1.);
                                seek(&k2, f, dur, cx);
                            }
                        })
                        .child(static_wave(k3, values, true, v.played.then(|| v.current / dur.max(0.001)), wave_colors(v.played, false)).size_full()),
                )
                .child(timer_text(format_secs(shown as u64), c(tok(idx::TEXT_DEFAULT))))
                .into_any_element()
        }
        _ => {
            let samples = match &d.phase {
                Phase::Recording(r) => r.samples(),
                _ => Vec::new(),
            };
            let levels: Vec<f32> = samples.iter().map(|&s| f32::from(s) / 255.).collect();
            let ms = if recording { elapsed.as_secs_f32() * 1000. } else { 0. };
            let warn = if !recording {
                None
            } else if elapsed >= vm::WARN_IMMINENT {
                Some(250u128)
            } else if elapsed >= vm::WARN_APPROACHING {
                Some(500u128)
            } else {
                None
            };
            let reduced = app.prefs.local.reduced_motion;
            let blink = warn.is_some_and(|p| {
                let period = if reduced { 1000 } else { p };
                (elapsed.as_millis() / period) % 2 == 1
            });
            let secs = if recording { elapsed.as_secs() } else { 0 };
            let ch = channel;
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap(z(16.))
                .when(recording, |d| {
                    d.child(chat_button("vm-stop", "stop", crate::t!("voice_msg.stop")).on_click(cx.listener(move |a, _, _, cx| stop_recording(a, ch, false, cx))))
                })
                .child(div().flex_1().min_w_0().h(z(32.)).overflow_hidden().child(live_wave(levels, ms, recording && !reduced, c(tok(idx::INTERACTIVE_TEXT_ACTIVE))).size_full()))
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(z(4.))
                        .child(div().flex_none().size(z(4.)).rounded_full().when(recording, |d| d.bg(c(tok(idx::TEXT_FEEDBACK_CRITICAL)))))
                        .child(timer_text(format_secs(secs), c(tok(if blink { idx::TEXT_FEEDBACK_CRITICAL } else { idx::TEXT_DEFAULT })))),
                )
                .into_any_element()
        }
    };
    let ch = channel;
    let send_color = c(tok(if can_send { idx::TEXT_BRAND } else { idx::ICON_MUTED }));
    Some(
        div()
            .flex_1()
            .min_w_0()
            .min_h(z(56.))
            .pl(z(15.))
            .flex()
            .items_center()
            .gap(z(8.))
            .child(capture)
            .child(div().flex_none().w(z(1.)).h(z(22.)).bg(c(tok(idx::BORDER_SUBTLE))))
            .child(chat_button("vm-trash", "chat/trash", crate::t!("voice_msg.delete")).on_click(cx.listener(move |_, _, _, cx| {
                discard(cx, ch);
                cx.notify();
            })))
            .child(
                div()
                    .id("vm-send")
                    .group("vm-send")
                    .size(z(32.))
                    .p(z(4.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(z(8.))
                    .tooltip(crate::ui::tooltip::tip(crate::t!("voice_msg.send")))
                    .when(can_send, |d| d.cursor_pointer().on_click(cx.listener(|a, _, _, cx| {
                        send(a, cx);
                    })))
                    .when(!can_send, |d| d.cursor(gpui::CursorStyle::OperationNotAllowed))
                    .child(
                        svg()
                            .path(crate::icons::path("send-message"))
                            .size(z(20.))
                            .flex_none()
                            .text_color(send_color)
                            .when(can_send, |s| s.group_hover("vm-send", |s| s.text_color(c(tok(idx::BRAND_430))))),
                    ),
            )
            .into_any_element(),
    )
}

pub fn player(url: &SharedString, meta: &VoiceMeta, cx: &App) -> AnyElement {
    let key = url.clone();
    let mut duration = meta.duration_secs as f32;
    if let Some(Audio::Ready(p)) = audio_for(cx, &key) {
        let d = p.duration().as_secs_f32();
        if d > 0. {
            duration = d;
        }
    }
    let duration = duration.max(0.001);
    let v = view(cx, &key);
    let (rate_ix, muted) = cx.try_global::<Store>().map_or((1, false), |s| (s.rate_ix, s.muted));
    let values = decode_waveform(&meta.waveform);
    let shown = if v.played { (duration - v.current).max(0.).ceil() } else { duration.ceil() };
    let white = c(tok(idx::WHITE));
    let (k1, k2, k3) = (key.clone(), key.clone(), key.clone());
    let u = url.clone();
    let id = |s: &str| gpui::ElementId::Name(format!("vm-{s}-{key}").into());
    let play_bg = if v.playing {
        gpui::Background::from(white)
    } else {
        gpui::linear_gradient(
            151.11,
            gpui::linear_color_stop(c(tok(idx::BRAND_460)), 0.1655),
            gpui::linear_color_stop(c(tok(idx::BRAND_560)), 1.0436),
        )
    };
    let play_hover = gpui::linear_gradient(
        151.11,
        gpui::linear_color_stop(c(tok(idx::BRAND_400)), 0.1778),
        gpui::linear_color_stop(c(tok(idx::BRAND_460)), 0.8222),
    );
    let vol_icon = if muted { "volume-off" } else { "speaker" };
    let vol_color = if v.playing { c(tok(idx::BRAND_200)) } else { c(tok(idx::INTERACTIVE_TEXT_DEFAULT)) };
    let vol_hover = if v.playing { white } else { c(tok(idx::INTERACTIVE_TEXT_HOVER)) };
    let rate_group: SharedString = format!("vm-rate-{key}").into();
    let vol_group: SharedString = format!("vm-vol-{key}").into();
    let (rate_bg, rate_fg) = if v.playing {
        (c(tok(idx::INTERACTIVE_BACKGROUND_ACTIVE)), c(tok(idx::BRAND_200)))
    } else {
        (c(tok(idx::BACKGROUND_MOD_SUBTLE)), c(tok(idx::INTERACTIVE_TEXT_DEFAULT)))
    };
    let rate_hover_fg = if v.playing { white } else { c(tok(idx::INTERACTIVE_TEXT_ACTIVE)) };
    div()
        .id(id("player"))
        .mt(z(4.))
        .self_start()
        .flex_none()
        .relative()
        .flex()
        .flex_row()
        .items_center()
        .gap(z(12.))
        .p(z(12.))
        .max_w_full()
        .min_w_0()
        .rounded(z(24.))
        .bg(c(tok(idx::BACKGROUND_MOD_SUBTLE)))
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left_0()
                .right_0()
                .rounded(z(24.))
                .border_1()
                .border_color(c(tok(idx::BORDER_SUBTLE)))
                .overflow_hidden()
                .when(v.playing, |d| d.bg(c(tok(idx::BRAND_530)))),
        )
        .child(
            div()
                .id(id("play"))
                .relative()
                .size(z(32.))
                .ml(z(-4.))
                .my(z(-4.))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .cursor_pointer()
                .bg(play_bg)
                .when(!v.playing, |d| d.hover(move |s| s.bg(play_hover)))
                .child(
                    svg()
                        .path(crate::icons::path(if v.playing { "pause" } else { "play" }))
                        .size(z(18.))
                        .flex_none()
                        .text_color(if v.playing { c(tok(idx::BRAND_500)) } else { white }),
                )
                .on_click(move |_, _, cx| toggle(&k1, Some(&u), cx)),
        )
        .child(
            div()
                .id(id("wave"))
                .relative()
                .flex_none()
                .w(z(canvas_width(f64::from(duration))))
                .h(z(32.))
                .my(z(-4.))
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, move |e, _, cx| {
                    if let Some(b) = cx.try_global::<Store>().and_then(|s| s.bounds.get(&k2).copied()) {
                        let f = f32::from(e.position.x - b.origin.x) / f32::from(b.size.width).max(1.);
                        seek(&k2, f, duration, cx);
                        cx.refresh_windows();
                    }
                })
                .child(static_wave(k3, values, false, v.played.then(|| v.current / duration), wave_colors(v.played, v.playing)).size_full()),
        )
        .child(
            div()
                .relative()
                .flex_none()
                .text_size(z(14.))
                .line_height(z(18.))
                .text_color(if v.playing { white } else { c(tok(idx::INTERACTIVE_TEXT_DEFAULT)) })
                .child(SharedString::from(format_secs(shown as u64))),
        )
        .child(
            div()
                .id(id("rate"))
                .relative()
                .group(rate_group.clone())
                .flex_none()
                .py(z(4.))
                .cursor_pointer()
                .child(
                    div()
                        .w(z(32.))
                        .rounded(z(4.))
                        .bg(rate_bg)
                        .text_size(z(12.))
                        .line_height(z(16.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rate_fg)
                        .flex()
                        .justify_center()
                        .group_hover(rate_group, move |s| s.bg(c(tok(idx::INTERACTIVE_BACKGROUND_HOVER))).text_color(rate_hover_fg))
                        .child(SharedString::from(rate_label(vm::PLAYBACK_RATES[rate_ix]))),
                )
                .on_click(|_, _, cx| {
                    cycle_rate(cx);
                    cx.refresh_windows();
                }),
        )
        .child(
            div()
                .id(id("vol"))
                .relative()
                .group(vol_group.clone())
                .flex_none()
                .cursor_pointer()
                .child(
                    svg()
                        .path(crate::icons::path(vol_icon))
                        .size(z(24.))
                        .flex_none()
                        .text_color(vol_color)
                        .group_hover(vol_group, move |s| s.text_color(vol_hover)),
                )
                .on_click(|_, _, cx| {
                    toggle_mute(cx);
                    cx.refresh_windows();
                }),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_format_matches_client() {
        assert_eq!(format_secs(0), "0:00");
        assert_eq!(format_secs(9), "0:09");
        assert_eq!(format_secs(75), "1:15");
        assert_eq!(format_secs(1199), "19:59");
    }

    #[test]
    fn canvas_width_follows_duration() {
        assert_eq!(canvas_width(0.2), 38.);
        assert_eq!(canvas_width(60.), 290.);
        assert_eq!(canvas_width(4.), 56.);
        assert_eq!((canvas_width(10.) + 4.) % 6., 0.);
    }

    #[test]
    fn rate_labels() {
        let l: Vec<String> = vm::PLAYBACK_RATES.iter().map(|&r| rate_label(r)).collect();
        assert_eq!(l, ["0.75X", "1X", "1.5X", "2X"].map(|s| s.strip_prefix('0').unwrap_or(s).to_string()));
        assert_eq!(rate_label(0.75), ".75X");
    }

    #[test]
    fn waveform_fit_pads_or_interpolates() {
        assert_eq!(fit_waveform(&[1., 0.5], 4, false), vec![1., 0.5, 0., 0.]);
        assert_eq!(fit_waveform(&[0., 1.], 3, true), vec![0., 0.5, 1.]);
        assert_eq!(fit_waveform(&[1., 1., 0., 0.], 2, false), vec![1., 0.]);
        assert_eq!(fit_waveform(&[], 2, true), vec![0., 0.]);
    }

    #[test]
    fn waveform_base64_roundtrip() {
        let s: Vec<u8> = (0..20).map(|i| i * 10).collect();
        let b = waveform_b64(&s);
        let back = decode_waveform(&b);
        assert_eq!(back.len(), 20);
        assert!((back[1] - 10. / 255.).abs() < 1e-6);
    }
}
