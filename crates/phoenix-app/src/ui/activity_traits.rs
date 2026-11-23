//! "New player" / "Resurrection" badges of the member list Activity cards.
//! The live client reads them from the content inventory feed
//! (`GET /content-inventory/users/@me`, traits FIRST_TIME = 1 and
//! RESURRECTED = 5) and refetches it every 15 minutes (`g = 15 * MINUTE`).

use std::cell::RefCell;
use std::time::{Duration, Instant};

use gpui::Context;

use crate::{app::PhoenixApp, commands::{ParityReply, ParityRequest}, model::Id};

type Entry = (Id, Option<Id>, Option<String>, bool, bool);

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

/// (first time, resurrected) for `user` playing `game`.
pub fn traits(user: Id, game: &str) -> (bool, bool) {
    FEED.with(|f| {
        f.borrow()
            .1
            .iter()
            .filter(|e| e.0 == user && e.2.as_deref().is_none_or(|n| n.eq_ignore_ascii_case(game)))
            .fold((false, false), |(a, b), e| (a || e.3, b || e.4))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_user_and_game() {
        FEED.with(|f| *f.borrow_mut() = (Some(Instant::now()), vec![(Id(1), None, Some("Hades".into()), true, false), (Id(2), None, None, false, true)]));
        assert_eq!(traits(Id(1), "hades"), (true, false));
        assert_eq!(traits(Id(1), "Celeste"), (false, false));
        assert_eq!(traits(Id(2), "x"), (false, true));
        assert!(!stale());
    }
}
