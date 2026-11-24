# Glitches seen with real data

Found by rendering Phoenix on the real account with `int-bin/real_shots` (offscreen,
read-only, 1600x900 at scale 1.25) across ~30 screens: busy text channels, forums,
announcements, voice and stage text chats, threads, DMs and group DMs. The renders are
private and stay in `/mnt/windata/project-phoenix/compare/hunt/`.

| # | Screen | Symptom | Cause | Fix (commit title) |
|---|--------|---------|-------|--------------------|
| 1 | Text channel with an image that is still loading or failed | A message showed a large empty hole above the next message | The image `with_loading` box used `--background-base-lower`, which is the chat background itself, and there was no `with_fallback` | Show a visible placeholder for chat images while they load or when they fail |
| 2 | Forum list | Post previews showed raw `**bold**` and `` `code` `` markers and unresolved mentions | The preview only stripped block prefixes | Render forum post previews without raw markdown markers |
| 3 | Forum with many tags | The last filter tag was cut in half next to "Tout" | The tag list was a non-wrapping flex row with `overflow: hidden`. The client uses `tagList_` (height 32, overflow hidden) around a wrapping `tagListInner_` | Wrap overflowing forum filter tags out of sight instead of cutting the last one |
| 4 | Edited announcement with a `#channel` mention | Text ran past the message column and was clipped at the right edge | The edited-message wrapper put the emoji text row in a wrapping flex row with no max width, so the row took its unwrapped width. The text segments themselves could also be shrunk below their wrap width | Keep edited messages with inline emoji inside the column; Keep message text segments beside emoji and channel icons inside the column |
| 5 | Announcement with a multi-line message and a mention | The `#` icon floated in the middle of a later line, and every line after it was indented by the icon width | A segment that held newlines was one tall flex item, and the icon was centred beside it | Break rich text at newlines beside inline emoji, and localise unknown mention names |
| 6 | Same | The mention of an unknown channel read "unknown" (English) | Hard-coded fallback names in `markdown.rs` | (same commit) uses `common.unknown_lower` / `common.unknown_user` |
| 7 | Voice text chat, announcement channel start | The channel welcome always showed the `#` icon | The icon was hard-coded | Use the channel type icon in the channel welcome header |
| 8 | Member list (bot statuses such as "/help · site ✨ · docs") | The custom status wrapped onto 3 lines and was drawn over the name | `emoji_text::label` split the text around the emoji without `nowrap` | Keep emoji labels on one line so member statuses no longer overlap the name |
| 9 | Music bot embeds | `<:deezer:…>` custom emoji showed as raw text | The embed markdown drew plain `StyledText` and ignored the emoji ranges | Draw custom and Unicode emoji as images in embed text |
| 10 | Music bot embeds | `[**Title** by **Artist**](url)` showed the asterisks | The masked link label was pushed as raw text | Parse inline markdown inside masked link labels |
| 11 | Stage channel chat | Rows showed only "⋯ name time" | Stage system types (27, 28, 29, 31) were mapped to `OtherSystem` | Render stage system messages (start, end, new speaker, topic) instead of empty rows |
| 12 | Thread | "Bienvenue dans #thread name !" with the `#` icon | The thread start reused the text channel welcome | Open threads on a thread start header instead of the channel welcome |
| 13 | 1:1 DM | A bare "⋯ name time" row for an unknown system message | Unknown types with no content still drew a row | Hide unknown empty system messages instead of drawing a bare row |
| 14 | Text channel join messages | The "Fais coucou !" button stretched across the whole message column | Block parent, so the button took the full width | Size the wave-to-say-hi button to its label instead of the full column |
| 15 | Bot embeds (DraftBot rules) | `#unknown` and `@unknown-role` in place of the server's channels and roles | Embed markdown was parsed with `NoResolve` | Resolve channel, role and user mentions in embed text |
| 16 | Channel header | The topic began with a raw `<:icon_public_channel_thread:…>` token | The topic was drawn as plain text | Render channel topics in the header with resolved mentions and custom emoji |
| 17 | Narrow window (1000 px), bot embed footer | "… 398 membres • Hier" lost its time, clipped at the right | The footer text was a shrunk flex item that kept its wider wrap | Wrap embed footers in narrow windows instead of clipping them |
| 18 | Friends page, forum | "Search" and "Search or create a post..." in English in the French UI; "Tous les amis - 196" used a hyphen | Inputs created before the synced settings set the locale kept the English placeholder | Refresh input placeholders when the synced locale arrives (friends and forum search); the French counts now use "—" |
| 19 | Member list, bot with a multi-line custom status | The status still stacked on 3 lines over the name after #8 | The status text held newlines | Fold newlines in one-line emoji labels such as member custom statuses |
| 20 | Rich text that opens on a blank line next to an emoji | The blank line was lost after #5 | The newline split dropped an empty first line | Keep blank lines that open a rich text block |
| 21 | Announcement, after #4 | The `#` icon ended one line and the channel name started the next | Icon and name were separate flex items | Keep a channel mention's icon and name together when the line wraps |
| 22 | Embeds and messages with an emoji, narrow window | The text after an emoji jumped to a new line (`🎶` alone, then "Started playing …") | Each text run between emoji was one flex item, so it could not flow on the emoji's line | Flow message and embed text word by word around inline emoji |
| 23 | Group DMs opened right after start | Times and day dividers were in English ("11/12/2026 8:20 PM", "November 13, 2026") | Rows rendered before the synced locale arrived stayed cached | Rebuild cached message rows when the locale changes so times and day dividers follow it |

## Seen but not fixed (known limits)

- Lottie stickers (Discord's own "Wave") show the "Autocollant : Wave" text fallback, because Lottie is not rendered.
- GPUI paints a text with the wrap from its last measure. When Taffy then shrinks that text as a flex item, it spills out of its box. #4, #17 and #22 work around it per site (`min-width: 0`, `flex: 1`, or no shrinking). The root fix is to re-wrap at the final width in GPUI's `TextLayout::prepaint`.
- Clamped profile bios cut a long URL on the line instead of wrapping it with `break-word` inside the 3-line clamp.
- Reply previews draw mentions as plain text, without the mention colour.
- Some large attachments (probably big GIFs) stay on the loading placeholder after 45 s in `real_shots`.
- A bot embed image (botillon.gg honeypot) is cropped on the right by `object-fit: cover`. The embed's width/height metadata probably does not match the served image. Not checked against the client yet.
- Group DM "a ajouté quelqu'un au groupe." does not name the added user, because the model does not keep message mentions.

## Overlays & pages

Found on offscreen `real_shots` renders of the real account (scale 1.25, `PHX_OPEN=…`,
`PHX_VIEW=…`, `PHX_PROFILE=peer`); renders stay private in
`/mnt/windata/project-phoenix/compare/realov`.

| Where | Glitch | Fix |
|---|---|---|
| Inbox mentions, pinned messages | Custom emoji shown as the raw `<:name:id>` token | `components::markdown_in` passes the message context so emoji render as images (a10c2a5) |
| Expression picker (DM) | Drawn over the DM profile panel, flush with the window edge | Right offset accounts for the 320px DM panel and the 268px member list, not a fixed 240 (5171e81) |
| Picker / search errors | English backend strings in the French UI ("Search isn't available.", "Discord is still indexing…") | Backend errors go through i18n (`backend.*`, `search.nothing`, `search.indexing`) (3c50aaa) |
| GIF picker home | "GIF à la une" tile without its preview image | Preview falls back to `gif_src` when Tenor's `src` is a video (1f12e76); root cause below (round 2) |
| Search results | Sidebar overlaid the chat: messages and composer buttons (GIF, emoji…) cut off | The chat reserves the 420px column instead of the member list / DM panel (d1c5e0d) |
| Profile modal | Last tab "17 serveurs en con" cut mid-word | The mutual servers tab shrinks with an ellipsis like the client ("9 s…") |
| Search results | Raw message text (`<@id>`, markdown, emoji tokens) | Results rendered with `markdown_in` (7a40a6a) |

Harness: `real_shots` gained `PHX_OPEN` (pins, threads, search, settings pages, server settings,
pickers, message menu), `PHX_PROFILE=peer`, and read-only forwards for pins, stickers, GIFs and search.

Checked, not bugs: Friends "Tous les amis - N" vs "Envoyée(s) — N" (client strings), "Marquer un message"
(client string), server settings fully black (user's Midnight theme overrides `PHX_THEME` once the synced
settings arrive), inbox tab ellipsis (client also ellipsizes at this width).

Open items of round 1 (search embeds, Favoris preview, trending tile) are fixed on main.

### Overlays & pages, round 2

| Where | Glitch | Fix |
|---|---|---|
| GIF picker "GIF à la une" | Tile blank: `/gifs/trending` returns a direct `static.klipy.com` URL and the image client only allowed Discord hosts | GIF provider hosts (klipy, Tenor, Giphy) allowed; this also unblocks the Tenor/Giphy renditions `gifv_animated` produces for gifv embeds |
| Guild header dropdown | Old 8-item menu ("Inviter des gens"), not the client's | Rebuilt with the client's items and order (Boost, Inviter sur le serveur, Analyses, Fils actifs, Répertoire, Signaler un raid…) in the shared menu panel |
| Server menu | "Afficher tous les salons" | "Montrer tous les salons" (client string) |
| Member menu | voice "Se déconnecter" | "Déconnecter" |
| Boost page | Literal ` ` shown in "18 boosts • Niveau 2" | Real no-break spaces in fr.yml |
| Quick switcher | Query/placeholder drawn at 16px | 22px on the input itself |
| Profile voice card | Guild acronym "AS" instead of the guild icon | Icon when the guild has one |
| Accessibility preview | Card fixed at 175px, sample button cut off | max-height 25vh of the real window |
| Guild header dropdown | "Paramètres de notificati…" ellipsized by a submenu caret | Plain row that opens the notification settings modal |
| Server settings | Close button pushed against / past the right edge by the roles list | Close button no longer shrinks, 20px margin |
| Account popout (user panel) | No profile widget card under the bio (client shows the first Board widget) | Same compact widget card as the profile popout |
| Invite / join dialogs | No close button | Top-right close button |

Harness: `PHX_OPEN` chanmenu/catmenu/guildmenu/guilddrop/usermenu/switcher/statuspicker/leave/invite,
`emoji:<row>`, `PHX_PROFILE=member:<n>`, `PHX_GUILD` alone, `PHX_DEBUG_GIF`.

Checked OK: emoji picker scrolled (unavailable emoji locked like the client), channel and category context menus (labels match `vanilla-menu-channel/category`), leave dialog.

Open: invite dialog has no friend list/search; Devices page lists only this device (no
`/auth/sessions`); Connections page doesn't list the account's connections; streaming activity card on bots
("En direct sur <status text>", empty art); popouts of some members stay on "Chargement du profil…".
