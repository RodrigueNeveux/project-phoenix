//! Client "super properties": the identity the desktop client advertises in
//! the `X-Super-Properties` header and in gateway Identify.

use base64::Engine;
use serde::Serialize;

/// Electron version of the impersonated desktop client.
pub const DEFAULT_ELECTRON_VERSION: &str = "37.6.0";
/// Desktop client (host) version.
pub const DEFAULT_CLIENT_VERSION: &str = "0.0.112";
/// Web client build number. Discord bumps this constantly; callers should
/// refresh it (e.g. scraped at startup) — an old value still works but looks stale.
pub const DEFAULT_BUILD_NUMBER: u64 = 460_000;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ClientProperties {
    pub os: String,
    pub browser: String,
    pub release_channel: String,
    pub client_version: String,
    pub os_version: String,
    pub os_arch: String,
    pub app_arch: String,
    pub system_locale: String,
    pub has_client_mods: bool,
    pub browser_user_agent: String,
    pub browser_version: String,
    pub window_manager: String,
    pub distro: String,
    pub runtime_environment: String,
    pub client_build_number: u64,
    pub native_build_number: Option<u64>,
    pub client_event_source: Option<String>,
    pub client_launch_id: String,
    pub client_heartbeat_session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub design_id: Option<u32>,
}

impl Default for ClientProperties {
    fn default() -> Self {
        Self::linux_desktop()
    }
}

impl ClientProperties {
    /// Properties matching the stable Linux desktop client.
    pub fn linux_desktop() -> Self {
        let electron = DEFAULT_ELECTRON_VERSION;
        let client = DEFAULT_CLIENT_VERSION;
        let ua = format!(
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) \
             discord/{client} Chrome/138.0.7204.251 Electron/{electron} Safari/537.36"
        );
        Self {
            os: "Linux".into(),
            browser: "Discord Client".into(),
            release_channel: "stable".into(),
            client_version: client.into(),
            os_version: "6.12.0".into(),
            os_arch: "x64".into(),
            app_arch: "x64".into(),
            system_locale: "en-US".into(),
            has_client_mods: false,
            browser_user_agent: ua,
            browser_version: electron.into(),
            window_manager: "KDE,unknown".into(),
            distro: "Debian GNU/Linux".into(),
            runtime_environment: "native".into(),
            client_build_number: DEFAULT_BUILD_NUMBER,
            native_build_number: None,
            client_event_source: None,
            client_launch_id: uuid::Uuid::new_v4().to_string(),
            client_heartbeat_session_id: uuid::Uuid::new_v4().to_string(),
            design_id: None,
        }
    }

    pub fn user_agent(&self) -> &str {
        &self.browser_user_agent
    }

    /// Base64(JSON) for the `X-Super-Properties` header.
    pub fn to_header(&self) -> String {
        let json = serde_json::to_vec(self).expect("properties serialize");
        base64::engine::general_purpose::STANDARD.encode(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_roundtrip() {
        let p = ClientProperties::default();
        let raw = base64::engine::general_purpose::STANDARD.decode(p.to_header()).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(v["browser"], "Discord Client");
        assert_eq!(v["os"], "Linux");
        assert!(v["client_build_number"].is_u64());
    }
}
