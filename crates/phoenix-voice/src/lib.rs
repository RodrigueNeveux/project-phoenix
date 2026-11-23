//! `phoenix-voice`: voice/video transport for Discord Phoenix.
//!
//! Clean-room implementation from public documentation (Discord developer docs
//! "Voice Connections", docs.discord.food, and the public DAVE protocol spec).
//! This crate is intentionally independent of `phoenix-api`: callers pass the
//! endpoint, token, session id, user id and server id as plain values.
//!
//! Modules:
//! - [`gateway`]: voice gateway v8 (JSON + DAVE binary opcodes, heartbeat with `seq_ack`, resume).
//! - [`udp`]: IP discovery and the UDP media socket.
//! - [`rtp`]: RTP/RTCP, codec packetizers (Opus, H.264, VP8, AV1), jitter buffer.
//! - [`crypto`]: transport encryption (`aead_aes256_gcm_rtpsize`, `aead_xchacha20_poly1305_rtpsize`).
//! - [`codec`]: codec policy (never send H.265; Apple-safe H.264 profiles).
//! - [`session`]: per-stream encoder sessions (camera + screen) with fallback.
//! - [`dave`]: DAVE end-to-end encryption frame transform and MLS interface.
//! - [`media`]: encoder / capture traits and implementations.

pub mod audio;
pub mod codec;
pub mod connection;
pub mod crypto;
pub mod dave;
pub mod error;
pub mod gateway;
#[cfg(target_os = "linux")]
pub mod hotkey;
pub mod leb128;
pub mod media;
pub mod rtp;
pub mod session;
pub mod udp;

pub use error::{Error, Result};

/// Samples per channel in one 20 ms Opus frame at 48 kHz.
pub const FRAME_SAMPLES: usize = 960;
