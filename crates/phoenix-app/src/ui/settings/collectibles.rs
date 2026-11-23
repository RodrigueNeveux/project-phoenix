//! Owned collectibles for the profile editor pickers (avatar decoration,
//! profile effect, nameplate, profile frame), from the endpoint the client
//! calls: `GET /users/@me/collectibles-purchases?variants_return_style=2`.
//! Each purchase has `items[]` typed 0 avatar decoration (`asset` →
//! `cdn/avatar-decoration-presets/{asset}.png`), 1 profile effect
//! (`thumbnailPreviewSrc` / `staticFrameSrc`), 2 nameplate (`asset` path →
//! `cdn/assets/collectibles/{asset}static.png`), 3 profile frame.

use gpui::SharedString;
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Decoration,
    Effect,
    Nameplate,
    Frame,
}

impl Kind {
    fn from_type(t: u64) -> Option<Self> {
        Some(match t {
            0 => Self::Decoration,
            1 => Self::Effect,
            2 => Self::Nameplate,
            3 => Self::Frame,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Collectible {
    pub sku: u64,
    pub kind: Kind,
    pub name: SharedString,
    pub preview: Option<SharedString>,
}

fn preview(kind: Kind, item: &Value) -> Option<SharedString> {
    let s = |k: &str| item.get(k).and_then(Value::as_str);
    let url = match kind {
        Kind::Decoration => format!("https://cdn.discordapp.com/avatar-decoration-presets/{}.png?size=96&passthrough=false", s("asset")?),
        Kind::Effect => s("thumbnailPreviewSrc").or(s("staticFrameSrc"))?.to_owned(),
        Kind::Nameplate => format!("https://cdn.discordapp.com/assets/collectibles/{}static.png", s("asset")?),
        Kind::Frame => s("asset")
            .map(|a| format!("https://cdn.discordapp.com/avatar-decoration-presets/{a}.png?size=96&passthrough=false"))
            .or_else(|| s("staticFrameSrc").map(str::to_owned))?,
    };
    Some(url.into())
}

pub fn parse_purchases(v: &Value) -> Vec<Collectible> {
    v.as_array()
        .into_iter()
        .flatten()
        .flat_map(|p| {
            let name = p.get("name").and_then(Value::as_str).unwrap_or_default().to_owned();
            p.get("items").and_then(Value::as_array).cloned().unwrap_or_default().into_iter().filter_map(move |it| {
                let kind = Kind::from_type(it.get("type")?.as_u64()?)?;
                Some(Collectible {
                    sku: it.get("sku_id")?.as_str()?.parse().ok()?,
                    kind,
                    name: name.clone().into(),
                    preview: preview(kind, &it),
                })
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_kinds_and_previews() {
        let v = json!([
            {"name": "Deco", "items": [{"type": 0, "sku_id": "1", "asset": "a_x"}]},
            {"name": "Fx", "items": [{"type": 1, "sku_id": "2", "thumbnailPreviewSrc": "https://t"}]},
            {"name": "Badge", "items": []},
            {"name": "Plate", "items": [{"type": 2, "sku_id": "3", "asset": "nameplates/x/"}]}
        ]);
        let c = parse_purchases(&v);
        assert_eq!(c.len(), 3);
        assert_eq!(c[0].kind, Kind::Decoration);
        assert!(c[0].preview.as_deref().unwrap().contains("avatar-decoration-presets/a_x.png"));
        assert_eq!(c[1].preview.as_deref(), Some("https://t"));
        assert_eq!(c[2].preview.as_deref(), Some("https://cdn.discordapp.com/assets/collectibles/nameplates/x/static.png"));
    }
}
