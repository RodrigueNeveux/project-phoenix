//! DAVE (Discord Audio & Video End-to-End Encryption), per the public
//! protocol description at github.com/discord/dave-protocol.
//!
//! - [`frame`]: the media frame transform (AES-128-GCM, 8-byte tag, truncated
//!   32-bit nonce, ULEB128 unencrypted ranges, supplemental size, magic `0xFAFA`).
//! - [`ratchet`]: per-sender key ratchet (MLS `ExpandWithLabel` hash ratchet).
//! - [`mls`]: the MLS group interface the gateway drives (opcodes 21-31).

pub mod frame;
pub mod mls;
pub mod ratchet;

/// Highest DAVE protocol version this client implements the frame format for.
pub const MAX_PROTOCOL_VERSION: u16 = 1;

/// MLS exporter label used to derive per-sender base secrets.
pub const EXPORTER_LABEL: &str = "Discord Secure Frames v0";
