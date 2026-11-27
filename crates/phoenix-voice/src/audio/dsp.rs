pub fn level_db(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return -100.0;
    }
    let sum: f32 = frame.iter().map(|s| s * s).sum();
    let rms = (sum / frame.len() as f32).sqrt();
    if rms <= 1e-5 { -100.0 } else { (20.0 * rms.log10()).max(-100.0) }
}

#[derive(Debug, Clone)]
pub struct NoiseGate {
    pub automatic: bool,
    pub threshold_db: f32,
    pub hangover_frames: u32,
    noise_floor: f32,
    hang: u32,
    open: bool,
}

impl Default for NoiseGate {
    fn default() -> Self {
        Self { automatic: true, threshold_db: -50.0, hangover_frames: 15, noise_floor: -70.0, hang: 0, open: false }
    }
}

impl NoiseGate {
    pub fn threshold(&self) -> f32 {
        if self.automatic { (self.noise_floor + 12.0).max(-60.0) } else { self.threshold_db }
    }

    pub fn process(&mut self, db: f32) -> bool {
        if db < self.noise_floor {
            self.noise_floor = 0.8 * self.noise_floor + 0.2 * db;
        } else {
            let k = if self.open { 0.02 } else { 0.1 };
            self.noise_floor += k * (db - self.noise_floor);
        }
        if db >= self.threshold() {
            self.open = true;
            self.hang = self.hangover_frames;
        } else if self.hang > 0 {
            self.hang -= 1;
        } else {
            self.open = false;
        }
        self.open
    }

    pub fn is_open(&self) -> bool {
        self.open
    }
}

#[derive(Debug, Clone)]
pub struct Resampler {
    from: u32,
    to: u32,
    channels: usize,
    pos: f64,
    last: Vec<f32>,
}

impl Resampler {
    pub fn new(from: u32, to: u32, channels: usize) -> Self {
        Self { from, to, channels: channels.max(1), pos: 0.0, last: vec![0.0; channels.max(1)] }
    }

    pub fn is_identity(&self) -> bool {
        self.from == self.to
    }

    pub fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        if self.is_identity() {
            out.extend_from_slice(input);
            return;
        }
        let ch = self.channels;
        let frames = input.len() / ch;
        let step = self.from as f64 / self.to as f64;
        while self.pos < frames as f64 {
            let i = self.pos.floor();
            let t = (self.pos - i) as f32;
            let i = i as isize;
            for c in 0..ch {
                let a = if i < 1 { self.last[c] } else { input[(i as usize - 1) * ch + c] };
                let b = if i < 0 { self.last[c] } else { input[i as usize * ch + c] };
                out.push(a + (b - a) * t);
            }
            self.pos += step;
        }
        self.pos -= frames as f64;
        if frames > 0 {
            self.last.copy_from_slice(&input[(frames - 1) * ch..frames * ch]);
        }
    }
}

pub fn convert_channels(input: &[f32], from: usize, to: usize) -> Vec<f32> {
    if from == to {
        return input.to_vec();
    }
    let frames = input.len() / from.max(1);
    let mut out = Vec::with_capacity(frames * to);
    for f in input.chunks_exact(from) {
        if to == 1 {
            out.push(f.iter().sum::<f32>() / from as f32);
        } else if from == 1 {
            out.extend(std::iter::repeat_n(f[0], to));
        } else {
            for c in 0..to {
                out.push(f[c.min(from - 1)]);
            }
        }
    }
    out
}

pub fn mix_into(acc: &mut [f32], src: &[i16], gain: f32) {
    for (a, s) in acc.iter_mut().zip(src) {
        *a += *s as f32 / 32768.0 * gain;
    }
}

pub fn to_i16(x: &[f32]) -> Vec<i16> {
    x.iter().map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_opens_on_speech_and_closes_after_hangover() {
        let mut g = NoiseGate { automatic: false, threshold_db: -40.0, hangover_frames: 2, ..Default::default() };
        assert!(!g.process(-70.0));
        assert!(g.process(-20.0));
        assert!(g.process(-70.0));
        assert!(g.process(-70.0));
        assert!(!g.process(-70.0));
    }

    #[test]
    fn automatic_gate_ignores_steady_noise() {
        let mut g = NoiseGate::default();
        for _ in 0..200 {
            g.process(-45.0);
        }
        assert!(!g.process(-45.0));
        assert!(g.process(-15.0));
    }

    #[test]
    fn level_of_full_scale_sine_is_about_minus_3db() {
        let f: Vec<f32> = (0..960).map(|i| (i as f32 * 0.1).sin()).collect();
        assert!((level_db(&f) + 3.0).abs() < 0.5);
        assert_eq!(level_db(&[0.0; 10]), -100.0);
    }

    #[test]
    fn resampler_rate_ratio() {
        let mut r = Resampler::new(44100, 48000, 1);
        let mut out = Vec::new();
        for _ in 0..10 {
            r.process(&[0.5; 441], &mut out);
        }
        assert!((out.len() as i32 - 4800).abs() <= 2, "{}", out.len());
        assert!(out[100..].iter().all(|s| (*s - 0.5).abs() < 1e-6));
    }

    #[test]
    fn channel_conversion() {
        assert_eq!(convert_channels(&[1.0, 0.0], 2, 1), vec![0.5]);
        assert_eq!(convert_channels(&[0.3], 1, 2), vec![0.3, 0.3]);
    }
}
