use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;

use super::{EncodedVideo, VideoEncoder, VideoFrame};
use crate::codec::{Codec, H264Profile, VideoEncoderConfig};
use crate::rtp::h264;
use crate::{Error, Result};

pub fn ffmpeg_command() -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new("ffmpeg");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

pub fn available() -> bool {
    ffmpeg_command().arg("-version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success())
}

pub fn split_access_units(buf: &mut Vec<u8>, out: &mut Vec<Vec<u8>>) {
    let aud = [0u8, 0, 0, 1, 9];
    let mut starts = Vec::new();
    let mut i = 0;
    while i + aud.len() <= buf.len() {
        if buf[i..i + aud.len()] == aud {
            starts.push(i);
            i += aud.len();
        } else {
            i += 1;
        }
    }
    if starts.len() < 2 {
        return;
    }
    for w in starts.windows(2) {
        out.push(buf[w[0]..w[1]].to_vec());
    }
    let last = *starts.last().unwrap_or(&0);
    buf.drain(..last);
}

pub struct FfmpegH264 {
    child: Child,
    stdin: Option<ChildStdin>,
    rx: mpsc::Receiver<Vec<u8>>,
    config: VideoEncoderConfig,
    frames: u64,
}

impl FfmpegH264 {
    pub fn spawn(config: &VideoEncoderConfig) -> Result<Self> {
        let kbps = (config.bitrate_bps / 1000).max(100);
        let profile = match config.h264_profile.unwrap_or(H264Profile::ConstrainedBaseline) {
            H264Profile::ConstrainedBaseline => "baseline",
            H264Profile::Main => "main",
        };
        let mut child = ffmpeg_command()
            .args(["-hide_banner", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgba", "-s"])
            .arg(format!("{}x{}", config.width, config.height))
            .args(["-r", &config.fps.to_string(), "-i", "-", "-c:v", "libx264", "-preset", "ultrafast", "-tune", "zerolatency", "-profile:v", profile])
            .args(["-pix_fmt", "yuv420p", "-bf", "0", "-g", &config.keyframe_interval.max(1).to_string()])
            .args(["-b:v", &format!("{kbps}k"), "-maxrate", &format!("{kbps}k"), "-bufsize", &format!("{}k", kbps * 2)])
            .args(["-x264-params", "aud=1:repeat-headers=1:sliced-threads=0", "-f", "h264", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Error::Encoder(format!("ffmpeg: {e}")))?;
        let stdin = child.stdin.take();
        let mut stdout = child.stdout.take().ok_or(Error::Encoder("ffmpeg stdout".into()))?;
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("phoenix-h264-out".into())
            .spawn(move || {
                let mut buf = Vec::new();
                let mut chunk = vec![0u8; 64 * 1024];
                let mut aus = Vec::new();
                while let Ok(n) = stdout.read(&mut chunk) {
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                    split_access_units(&mut buf, &mut aus);
                    for au in aus.drain(..) {
                        if tx.send(au).is_err() {
                            return;
                        }
                    }
                }
                if !buf.is_empty() {
                    let _ = tx.send(buf);
                }
            })
            .map_err(Error::Io)?;
        Ok(Self { child, stdin, rx, config: config.clone(), frames: 0 })
    }
}

impl Drop for FfmpegH264 {
    fn drop(&mut self) {
        self.stdin.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl VideoEncoder for FfmpegH264 {
    fn codec(&self) -> Codec {
        Codec::H264
    }

    fn encode(&mut self, frame: &VideoFrame, force_keyframe: bool) -> Result<Option<EncodedVideo>> {
        if frame.width != self.config.width || frame.height != self.config.height {
            return Err(Error::Encoder("frame size changed".into()));
        }
        if force_keyframe && self.frames > 0 {
            *self = Self::spawn(&self.config)?;
        }
        let stdin = self.stdin.as_mut().ok_or(Error::Encoder("ffmpeg closed".into()))?;
        stdin.write_all(&frame.data).map_err(|e| Error::Encoder(format!("ffmpeg write: {e}")))?;
        self.frames += 1;
        let mut latest: Option<Vec<u8>> = None;
        while let Ok(au) = self.rx.try_recv() {
            latest = Some(match latest {
                Some(mut prev) if !h264::is_keyframe(&au) => {
                    prev.extend_from_slice(&au);
                    prev
                }
                _ => au,
            });
        }
        Ok(latest.map(|data| EncodedVideo { codec: Codec::H264, keyframe: h264::is_keyframe(&data), data, pts_us: frame.pts_us }))
    }

    fn set_bitrate(&mut self, bps: u32) -> Result<()> {
        self.config.bitrate_bps = bps;
        Ok(())
    }
}

pub const DECODE_W: u32 = 640;
pub const DECODE_H: u32 = 360;

pub struct FfmpegDecoder {
    child: Child,
    stdin: Option<ChildStdin>,
    pub latest: std::sync::Arc<std::sync::Mutex<Option<VideoFrame>>>,
}

impl FfmpegDecoder {
    pub fn spawn() -> Result<Self> {
        Self::spawn_sized(DECODE_W, DECODE_H)
    }

    pub fn spawn_sized(w: u32, h: u32) -> Result<Self> {
        let mut child = ffmpeg_command()
            .args(["-hide_banner", "-loglevel", "error", "-probesize", "32", "-analyzeduration", "0", "-fflags", "nobuffer", "-flags", "low_delay"])
            .args(["-f", "h264", "-i", "-", "-vf"])
            .arg(format!("scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2"))
            .args(["-f", "rawvideo", "-pix_fmt", "bgra", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Error::Encoder(format!("ffmpeg decoder: {e}")))?;
        let stdin = child.stdin.take();
        let mut stdout = child.stdout.take().ok_or(Error::Encoder("ffmpeg decoder stdout".into()))?;
        let latest: std::sync::Arc<std::sync::Mutex<Option<VideoFrame>>> = Default::default();
        let slot = latest.clone();
        std::thread::Builder::new()
            .name("phoenix-h264-in".into())
            .spawn(move || {
                let len = (w * h * 4) as usize;
                let mut n = 0u64;
                loop {
                    let mut buf = vec![0u8; len];
                    if stdout.read_exact(&mut buf).is_err() {
                        break;
                    }
                    n += 1;
                    if let Ok(mut s) = slot.lock() {
                        *s = Some(VideoFrame { width: w, height: h, pts_us: n, data: std::sync::Arc::from(buf) });
                    }
                }
            })
            .map_err(Error::Io)?;
        Ok(Self { child, stdin, latest })
    }

    pub fn push(&mut self, au: &[u8]) -> Result<()> {
        let stdin = self.stdin.as_mut().ok_or(Error::Encoder("ffmpeg decoder closed".into()))?;
        stdin.write_all(au).map_err(|e| Error::Encoder(format!("ffmpeg decoder write: {e}")))
    }
}

impl Drop for FfmpegDecoder {
    fn drop(&mut self) {
        self.stdin.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_on_access_unit_delimiters() {
        let mut buf = vec![0, 0, 0, 1, 9, 0x10, 0, 0, 0, 1, 0x65, 1, 2, 0, 0, 0, 1, 9, 0x10, 0, 0, 0, 1, 0x41, 3];
        let mut out = Vec::new();
        split_access_units(&mut buf, &mut out);
        assert_eq!(out.len(), 1);
        assert!(h264::is_keyframe(&out[0]));
        assert_eq!(buf[..5], [0, 0, 0, 1, 9]);
    }

    #[test]
    #[ignore]
    fn encodes_a_test_pattern() {
        if !available() {
            return;
        }
        let cfg = VideoEncoderConfig::new(Codec::H264, 320, 180, 30, 500_000, false);
        let mut enc = FfmpegH264::spawn(&cfg).unwrap();
        let mut src = super::super::testsrc::TestPattern::new(320, 180, 30);
        let mut got = 0;
        for _ in 0..60 {
            let f = src.frame();
            if enc.encode(&f, false).unwrap().is_some() {
                got += 1;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(got > 0);
    }
}
