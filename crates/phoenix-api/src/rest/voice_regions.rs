use reqwest::Method;

use super::RestClient;
use crate::error::Result;

#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
pub struct VoiceRegion {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub optimal: bool,
    #[serde(default)]
    pub deprecated: bool,
    #[serde(default)]
    pub custom: bool,
}

pub fn sorted(mut regions: Vec<VoiceRegion>) -> Vec<VoiceRegion> {
    regions.sort_by(|a, b| a.name.cmp(&b.name));
    regions
}

impl RestClient {
    pub async fn voice_regions(&self) -> Result<Vec<VoiceRegion>> {
        let regions: Vec<VoiceRegion> = self.call(Method::GET, "/voice/regions", super::Body::None).await?;
        Ok(sorted(regions))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_parse_and_sort_by_name_like_the_region_store() {
        let raw = r#"[{"id":"rotterdam","name":"Rotterdam","optimal":true,"deprecated":false,"custom":false},{"id":"atlanta","name":"Atlanta"}]"#;
        let regions = sorted(serde_json::from_str::<Vec<VoiceRegion>>(raw).unwrap());
        assert_eq!(regions.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["atlanta", "rotterdam"]);
        assert!(regions[1].optimal);
    }
}
