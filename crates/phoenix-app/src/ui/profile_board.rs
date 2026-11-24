//! Full-profile Board tab data and game widget cards.
//!
//! Live modal (dump vanilla-profile-modal-board): `container_FNEkcu` cards
//! (padding 12 16 16, gap 8, --radius-lg 16, --user-profile-overlay-background),
//! `headerRow_ltNcM4` 24 high with a heading-xs/medium 14/18 title; game
//! covers `cover_JeyAv-` 88x117 radius 8 in `grid_W5z7fj` (gap 16, wrapping);
//! the favourite game is `card_2y0Mhw` (gap 16): the cover, then
//! `details_2y0Mhw` (padding-inline-start 7, gap 4) with the name 14/18 500,
//! vertically centred.

use std::collections::HashMap;

use gpui::{div, img, prelude::*, Context, Div, FontWeight, Global, Hsla, SharedString};

use crate::{
    app::PhoenixApp,
    model::Id,
    theme::z,
    ui::{
        profile_games::{GameWidget, Kind},
        settings::{profile_activity::RecentGame, profile_widgets::AppWidget},
    },
};

#[derive(Clone, Default)]
pub struct Board {
    pub widgets: Vec<AppWidget>,
    pub games: Vec<GameWidget>,
    pub recent: Vec<RecentGame>,
}

impl Board {
    pub fn is_empty(&self) -> bool {
        self.widgets.is_empty() && self.games.iter().all(|g| g.games.is_empty())
    }
}

#[derive(Default)]
struct Store(HashMap<Id, Board>);
impl Global for Store {}

/// The user's Board data; fetched once (three read-only requests) on first use.
pub fn get(app: &PhoenixApp, user: Id, cx: &mut Context<PhoenixApp>) -> Board {
    let s = cx.default_global::<Store>();
    if let Some(b) = s.0.get(&user) {
        return b.clone();
    }
    s.0.insert(user, Board::default());
    let (w, g, r) = (app.backend.profile_widgets(user), app.backend.game_widgets(user), app.backend.recent_activity(user));
    cx.spawn(async move |this, cx| {
        let (w, g, r) = futures::join!(w, g, r);
        let _ = this.update(cx, |_, cx| {
            let b = cx.default_global::<Store>().0.entry(user).or_default();
            b.widgets = w.unwrap_or_default();
            b.games = g.unwrap_or_default();
            b.recent = r.unwrap_or_default();
            cx.notify();
        });
    })
    .detach();
    Board::default()
}

pub struct Colors {
    pub strong: Hsla,
    pub card: Hsla,
    pub cover: Hsla,
}

fn title(k: Kind) -> SharedString {
    match k {
        Kind::Favorite => crate::t!("board.favorite_game"),
        Kind::Current => crate::t!("board.current_games"),
        Kind::WantToPlay => crate::t!("board.want_to_play"),
        Kind::Played => crate::t!("board.played_games"),
    }
}

fn cover(g: &crate::ui::profile_games::Game, col: &Colors) -> Div {
    // cover_JeyAv- 88x117, radius 8
    div()
        .w(z(88.))
        .h(z(117.))
        .flex_none()
        .rounded(z(8.))
        .overflow_hidden()
        .bg(col.cover)
        .children(g.cover.clone().map(|u| img(u).size_full()))
}

/// One game widget card.
pub fn card(w: &GameWidget, col: &Colors) -> Option<Div> {
    if w.games.is_empty() {
        return None;
    }
    let header = div()
        .h(z(24.))
        .flex()
        .items_center()
        .child(div().text_size(z(14.)).line_height(z(18.)).font_weight(FontWeight::MEDIUM).text_color(col.strong).child(title(w.kind)));
    let body = if w.kind == Kind::Favorite {
        let g = &w.games[0];
        div().flex().gap(z(16.)).child(cover(g, col)).child(
            div().flex_1().min_w_0().pl(z(7.)).flex().flex_col().justify_center().gap(z(4.)).child(
                div().truncate().text_size(z(14.)).line_height(z(18.)).font_weight(FontWeight::MEDIUM).text_color(col.strong).child(g.name.clone()),
            ),
        )
    } else {
        div().flex().flex_wrap().gap(z(16.)).children(w.games.iter().map(|g| cover(g, col)))
    };
    Some(div().flex().flex_col().gap(z(8.)).pt(z(12.)).px(z(16.)).pb(z(16.)).rounded(z(16.)).bg(col.card).child(header).child(body))
}
