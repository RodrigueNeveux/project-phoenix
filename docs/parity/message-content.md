# Message content parity

Values come from `~/discord-phoenix-ref/current-client/css/` (2026 client) and the Text
variants named in the client JS bundles of the local Discord cache (only values are recorded).
Mock render: `content_shots out.png dark` (mock `#showcase` channel of the first guild, filled
by `src/mock_showcase.rs` with one message of each kind). ✅ = matches CSS/JS values,
⚠ = rendered but not yet checked against a live dump, ❌ = missing.

| feature | CSS / JS source | values | our code | status |
|---|---|---|---|---|
| Bold / italic / underline / strike | `markup_` | strong 700, em italic, u, s | markdown.rs `DELIMS` | ✅ |
| `***bold italic***`, `__*u+i*__` | – | nested emphasis | markdown.rs `find_close` (run end) | ✅ (was rendering `*…*` literally) |
| Headers `#`/`##`/`###`, `-#` subtext | `markup_ h1/h2/h3`, `small` | 24/20/16, subtext 12 --text-subtle | markdown.rs `line_block`, ui/components.rs | ✅ (see text.md) |
| Lists (nested, ordered) | `markup_ ul/ol` | | markdown.rs `ListItem` | ⚠ |
| Block quotes | `blockquoteContainer_` | 4px bar --border-normal radius 4, padding 0 8 0 12 | ui/chat.rs `block` | ⚠ |
| Spoilers (text, click to reveal) | `spoilerContent_` | --spoiler-hidden-background | richtext.rs | ⚠ |
| Masked links + hover underline | `anchor_ anchorUnderlineOnHover_` | --text-link, underline on hover | richtext.rs | ⚠ |
| `<t:…>` timestamps | `timestamp_` pill | --background-mod-normal | markdown.rs `format_timestamp` | ⚠ |
| Inline code | `markup_ code.inline` | --font-code .85em, --background-code | richtext.rs | ⚠ |
| Code block + highlighting | `codeContainer_`, `hljs` | --font-code 14/18 | richtext/highlight.rs | ⚠ |
| Rich embed frame | `embedFull_ grid_` | 4px start border, radius 4, padding 2 16 16 12, max 516 | ui/chat.rs `embed` | ✅ |
| Embed provider | `embedProvider_` body-xs/normal | 12/16 --interactive-text-default | ui/chat.rs `embed` | ✅ |
| Embed author | `embedAuthorName_` .875rem semibold --text-strong; icon 24 + 8 | | ui/chat.rs `embed` | ✅ |
| Embed title | `embedTitle_` 1rem semibold --text-strong / link --text-link | | ui/chat.rs `embed` | ✅ |
| Embed description | `embedDescription_` body-sm/normal --text-default | 14/18 | ui/chat.rs `embed_markdown` | ✅ |
| Embed fields grid | `embedFields_` grid gap 8, mt 8; name body-sm/semibold --text-strong mb 2; value body-sm/normal | inline ≤3 per row (2 with thumb) | ui/chat.rs `embed` | ✅ |
| Embed footer | `embedFooterText_` body-xs/medium --text-default; icon 20 + 8 | | ui/chat.rs `embed` | ✅ |
| Embed thumbnail / media | `embedThumbnail_` ml 16 mt 8; `embedMedia_` mt 16 radius 4 | | ui/chat.rs `embed` | ✅ |
| Link preview (article) | same grid, side thumbnail 80 | | ui/chat.rs `embed` | ✅ |
| Image / gifv embeds | bare media, gifv max-h 300 | | ui/chat.rs `embed` | ✅ (real compare earlier) |
| Video embed (YouTube) | `embedVideoActions_` centred; `wrapper_lj7V-K` --opacity-black-60, radius 24, h 48, p 12; `iconWrapper_` opacity .6 → 1, 24px play + external | | ui/chat/attach.rs `video_embed_actions` | ✅ |
| Image attachment | `visualMediaItemContainer_` max 550x350, radius 8 | | ui/chat.rs `media_image` | ✅ |
| Image mosaic (2–10) | `mosaicItem_` rows | | ui/chat/mosaic.rs | ✅ |
| Spoiler image | blurred + "SPOILER" pill | | ui/chat.rs `spoiler_cover` | ⚠ |
| Video attachment chrome | `wrapper_FJ99sr` radius, `playPausePop_` --opacity-black-60 p12 icon 24, `videoControls_` 32px --opacity-black-60 on hover | | ui/chat/attach.rs `video_card` | ✅ |
| Audio player | `wrapperAudio_ newMosaicStyle_` 432, p16, radius 8, --background-base-lower, 1px --border-subtle; `audioControls_` --opacity-black-28 radius-sm mt 4; `controlIcon_` 24+4; duration --font-code 12/12 500; seek `mediaBarWrapper_` 6px | | ui/chat/attach.rs `audio_card` | ✅ |
| Generic file card | `fileWrapper_` 432; `file_` --background-surface-high, 1px --border-subtle, radius 8, --shadow-low, p16; icon 30x40 mr 8; name 16/20 --text-link hover underline; size body-xs 12/16 --text-muted (dump vanilla-thread-channel node fileWrapper_) | | ui/chat/attach.rs `file_card` | ✅ |
| File download hover button | `hoverButtonGroup_ nonMediaMosaicItem_` -8/-8, --background-base-low, radius 5; `hoverButton_` p6, 20px, hover --interactive-background-hover | | ui/chat/attach.rs `hover_download` | ✅ |
| Stickers | `stickerAsset_` 160 | | ui/chat.rs `attachment` | ⚠ (name fallback in mock: no network) |
| Poll container | `container_HOJNvw` --background-surface-high, 1px --border-subtle, radius-sm, p16, max 472 / min 302 | | ui/components.rs `render_poll` | ✅ |
| Poll question / prompt | body-md/medium (16/20 500) --text-strong / body-sm/normal --text-muted mt 4 | | ui/components.rs | ✅ |
| Poll answers | `answersContainer_` gap 8, margin 8 0 16; `answerInner_` min-h 50, p 8 16, gap 8, --background-mod-muted, outline (voted --control-brand-foreground, victor --status-positive, hover --border-strong); label body-sm/semibold | | ui/components.rs | ✅ |
| Poll results bar | `votePercentageBar_` --interactive-background-active / --polls-voted-fill / --polls-victor-fill; `votesData_` gap 8: count body-xs/semibold + % body-md/semibold | | ui/components.rs | ✅ |
| Poll bottom bar | `bottomBar_` gap 16, min-h --custom-button-button-md-height; details body-sm/normal muted | | ui/components.rs | ✅ |
| Forwarded messages | `container_4dy_KE` | | ui/chat/forward.rs | ✅ (forward agent) |
| System: join, boost, pin, thread, call | `systemMessage_` | | ui/chat/system.rs | ⚠ |
| `(edited)` label | `edited_` --text-muted .625rem | | ui/chat.rs `edited_marker` | ⚠ |

Coverage: 27 ✅ / 11 ⚠ / 0 ❌ of 38 rows. No real-data compare score for these
components yet (none of them is on the live channel the user shows).
