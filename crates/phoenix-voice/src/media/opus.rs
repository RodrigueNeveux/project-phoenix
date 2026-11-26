use audiopus::coder::{Decoder, Encoder};
use audiopus::{Application, Bitrate, Channels, SampleRate};

use super::{AudioDecoder, AudioEncoder};
use crate::{Error, Result};

pub const FRAME_SAMPLES: usize = 960;

fn err(e: audiopus::Error) -> Error {
    Error::Encoder(format!("opus: {e}"))
}

pub struct OpusEncoder {
    inner: Encoder,
    channels: usize,
    buf: Vec<u8>,
}

impl OpusEncoder {
    pub fn new(stereo: bool, bitrate_bps: i32) -> Result<Self> {
        let ch = if stereo { Channels::Stereo } else { Channels::Mono };
        let mut inner = Encoder::new(SampleRate::Hz48000, ch, Application::Voip).map_err(err)?;
        inner.set_bitrate(Bitrate::BitsPerSecond(bitrate_bps)).map_err(err)?;
        Ok(Self { inner, channels: if stereo { 2 } else { 1 }, buf: vec![0; 1275] })
    }
}

impl OpusEncoder {
    pub fn for_media(stereo: bool) -> Result<Self> {
        let ch = if stereo { Channels::Stereo } else { Channels::Mono };
        let mut inner = Encoder::new(SampleRate::Hz48000, ch, Application::Audio).map_err(err)?;
        inner.set_bitrate(Bitrate::Auto).map_err(err)?;
        Ok(Self { inner, channels: if stereo { 2 } else { 1 }, buf: vec![0; 4000] })
    }
}

impl OpusEncoder {
    pub fn enable_fec(&mut self, expected_loss_pct: u8) -> Result<()> {
        self.inner.set_inband_fec(true).map_err(err)?;
        self.inner.set_packet_loss_perc(expected_loss_pct.min(100)).map_err(err)
    }
}

impl AudioEncoder for OpusEncoder {
    fn encode(&mut self, pcm: &[i16]) -> Result<Vec<u8>> {
        if pcm.len() != FRAME_SAMPLES * self.channels {
            return Err(Error::Encoder("opus: expected 20 ms of PCM".into()));
        }
        let n = self.inner.encode(pcm, &mut self.buf).map_err(err)?;
        Ok(self.buf[..n].to_vec())
    }

    fn set_bitrate(&mut self, bps: u32) -> Result<()> {
        let b = i32::try_from(bps).map_err(|_| Error::Encoder("bitrate".into()))?;
        self.inner.set_bitrate(Bitrate::BitsPerSecond(b)).map_err(err)
    }
}

pub struct OpusDecoder {
    inner: Decoder,
    channels: usize,
}

impl OpusDecoder {
    pub fn new(stereo: bool) -> Result<Self> {
        let ch = if stereo { Channels::Stereo } else { Channels::Mono };
        Ok(Self { inner: Decoder::new(SampleRate::Hz48000, ch).map_err(err)?, channels: if stereo { 2 } else { 1 } })
    }

    pub fn decode_fec(&mut self, next: &[u8]) -> Result<Vec<i16>> {
        let mut out = vec![0i16; FRAME_SAMPLES * self.channels];
        let n = self.inner.decode(Some(next), &mut out, true).map_err(err)?;
        out.truncate(n * self.channels);
        Ok(out)
    }
}

impl AudioDecoder for OpusDecoder {
    fn decode(&mut self, packet: Option<&[u8]>) -> Result<Vec<i16>> {
        let per_ch = if packet.is_some() { FRAME_SAMPLES * 6 } else { FRAME_SAMPLES };
        let mut out = vec![0i16; per_ch * self.channels];
        let n = self.inner.decode(packet, &mut out, false).map_err(err)?;
        out.truncate(n * self.channels);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opus_roundtrip() {
        let mut e = OpusEncoder::new(true, 64_000).unwrap();
        let mut d = OpusDecoder::new(true).unwrap();
        let pcm: Vec<i16> = (0..FRAME_SAMPLES * 2).map(|i| ((i as f32 * 0.05).sin() * 8000.0) as i16).collect();
        let pkt = e.encode(&pcm).unwrap();
        assert!(!pkt.is_empty());
        assert_eq!(d.decode(Some(&pkt)).unwrap().len(), FRAME_SAMPLES * 2);
        assert_eq!(d.decode(None).unwrap().len(), FRAME_SAMPLES * 2);
        assert!(e.encode(&pcm[..10]).is_err());
    }

    #[test]
    fn fec_recovers_a_lost_frame() {
        let mut e = OpusEncoder::new(true, 64_000).unwrap();
        e.enable_fec(10).unwrap();
        let mut d = OpusDecoder::new(true).unwrap();
        let pcm: Vec<i16> = (0..FRAME_SAMPLES * 2).map(|i| ((i as f32 * 0.03).sin() * 9000.0) as i16).collect();
        let pkts: Vec<Vec<u8>> = (0..4).map(|_| e.encode(&pcm).unwrap()).collect();
        d.decode(Some(&pkts[0])).unwrap();
        d.decode(Some(&pkts[1])).unwrap();
        let rec = d.decode_fec(&pkts[3]).unwrap();
        assert_eq!(rec.len(), FRAME_SAMPLES * 2);
        assert_eq!(d.decode(Some(&pkts[3])).unwrap().len(), FRAME_SAMPLES * 2);
    }
}
