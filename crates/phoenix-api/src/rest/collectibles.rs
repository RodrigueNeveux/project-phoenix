//! Read-only collectibles catalogue, as the live client fetches it when a
//! profile collectible picker opens:
//! `GET /collectibles-categories/v2?include_bundles=true&variants_return_style=2&skip_num_categories=0`
//! (`{ categories: [{ products: [{ type, premium_type, items: [...] }] }] }`)
//! and the owned items, `GET /users/@me/collectibles-purchases`. Nothing here
//! buys, claims or equips anything.

use reqwest::Method;
use serde_json::Value;

use super::{Body, RestClient};
use crate::error::Result;

/// `COLLECTIBLES_CATEGORIES_V2` with the query string the client sends.
pub const CATEGORIES_V2: &str = "/collectibles-categories/v2?include_bundles=true&variants_return_style=2&skip_num_categories=0";
/// `COLLECTIBLES_PURCHASES`.
pub const PURCHASES: &str = "/users/@me/collectibles-purchases?variants_return_style=2";

/// Product `premium_type` of the "Nitro exclusive" picker section.
pub const PREMIUM_TYPE_NITRO: u64 = 2;

/// Flattens the catalogue into its products, in category order.
pub fn products(v: &Value) -> impl Iterator<Item = &Value> {
    v.get("categories")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|c| c.get("products").and_then(Value::as_array))
        .flatten()
}

impl RestClient {
    /// The shop catalogue (raw JSON; CDN asset URLs are read at runtime).
    pub async fn get_collectibles_categories(&self) -> Result<Value> {
        self.call(Method::GET, CATEGORIES_V2, Body::None).await
    }

    /// Owned collectibles (raw JSON array of purchases).
    pub async fn get_collectibles_purchases(&self) -> Result<Value> {
        self.call(Method::GET, PURCHASES, Body::None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn flattens_products() {
        let v = json!({"categories": [{"products": [{"sku_id": "1"}, {"sku_id": "2"}]}, {"products": []}, {}]});
        assert_eq!(products(&v).count(), 2);
        assert_eq!(products(&json!({})).count(), 0);
    }
}
