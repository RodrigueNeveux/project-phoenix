use reqwest::Method;

use super::RestClient;
use crate::error::Result;

#[derive(Debug, Clone, serde::Deserialize, PartialEq, Default)]
pub struct StreamPreview {
    #[serde(default)]
    pub url: Option<String>,
}

impl RestClient {
    pub async fn upload_stream_preview(&self, stream_key: &str, thumbnail_data_url: &str) -> Result<()> {
        let body = serde_json::json!({ "thumbnail": thumbnail_data_url });
        self.call_empty(Method::POST, &format!("/streams/{stream_key}/preview"), super::Body::Json(body)).await
    }

    pub async fn stream_preview(&self, stream_key: &str) -> Result<StreamPreview> {
        let version = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or_default();
        self.call(Method::GET, &format!("/streams/{stream_key}/preview?version={version}"), super::Body::None).await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn preview_url_parses() {
        let p: super::StreamPreview = serde_json::from_str(r#"{"url":"https://media.discordapp.net/x.jpeg"}"#).unwrap();
        assert_eq!(p.url.as_deref(), Some("https://media.discordapp.net/x.jpeg"));
    }
}
