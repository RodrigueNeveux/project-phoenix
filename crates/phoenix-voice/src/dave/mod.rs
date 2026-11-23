//! DAVE (Discord Audio & Video End-to-End Encryption), per the public
//! protocol description at github.com/discord/dave-protocol.
//!
//! - [`frame`]: the media frame transform (AES-128-GCM, 8-byte tag, truncated
//!   32-bit nonce, ULEB128 unencrypted ranges, supplemental size, magic `0xFAFA`).
//! - [`ratchet`]: per-sender key ratchet (MLS `ExpandWithLabel` hash ratchet).
//! - [`mls`]: the MLS group interface the gateway drives (opcodes 21-31).

pub mod frame;
pub mod mls;
pub mod openmls_session;
pub mod ratchet;
pub mod session;

/// Highest DAVE protocol version this client implements the frame format for.
pub const MAX_PROTOCOL_VERSION: u16 = 1;

/// MLS exporter label used to derive per-sender base secrets.
pub const EXPORTER_LABEL: &str = "Discord Secure Frames v0";

/// DAVE displayable code: `len` digits built from groups of `group` bytes
/// (big-endian, modulo 10^group, zero padded), as in the public DAVE
/// protocol. The voice privacy code is `displayable_code(epoch_authenticator, 30, 5)`.
pub fn displayable_code(data: &[u8], len: usize, group: usize) -> Option<String> {
    if group == 0 || group > 8 || !len.is_multiple_of(group) || data.len() < len {
        return None;
    }
    let modulus = 10u64.pow(group as u32);
    let mut out = String::with_capacity(len);
    for chunk in data[..len].chunks(group) {
        let v = chunk.iter().fold(0u64, |acc, b| (acc << 8) | *b as u64) % modulus;
        out.push_str(&format!("{v:0width$}", width = group));
    }
    Some(out)
}

/// Voice privacy code of an MLS epoch authenticator, grouped for display
/// ("12345 67890 ...").
pub fn privacy_code(epoch_authenticator: &[u8]) -> Option<String> {
    let code = displayable_code(epoch_authenticator, 30, 5)?;
    Some(code.as_bytes().chunks(5).map(|c| String::from_utf8_lossy(c).into_owned()).collect::<Vec<_>>().join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displayable_code_groups() {
        let data: Vec<u8> = (0..32).collect();
        // 00 01 02 03 04 = 0x0001020304 = 16909060 -> % 100000 = 09060
        let c = displayable_code(&data, 30, 5).unwrap();
        assert_eq!(c.len(), 30);
        assert_eq!(&c[..5], "09060");
        assert!(c.bytes().all(|b| b.is_ascii_digit()));
        assert_eq!(privacy_code(&data).unwrap().split(' ').count(), 6);
        assert!(displayable_code(&data[..10], 30, 5).is_none());
    }
}
