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
| 4 | Announcement with a `#channel` mention | Text ran past the message column and was clipped at the right edge | The text segments next to an inline icon or emoji were flex items that Taffy shrank, but GPUI kept the line wrap from the wider measure | Keep message text segments beside emoji and channel icons inside the column |
| 5 | Announcement with a multi-line message and a mention | The `#` icon floated in the middle of a later line, and every line after it was indented by the icon width | A segment that held newlines was one tall flex item, and the icon was centred beside it | Break rich text at newlines beside inline emoji, and localise unknown mention names |
| 6 | Same | The mention of an unknown channel read "unknown" (English) | Hard-coded fallback names in `markdown.rs` | (same commit) uses `common.unknown_lower` / `common.unknown_user` |
| 7 | Voice text chat, announcement channel start | The channel welcome always showed the `#` icon | The icon was hard-coded | Use the channel type icon in the channel welcome header |
| 8 | Member list (bot statuses such as "/help · site ✨ · docs") | The custom status wrapped onto 3 lines and was drawn over the name | `emoji_text::label` split the text around the emoji without `nowrap` | Keep emoji labels on one line so member statuses no longer overlap the name |
| 9 | Music bot embeds | `<:deezer:…>` custom emoji showed as raw text | The embed markdown drew plain `StyledText` and ignored the emoji ranges | Draw custom and Unicode emoji as images in embed text |
| 10 | Music bot embeds | `[**Title** by **Artist**](url)` showed the asterisks | The masked link label was pushed as raw text | Parse inline markdown inside masked link labels |
| 11 | Stage channel chat | Rows showed only "⋯ name time" | Stage system types (27, 28, 29, 31) were mapped to `OtherSystem` | Render stage system messages (start, end, new speaker, topic) instead of empty rows |
| 12 | Thread | "Bienvenue dans #thread name !" with the `#` icon | The thread start reused the text channel welcome | Open threads on a thread start header instead of the channel welcome |
| 13 | 1:1 DM | A bare "⋯ name time" row for an unknown system message | Unknown types with no content still drew a row | Hide unknown empty system messages instead of drawing a bare row |

## Seen but not fixed (known limits)

- Lottie stickers (Discord's own "Wave") show the "Autocollant : Wave" text fallback, because Lottie is not rendered.
- Text before an inline emoji still ends its flex line. A word after the emoji starts on a new line instead of flowing on the same line, as Chromium's inline layout does. Fixing it needs inline boxes in GPUI's text layout.
- Clamped profile bios cut a long URL on the line instead of wrapping it with `break-word` inside the 3-line clamp.
- Reply previews draw mentions as plain text, without the mention colour.
- Group DM "a ajouté quelqu'un au groupe." does not name the added user, because the model does not keep message mentions.
