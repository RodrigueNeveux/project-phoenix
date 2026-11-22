//! MLS group interface for DAVE.
//!
//! The voice gateway drives this trait from DAVE opcodes:
//! - 25 `dave_mls_external_sender_package` -> [`MlsSession::set_external_sender`]
//! - 26 `dave_mls_key_package` (client sends) <- [`MlsSession::key_package`]
//! - 27 `dave_mls_proposals` -> [`MlsSession::process_proposals`], may yield
//! - 28 `dave_mls_commit_welcome` (client sends)
//! - 29 `dave_mls_announce_commit_transition` -> [`MlsSession::process_commit`]
//! - 30 `dave_mls_welcome` -> [`MlsSession::process_welcome`]
//! - 31 `dave_mls_invalid_commit_welcome` (client sends) on failure, then reset.
//!
//! DAVE v1 uses MLS ciphersuite 2 (`DHKEMP256_AES128GCM_SHA256_P256`), basic
//! credentials whose identity is the big-endian 64-bit user id, and the
//! voice server as the single external sender.
//!
//! TODO(openmls): provide an `OpenMlsSession` implementing this trait with the
//! `openmls` crate (ciphersuite 2, external sender extension, proposal
//! filtering against `recognized_user_ids`, exporter for sender keys). It is
//! not included yet: it pulls a large dependency tree and needs interop
//! testing against the real voice servers before shipping. Until then callers
//! must announce `max_dave_protocol_version = 0` (transport encryption only).

use crate::Result;

/// Output of handling proposals: the commit (and welcome, if members were
/// added) that the client must send as opcode 28.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitWelcome {
    pub commit: Vec<u8>,
    pub welcome: Option<Vec<u8>>,
}

/// MLS group state for one DAVE session.
pub trait MlsSession: Send {
    /// DAVE protocol version this session implements.
    fn protocol_version(&self) -> u16;
    /// Reset the group (new epoch 0 / downgrade / invalid commit).
    fn reset(&mut self);
    /// Store the voice server's external sender (opcode 25 payload).
    fn set_external_sender(&mut self, package: &[u8]) -> Result<()>;
    /// Serialize a fresh MLS KeyPackage (opcode 26 payload).
    fn key_package(&mut self) -> Result<Vec<u8>>;
    /// Handle proposals (opcode 27 payload). Only proposals adding/removing
    /// `recognized_user_ids` may be accepted.
    fn process_proposals(&mut self, payload: &[u8], recognized_user_ids: &[u64]) -> Result<Option<CommitWelcome>>;
    /// Apply an announced commit (opcode 29) for `transition_id`.
    fn process_commit(&mut self, transition_id: u16, commit: &[u8]) -> Result<()>;
    /// Join from a Welcome (opcode 30).
    fn process_welcome(&mut self, transition_id: u16, welcome: &[u8], recognized_user_ids: &[u64]) -> Result<()>;
    /// MLS exporter output for `user_id` (16 bytes), seeds the sender's ratchet.
    fn export_sender_secret(&self, user_id: u64) -> Result<[u8; 16]>;
    /// Epoch authenticator, used for the user-visible privacy/verification code.
    fn epoch_authenticator(&self) -> Result<Vec<u8>>;
}

/// Build the ratchet for a sender from the current MLS epoch.
pub fn sender_ratchet(mls: &dyn MlsSession, user_id: u64) -> Result<super::ratchet::HashRatchet> {
    Ok(super::ratchet::HashRatchet::new(&mls.export_sender_secret(user_id)?))
}
