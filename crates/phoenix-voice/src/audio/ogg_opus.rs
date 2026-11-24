use super::effects::Pcm;
use crate::media::AudioDecoder;
use crate::media::opus::OpusDecoder;

pub fn is_ogg(bytes: &[u8]) -> bool {
    bytes.starts_with(b"OggS")
}

pub fn packets(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut cur: Vec<u8> = Vec::new();
    let mut i = 0usize;
    while i + 27 <= bytes.len() && &bytes[i..i + 4] == b"OggS" {
        let nseg = bytes[i + 26] as usize;
        let table_end = i + 27 + nseg;
        if table_end > bytes.len() {
            break;
        }
        let mut pos = table_end;
        for &len in &bytes[i + 27..table_end] {
            let len = len as usize;
            if pos + len > bytes.len() {
                return out;
            }
            cur.extend_from_slice(&bytes[pos..pos + len]);
            pos += len;
            if len < 255 {
                out.push(std::mem::take(&mut cur));
            }
        }
        i = pos;
    }
    out
}

pub fn decode(bytes: &[u8]) -> Option<Pcm> {
    let pk = packets(bytes);
    let head = pk.first()?;
    if !head.starts_with(b"OpusHead") || head.len() < 19 {
        return None;
    }
    let channels = head[9].clamp(1, 2) as u16;
    let pre_skip = u16::from_le_bytes([head[10], head[11]]) as usize;
    let mut dec = OpusDecoder::new(channels == 2).ok()?;
    let mut samples: Vec<f32> = Vec::new();
    for p in pk.iter().skip(2) {
        if p.is_empty() {
            continue;
        }
        if let Ok(pcm) = dec.decode(Some(p.as_slice())) {
            samples.extend(pcm.iter().map(|s| *s as f32 / 32768.0));
        }
    }
    let skip = (pre_skip * channels as usize).min(samples.len());
    samples.drain(..skip);
    (!samples.is_empty()).then_some(Pcm { rate: 48_000, channels, samples })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::AudioEncoder;
    use crate::media::opus::OpusEncoder;

    fn page(seq: u32, packets: &[&[u8]]) -> Vec<u8> {
        let mut table = Vec::new();
        let mut data = Vec::new();
        for p in packets {
            let mut n = p.len();
            while n >= 255 {
                table.push(255u8);
                n -= 255;
            }
            table.push(n as u8);
            data.extend_from_slice(p);
        }
        let mut out = b"OggS".to_vec();
        out.extend([0u8, 0]);
        out.extend([0u8; 8]);
        out.extend(1u32.to_le_bytes());
        out.extend(seq.to_le_bytes());
        out.extend([0u8; 4]);
        out.push(table.len() as u8);
        out.extend(table);
        out.extend(data);
        out
    }

    #[test]
    fn demuxes_and_decodes() {
        let mut head = b"OpusHead".to_vec();
        head.extend([1u8, 2]);
        head.extend(312u16.to_le_bytes());
        head.extend(48_000u32.to_le_bytes());
        head.extend([0u8, 0, 0]);
        let mut enc = OpusEncoder::new(true, 64_000).unwrap();
        let pcm: Vec<i16> = (0..960 * 2).map(|i| ((i as f32 * 0.05).sin() * 8000.0) as i16).collect();
        let a = enc.encode(&pcm).unwrap();
        let b = enc.encode(&pcm).unwrap();
        let big = vec![7u8; 600];
        let mut file = page(0, &[head.as_slice()]);
        file.extend(page(1, &[b"OpusTags".as_slice()]));
        file.extend(page(2, &[a.as_slice(), b.as_slice()]));
        assert_eq!(packets(&page(3, &[big.as_slice()]))[0].len(), 600);
        let out = decode(&file).unwrap();
        assert_eq!(out.channels, 2);
        assert_eq!(out.samples.len(), (960 * 2 - 312) * 2);
        assert!(is_ogg(&file));
    }
}
