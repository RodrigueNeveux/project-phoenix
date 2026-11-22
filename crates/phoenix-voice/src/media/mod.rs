//! Media encoder / capture abstractions.

pub mod mock;
#[cfg(feature = "opus")]
pub mod opus;
pub mod video;

use crate::codec::{Codec, VideoEncoderConfig};
use crate::Result;

/// Raw video frame (I420 planar, 8-bit).
#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    /// Presentation time in microseconds.
    pub pts_us: u64,
    pub data: std::sync::Arc<[u8]>,
}

/// One encoded video access unit.
#[derive(Debug, Clone)]
pub struct EncodedVideo {
    pub codec: Codec,
    pub keyframe: bool,
    /// H.264: Annex-B. AV1: low-overhead OBU stream. VP8: raw frame.
    pub data: Vec<u8>,
    pub pts_us: u64,
}

/// A video encoder session. Implementations must never panic; any failure
/// is reported as `Err` so the session can fall back to another codec.
pub trait VideoEncoder: Send {
    fn codec(&self) -> Codec;
    fn encode(&mut self, frame: &VideoFrame, force_keyframe: bool) -> Result<Option<EncodedVideo>>;
    fn set_bitrate(&mut self, bps: u32) -> Result<()>;
}

/// Creates encoders for a codec. Returning `Err` means "unavailable here".
pub trait VideoEncoderFactory: Send + Sync {
    fn create(&self, config: &VideoEncoderConfig) -> Result<Box<dyn VideoEncoder>>;
}

/// Audio encoder (48 kHz, 20 ms frames).
pub trait AudioEncoder: Send {
    /// `pcm` is interleaved i16 at 48 kHz, `channels` channels, 960 samples per channel.
    fn encode(&mut self, pcm: &[i16]) -> Result<Vec<u8>>;
    fn set_bitrate(&mut self, bps: u32) -> Result<()>;
}

/// Audio decoder producing interleaved i16 PCM at 48 kHz.
pub trait AudioDecoder: Send {
    fn decode(&mut self, packet: Option<&[u8]>) -> Result<Vec<i16>>;
}

/// Which kind of source a capture provides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceKind {
    Camera,
    Screen,
    Microphone,
}

/// A capture source (camera, screen via PipeWire portal, microphone).
pub trait CaptureSource: Send {
    fn kind(&self) -> SourceKind;
    /// Pull the next frame, `Ok(None)` when no frame is ready.
    fn next_video_frame(&mut self) -> Result<Option<VideoFrame>> {
        Ok(None)
    }
    /// Pull 20 ms of PCM.
    fn next_audio_frame(&mut self) -> Result<Option<Vec<i16>>> {
        Ok(None)
    }
    fn stop(&mut self) -> Result<()>;
}
