use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::ffmpeg::FfmpegH264;
use super::testsrc::TestPattern;
use super::{EncodedVideo, VideoEncoder, VideoFrame};
use crate::codec::{Codec, VideoEncoderConfig};
use crate::{Error, Result};

pub const TEST_PATTERN: &str = "phoenix-test-pattern";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    TestPattern,
    Device(String),
}

impl Input {
    pub fn from_name(name: Option<&str>) -> Self {
        match name {
            Some(TEST_PATTERN) => Self::TestPattern,
            Some(d) if d.starts_with("/dev/video") => Self::Device(d.to_owned()),
            _ => Self::Device("/dev/video0".into()),
        }
    }
}

pub type Preview = Arc<Mutex<Option<VideoFrame>>>;

pub struct Camera {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    pub preview: Preview,
}

struct DeviceReader {
    child: Child,
    stdout: std::process::ChildStdout,
}

impl DeviceReader {
    fn open(dev: &str, width: u32, height: u32, fps: u32) -> Result<Self> {
        let mut child = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-f", "v4l2", "-framerate", &fps.to_string(), "-i", dev])
            .args(["-vf", &format!("scale={width}:{height}:force_original_aspect_ratio=increase,crop={width}:{height}")])
            .args(["-f", "rawvideo", "-pix_fmt", "rgba", "-"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Error::Encoder(format!("ffmpeg v4l2: {e}")))?;
        let stdout = child.stdout.take().ok_or(Error::Encoder("ffmpeg v4l2 stdout".into()))?;
        Ok(Self { child, stdout })
    }

    fn frame(&mut self, len: usize) -> Option<Vec<u8>> {
        let mut buf = vec![0u8; len];
        self.stdout.read_exact(&mut buf).ok()?;
        Some(buf)
    }
}

impl Drop for DeviceReader {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Camera {
    pub fn start(
        input: Input,
        width: u32,
        height: u32,
        fps: u32,
        mut on_frame: impl FnMut(EncodedVideo) + Send + 'static,
    ) -> Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let preview: Preview = Arc::new(Mutex::new(None));
        let cfg = VideoEncoderConfig::new(Codec::H264, width, height, fps, 1_500_000, false);
        let mut enc = FfmpegH264::spawn(&cfg)?;
        let (s2, p2) = (stop.clone(), preview.clone());
        let thread = std::thread::Builder::new()
            .name("phoenix-camera".into())
            .spawn(move || {
                let mut pattern = TestPattern::new(width, height, fps);
                let mut device = match &input {
                    Input::Device(d) => DeviceReader::open(d, width, height, fps).ok(),
                    Input::TestPattern => None,
                };
                let started = Instant::now();
                let period = Duration::from_micros(1_000_000 / u64::from(fps.max(1)));
                let mut next = Instant::now();
                let mut first = true;
                while !s2.load(Ordering::Relaxed) {
                    let frame = match (&input, device.as_mut()) {
                        (Input::Device(_), Some(d)) => match d.frame((width * height * 4) as usize) {
                            Some(data) => VideoFrame { width, height, pts_us: started.elapsed().as_micros() as u64, data: Arc::from(data) },
                            None => break,
                        },
                        (Input::Device(_), None) => break,
                        (Input::TestPattern, _) => pattern.frame(),
                    };
                    if let Ok(mut p) = p2.lock() {
                        *p = Some(frame.clone());
                    }
                    match enc.encode(&frame, first) {
                        Ok(Some(e)) => on_frame(e),
                        Ok(None) => {}
                        Err(e) => {
                            tracing::warn!(error = %e, "camera: encoder failed");
                            break;
                        }
                    }
                    first = false;
                    if matches!(input, Input::TestPattern) {
                        next += period;
                        let now = Instant::now();
                        if next > now {
                            std::thread::sleep(next - now);
                        } else {
                            next = now;
                        }
                    }
                }
            })
            .map_err(Error::Io)?;
        Ok(Self { stop, thread: Some(thread), preview })
    }

    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for Camera {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_names() {
        assert_eq!(Input::from_name(Some(TEST_PATTERN)), Input::TestPattern);
        assert_eq!(Input::from_name(Some("/dev/video2")), Input::Device("/dev/video2".into()));
        assert_eq!(Input::from_name(None), Input::Device("/dev/video0".into()));
    }
}
