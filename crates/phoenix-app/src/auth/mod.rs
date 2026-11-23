//! Login flow state beyond email/password: MFA method choice (TOTP, SMS,
//! backup code) and the QR code (remote auth) panel.

use gpui::{Context, SharedString};

use crate::{app::PhoenixApp, backend::ReadyData};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MfaOptions {
    pub totp: bool,
    pub sms: bool,
    pub backup: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MfaMethod {
    #[default]
    Totp,
    Sms,
    Backup,
}

impl MfaMethod {
    pub fn label(self) -> &'static str {
        match self {
            Self::Totp => "Authenticator app",
            Self::Sms => "SMS",
            Self::Backup => "Backup code",
        }
    }
}

/// QR login progress pushed by the backend.
#[derive(Clone, Debug, PartialEq)]
pub enum RemoteAuthUpdate {
    Qr(String),
    Scanned { username: String },
    /// Session ended (timeout / phone cancelled) or failed with a message.
    Ended(Option<String>),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum QrState {
    /// Not started yet (or restart requested).
    #[default]
    Idle,
    Connecting,
    Code {
        url: String,
        /// Module matrix, row-major, `size * size`.
        modules: Vec<bool>,
        size: usize,
    },
    Scanned(String),
    Unavailable(String),
}

#[derive(Default)]
pub struct LoginUi {
    pub mfa_method: MfaMethod,
    pub sms_phone: Option<SharedString>,
    pub qr: QrState,
}

/// Encode `url` as a QR module matrix.
pub fn qr_matrix(url: &str) -> Option<(Vec<bool>, usize)> {
    let code = qrcode::QrCode::with_error_correction_level(url, qrcode::EcLevel::L).ok()?;
    let size = code.width();
    let modules = code.to_colors().into_iter().map(|c| c == qrcode::Color::Dark).collect();
    Some((modules, size))
}

/// Start the QR session if the login screen shows and none is running.
pub fn ensure_qr(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) {
    if app.screen != crate::app::Screen::Login || app.logging_in || app.login_ui.qr != QrState::Idle {
        return;
    }
    let Some(fut) = app.backend.remote_auth() else {
        app.login_ui.qr = QrState::Unavailable("QR login isn't available offline.".into());
        return;
    };
    app.login_ui.qr = QrState::Connecting;
    cx.spawn(async move |this, cx| {
        let res = fut.await;
        let _ = this.update(cx, |this, cx| finish(this, res, cx));
    })
    .detach();
}

fn finish(app: &mut PhoenixApp, res: Result<ReadyData, crate::backend::BackendError>, cx: &mut Context<PhoenixApp>) {
    match res {
        Ok(ready) => {
            app.login_ui.qr = QrState::Idle;
            app.login_error = None;
            app.apply_ready(ready, cx);
        }
        Err(e) if e.0.is_empty() => {
            // Cancelled/expired: a fresh code is generated on the next frame.
            if !matches!(app.login_ui.qr, QrState::Unavailable(_)) {
                app.login_ui.qr = QrState::Idle;
            }
        }
        Err(e) => app.login_ui.qr = QrState::Unavailable(e.0),
    }
    cx.notify();
}

pub fn on_remote_auth(app: &mut PhoenixApp, u: RemoteAuthUpdate, _cx: &mut Context<PhoenixApp>) {
    app.login_ui.qr = match u {
        RemoteAuthUpdate::Qr(url) => match qr_matrix(&url) {
            Some((modules, size)) => QrState::Code { url, modules, size },
            None => QrState::Unavailable("Could not draw the QR code.".into()),
        },
        RemoteAuthUpdate::Scanned { username } => QrState::Scanned(username),
        RemoteAuthUpdate::Ended(Some(msg)) => QrState::Unavailable(msg),
        RemoteAuthUpdate::Ended(None) => return,
    };
}

/// Retry after the QR panel showed an error.
pub fn retry_qr(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) {
    app.backend.cancel_remote_auth();
    app.login_ui.qr = QrState::Idle;
    cx.notify();
}

/// Second step of a password login with the chosen method.
pub fn submit_mfa(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) {
    let code = app.login_mfa.read(cx).value().to_string();
    let fut = app.backend.login_mfa(app.login_ui.mfa_method, code);
    app.logging_in = true;
    app.login_error = None;
    cx.notify();
    cx.spawn(async move |this, cx| {
        let res = fut.await;
        let _ = this.update(cx, |this, cx| {
            this.logging_in = false;
            match res {
                Ok(ready) => {
                    this.login_mfa_step = false;
                    this.login_ui = LoginUi::default();
                    this.backend.cancel_remote_auth();
                    this.apply_ready(ready, cx);
                }
                Err(e) => {
                    this.login_mfa_step = this.backend.needs_mfa();
                    this.login_error = Some(e.0.into());
                }
            }
            cx.notify();
        });
    })
    .detach();
}

pub fn choose_mfa(app: &mut PhoenixApp, m: MfaMethod, cx: &mut Context<PhoenixApp>) {
    app.login_ui.mfa_method = m;
    app.login_error = None;
    if m == MfaMethod::Sms && app.login_ui.sms_phone.is_none() {
        let fut = app.backend.send_mfa_sms();
        cx.spawn(async move |this, cx| {
            let res = fut.await;
            let _ = this.update(cx, |this, cx| {
                match res {
                    Ok(phone) => this.login_ui.sms_phone = Some(phone.into()),
                    Err(e) => this.login_error = Some(e.0.into()),
                }
                cx.notify();
            });
        })
        .detach();
    }
    cx.notify();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qr_matrix_is_square_with_finders() {
        let (m, n) = qr_matrix("https://discord.com/ra/abcdefghijklmnopqrstuvwxyz0123456789ABCDEFG").unwrap();
        assert_eq!(m.len(), n * n);
        assert!(n >= 21);
        // Top-left finder pattern: dark corner, light ring at (1,1).
        assert!(m[0] && !m[n + 1] && m[2 * n + 2]);
    }
}
