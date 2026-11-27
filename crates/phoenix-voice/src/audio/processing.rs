use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::time::{Duration, Instant};

use nnnoiseless::DenoiseState;
use sonora::config::{
    AdaptiveDigital, EchoCanceller, GainController2, HighPassFilter, MaxProcessingRate, NoiseSuppression, NoiseSuppressionLevel, Pipeline,
};
use sonora::{AudioProcessing, Config, StreamConfig};

use super::AudioControl;

pub const RATE: u32 = 48_000;
pub const BLOCK: usize = 480;
const RENDER_QUEUE: usize = 64;
const MAX_PENDING_RENDER: usize = BLOCK * 50;
const PCM16: f32 = 32768.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChainSettings {
    pub echo_cancellation: bool,
    pub noise_suppression: bool,
    pub noise_cancellation: bool,
    pub agc: bool,
}

impl ChainSettings {
    pub const OFF: Self = Self { echo_cancellation: false, noise_suppression: false, noise_cancellation: false, agc: false };

    pub fn from_control(c: &AudioControl) -> Self {
        let krisp = c.noise_cancellation();
        Self { echo_cancellation: c.echo_cancellation(), noise_suppression: c.noise_suppression() && !krisp, noise_cancellation: krisp, agc: c.agc() }
    }

    fn front(&self) -> bool {
        self.echo_cancellation || self.noise_suppression || self.noise_cancellation
    }

    pub fn label(&self) -> String {
        let mut v = Vec::new();
        if self.echo_cancellation {
            v.push("aec3");
        }
        if self.noise_suppression {
            v.push("webrtc-ns");
        }
        if self.noise_cancellation {
            v.push("rnnoise");
        }
        if self.agc {
            v.push("agc2");
        }
        if v.is_empty() { "off".into() } else { v.join("+") }
    }
}

struct RenderBlock {
    mono: Vec<f32>,
    ahead_ms: u32,
}

#[derive(Clone)]
pub struct RenderTap {
    tx: SyncSender<RenderBlock>,
}

impl RenderTap {
    pub fn push(&self, interleaved: &[f32], channels: usize, ahead_ms: u32) {
        let ch = channels.max(1);
        let mono = if ch == 1 {
            interleaved.to_vec()
        } else {
            interleaved.chunks_exact(ch).map(|f| f.iter().sum::<f32>() / ch as f32).collect()
        };
        let _ = self.tx.try_send(RenderBlock { mono, ahead_ms });
    }
}

pub struct RenderFeed {
    rx: Receiver<RenderBlock>,
    pending: Vec<f32>,
    ahead_ms: u32,
}

pub fn render_channel() -> (RenderTap, RenderFeed) {
    let (tx, rx) = sync_channel(RENDER_QUEUE);
    (RenderTap { tx }, RenderFeed { rx, pending: Vec::with_capacity(BLOCK * 8), ahead_ms: 0 })
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ChainStats {
    pub blocks: u64,
    pub busy: Duration,
    pub max_block: Duration,
    pub render_blocks: u64,
}

impl ChainStats {
    pub fn avg_block_us(&self) -> f64 {
        if self.blocks == 0 { 0.0 } else { self.busy.as_secs_f64() * 1e6 / self.blocks as f64 }
    }

    pub fn load_pct(&self) -> f64 {
        self.avg_block_us() / 10_000.0 * 100.0
    }
}

fn pipeline() -> Pipeline {
    Pipeline {
        maximum_internal_processing_rate: MaxProcessingRate::Rate48kHz,
        multi_channel_render: false,
        multi_channel_capture: false,
        ..Default::default()
    }
}

fn build(config: Config) -> AudioProcessing {
    let mono = StreamConfig::new(RATE, 1);
    AudioProcessing::builder().config(config).capture_config(mono).render_config(mono).build()
}

fn front_apm(s: ChainSettings) -> AudioProcessing {
    build(Config {
        pipeline: pipeline(),
        high_pass_filter: Some(HighPassFilter::default()),
        echo_canceller: s.echo_cancellation.then(EchoCanceller::default),
        noise_suppression: s.noise_suppression.then(|| NoiseSuppression { level: NoiseSuppressionLevel::High, ..Default::default() }),
        ..Default::default()
    })
}

fn agc_apm() -> AudioProcessing {
    build(Config {
        pipeline: pipeline(),
        gain_controller2: Some(GainController2 { input_volume_controller: false, adaptive_digital: Some(AdaptiveDigital::default()), ..Default::default() }),
        ..Default::default()
    })
}

pub struct InputChain {
    settings: ChainSettings,
    front: Option<AudioProcessing>,
    rnn: Option<Box<DenoiseState<'static>>>,
    agc: Option<AudioProcessing>,
    feed: Option<RenderFeed>,
    capture_delay_ms: u32,
    scratch: Vec<f32>,
    out: Vec<f32>,
    render_out: Vec<f32>,
    stats: ChainStats,
}

impl InputChain {
    pub fn new(settings: ChainSettings, feed: Option<RenderFeed>) -> Self {
        let mut c = Self {
            settings: ChainSettings::OFF,
            front: None,
            rnn: None,
            agc: None,
            feed,
            capture_delay_ms: 10,
            scratch: vec![0.0; BLOCK],
            out: vec![0.0; BLOCK],
            render_out: vec![0.0; BLOCK],
            stats: ChainStats::default(),
        };
        c.configure(settings);
        c
    }

    pub fn settings(&self) -> ChainSettings {
        self.settings
    }

    pub fn stats(&self) -> ChainStats {
        self.stats
    }

    pub fn reset_stats(&mut self) {
        self.stats = ChainStats::default();
    }

    pub fn set_capture_delay_ms(&mut self, ms: u32) {
        self.capture_delay_ms = ms;
    }

    pub fn latency_samples(&self) -> usize {
        if self.rnn.is_some() { BLOCK } else { 0 }
    }

    pub fn echo_return_loss_enhancement(&self) -> Option<f64> {
        self.front.as_ref().filter(|_| self.settings.echo_cancellation).and_then(|a| a.statistics().echo_return_loss_enhancement)
    }

    pub fn configure(&mut self, s: ChainSettings) {
        let old = self.settings;
        if s == old {
            return;
        }
        let front_changed = s.front() != old.front() || s.echo_cancellation != old.echo_cancellation || s.noise_suppression != old.noise_suppression;
        if front_changed {
            self.front = s.front().then(|| front_apm(s));
        }
        if s.noise_cancellation != old.noise_cancellation {
            self.rnn = s.noise_cancellation.then(DenoiseState::new);
        }
        if s.agc != old.agc {
            self.agc = s.agc.then(agc_apm);
        }
        self.settings = s;
    }

    pub fn process(&mut self, frame: &mut [f32]) -> Option<f32> {
        let start = Instant::now();
        let mut vad: Option<f32> = None;
        for block in frame.chunks_exact_mut(BLOCK) {
            if let Some(p) = self.process_block(block) {
                vad = Some(vad.map_or(p, |v| v.max(p)));
            }
        }
        let took = start.elapsed();
        let blocks = (frame.len() / BLOCK) as u64;
        self.stats.blocks += blocks;
        self.stats.busy += took;
        if blocks > 0 {
            self.stats.max_block = self.stats.max_block.max(took / blocks as u32);
        }
        vad
    }

    fn pull_render(&mut self) {
        let Some(feed) = self.feed.as_mut() else { return };
        while let Ok(b) = feed.rx.try_recv() {
            feed.pending.extend_from_slice(&b.mono);
            feed.ahead_ms = b.ahead_ms;
        }
        if feed.pending.len() > MAX_PENDING_RENDER {
            let excess = feed.pending.len() - MAX_PENDING_RENDER;
            feed.pending.drain(..excess);
        }
        let aec = self.front.as_mut().filter(|_| self.settings.echo_cancellation);
        let mut done = 0;
        if let Some(apm) = aec {
            while feed.pending.len() - done >= BLOCK {
                let blk = &feed.pending[done..done + BLOCK];
                let _ = apm.process_render_f32(&[blk], &mut [&mut self.render_out[..]]);
                done += BLOCK;
                self.stats.render_blocks += 1;
            }
            let _ = apm.set_stream_delay_ms((feed.ahead_ms + self.capture_delay_ms) as i32);
        } else {
            done = feed.pending.len() / BLOCK * BLOCK;
        }
        feed.pending.drain(..done);
    }

    fn process_block(&mut self, x: &mut [f32]) -> Option<f32> {
        self.pull_render();
        if let Some(apm) = self.front.as_mut() {
            if self.feed.is_none() && self.settings.echo_cancellation {
                let _ = apm.set_stream_delay_ms(self.capture_delay_ms as i32);
            }
            self.scratch.copy_from_slice(x);
            let _ = apm.process_capture_f32(&[&self.scratch[..]], &mut [&mut *x]);
        }
        let mut vad = None;
        if let Some(rnn) = self.rnn.as_mut() {
            for (d, s) in self.scratch.iter_mut().zip(x.iter()) {
                *d = s * PCM16;
            }
            vad = Some(rnn.process_frame(&mut self.out, &self.scratch));
            for (d, s) in x.iter_mut().zip(&self.out) {
                *d = (s / PCM16).clamp(-1.0, 1.0);
            }
        }
        if let Some(apm) = self.agc.as_mut() {
            self.scratch.copy_from_slice(x);
            let _ = apm.process_capture_f32(&[&self.scratch[..]], &mut [&mut *x]);
        }
        vad
    }
}

pub struct KrispGate {
    hang: u32,
}

impl KrispGate {
    pub const TRAILING_FRAMES: u32 = 12;

    pub fn new() -> Self {
        Self { hang: 0 }
    }

    pub fn threshold(agc: bool) -> f32 {
        if agc { 0.8 } else { 0.5 }
    }

    pub fn process(&mut self, prob: f32, threshold: f32) -> bool {
        if prob >= threshold {
            self.hang = Self::TRAILING_FRAMES;
            true
        } else if self.hang > 0 {
            self.hang -= 1;
            true
        } else {
            false
        }
    }
}

impl Default for KrispGate {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn sample(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
    }
}

#[derive(Clone, Copy)]
struct Resonator {
    a1: f32,
    a2: f32,
    b0: f32,
    y1: f32,
    y2: f32,
}

impl Resonator {
    fn new(freq: f32, bw: f32) -> Self {
        let r = (-std::f32::consts::PI * bw / RATE as f32).exp();
        let a1 = 2.0 * r * (std::f32::consts::TAU * freq / RATE as f32).cos();
        let a2 = -r * r;
        Self { a1, a2, b0: 1.0 - a1 - a2, y1: 0.0, y2: 0.0 }
    }

    fn retune(&mut self, freq: f32, bw: f32) {
        let n = Self::new(freq, bw);
        self.a1 = n.a1;
        self.a2 = n.a2;
        self.b0 = n.b0;
    }

    fn tick(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.a1 * self.y1 + self.a2 * self.y2;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

const VOWELS: [[f32; 3]; 5] = [[730.0, 1090.0, 2440.0], [270.0, 2290.0, 3010.0], [530.0, 1840.0, 2480.0], [570.0, 840.0, 2410.0], [300.0, 870.0, 2240.0]];
const BANDWIDTHS: [f32; 3] = [80.0, 100.0, 140.0];
const SYLLABLE: usize = RATE as usize / 4;
const VOICED: usize = SYLLABLE * 7 / 10;
const PHRASE: usize = RATE as usize * 2;
const PAUSE: usize = RATE as usize * 6 / 10;

#[derive(Clone)]
pub struct SyntheticVoice {
    n: usize,
    phase: f32,
    f0: f32,
    prev_pulse: f32,
    formants: [Resonator; 3],
    rng: Rng,
    syllable: usize,
    scale: f32,
}

impl SyntheticVoice {
    pub fn new(f0: f32, seed: u64) -> Self {
        let formants = [0, 1, 2].map(|i| Resonator::new(VOWELS[0][i], BANDWIDTHS[i]));
        let mut v = Self { n: 0, phase: 0.0, f0, prev_pulse: 0.0, formants, rng: Rng(seed | 1), syllable: usize::MAX, scale: 1.0 };
        let mut probe = v.clone();
        let mut buf = vec![0.0; PHRASE * 5];
        probe.fill(&mut buf);
        let (mut sum, mut cnt) = (0.0f64, 0usize);
        for (i, s) in buf.iter().enumerate() {
            if Self::active_at(i) {
                sum += f64::from(*s) * f64::from(*s);
                cnt += 1;
            }
        }
        let rms = (sum / cnt.max(1) as f64).sqrt() as f32;
        v.scale = if rms > 0.0 { 0.1 / rms } else { 1.0 };
        v
    }

    pub fn active_at(n: usize) -> bool {
        let p = n % PHRASE;
        p < PHRASE - PAUSE && p % SYLLABLE < VOICED
    }

    pub fn fill(&mut self, out: &mut [f32]) {
        for o in out.iter_mut() {
            let n = self.n;
            self.n += 1;
            let p = n % PHRASE;
            let syl = n / SYLLABLE;
            if syl != self.syllable {
                self.syllable = syl;
                let v = VOWELS[syl % VOWELS.len()];
                for (i, f) in self.formants.iter_mut().enumerate() {
                    f.retune(v[i] * (1.0 + 0.03 * self.rng.sample()), BANDWIDTHS[i]);
                }
            }
            if p >= PHRASE - PAUSE || p % SYLLABLE >= VOICED {
                *o = 0.0;
                for f in self.formants.iter_mut() {
                    f.tick(0.0);
                }
                continue;
            }
            let t = n as f32 / RATE as f32;
            let k = (p % SYLLABLE) as f32 / VOICED as f32;
            let env = (std::f32::consts::PI * k).sin().powf(0.6);
            let f0 = self.f0 * (1.0 + 0.12 * (std::f32::consts::TAU * 0.7 * t).sin() + 0.06 * (1.0 - k)) * (1.0 + 0.004 * self.rng.sample());
            self.phase += f0 / RATE as f32;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            let pulse = if self.phase < 0.4 {
                0.5 * (1.0 - (std::f32::consts::PI * self.phase / 0.4).cos())
            } else if self.phase < 0.56 {
                (std::f32::consts::FRAC_PI_2 * (self.phase - 0.4) / 0.16).cos()
            } else {
                0.0
            };
            let src = (pulse - self.prev_pulse) * 30.0 + 0.02 * self.rng.sample();
            self.prev_pulse = pulse;
            let mut y = src;
            for f in self.formants.iter_mut() {
                y = f.tick(y);
            }
            *o = (y * env * self.scale).clamp(-1.0, 1.0);
        }
    }
}

pub struct SyntheticNoise {
    rng: Rng,
    pink: [f32; 3],
    amp: f32,
}

impl SyntheticNoise {
    pub fn new(level_db: f32, seed: u64) -> Self {
        Self { rng: Rng(seed | 1), pink: [0.0; 3], amp: 10f32.powf(level_db / 20.0) }
    }

    pub fn fill(&mut self, out: &mut [f32]) {
        for o in out.iter_mut() {
            let w = self.rng.sample();
            self.pink[0] = 0.974 * self.pink[0] + 0.072 * w;
            self.pink[1] = 0.877 * self.pink[1] + 0.152 * w;
            self.pink[2] = 0.593 * self.pink[2] + 0.255 * w;
            let p = self.pink.iter().sum::<f32>() + 0.55 * w;
            *o = p * self.amp * 2.0;
        }
    }
}

pub struct SyntheticNoisyVoice {
    voice: SyntheticVoice,
    noise: SyntheticNoise,
    buf: Vec<f32>,
}

impl SyntheticNoisyVoice {
    pub fn new() -> Self {
        Self { voice: SyntheticVoice::new(120.0, 0x5eed), noise: SyntheticNoise::new(-36.0, 0xfeed), buf: Vec::new() }
    }

    pub fn fill(&mut self, out: &mut [f32]) {
        self.voice.fill(out);
        self.buf.resize(out.len(), 0.0);
        self.noise.fill(&mut self.buf);
        for (o, n) in out.iter_mut().zip(&self.buf) {
            *o = (*o + n).clamp(-1.0, 1.0);
        }
    }
}

impl Default for SyntheticNoisyVoice {
    fn default() -> Self {
        Self::new()
    }
}

fn energy_db(x: &[f32], mask: impl Fn(usize) -> bool) -> f32 {
    let (mut sum, mut n) = (0.0f64, 0usize);
    for (i, s) in x.iter().enumerate() {
        if mask(i) {
            sum += f64::from(*s) * f64::from(*s);
            n += 1;
        }
    }
    if n == 0 || sum <= 0.0 { -120.0 } else { (10.0 * (sum / n as f64).log10()) as f32 }
}

fn best_lag(reference: &[f32], output: &[f32], max_lag: usize) -> usize {
    let mut best = (0usize, f64::MIN);
    let len = reference.len().saturating_sub(max_lag);
    for lag in 0..=max_lag {
        let mut acc = 0.0f64;
        for i in (0..len).step_by(2) {
            acc += f64::from(reference[i]) * f64::from(output[i + lag]);
        }
        if acc > best.1 {
            best = (lag, acc);
        }
    }
    best.0
}

#[derive(Debug, Clone)]
pub struct ModeReport {
    pub settings: ChainSettings,
    pub noise_in_db: f32,
    pub noise_out_db: f32,
    pub speech_in_db: f32,
    pub speech_out_db: f32,
    pub latency_samples: usize,
    pub avg_block_us: f64,
    pub max_block_us: f64,
}

impl ModeReport {
    pub fn noise_reduction_db(&self) -> f32 {
        self.noise_in_db - self.noise_out_db
    }

    pub fn snr_gain_db(&self) -> f32 {
        (self.speech_out_db - self.noise_out_db) - (self.speech_in_db - self.noise_in_db)
    }
}

#[derive(Debug, Clone)]
pub struct EchoReport {
    pub echo_in_db: f32,
    pub echo_out_db: f32,
    pub near_in_db: f32,
    pub near_out_db: f32,
    pub erle: Option<f64>,
}

impl EchoReport {
    pub fn echo_reduction_db(&self) -> f32 {
        self.echo_in_db - self.echo_out_db
    }
}

pub fn noise_report(settings: ChainSettings, seconds: usize) -> ModeReport {
    let total = RATE as usize * seconds;
    let mut speech = vec![0.0; total];
    SyntheticVoice::new(120.0, 0x5eed).fill(&mut speech);
    let mut noise = vec![0.0; total];
    SyntheticNoise::new(-36.0, 0xfeed).fill(&mut noise);
    let input: Vec<f32> = speech.iter().zip(&noise).map(|(s, n)| s + n).collect();
    let mut chain = InputChain::new(settings, None);
    let mut out = input.clone();
    for frame in out.chunks_exact_mut(BLOCK * 2) {
        chain.process(frame);
    }
    let window = RATE as usize..RATE as usize * 3;
    let lat = best_lag(&input[window.clone()], &out[window], BLOCK * 2);
    let settle = RATE as usize * 2;
    let guard = BLOCK * 6;
    let quiet = |i: usize| i >= settle && (0..=guard).step_by(BLOCK).all(|d| !SyntheticVoice::active_at(i.saturating_sub(d)) && !SyntheticVoice::active_at(i + d));
    let loud = |i: usize| i >= settle && SyntheticVoice::active_at(i);
    let st = chain.stats();
    ModeReport {
        settings,
        noise_in_db: energy_db(&input, quiet),
        noise_out_db: energy_db(&out, |i| i >= lat && quiet(i - lat)),
        speech_in_db: energy_db(&input, loud),
        speech_out_db: energy_db(&out, |i| i >= lat && loud(i - lat)),
        latency_samples: lat,
        avg_block_us: st.avg_block_us(),
        max_block_us: st.max_block.as_secs_f64() * 1e6,
    }
}

pub fn echo_report(seconds: usize, delay_ms: u32) -> EchoReport {
    let total = RATE as usize * seconds;
    let delay = RATE as usize * delay_ms as usize / 1000;
    let mut far = vec![0.0; total];
    SyntheticVoice::new(190.0, 0xabcd).fill(&mut far);
    let mut near = vec![0.0; total];
    let double_talk = total * 3 / 4;
    let mut talker = SyntheticVoice::new(105.0, 0x1234);
    talker.fill(&mut near[double_talk..]);
    let mut floor = vec![0.0; total];
    SyntheticNoise::new(-65.0, 0x77).fill(&mut floor);
    let echo: Vec<f32> = (0..total)
        .map(|i| {
            let tap = |d: usize, g: f32| if i >= d { far[i - d] * g } else { 0.0 };
            tap(delay, 0.35) + tap(delay + 96, 0.12) + tap(delay + 310, 0.05)
        })
        .collect();
    let input: Vec<f32> = (0..total).map(|i| echo[i] + near[i] * 0.5 + floor[i]).collect();
    let (tap, feed) = render_channel();
    let mut chain = InputChain::new(ChainSettings { echo_cancellation: true, ..ChainSettings::OFF }, Some(feed));
    chain.set_capture_delay_ms(delay_ms);
    let mut out = input.clone();
    for (i, frame) in out.chunks_exact_mut(BLOCK).enumerate() {
        tap.push(&far[i * BLOCK..(i + 1) * BLOCK], 1, 0);
        chain.process(frame);
    }
    let settle = RATE as usize * 3;
    let echo_only = |i: usize| i >= settle && i < double_talk;
    let dt = |i: usize| i >= double_talk && SyntheticVoice::active_at(i - double_talk);
    let near_scaled: Vec<f32> = near.iter().map(|s| s * 0.5).collect();
    EchoReport {
        echo_in_db: energy_db(&input, echo_only),
        echo_out_db: energy_db(&out, echo_only),
        near_in_db: energy_db(&near_scaled, dt),
        near_out_db: energy_db(&out, dt),
        erle: chain.echo_return_loss_enhancement(),
    }
}

pub fn self_test_lines() -> Vec<String> {
    let modes = [
        ChainSettings::OFF,
        ChainSettings { noise_suppression: true, ..ChainSettings::OFF },
        ChainSettings { noise_cancellation: true, ..ChainSettings::OFF },
        ChainSettings { echo_cancellation: true, noise_cancellation: true, agc: true, ..ChainSettings::OFF },
        ChainSettings { echo_cancellation: true, noise_suppression: true, agc: true, ..ChainSettings::OFF },
    ];
    let mut lines: Vec<String> = modes
        .iter()
        .map(|m| {
            let r = noise_report(*m, 8);
            format!(
                "voice-proc selftest mode={} noise_in={:.1}dB noise_out={:.1}dB reduction={:.1}dB speech_in={:.1}dB speech_out={:.1}dB snr_gain={:.1}dB latency={:.2}ms cpu_avg={:.0}us cpu_max={:.0}us load={:.2}%",
                m.label(),
                r.noise_in_db,
                r.noise_out_db,
                r.noise_reduction_db(),
                r.speech_in_db,
                r.speech_out_db,
                r.snr_gain_db(),
                r.latency_samples as f64 * 1000.0 / RATE as f64,
                r.avg_block_us,
                r.max_block_us,
                r.avg_block_us / 100.0
            )
        })
        .collect();
    let e = echo_report(12, 60);
    lines.push(format!(
        "voice-proc selftest mode=aec3 echo_in={:.1}dB echo_out={:.1}dB echo_reduction={:.1}dB near_in={:.1}dB near_out={:.1}dB erle={:?}",
        e.echo_in_db,
        e.echo_out_db,
        e.echo_reduction_db(),
        e.near_in_db,
        e.near_out_db,
        e.erle
    ));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_voice_is_calibrated_and_has_pauses() {
        let mut v = SyntheticVoice::new(120.0, 1);
        let mut buf = vec![0.0; PHRASE * 2];
        v.fill(&mut buf);
        let active = energy_db(&buf, SyntheticVoice::active_at);
        let silent = energy_db(&buf, |i| !SyntheticVoice::active_at(i));
        assert!((active + 20.0).abs() < 3.0, "{active}");
        assert!(silent < -100.0, "{silent}");
    }

    #[test]
    fn bypass_is_transparent() {
        let r = noise_report(ChainSettings::OFF, 4);
        assert_eq!(r.latency_samples, 0);
        assert!(r.noise_reduction_db().abs() < 0.1, "{r:?}");
    }

    #[test]
    fn webrtc_noise_suppression_reduces_noise() {
        let r = noise_report(ChainSettings { noise_suppression: true, ..ChainSettings::OFF }, 6);
        assert!(r.noise_reduction_db() > 8.0, "{r:?}");
        assert!(r.snr_gain_db() > 4.0, "{r:?}");
        assert!(r.latency_samples <= BLOCK, "{r:?}");
    }

    #[test]
    fn rnnoise_reduces_noise_with_one_block_of_latency() {
        let r = noise_report(ChainSettings { noise_cancellation: true, ..ChainSettings::OFF }, 6);
        assert!(r.noise_reduction_db() > 10.0, "{r:?}");
        assert!(r.snr_gain_db() > 6.0, "{r:?}");
        assert!(r.latency_samples <= BLOCK * 2, "{r:?}");
    }

    #[test]
    fn aec3_removes_simulated_echo() {
        let e = echo_report(10, 60);
        assert!(e.echo_reduction_db() > 12.0, "{e:?}");
        assert!(e.near_out_db > e.near_in_db - 12.0, "{e:?}");
    }

    #[test]
    fn configure_rebuilds_only_changed_stages() {
        let mut c = InputChain::new(ChainSettings::OFF, None);
        assert!(c.front.is_none() && c.rnn.is_none() && c.agc.is_none());
        c.configure(ChainSettings { noise_cancellation: true, agc: true, ..ChainSettings::OFF });
        assert!(c.front.is_some() && c.rnn.is_some() && c.agc.is_some());
        assert_eq!(c.latency_samples(), BLOCK);
        c.configure(ChainSettings::OFF);
        assert!(c.front.is_none() && c.rnn.is_none() && c.agc.is_none());
    }

    #[test]
    fn krisp_gate_holds_then_closes() {
        let mut g = KrispGate::new();
        assert!(!g.process(0.1, 0.8));
        assert!(g.process(0.9, 0.8));
        for _ in 0..KrispGate::TRAILING_FRAMES {
            assert!(g.process(0.1, 0.8));
        }
        assert!(!g.process(0.1, 0.8));
    }

    #[test]
    fn render_tap_downmixes_stereo() {
        let (tap, feed) = render_channel();
        tap.push(&[1.0, 0.0, 0.5, 0.5], 2, 40);
        let b = feed.rx.try_recv().unwrap();
        assert_eq!(b.mono, vec![0.5, 0.5]);
        assert_eq!(b.ahead_ms, 40);
    }

    #[test]
    fn chain_processes_a_twenty_ms_frame_in_two_blocks() {
        let mut c = InputChain::new(ChainSettings { noise_cancellation: true, ..ChainSettings::OFF }, None);
        let mut f = vec![0.0; BLOCK * 2];
        assert!(c.process(&mut f).is_some());
        assert_eq!(c.stats().blocks, 2);
    }
}
