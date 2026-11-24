//! Full-profile Wishlist tab data.
//!
//! The client (`UserProfileStore.getWishlistIds`) takes the keys of the
//! profile's `wishlist_settings` and loads the first with
//! `GET /wishlists/{id}?source=user_profile` → `wishlist_items[]`.

use gpui::SharedString;
use serde_json::Value;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WishItem {
    pub name: SharedString,
    pub image: Option<SharedString>,
}

/// First wishlist id of a `GET /users/{id}/profile` payload.
pub fn first_id(profile: &Value) -> Option<String> {
    profile.get("wishlist_settings")?.as_object()?.keys().next().cloned()
}

fn str_at<'a>(v: &'a Value, paths: &[&str]) -> Option<&'a str> {
    paths.iter().find_map(|p| v.pointer(p).and_then(Value::as_str).filter(|s| !s.is_empty()))
}

/// Items of a `GET /wishlists/{id}` payload (name + preview image).
pub fn items(v: &Value) -> Vec<WishItem> {
    v.get("wishlist_items")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|it| {
            // Shape (ref-cache wishlist-shape.txt): {sku_id, sku_name, sku: {name, ...}};
            // the tile shows the collectibles-shop static preview of the SKU.
            let name = str_at(it, &["/sku_name", "/sku/name"]).unwrap_or_default();
            let sku = str_at(it, &["/sku_id", "/sku/id"])?;
            let image = Some(SharedString::from(format!("https://cdn.discordapp.com/media/v1/collectibles-shop/{sku}/static")));
            Some(WishItem { name: name.to_owned().into(), image })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn reads_first_wishlist_id_and_items() {
        assert_eq!(super::first_id(&json!({"wishlist_settings": {"42": {}}})).as_deref(), Some("42"));
        let v = json!({"wishlist_items": [{"sku_id": "7", "sku_name": "Crown"}]});
        assert_eq!(super::items(&v)[0].name.as_ref(), "Crown");
    }
}
