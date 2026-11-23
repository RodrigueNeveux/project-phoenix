//! Shop catalogue for the collectible pickers (read-only GET).

use super::{DiscordBackend, api_err};
use crate::{backend::BoxFuture, ui::settings::collectibles::{ShopCatalog, parse_shop}};

impl DiscordBackend {
    pub(super) fn collectibles_shop_impl(&self) -> BoxFuture<ShopCatalog> {
        self.with_session(move |_, s| async move {
            let v = s.rest.get_collectibles_categories().await.map_err(api_err)?;
            Ok(parse_shop(&v))
        })
    }
}
