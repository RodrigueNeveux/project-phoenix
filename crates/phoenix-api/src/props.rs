//! Client "super properties": the identity the desktop client advertises in
//! the `X-Super-Properties` header and in gateway Identify.

use base64::Engine;
use serde::Serialize;

/// Electron version of the impersonated desktop client.
pub const DEFAULT_ELECTRON_VERSION: &str = "44.10.2";
/// Desktop client (host) version.
pub const DEFAULT_CLIENT_VERSION: &str = "1.0.170";
/// Web client build number. Discord bumps this constantly; callers should
/// refresh it (e.g. scraped at startup). A stale value still logs in, but the
/// API hides newer content from old builds: with 460000 the shop catalogue
/// (`/collectibles-categories/v2`) returns no profile frames at all, with the
/// official client's current build it returns all 41. Values below match the
/// stable Linux desktop client of 2026-11-23 (`build_info.json` 1.0.170,
/// `build_number` in its web bundle, Chrome/Electron from its user agent).
pub const DEFAULT_BUILD_NUMBER: u64 = 679_055;
/// Chromium version bundled with [`DEFAULT_ELECTRON_VERSION`].
pub const DEFAULT_CHROME_VERSION: &str = "152.0.7977.169";

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
    /// Built from the compiled-in constants only.
    pub fn linux_desktop() -> Self {
        Self::from_build_info(&crate::build_info::BuildInfo::default())
    }

    /// Properties using the versions of the locally installed Discord when
    /// found (see [`crate::build_info::BuildInfo::detect_fast`]), else the
    /// constants. Cheap: reads a few small files, never scans or networks.
    pub fn current() -> Self {
        Self::from_build_info(&crate::build_info::BuildInfo::detect_fast())
    }

    pub fn from_build_info(b: &crate::build_info::BuildInfo) -> Self {
        let electron = b.electron_version.as_str();
        let client = b.client_version.as_str();
        let chrome = b.chrome_version.as_str();
        let ua = format!(
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) \
             discord/{client} Chrome/{chrome} Electron/{electron} Safari/537.36"
        );
        Self {
            os: "Linux".into(),
            browser: "Discord Client".into(),
            release_channel: b.release_channel.clone(),
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
            client_build_number: b.build_number,
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
