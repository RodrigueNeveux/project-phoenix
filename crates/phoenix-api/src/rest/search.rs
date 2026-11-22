//! Message search, GIF (Tenor proxy) and sticker endpoints.
//!
//! - `GET /guilds/{id}/messages/search` and `GET /channels/{id}/messages/search`
//! - `GET /gifs/trending`, `GET /gifs/trending-gifs`, `GET /gifs/search`, `GET /gifs/suggest`
//! - `GET /sticker-packs`, `GET /guilds/{id}/stickers`

use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use url::form_urlencoded::Serializer;

use super::{Body, RestClient};
use crate::error::Result;
use crate::models::{ChannelId, GuildId, Message, Snowflake, UserId};

/// `has:` filter values accepted by Discord search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HasFilter {
    Link,
    Embed,
    File,
    Video,
    Image,
    Sound,
    Sticker,
    Poll,
}

impl HasFilter {
    pub const ALL: [HasFilter; 8] = [
        Self::Link,
        Self::Embed,
        Self::File,
        Self::Video,
        Self::Image,
        Self::Sound,
        Self::Sticker,
        Self::Poll,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Link => "link",
            Self::Embed => "embed",
            Self::File => "file",
            Self::Video => "video",
            Self::Image => "image",
            Self::Sound => "sound",
            Self::Sticker => "sticker",
            Self::Poll => "poll",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|h| h.as_str().eq_ignore_ascii_case(s))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchSort {
    /// Newest first (Discord's default).
    #[default]
    Newest,
    Oldest,
    Relevance,
}

/// Parameters of a message search. Empty fields are omitted from the query.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SearchQuery {
    pub content: Option<String>,
    pub author_ids: Vec<UserId>,
    pub mentions: Vec<UserId>,
    pub channel_ids: Vec<ChannelId>,
    pub has: Vec<HasFilter>,
    /// Snowflake bounds (`before:` / `after:` dates are converted by the caller).
    pub min_id: Option<Snowflake>,
    pub max_id: Option<Snowflake>,
    pub pinned: Option<bool>,
    pub sort: SearchSort,
    /// Results to skip (pages of 25).
    pub offset: u32,
    pub include_nsfw: bool,
}

impl SearchQuery {
    pub fn text(content: impl Into<String>) -> Self {
        Self { content: Some(content.into()), ..Default::default() }
    }

    pub fn is_empty(&self) -> bool {
        self.content.as_deref().is_none_or(|c| c.trim().is_empty())
            && self.author_ids.is_empty()
            && self.mentions.is_empty()
            && self.channel_ids.is_empty()
            && self.has.is_empty()
            && self.min_id.is_none()
            && self.max_id.is_none()
            && self.pinned.is_none()
    }

    /// URL query string (without the leading `?`).
    pub fn to_query(&self) -> String {
        let mut s = Serializer::new(String::new());
        if let Some(c) = self.content.as_deref().map(str::trim).filter(|c| !c.is_empty()) {
            s.append_pair("content", c);
        }
        for a in &self.author_ids {
            s.append_pair("author_id", &a.to_string());
        }
        for m in &self.mentions {
            s.append_pair("mentions", &m.to_string());
        }
        for c in &self.channel_ids {
            s.append_pair("channel_id", &c.to_string());
        }
        for h in &self.has {
            s.append_pair("has", h.as_str());
        }
        if let Some(v) = self.min_id {
            s.append_pair("min_id", &v.to_string());
        }
        if let Some(v) = self.max_id {
            s.append_pair("max_id", &v.to_string());
        }
        if let Some(p) = self.pinned {
            s.append_pair("pinned", if p { "true" } else { "false" });
        }
        match self.sort {
            SearchSort::Newest => {
                s.append_pair("sort_by", "timestamp");
                s.append_pair("sort_order", "desc");
            }
            SearchSort::Oldest => {
                s.append_pair("sort_by", "timestamp");
                s.append_pair("sort_order", "asc");
            }
            SearchSort::Relevance => {
                s.append_pair("sort_by", "relevance");
                s.append_pair("sort_order", "desc");
            }
        }
        if self.offset > 0 {
            s.append_pair("offset", &self.offset.to_string());
        }
        if self.include_nsfw {
            s.append_pair("include_nsfw", "true");
        }
        s.finish()
    }
}

/// Filters typed in the search box, before user/channel names are resolved
/// to ids by the caller (it owns the cache).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedSearch {
    pub content: String,
    /// `from:name`
    pub from: Vec<String>,
    /// `mentions:name`
    pub mentions: Vec<String>,
    /// `in:channel` (leading `#` stripped)
    pub in_channels: Vec<String>,
    pub has: Vec<HasFilter>,
    /// `before:` / `during:` / `after:` raw values (YYYY-MM-DD).
    pub before: Option<String>,
    pub after: Option<String>,
    pub during: Option<String>,
    pub pinned: Option<bool>,
}

/// Split `hello from:bob in:#general has:image` into filters and free text.
/// Values may be quoted: `from:"Some Name"`. Unknown `key:` prefixes stay in
/// the content (so `http://` searches keep working).
pub fn parse_search(input: &str) -> ParsedSearch {
    let mut out = ParsedSearch::default();
    let mut words = Vec::new();
    for tok in tokenize(input) {
        let Some((key, val)) = tok.split_once(':') else {
            words.push(tok);
            continue;
        };
        let val = val.trim_matches('"').to_string();
        let handled = match key.to_ascii_lowercase().as_str() {
            _ if val.is_empty() => false,
            "from" => {
                out.from.push(val.trim_start_matches('@').to_string());
                true
            }
            "mentions" => {
                out.mentions.push(val.trim_start_matches('@').to_string());
                true
            }
            "in" => {
                out.in_channels.push(val.trim_start_matches('#').to_string());
                true
            }
            "has" => match HasFilter::parse(&val) {
                Some(h) => {
                    if !out.has.contains(&h) {
                        out.has.push(h);
                    }
                    true
                }
                None => false,
            },
            "before" => {
                out.before = Some(val);
                true
            }
            "after" => {
                out.after = Some(val);
                true
            }
            "during" | "on" => {
                out.during = Some(val);
                true
            }
            "pinned" => {
                out.pinned = Some(val.eq_ignore_ascii_case("true") || val == "yes");
                true
            }
            _ => false,
        };
        if !handled {
            words.push(tok);
        }
    }
    out.content = words.join(" ");
    out
}

fn tokenize(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for ch in input.chars() {
        match ch {
            '"' => {
                quoted = !quoted;
                cur.push(ch);
            }
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Convert a `YYYY-MM-DD` date (UTC midnight) into a snowflake bound.
pub fn date_to_snowflake(date: &str) -> Option<Snowflake> {
    let mut it = date.split('-');
    let y: i64 = it.next()?.parse().ok()?;
    let m: u32 = it.next()?.parse().ok()?;
    let d: u32 = it.next()?.parse().ok()?;
    if it.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let ms = days_from_civil(y, m, d) * 86_400_000;
    let rel = ms - DISCORD_EPOCH_MS;
    (rel >= 0).then_some(Snowflake((rel as u64) << 22))
}

const DISCORD_EPOCH_MS: i64 = 1_420_070_400_000;

/// Days since 1970-01-01 (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// One page of search results. Each entry is a hit; Discord returns it as a
/// one-element array (formerly with surrounding context messages), the hit
/// being the message flagged `hit` or the first one.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SearchResponse {
    #[serde(default)]
    pub total_results: u32,
    #[serde(default)]
    pub messages: Vec<Vec<Message>>,
    #[serde(default)]
    pub doing_deep_historical_index: bool,
}

impl SearchResponse {
    pub fn hits(&self) -> impl Iterator<Item = &Message> {
        self.messages.iter().filter_map(|group| group.first())
    }
}

#[derive(Debug, Clone)]
pub enum SearchOutcome {
    Results(SearchResponse),
    /// HTTP 202: the guild is still being indexed; retry later.
    Indexing { retry_after: f64 },
}

fn parse_search_reply(status: StatusCode, bytes: &[u8]) -> Result<SearchOutcome> {
    if status == StatusCode::ACCEPTED {
        #[derive(Deserialize)]
        struct Pending {
            #[serde(default)]
            retry_after: Option<f64>,
        }
        let p: Option<Pending> = serde_json::from_slice(bytes).ok();
        let retry_after = p.and_then(|p| p.retry_after).unwrap_or(2.0).clamp(0.5, 60.0);
        return Ok(SearchOutcome::Indexing { retry_after });
    }
    Ok(SearchOutcome::Results(serde_json::from_slice(bytes)?))
}

// ---- GIFs ------------------------------------------------------------------

/// A GIF result from Discord's Tenor proxy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Gif {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    /// Page URL; this is what gets sent as the message content.
    #[serde(default)]
    pub url: String,
    /// Media in the requested `media_format` (mp4 by default).
    #[serde(default)]
    pub src: String,
    #[serde(default)]
    pub gif_src: String,
    #[serde(default)]
    pub preview: String,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GifCategory {
    pub name: String,
    /// Preview media of the category tile.
    #[serde(default)]
    pub src: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct GifTrending {
    #[serde(default)]
    pub categories: Vec<GifCategory>,
    /// The "Trending GIFs" tile preview.
    #[serde(default)]
    pub gifs: Vec<Gif>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GifFormat {
    #[default]
    Mp4,
    Gif,
    TinyGif,
    Webm,
}

impl GifFormat {
    fn as_str(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Gif => "gif",
            Self::TinyGif => "tinygif",
            Self::Webm => "webm",
        }
    }
}

fn gif_query(extra: &[(&str, &str)], locale: &str, fmt: GifFormat) -> String {
    let mut s = Serializer::new(String::new());
    for (k, v) in extra {
        s.append_pair(k, v);
    }
    s.append_pair("media_format", fmt.as_str());
    s.append_pair("provider", "tenor");
    s.append_pair("locale", locale);
    s.finish()
}

// ---- stickers --------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Sticker {
    pub id: Snowflake,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Comma-separated autocomplete keywords (unicode emoji for standard ones).
    #[serde(default)]
    pub tags: String,
    /// 1 png, 2 apng, 3 lottie, 4 gif.
    #[serde(default)]
    pub format_type: u8,
    #[serde(default)]
    pub pack_id: Option<Snowflake>,
    #[serde(default)]
    pub guild_id: Option<GuildId>,
    #[serde(default = "yes")]
    pub available: bool,
}

fn yes() -> bool {
    true
}

impl Sticker {
    pub fn url(&self) -> String {
        let ext = match self.format_type {
            3 => "json",
            4 => "gif",
            _ => "png",
        };
        format!("{}/stickers/{}.{ext}", crate::models::CDN_BASE, self.id)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct StickerPack {
    pub id: Snowflake,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub stickers: Vec<Sticker>,
    #[serde(default)]
    pub cover_sticker_id: Option<Snowflake>,
}

#[derive(Deserialize)]
struct StickerPacks {
    #[serde(default)]
    sticker_packs: Vec<StickerPack>,
}

impl RestClient {
    /// `GET /guilds/{id}/messages/search`
    pub async fn search_guild_messages(&self, guild: GuildId, q: &SearchQuery) -> Result<SearchOutcome> {
        let path = format!("/guilds/{guild}/messages/search?{}", q.to_query());
        let (status, bytes) = self.raw(Method::GET, &path, Body::None).await?;
        parse_search_reply(status, &bytes)
    }

    /// `GET /channels/{id}/messages/search` (DMs and group DMs).
    pub async fn search_channel_messages(&self, channel: ChannelId, q: &SearchQuery) -> Result<SearchOutcome> {
        let path = format!("/channels/{channel}/messages/search?{}", q.to_query());
        let (status, bytes) = self.raw(Method::GET, &path, Body::None).await?;
        parse_search_reply(status, &bytes)
    }

    /// `GET /gifs/trending`: category tiles for the GIF picker home.
    pub async fn gif_trending(&self, locale: &str, fmt: GifFormat) -> Result<GifTrending> {
        let path = format!("/gifs/trending?{}", gif_query(&[], locale, fmt));
        self.call(Method::GET, &path, Body::None).await
    }

    /// `GET /gifs/trending-gifs`: the "Trending GIFs" grid.
    pub async fn gif_trending_gifs(&self, locale: &str, fmt: GifFormat) -> Result<Vec<Gif>> {
        let path = format!("/gifs/trending-gifs?{}", gif_query(&[], locale, fmt));
        self.call(Method::GET, &path, Body::None).await
    }

    /// `GET /gifs/search?q=`
    pub async fn gif_search(&self, query: &str, locale: &str, fmt: GifFormat) -> Result<Vec<Gif>> {
        let path = format!("/gifs/search?{}", gif_query(&[("q", query)], locale, fmt));
        self.call(Method::GET, &path, Body::None).await
    }

    /// `GET /gifs/suggest?q=`: search term suggestions.
    pub async fn gif_suggest(&self, query: &str, limit: u8, locale: &str) -> Result<Vec<String>> {
        let mut s = Serializer::new(String::new());
        s.append_pair("q", query);
        s.append_pair("limit", &limit.clamp(1, 20).to_string());
        s.append_pair("provider", "tenor");
        s.append_pair("locale", locale);
        let path = format!("/gifs/suggest?{}", s.finish());
        self.call(Method::GET, &path, Body::None).await
    }

    /// `GET /sticker-packs`: Discord's standard sticker packs.
    pub async fn get_sticker_packs(&self, locale: &str) -> Result<Vec<StickerPack>> {
        let path = format!("/sticker-packs?locale={}", url::form_urlencoded::byte_serialize(locale.as_bytes()).collect::<String>());
        let r: StickerPacks = self.call(Method::GET, &path, Body::None).await?;
        Ok(r.sticker_packs)
    }

    /// `GET /guilds/{id}/stickers`
    pub async fn get_guild_stickers(&self, guild: GuildId) -> Result<Vec<Sticker>> {
        self.call(Method::GET, &format!("/guilds/{guild}/stickers"), Body::None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_filters() {
        let p = parse_search(r#"hello from:bob in:#general has:image has:link world from:"Big Al""#);
        assert_eq!(p.content, "hello world");
        assert_eq!(p.from, vec!["bob", "Big Al"]);
        assert_eq!(p.in_channels, vec!["general"]);
        assert_eq!(p.has, vec![HasFilter::Image, HasFilter::Link]);
    }

    #[test]
    fn unknown_keys_stay_in_content() {
        let p = parse_search("see https://x.y has:nope from:");
        assert_eq!(p.content, "see https://x.y has:nope from:");
        assert!(p.from.is_empty() && p.has.is_empty());
    }

    #[test]
    fn dates_and_pinned() {
        let p = parse_search("before:2024-01-02 after:2023-05-06 pinned:true");
        assert_eq!(p.before.as_deref(), Some("2024-01-02"));
        assert_eq!(p.after.as_deref(), Some("2023-05-06"));
        assert_eq!(p.pinned, Some(true));
        assert!(p.content.is_empty());
    }

    #[test]
    fn date_snowflakes() {
        // 2015-01-01 is the Discord epoch.
        assert_eq!(date_to_snowflake("2015-01-01"), Some(Snowflake(0)));
        let s = date_to_snowflake("2015-01-02").unwrap();
        assert_eq!(s.0 >> 22, 86_400_000);
        assert_eq!(date_to_snowflake("2014-12-31"), None);
        assert_eq!(date_to_snowflake("2024-13-01"), None);
        assert_eq!(date_to_snowflake("garbage"), None);
    }

    #[test]
    fn query_string() {
        let q = SearchQuery {
            content: Some("a b&c".into()),
            author_ids: vec![Snowflake(1), Snowflake(2)],
            channel_ids: vec![Snowflake(3)],
            has: vec![HasFilter::Image],
            offset: 25,
            ..Default::default()
        };
        assert_eq!(
            q.to_query(),
            "content=a+b%26c&author_id=1&author_id=2&channel_id=3&has=image&sort_by=timestamp&sort_order=desc&offset=25"
        );
        assert!(!q.is_empty());
        assert!(SearchQuery::text("  ").is_empty());
        let r = SearchQuery { sort: SearchSort::Relevance, ..Default::default() };
        assert_eq!(r.to_query(), "sort_by=relevance&sort_order=desc");
    }

    #[test]
    fn search_reply() {
        let body = br#"{"total_results":2,"analytics_id":"x","messages":[[{"id":"10","channel_id":"5","author":{"id":"1","username":"a","discriminator":"0"},"content":"hi","timestamp":"2024-01-01T00:00:00+00:00","edited_timestamp":null,"tts":false,"mention_everyone":false,"mentions":[],"mention_roles":[],"attachments":[],"embeds":[],"pinned":false,"type":0,"hit":true}]]}"#;
        match parse_search_reply(StatusCode::OK, body).unwrap() {
            SearchOutcome::Results(r) => {
                assert_eq!(r.total_results, 2);
                assert_eq!(r.hits().next().unwrap().content, "hi");
            }
            _ => panic!(),
        }
        match parse_search_reply(StatusCode::ACCEPTED, br#"{"message":"Index not yet available","code":110000,"retry_after":3}"#).unwrap() {
            SearchOutcome::Indexing { retry_after } => assert_eq!(retry_after, 3.0),
            _ => panic!(),
        }
    }

    #[test]
    fn gif_shapes() {
        let t: GifTrending = serde_json::from_str(
            r#"{"categories":[{"name":"happy","src":"https://m/x.mp4"}],"gifs":[{"type":"gif","src":"https://m/t.mp4"}]}"#,
        )
        .unwrap();
        assert_eq!(t.categories[0].name, "happy");
        let g: Vec<Gif> = serde_json::from_str(
            r#"[{"id":"1","title":"t","url":"https://tenor.com/v","src":"https://m/a.mp4","gif_src":"https://m/a.gif","width":220,"height":124,"preview":"https://m/p.png"}]"#,
        )
        .unwrap();
        assert_eq!(g[0].width, 220);
        assert_eq!(gif_query(&[("q", "cat dog")], "en-US", GifFormat::Mp4), "q=cat+dog&media_format=mp4&provider=tenor&locale=en-US");
    }

    #[test]
    fn sticker_shapes() {
        let p: StickerPacks = serde_json::from_str(
            r#"{"sticker_packs":[{"id":"9","name":"Wumpus","description":"","cover_sticker_id":"7","stickers":[{"id":"7","name":"Wave","tags":"👋","format_type":3,"pack_id":"9","type":1}]}]}"#,
        )
        .unwrap();
        let s = &p.sticker_packs[0].stickers[0];
        assert_eq!(s.name, "Wave");
        assert!(s.url().ends_with("/stickers/7.json"));
    }
}
