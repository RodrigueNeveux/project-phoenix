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
            .or_else(|| s("staticFrameSrc").map(str::to_owned))
            .or_else(|| frame_layer(item))?,
    };
    Some(url.into())
}

/// Profile frames carry `layers[]`; the client loads each one from
/// `media/v1/collectibles-shop/{sku}/{layer_id}/static`
/// (`getCollectiblesItemAssetUrl`). The tile shows the first front staple.
fn frame_layer(item: &Value) -> Option<String> {
    let sku = item.get("sku_id")?.as_str()?;
    let layers = item.get("layers")?.as_array()?;
    let is = |l: &&Value, k: &str, v: &str| l.get(k).and_then(Value::as_str) == Some(v);
    let l = layers.iter().find(|l| is(l, "type", "staple") && is(l, "order", "front")).or(layers.first())?;
    Some(format!("https://cdn.discordapp.com/media/v1/collectibles-shop/{sku}/{}/static", l.get("id")?.as_str()?))
}

/// Picker sections fed by the shop catalogue
/// (`GET /collectibles-categories/v2`, see `phoenix_api::rest::collectibles`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShopCatalog {
    /// "Nitro exclusive": products with `premium_type` 2.
    pub nitro: Vec<Collectible>,
    /// "Check out the Shop": every other single-item product.
    pub shop: Vec<Collectible>,
}

pub fn parse_shop(v: &Value) -> ShopCatalog {
    let mut out = ShopCatalog::default();
    let mut seen = std::collections::HashSet::new();
    for p in phoenix_api::rest::collectibles::products(v) {
        // Bundles (1000) and variant groups (2000) are not picker tiles.
        let Some(kind) = p.get("type").and_then(Value::as_u64).and_then(Kind::from_type) else { continue };
        let Some(it) = p.get("items").and_then(Value::as_array).and_then(|i| i.first()) else { continue };
        let Some(sku) = it.get("sku_id").and_then(Value::as_str).and_then(|s| s.parse().ok()) else { continue };
        if !seen.insert(sku) {
            continue;
        }
        let c = Collectible {
            sku,
            kind,
            name: p.get("name").and_then(Value::as_str).unwrap_or_default().to_owned().into(),
            preview: preview(kind, it),
        };
        if p.get("premium_type").and_then(Value::as_u64) == Some(phoenix_api::rest::collectibles::PREMIUM_TYPE_NITRO) {
            out.nitro.push(c);
        } else {
            out.shop.push(c);
        }
    }
    out
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

    #[test]
    fn splits_shop_sections() {
        let v = json!({"categories": [{"products": [
            {"type": 0, "premium_type": 2, "name": "N", "items": [{"type": 0, "sku_id": "1", "asset": "a"}]},
            {"type": 1, "premium_type": 0, "name": "E", "items": [{"type": 1, "sku_id": "2", "thumbnailPreviewSrc": "https://t"}]},
            {"type": 3, "premium_type": 0, "name": "F", "items": [{"type": 3, "sku_id": "3", "layers": [
                {"id": "9", "type": "border", "order": "front"}, {"id": "8", "type": "staple", "order": "front"}]}]},
            {"type": 1000, "premium_type": 0, "name": "B", "items": [{"type": 0, "sku_id": "4"}]},
            {"type": 1, "premium_type": 0, "name": "dup", "items": [{"type": 1, "sku_id": "2"}]}
        ]}]});
        let s = parse_shop(&v);
        assert_eq!(s.nitro.len(), 1);
        assert_eq!(s.nitro[0].kind, Kind::Decoration);
        assert_eq!(s.shop.len(), 2);
        assert_eq!(s.shop[1].preview.as_deref(), Some("https://cdn.discordapp.com/media/v1/collectibles-shop/3/8/static"));
    }
}
