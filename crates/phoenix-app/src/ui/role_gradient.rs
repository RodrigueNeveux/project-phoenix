//! Gradient role colours (`role.colors.secondary_color` / `tertiary_color`),
//! kept beside the role cache so `model::Role` stays unchanged. A member
//! whose top coloured role has a secondary colour shows its name with
//! `twoColorGradient_` (c1, c2, c1), tiled every 100px by `usernameGradient_`.

use std::collections::HashMap;
use std::sync::OnceLock;

use parking_lot::RwLock;

use crate::model::Id;

fn table() -> &'static RwLock<HashMap<Id, [u32; 3]>> {
    static T: OnceLock<RwLock<HashMap<Id, [u32; 3]>>> = OnceLock::new();
    T.get_or_init(Default::default)
}

/// Records role `id`'s gradient (or clears it when it has none).
pub fn set(id: Id, primary: u32, secondary: Option<u32>, tertiary: Option<u32>) {
    match secondary {
        Some(s) => {
            // holographic roles add a tertiary stop; plain gradients go back to c1
            table().write().insert(id, [primary, s, tertiary.unwrap_or(primary)]);
        }
        None => {
            table().write().remove(&id);
        }
    }
}

/// Gradient stops of role `id`, if it is a gradient role.
pub fn get(id: Id) -> Option<[u32; 3]> {
    table().read().get(&id).copied()
}

/// Gradient of the member's top coloured role (the one `member_color` picks).
pub fn for_member(app: &crate::app::PhoenixApp, guild: Id, m: &crate::model::Member) -> Option<[u32; 3]> {
    let roles = app.roles.get(&guild)?;
    let top = roles.iter().filter(|r| m.roles.contains(&r.id) && r.color.is_some()).max_by_key(|r| r.position)?;
    get(top.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_colour_goes_back() {
        set(Id(91), 0x369876, Some(0x71ff9e), None);
        assert_eq!(get(Id(91)), Some([0x369876, 0x71ff9e, 0x369876]));
        set(Id(91), 0x369876, None, None);
        assert_eq!(get(Id(91)), None);
    }
}
