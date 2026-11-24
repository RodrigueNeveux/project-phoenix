//! "New player" / "Resurrection" badges of the member list Activity cards.
//! The live client reads them from the content inventory feed
//! (`GET /content-inventory/users/@me`, traits FIRST_TIME = 1 and
//! RESURRECTED = 5) and refetches it every 15 minutes (`g = 15 * MINUTE`).

use std::cell::RefCell;
use std::time::{Duration, Instant};

use gpui::Context;

use crate::{app::PhoenixApp, commands::{ParityReply, ParityRequest}, model::Id};

type Entry = crate::commands::InventoryEntry;

thread_local! {
    static FEED: RefCell<(Option<Instant>, Vec<Entry>)> = const { RefCell::new((None, Vec::new())) };
}

const REFRESH: Duration = Duration::from_secs(15 * 60);

/// True when the feed should be (re)fetched.
pub fn stale() -> bool {
    FEED.with(|f| f.borrow().0.is_none_or(|t| t.elapsed() > REFRESH))
}

/// Fetches the feed (read-only) if stale.
pub fn ensure(app: &mut PhoenixApp, cx: &mut Context<PhoenixApp>) {
    if !stale() {
        return;
    }
    FEED.with(|f| f.borrow_mut().0 = Some(Instant::now()));
    app.parity_request(ParityRequest::ContentInventory, cx, |_, r, cx| {
        if let Ok(ParityReply::Inventory(v)) = r {
            FEED.with(|f| f.borrow_mut().1 = v);
            cx.notify();
        }
    });
}

/// The feed entries (GLOBAL_FEED ranked order), empty until fetched.
pub fn entries() -> Vec<Entry> {
    FEED.with(|f| f.borrow().1.clone())
}

/// True once the feed answered and `user` authored an entry.
pub fn in_feed(user: Id) -> bool {
    FEED.with(|f| f.borrow().1.iter().any(|e| e.author == user))
}

/// (first time, resurrected) for `user` playing `game`.
pub fn traits(user: Id, game: &str) -> (bool, bool) {
    FEED.with(|f| {
        f.borrow()
            .1
            .iter()
            .filter(|e| e.author == user && e.game.as_deref().is_none_or(|n| n.eq_ignore_ascii_case(game)))
            .fold((false, false), |(a, b), e| (a || e.first_time, b || e.resurrected))
    })
}

/// Seed the feed (offscreen mock renders only; never used with a real account).
#[doc(hidden)]
pub fn seed(v: Vec<Entry>) {
    FEED.with(|f| *f.borrow_mut() = (Some(Instant::now()), v));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_user_and_game() {
        FEED.with(|f| *f.borrow_mut() = (Some(Instant::now()), vec![
                Entry { author: Id(1), game: Some("Hades".into()), first_time: true, ..Default::default() },
                Entry { author: Id(2), resurrected: true, ..Default::default() },
            ]));
        assert_eq!(traits(Id(1), "hades"), (true, false));
        assert_eq!(traits(Id(1), "Celeste"), (false, false));
        assert_eq!(traits(Id(2), "x"), (false, true));
        assert!(!stale());
    }
}
