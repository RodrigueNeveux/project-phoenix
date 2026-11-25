use gpui::SharedString;
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Favorite,
    Current,
    WantToPlay,
    Played,
}

impl Kind {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "favorite_games" => Self::Favorite,
            "current_games" => Self::Current,
            "want_to_play_games" => Self::WantToPlay,
            "played_games" => Self::Played,
            _ => return None,
        })
    }
    pub fn cap(self) -> usize {
        match self {
            Self::Favorite => 1,
            Self::Current => 5,
            _ => 20,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Game {
    pub id: u64,
    pub name: SharedString,
    pub cover: Option<SharedString>,
    pub icon: Option<SharedString>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GameWidget {
    pub kind: Kind,
    pub games: Vec<Game>,
    pub order: usize,
}

pub fn widget_ids(widgets: &[Value]) -> Vec<(usize, Kind, Vec<u64>)> {
    widgets
        .iter()
        .enumerate()
        .filter_map(|(order, w)| {
            let d = w.get("data")?;
            let kind = Kind::parse(d.get("type")?.as_str()?)?;
            let ids = d
                .get("games")?
                .as_array()?
                .iter()
                .filter_map(|g| g.get("game_id")?.as_str()?.parse().ok())
                .take(kind.cap())
                .collect();
            Some((order, kind, ids))
        })
        .collect()
}

pub fn parse_game(id: u64, v: &Value) -> Game {
    let cover = v
        .get("media")
        .and_then(|m| m.get("cover_url").or_else(|| m.get("cover").and_then(|c| c.get("value"))))
        .and_then(Value::as_str)
        .filter(|u| !u.is_empty())
        .map(|u| if u.starts_with("//") { format!("https:{u}") } else { u.to_owned() })
        .map(Into::into);
    let name = v.get("name").and_then(Value::as_str).unwrap_or_default().to_owned();
    let icon = v
        .get("icon_hash")
        .and_then(Value::as_str)
        .filter(|h| !h.is_empty())
        .map(|h| format!("https://cdn.discordapp.com/app-icons/{id}/{h}.png?size=48").into());
    Game { id, name: name.into(), cover, icon }
}

pub fn collection_icons(widgets: &[GameWidget]) -> (Vec<SharedString>, usize) {
    let mut seen = std::collections::HashSet::new();
    let games: Vec<&Game> = widgets.iter().flat_map(|w| &w.games).filter(|g| seen.insert(g.id)).collect();
    let icons: Vec<SharedString> = games.iter().filter_map(|g| g.icon.clone()).take(4).collect();
    let extra = games.len().saturating_sub(icons.len());
    (icons, extra)
}

pub fn breadcrumb(widgets: &[GameWidget], custom_theme: bool) -> Option<gpui::Stateful<gpui::Div>> {
    use crate::theme::{generated::idx, z};
    use gpui::{div, img, prelude::*, FontWeight};
    let (icons, extra) = collection_icons(widgets);
    if icons.is_empty() {
        return None;
    }
    let tile_bg = if custom_theme { idx::OPACITY_WHITE_24 } else { idx::BACKGROUND_MOD_STRONG };
    let last = icons.len() - 1;
    let tiles = icons.into_iter().enumerate().map(|(i, u)| {
        let count = i == last && extra > 0;
        div()
            .relative()
            .size(z(24.))
            .flex_none()
            .rounded(z(4.))
            .bg(crate::ui::chat::tk(tile_bg))
            .child(img(u).size(z(24.)).rounded(z(4.)).when(count, |d| d.opacity(0.5)))
            .when(count, |d| {
                d.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size(z(24.))
                        .rounded(z(4.))
                        .bg(gpui::rgba(0x0000_0099))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(z(12.))
                        .line_height(z(16.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(crate::ui::chat::tk(idx::NEUTRAL_9))
                        .child(SharedString::from(format!("+{extra}"))),
                )
            })
    });
    Some(
        div()
            .id("game-collection-breadcrumb")
            .flex_none()
            .h(z(40.))
            .p(z(8.))
            .rounded(z(8.))
            .flex()
            .items_center()
            .justify_between()
            .cursor_pointer()
            .bg(crate::ui::chat::tk(idx::USER_PROFILE_OVERLAY_BACKGROUND))
            .hover(|s| s.bg(crate::ui::chat::tk(idx::USER_PROFILE_OVERLAY_BACKGROUND_HOVER)))
            .child(
                div()
                    .text_size(z(12.))
                    .line_height(z(16.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(crate::ui::chat::tk(idx::TEXT_STRONG))
                    .child(crate::t!("board.game_collection")),
            )
            .child(div().flex().items_center().gap(z(4.)).children(tiles)),
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn reads_game_widgets_in_order_and_caps() {
        let w = json!([
            {"data": {"type": "application", "application_id": "1"}},
            {"data": {"type": "favorite_games", "games": [{"game_id": "5"}, {"game_id": "6"}]}},
            {"data": {"type": "played_games", "games": [{"game_id": "7"}]}}
        ]);
        let v = super::widget_ids(w.as_array().unwrap());
        assert_eq!(v, vec![(1, super::Kind::Favorite, vec![5]), (2, super::Kind::Played, vec![7])]);
        let g = super::parse_game(5, &json!({"name": "P", "media": {"cover_url": "https://x/c.png"}}));
        assert_eq!((g.name.as_ref(), g.cover.as_deref()), ("P", Some("https://x/c.png")));
    }

    #[test]
    fn collection_breadcrumb_shows_four_icons_and_counts_the_rest() {
        let game = |id: u64, icon: bool| {
            super::parse_game(id, &if icon { json!({"name": "g", "icon_hash": "h"}) } else { json!({"name": "g"}) })
        };
        assert_eq!(game(9, true).icon.as_deref(), Some("https://cdn.discordapp.com/app-icons/9/h.png?size=48"));
        let w = |kind, ids: &[(u64, bool)]| super::GameWidget { kind, games: ids.iter().map(|&(i, b)| game(i, b)).collect(), order: 0 };
        let ws = vec![
            w(super::Kind::Favorite, &[(1, true)]),
            w(super::Kind::Played, &[(1, true), (2, false), (3, true), (4, true), (5, true), (6, true), (7, true)]),
        ];
        let (icons, extra) = super::collection_icons(&ws);
        assert_eq!((icons.len(), extra), (4, 3));
    }
}
