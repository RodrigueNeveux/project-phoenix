//! Shop catalogue for the collectible pickers (read-only GET).

use super::{DiscordBackend, api_err};
use crate::{backend::BoxFuture, ui::settings::collectibles::{ShopCatalog, parse_shop, promote}};

impl DiscordBackend {
    pub(super) fn collectibles_shop_impl(&self) -> BoxFuture<ShopCatalog> {
        self.with_session(move |_, s| async move {
            let v = s.rest.get_collectibles_categories().await.map_err(api_err)?;
            let mut cat = parse_shop(&v);
            // The picker moves the recommended SKUs to the front of its Shop
            // section (client `collectibles_picker` recommendations); best effort.
            if let Ok(rec) = s.rest.get_recommended_collectibles().await {
                promote(&mut cat.shop, &rec);
            }
            Ok(cat)
        })
    }
}
