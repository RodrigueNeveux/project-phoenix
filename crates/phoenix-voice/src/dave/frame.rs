//! DAVE media frame transform.
//!
//! Encrypted frame layout:
//! ```text
//! | interleaved frame (clear ranges + ciphertext) | tag (8) | ULEB128 nonce |
//! | ULEB128 (offset, len) pairs of clear ranges | supplemental size (1) | 0xFA 0xFA |
//! ```
//! `supplemental size` counts everything after the frame body, itself and the
//! magic included. Cipher: AES-128-GCM with an 8-byte tag; the 12-byte nonce
//! has the 32-bit truncated nonce (little-endian) in bytes 8..12. The AAD is
//! the concatenation of the clear ranges. The key generation is
//! `truncated_nonce >> 24`.
//!
//! Clear ranges keep the bits that SFUs / packetizers need visible:
//! - Opus: none (silence frames `F8 FF FE` are sent unencrypted as-is).
//! - VP8: 10-byte keyframe header or 1-byte delta header.
//! - VP9: none.
//! - H.264: Annex-B start codes + NAL header byte of slices; non-VCL NALs
//!   (SPS/PPS/SEI/AUD) fully clear. Ciphertext must not emulate a start code,
//!   so encryption is retried with a fresh nonce when it does.
//! - AV1: OBU headers and size fields clear, OBU payloads encrypted.

use aes_gcm::aead::consts::{U12, U8};
use aes_gcm::aead::{AeadInPlace, KeyInit};
use aes_gcm::aes::Aes128;
use aes_gcm::AesGcm;

use super::ratchet::KeyRatchet;
use crate::codec::Codec;
use crate::{leb128, Error, Result};

type Aes128Gcm8 = AesGcm<Aes128, U12, U8>;

pub const MAGIC: [u8; 2] = [0xFA, 0xFA];
pub const TAG_LEN: usize = 8;
pub const OPUS_SILENCE: [u8; 3] = [0xF8, 0xFF, 0xFE];
const GENERATION_SHIFT: u32 = 24;
const MAX_START_CODE_RETRIES: usize = 16;

/// A clear (unencrypted) byte range in the original frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub offset: usize,
    pub len: usize,
}

/// Compute clear ranges for a codec frame. Ranges are sorted and non-overlapping.
pub fn unencrypted_ranges(codec: Codec, frame: &[u8]) -> Result<Vec<Range>> {
    let mut r = match codec {
        Codec::Opus | Codec::Vp9 => Vec::new(),
        Codec::Vp8 => {
            let n = if crate::rtp::vp8::is_keyframe(frame) { 10 } else { 1 };
            vec![Range { offset: 0, len: n.min(frame.len()) }]
        }
        Codec::H264 | Codec::H265 => annex_b_ranges(codec, frame),
        Codec::Av1 => av1_ranges(frame)?,
    };
    r.retain(|x| x.len > 0);
    Ok(merge(r))
}

fn merge(mut v: Vec<Range>) -> Vec<Range> {
    v.sort_by_key(|r| r.offset);
    let mut out: Vec<Range> = Vec::with_capacity(v.len());
    for r in v {
        if let Some(l) = out.last_mut()
            && l.offset + l.len >= r.offset
        {
            l.len = (r.offset + r.len).max(l.offset + l.len) - l.offset;
            continue;
        }
        out.push(r);
    }
    out
}

fn annex_b_ranges(codec: Codec, f: &[u8]) -> Vec<Range> {
    let mut out = Vec::new();
    let mut starts = Vec::new(); // (start code offset, nal offset)
    let mut i = 0;
    while i + 3 <= f.len() {
        if f[i] == 0 && f[i + 1] == 0 && f[i + 2] == 1 {
            let sc = if i > 0 && f[i - 1] == 0 { i - 1 } else { i };
            starts.push((sc, i + 3));
            i += 3;
        } else {
            i += 1;
        }
    }
    for (k, &(sc, nal)) in starts.iter().enumerate() {
        let end = starts.get(k + 1).map(|s| s.0).unwrap_or(f.len());
        let (hdr_len, vcl) = match codec {
            Codec::H265 => (2, f.get(nal).is_some_and(|b| (b >> 1) & 0x3f < 32)),
            _ => (1, f.get(nal).is_some_and(|b| matches!(b & 0x1f, 1..=5))),
        };
        let clear_end = if vcl { (nal + hdr_len).min(end) } else { end };
        out.push(Range { offset: sc, len: clear_end - sc });
    }
    out
}

fn av1_ranges(f: &[u8]) -> Result<Vec<Range>> {
    let mut out = Vec::new();
    let mut o = 0;
    while o < f.len() {
        let h = f[o];
        let start = o;
        o += if h & 0x04 != 0 { 2 } else { 1 };
        let len = if h & 0x02 != 0 {
            let (l, n) = leb128::read(f.get(o..).ok_or(Error::Malformed("av1 obu"))?)?;
            o += n;
            l as usize
        } else {
            f.len().saturating_sub(o)
        };
        out.push(Range { offset: start, len: o - start });
        o = o.checked_add(len).filter(|e| *e <= f.len()).ok_or(Error::Malformed("av1 obu size"))?;
    }
    Ok(out)
}

fn split(frame: &[u8], ranges: &[Range]) -> (Vec<u8>, Vec<u8>) {
    let mut clear = Vec::new();
    let mut enc = Vec::with_capacity(frame.len());
    let mut o = 0;
    for r in ranges {
        enc.extend_from_slice(&frame[o..r.offset]);
        clear.extend_from_slice(&frame[r.offset..r.offset + r.len]);
        o = r.offset + r.len;
    }
    enc.extend_from_slice(&frame[o..]);
    (clear, enc)
}

fn interleave(clear: &[u8], enc: &[u8], ranges: &[Range], total: usize) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(total);
    let (mut c, mut e) = (0usize, 0usize);
    for r in ranges {
        let gap = r.offset.checked_sub(out.len()).ok_or(Error::Malformed("dave ranges overlap"))?;
        out.extend_from_slice(enc.get(e..e + gap).ok_or(Error::Malformed("dave ranges"))?);
        e += gap;
        out.extend_from_slice(clear.get(c..c + r.len).ok_or(Error::Malformed("dave ranges"))?);
        c += r.len;
    }
    out.extend_from_slice(enc.get(e..).ok_or(Error::Malformed("dave ranges"))?);
    Ok(out)
}

fn nonce_bytes(truncated: u32) -> [u8; 12] {
    let mut n = [0u8; 12];
    n[8..].copy_from_slice(&truncated.to_le_bytes());
    n
}

/// True if `data` contains `00 00 0x` (x <= 3) not fully inside a clear range.
fn emulates_start_code(data: &[u8], clear: &[Range]) -> bool {
    data.windows(3).enumerate().any(|(i, w)| {
        w[0] == 0
            && w[1] == 0
            && w[2] <= 3
            && !clear.iter().any(|r| i >= r.offset && i + 3 <= r.offset + r.len)
    })
}

/// Sender-side transform (one per outgoing media stream).
pub struct FrameEncryptor {
    ratchet: Box<dyn KeyRatchet>,
    nonce: u32,
    key: Option<(u32, Aes128Gcm8)>,
}

impl FrameEncryptor {
    pub fn new(ratchet: Box<dyn KeyRatchet>) -> Self {
        Self { ratchet, nonce: 0, key: None }
    }

    /// Replace the ratchet (new MLS epoch); nonce restarts at 0.
    pub fn set_ratchet(&mut self, ratchet: Box<dyn KeyRatchet>) {
        self.ratchet = ratchet;
        self.nonce = 0;
        self.key = None;
    }

    fn cipher(&mut self, generation: u32) -> Result<&Aes128Gcm8> {
        if self.key.as_ref().is_none_or(|(g, _)| *g != generation) {
            let k = self.ratchet.key_for_generation(generation)?;
            let c = Aes128Gcm8::new_from_slice(&k).map_err(|_| Error::Crypto("dave key"))?;
            self.key = Some((generation, c));
            if generation > 0 {
                self.ratchet.erase_before(generation);
            }
        }
        self.key.as_ref().map(|(_, c)| c).ok_or(Error::Crypto("dave key"))
    }

    pub fn encrypt(&mut self, codec: Codec, frame: &[u8]) -> Result<Vec<u8>> {
        if codec == Codec::Opus && frame == OPUS_SILENCE {
            return Ok(frame.to_vec());
        }
        let ranges = unencrypted_ranges(codec, frame)?;
        let (clear, plain) = split(frame, &ranges);
        let check_sc = matches!(codec, Codec::H264 | Codec::H265);
        for _ in 0..MAX_START_CODE_RETRIES {
            let truncated = self.nonce;
            self.nonce = self.nonce.wrapping_add(1);
            let generation = truncated >> GENERATION_SHIFT;
            let cipher = self.cipher(generation)?;
            let mut body = plain.clone();
            let tag = cipher
                .encrypt_in_place_detached((&nonce_bytes(truncated)).into(), &clear, &mut body)
                .map_err(|_| Error::Crypto("dave encrypt"))?;
            let mut footer = Vec::with_capacity(32);
            footer.extend_from_slice(&tag);
            leb128::write(truncated as u64, &mut footer);
            for r in &ranges {
                leb128::write(r.offset as u64, &mut footer);
                leb128::write(r.len as u64, &mut footer);
            }
            let supp = footer.len() + 1 + MAGIC.len();
            let supp_u8 = u8::try_from(supp).map_err(|_| Error::Unsupported("too many clear ranges".into()))?;
            footer.push(supp_u8);
            footer.extend_from_slice(&MAGIC);
            let mut out = interleave(&clear, &body, &ranges, frame.len() + supp)?;
            if check_sc && emulates_start_code(&out, &ranges) {
                continue;
            }
            out.extend_from_slice(&footer);
            return Ok(out);
        }
        Err(Error::Crypto("could not avoid start code emulation"))
    }
}

/// Parsed footer of an encrypted DAVE frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameFooter {
    pub tag: [u8; TAG_LEN],
    pub truncated_nonce: u32,
    pub ranges: Vec<Range>,
    pub body_len: usize,
}

/// True if `frame` ends with the DAVE magic marker.
pub fn is_encrypted(frame: &[u8]) -> bool {
    frame.len() >= TAG_LEN + 4 && frame[frame.len() - 2..] == MAGIC
}

pub fn parse_footer(frame: &[u8]) -> Result<FrameFooter> {
    if !is_encrypted(frame) {
        return Err(Error::Malformed("missing dave magic"));
    }
    let supp = frame[frame.len() - 3] as usize;
    if supp < TAG_LEN + 1 + 3 || supp > frame.len() {
        return Err(Error::Malformed("dave supplemental size"));
    }
    let body_len = frame.len() - supp;
    let s = &frame[body_len..frame.len() - 3];
    let mut tag = [0u8; TAG_LEN];
    tag.copy_from_slice(&s[..TAG_LEN]);
    let mut o = TAG_LEN;
    let (nonce, n) = leb128::read(&s[o..])?;
    o += n;
    let truncated_nonce = u32::try_from(nonce).map_err(|_| Error::Malformed("dave nonce"))?;
    let mut ranges = Vec::new();
    let mut last_end = 0usize;
    while o < s.len() {
        let (off, n1) = leb128::read(&s[o..])?;
        o += n1;
        let (len, n2) = leb128::read(s.get(o..).ok_or(Error::Malformed("dave range"))?)?;
        o += n2;
        let r = Range { offset: off as usize, len: len as usize };
        if r.offset < last_end || r.offset.saturating_add(r.len) > body_len {
            return Err(Error::Malformed("dave range bounds"));
        }
        last_end = r.offset + r.len;
        ranges.push(r);
    }
    Ok(FrameFooter { tag, truncated_nonce, ranges, body_len })
}

/// Receiver-side transform (one per remote sender).
pub struct FrameDecryptor {
    ratchet: Box<dyn KeyRatchet>,
    keys: std::collections::BTreeMap<u32, Aes128Gcm8>,
    /// If true, frames without the magic marker are passed through (transition periods).
    pub allow_passthrough: bool,
}

impl FrameDecryptor {
    pub fn new(ratchet: Box<dyn KeyRatchet>) -> Self {
        Self { ratchet, keys: Default::default(), allow_passthrough: false }
    }

    pub fn set_ratchet(&mut self, ratchet: Box<dyn KeyRatchet>) {
        self.ratchet = ratchet;
        self.keys.clear();
    }

    pub fn decrypt(&mut self, frame: &[u8]) -> Result<Vec<u8>> {
        if frame == OPUS_SILENCE {
            return Ok(frame.to_vec());
        }
        if !is_encrypted(frame) {
            return if self.allow_passthrough {
                Ok(frame.to_vec())
            } else {
                Err(Error::Crypto("unencrypted frame while E2EE required"))
            };
        }
        let f = parse_footer(frame)?;
        let body = &frame[..f.body_len];
        let (clear, mut enc) = split(body, &f.ranges);
        let generation = f.truncated_nonce >> GENERATION_SHIFT;
        if !self.keys.contains_key(&generation) {
            let k = self.ratchet.key_for_generation(generation)?;
            let c = Aes128Gcm8::new_from_slice(&k).map_err(|_| Error::Crypto("dave key"))?;
            self.keys.insert(generation, c);
            while self.keys.len() > 4 {
                if let Some((&g, _)) = self.keys.iter().next() {
                    self.keys.remove(&g);
                }
            }
        }
        let cipher = self.keys.get(&generation).ok_or(Error::Crypto("dave key"))?;
        cipher
            .decrypt_in_place_detached((&nonce_bytes(f.truncated_nonce)).into(), &clear, &mut enc, (&f.tag).into())
            .map_err(|_| Error::Crypto("dave authentication failed"))?;
        interleave(&clear, &enc, &f.ranges, f.body_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dave::ratchet::{HashRatchet, StaticRatchet};

    fn pair() -> (FrameEncryptor, FrameDecryptor) {
        (
            FrameEncryptor::new(Box::new(HashRatchet::new(&[9u8; 16]))),
            FrameDecryptor::new(Box::new(HashRatchet::new(&[9u8; 16]))),
        )
    }

    #[test]
    fn opus_roundtrip_and_layout() {
        let (mut e, mut d) = pair();
        let frame = b"some opus frame bytes".to_vec();
        let enc = e.encrypt(Codec::Opus, &frame).unwrap();
        assert_eq!(&enc[enc.len() - 2..], &MAGIC);
        let f = parse_footer(&enc).unwrap();
        assert_eq!(f.truncated_nonce, 0);
        assert!(f.ranges.is_empty());
        assert_eq!(f.body_len, frame.len());
        assert_eq!(enc[enc.len() - 3] as usize, TAG_LEN + 1 + 1 + 2);
        assert_eq!(d.decrypt(&enc).unwrap(), frame);
        let enc2 = e.encrypt(Codec::Opus, &frame).unwrap();
        assert_ne!(enc, enc2);
        assert_eq!(parse_footer(&enc2).unwrap().truncated_nonce, 1);
        assert_eq!(d.decrypt(&enc2).unwrap(), frame);
        // silence passthrough
        assert_eq!(e.encrypt(Codec::Opus, &OPUS_SILENCE).unwrap(), OPUS_SILENCE);
    }

    #[test]
    fn tamper_detected() {
        let (mut e, mut d) = pair();
        let mut enc = e.encrypt(Codec::Vp8, &[0x10, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]).unwrap();
        enc[0] ^= 0x01; // clear range is authenticated
        assert!(d.decrypt(&enc).is_err());
    }

    #[test]
    fn vp8_keyframe_header_clear() {
        let (mut e, mut d) = pair();
        let frame: Vec<u8> = (0..200u8).map(|b| b & 0xfe).collect();
        let enc = e.encrypt(Codec::Vp8, &frame).unwrap();
        assert_eq!(&enc[..10], &frame[..10]);
        assert_eq!(parse_footer(&enc).unwrap().ranges, vec![Range { offset: 0, len: 10 }]);
        assert_eq!(d.decrypt(&enc).unwrap(), frame);
    }

    #[test]
    fn h264_nal_headers_clear_no_start_code_emulation() {
        let mut frame = vec![0, 0, 0, 1, 0x67, 0x42, 0xe0, 0x1f, 0, 0, 0, 1, 0x68, 0xce, 0, 0, 1, 0x65];
        frame.extend((0..3000u32).map(|i| (i % 200) as u8 + 2));
        frame.extend_from_slice(&[0, 0, 1, 0x41]);
        frame.extend((0..500u32).map(|i| (i % 100) as u8 + 2));
        let mut e = FrameEncryptor::new(Box::new(StaticRatchet([1; 16])));
        let mut d = FrameDecryptor::new(Box::new(StaticRatchet([1; 16])));
        for _ in 0..20 {
            let enc = e.encrypt(Codec::H264, &frame).unwrap();
            let f = parse_footer(&enc).unwrap();
            // SPS+PPS fully clear, then IDR header and slice header byte.
            assert_eq!(f.ranges[0], Range { offset: 0, len: 18 });
            assert_eq!(&enc[..18], &frame[..18]);
            let body = &enc[..f.body_len];
            let starts = body.windows(3).filter(|w| w == &[0, 0, 1]).count();
            assert_eq!(starts, 4, "no emulated start codes");
            assert_eq!(d.decrypt(&enc).unwrap(), frame);
        }
    }

    #[test]
    fn av1_obu_headers_clear() {
        let mut frame = vec![0x0a, 0x03, 1, 2, 3]; // seq header with size
        frame.extend_from_slice(&[0x32, 0x05, 9, 9, 9, 9, 9]); // frame OBU
        let ranges = unencrypted_ranges(Codec::Av1, &frame).unwrap();
        assert_eq!(ranges, vec![Range { offset: 0, len: 2 }, Range { offset: 5, len: 2 }]);
        let (mut e, mut d) = pair();
        let enc = e.encrypt(Codec::Av1, &frame).unwrap();
        assert_eq!(d.decrypt(&enc).unwrap(), frame);
    }

    #[test]
    fn generation_rollover_uses_new_key() {
        let (mut e, mut d) = pair();
        e.nonce = (1 << 24) - 1;
        let a = e.encrypt(Codec::Opus, b"x1").unwrap();
        let b = e.encrypt(Codec::Opus, b"x2").unwrap();
        assert_eq!(parse_footer(&b).unwrap().truncated_nonce >> 24, 1);
        assert_eq!(d.decrypt(&a).unwrap(), b"x1");
        assert_eq!(d.decrypt(&b).unwrap(), b"x2");
    }

    #[test]
    fn passthrough_policy() {
        let (_, mut d) = pair();
        assert!(d.decrypt(b"plain").is_err());
        d.allow_passthrough = true;
        assert_eq!(d.decrypt(b"plain").unwrap(), b"plain");
    }
}
