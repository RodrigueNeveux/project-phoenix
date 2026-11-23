//! Voice calls in the app.
//!
//! - UI side (this file): [`VoiceUi`] state kept by `PhoenixApp`, updated
//!   from [`VoiceUiEvent`]s (delivered as `AppEvent::Voice`), and the
//!   [`VoiceControl`] commands the UI sends through `Backend::voice_control`.
//! - Network side ([`call`]): joins through gateway op 4, waits for our
//!   VOICE_STATE_UPDATE (session id) and VOICE_SERVER_UPDATE (endpoint,
//!   token), then runs a `phoenix_voice` connection with the cpal audio engine.

pub mod call;

use std::collections::HashSet;

use crate::model::Id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VoiceStatus {
    #[default]
    Idle,
    /// Waiting for the voice server / handshake.
    Connecting,
    Connected,
    Reconnecting,
    /// Connection lost; the message explains why.
    Failed,
}

/// Events from the voice runtime for the UI.
#[derive(Debug, Clone, PartialEq)]
pub enum VoiceUiEvent {
    Status(VoiceStatus),
    Ping(u32),
    Speaking { user: Id, speaking: bool },
    /// DAVE end-to-end encryption on/off.
    Encryption(bool),
    Error(String),
}

/// Commands from the UI.
#[derive(Debug, Clone, PartialEq)]
pub enum VoiceControl {
    SetMute(bool),
    SetDeaf(bool),
    /// `None` = system default device.
    SetInputDevice(Option<String>),
    SetOutputDevice(Option<String>),
    /// `None` = automatic input sensitivity, else threshold in dBFS.
    SetSensitivity(Option<f32>),
    SetInputVolume(f32),
    SetOutputVolume(f32),
    SetUserVolume(Id, f32),
}

/// Voice state shown by the UI.
#[derive(Debug, Clone, Default)]
pub struct VoiceUi {
    pub status: VoiceStatus,
    pub ping_ms: Option<u32>,
    pub speaking: HashSet<Id>,
    pub e2ee: bool,
    pub last_error: Option<String>,
}

impl VoiceUi {
    pub fn apply(&mut self, ev: VoiceUiEvent) {
        match ev {
            VoiceUiEvent::Status(s) => {
                self.status = s;
                if matches!(s, VoiceStatus::Idle | VoiceStatus::Failed) {
                    self.speaking.clear();
                    self.ping_ms = None;
                    self.e2ee = false;
                }
                if s == VoiceStatus::Connected {
                    self.last_error = None;
                }
            }
            VoiceUiEvent::Ping(p) => self.ping_ms = Some(p),
            VoiceUiEvent::Speaking { user, speaking } => {
                if speaking {
                    self.speaking.insert(user);
                } else {
                    self.speaking.remove(&user);
                }
            }
            VoiceUiEvent::Encryption(on) => self.e2ee = on,
            VoiceUiEvent::Error(e) => self.last_error = Some(e),
        }
    }

    pub fn is_speaking(&self, user: Id) -> bool {
        self.speaking.contains(&user)
    }

    /// Discord's connection quality colour buckets.
    pub fn quality(&self) -> Quality {
        match self.ping_ms {
            None => Quality::Unknown,
            Some(p) if p < 150 => Quality::Good,
            Some(p) if p < 400 => Quality::Fair,
            Some(_) => Quality::Bad,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    Unknown,
    Good,
    Fair,
    Bad,
}

/// `PhoenixApp::join_voice` hook: show "RTC Connecting" right away.
pub fn on_join_clicked(app: &mut crate::app::PhoenixApp, channel: Option<Id>) {
    app.voice.apply(VoiceUiEvent::Status(if channel.is_some() { VoiceStatus::Connecting } else { VoiceStatus::Idle }));
}

/// Push the user panel's mute/deafen toggles to the voice runtime.
pub fn sync_self_state(app: &crate::app::PhoenixApp) {
    // The real backend applies controls synchronously; the future is a no-op.
    drop(app.backend.voice_control(VoiceControl::SetMute(app.mic_muted || app.deafened)));
    drop(app.backend.voice_control(VoiceControl::SetDeaf(app.deafened)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_state_follows_events() {
        let mut v = VoiceUi::default();
        v.apply(VoiceUiEvent::Status(VoiceStatus::Connected));
        v.apply(VoiceUiEvent::Ping(42));
        v.apply(VoiceUiEvent::Speaking { user: Id(5), speaking: true });
        assert!(v.is_speaking(Id(5)));
        assert_eq!(v.quality(), Quality::Good);
        v.apply(VoiceUiEvent::Speaking { user: Id(5), speaking: false });
        assert!(!v.is_speaking(Id(5)));
        v.apply(VoiceUiEvent::Speaking { user: Id(6), speaking: true });
        v.apply(VoiceUiEvent::Status(VoiceStatus::Idle));
        assert!(v.speaking.is_empty() && v.ping_ms.is_none());
    }
}
