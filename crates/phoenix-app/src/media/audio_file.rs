use phoenix_voice::audio::effects::Pcm;
use symphonia::core::{
    audio::SampleBuffer,
    codecs::{CODEC_TYPE_NULL, DecoderOptions},
    errors::Error,
    formats::{FormatOptions, FormatReader},
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};

pub const METADATA_RANGE: &str = "bytes=0-262143";

pub const MAX_BYTES: usize = 128 * 1024 * 1024;

fn extension(name: &str) -> String {
    name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default()
}

fn open(bytes: &[u8], name: &str) -> Option<Box<dyn FormatReader>> {
    let mss = MediaSourceStream::new(Box::new(std::io::Cursor::new(bytes.to_vec())), Default::default());
    let mut hint = Hint::new();
    let ext = extension(name);
    if !ext.is_empty() {
        hint.with_extension(&ext);
    }
    let opts = FormatOptions { enable_gapless: true, ..Default::default() };
    symphonia::default::get_probe().format(&hint, mss, &opts, &MetadataOptions::default()).ok().map(|p| p.format)
}

pub fn probe_duration(bytes: &[u8], name: &str) -> Option<f64> {
    let format = open(bytes, name)?;
    let track = format.tracks().iter().find(|t| t.codec_params.codec != CODEC_TYPE_NULL)?;
    let p = &track.codec_params;
    let frames = p.n_frames?;
    if let Some(tb) = p.time_base {
        let t = tb.calc_time(frames);
        let secs = t.seconds as f64 + t.frac;
        return (secs > 0.).then_some(secs);
    }
    let rate = p.sample_rate?;
    (rate > 0 && frames > 0).then(|| frames as f64 / f64::from(rate))
}

pub fn decode(bytes: &[u8], name: &str) -> Option<Pcm> {
    if let Some(pcm) = decode_symphonia(bytes, name) {
        return Some(pcm);
    }
    phoenix_voice::audio::ogg_opus::decode(bytes)
}

fn decode_symphonia(bytes: &[u8], name: &str) -> Option<Pcm> {
    let mut format = open(bytes, name)?;
    let track = format.tracks().iter().find(|t| t.codec_params.codec != CODEC_TYPE_NULL)?;
    let tid = track.id;
    let mut dec = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default()).ok()?;
    let (mut samples, mut rate, mut channels) = (Vec::new(), 0u32, 0u16);
    while let Ok(pkt) = format.next_packet() {
        if pkt.track_id() != tid {
            continue;
        }
        match dec.decode(&pkt) {
            Ok(buf) => {
                let spec = *buf.spec();
                rate = spec.rate;
                channels = spec.channels.count() as u16;
                let mut sb = SampleBuffer::<f32>::new(buf.capacity() as u64, spec);
                sb.copy_interleaved_ref(buf);
                samples.extend_from_slice(sb.samples());
            }
            Err(Error::DecodeError(_)) => continue,
            Err(_) => break,
        }
    }
    (rate > 0 && channels > 0 && !samples.is_empty()).then_some(Pcm { rate, channels, samples })
}

pub fn format_time(secs: f64) -> String {
    let t = if secs.is_finite() && secs > 0. { secs as u64 } else { 0 };
    format!("{}:{:02}", t / 60, t % 60)
}

pub fn duration_text(current: Option<f64>, duration: Option<f64>) -> (String, String) {
    let d = duration.map_or_else(|| "-:--".to_string(), format_time);
    let mut c = current.map_or_else(|| "-:--".to_string(), format_time);
    while c.chars().count() < d.chars().count() {
        c.insert(0, '0');
    }
    (c, d)
}

#[cfg(test)]
pub fn wav(rate: u32, channels: u16, frames: u32) -> Vec<u8> {
    let data_len = frames * u32::from(channels) * 2;
    let mut v = Vec::new();
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + data_len).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&channels.to_le_bytes());
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
    v.extend_from_slice(&(channels * 2).to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..frames * u32::from(channels) {
        let s = ((i as f32 * 0.05).sin() * 8000.) as i16;
        v.extend_from_slice(&s.to_le_bytes());
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_duration_comes_from_the_header() {
        let w = wav(44_100, 2, 44_100 * 3 + 22_050);
        let d = probe_duration(&w, "tonalite.wav").unwrap();
        assert!((d - 3.5).abs() < 1e-6, "{d}");
        let head = &w[..4096];
        let d = probe_duration(head, "tonalite.wav").unwrap();
        assert!((d - 3.5).abs() < 1e-6, "partial file still knows its length: {d}");
    }

    #[test]
    fn wav_decodes_to_interleaved_pcm() {
        let w = wav(8_000, 2, 800);
        let p = decode(&w, "a.wav").unwrap();
        assert_eq!((p.rate, p.channels), (8_000, 2));
        assert_eq!(p.samples.len(), 1600);
        assert!((p.duration().as_secs_f64() - 0.1).abs() < 1e-6);
    }

    #[test]
    fn garbage_is_not_audio() {
        assert!(decode(b"not audio at all", "x.mp3").is_none());
        assert!(probe_duration(b"nope", "x.wav").is_none());
    }

    #[test]
    fn duration_text_matches_the_client() {
        assert_eq!(duration_text(None, None), ("-:--".into(), "-:--".into()));
        assert_eq!(duration_text(Some(0.), Some(7.4)), ("0:00".into(), "0:07".into()));
        assert_eq!(duration_text(Some(5.), Some(754.)), ("00:05".into(), "12:34".into()));
        assert_eq!(duration_text(Some(61.9), Some(3600.)), ("01:01".into(), "60:00".into()));
    }
}
