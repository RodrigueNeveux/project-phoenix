//! Video encoder backends behind cargo features.
//!
//! TODO(ffmpeg): `ffmpeg` feature: libavcodec (`h264_vaapi`, `h264_nvenc`,
//! `av1_vaapi`, `av1_nvenc`, `libvpx` for VP8, `libopenh264` software fallback).
//! Must force `profile=constrained_baseline|main`, `pix_fmt=yuv420p`, Annex-B
//! output, and never use `hevc_*` encoders.
//! TODO(vaapi): direct VA-API encoder via `cros-libva` for zero-copy dmabuf from PipeWire.
//! TODO(nvenc): direct NVENC (via `nvidia-video-codec-sdk`) for Pascal+ GPUs; AV1 only on Ada+.
//!
//! Until these land, the stubs below report `Unsupported`, which makes the
//! session fall through the codec preference list without panicking.

use super::{VideoEncoder, VideoEncoderFactory};
use crate::codec::{Codec, VideoEncoderConfig};
use crate::{Error, Result};

/// Probe hardware encoders. Currently returns software-only capabilities.
pub fn probe_capabilities() -> crate::codec::LocalCapabilities {
    crate::codec::LocalCapabilities {
        // TODO: detect via VA-API vaQueryConfigEntrypoints / NVENC caps.
        hw_av1_encoder: false,
        h264_encoder: cfg!(any(feature = "ffmpeg", feature = "vaapi", feature = "nvenc")),
        vp8_encoder: cfg!(feature = "ffmpeg"),
        decoders: vec![Codec::H264, Codec::Vp8, Codec::Av1],
    }
}

/// Factory dispatching to compiled-in backends.
#[derive(Default)]
pub struct SystemEncoderFactory;

impl VideoEncoderFactory for SystemEncoderFactory {
    fn create(&self, config: &VideoEncoderConfig) -> Result<Box<dyn VideoEncoder>> {
        if !config.codec.sendable() {
            return Err(Error::Unsupported(format!("{} is never used for sending", config.codec.name())));
        }
        Err(Error::Unsupported(format!(
            "no {} encoder backend compiled in (enable ffmpeg/vaapi/nvenc)",
            config.codec.name()
        )))
    }
}
