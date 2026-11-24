# Composer parity

Sources: the 2026 client CSS (`~/discord-phoenix-ref/current-client/css`, file prefix + hashed
class) and its JS bundle (behaviour). The base composer box, buttons, placeholder, reply bar and
typing indicator were audited in `chat.md` (rows "composer", "reply bar", "typing").

Nothing was typed into the live client (typing would broadcast "is typing"). Popups that only open
from typed text are built from the CSS rules alone, marked **CSS only**: no live dump, no
side-by-side compare.

Renders: `cargo run -p phoenix-app --example shortcuts_shot -- out.png <mention|emoji|channel|long|upload>` (mock data).

| element | CSS / JS source | property | value | our code | status |
|---|---|---|---|---|---|
| autocomplete container | 116815 `autocompleteAttached_rg6La1` | position | absolute, bottom calc(100% + 8px) of channelTextArea_, inset-inline 8 / 16 | ui/autocomplete.rs render | ✅ CSS only |
| autocomplete container | 116815 `autocomplete_rg6La1`, 790340 `autocomplete_CAMiwx` | look | radius 5, --shadow-border + --shadow-high, --background-surface-high | autocomplete.rs | ✅ CSS only |
| autocomplete list | 790340 `autocompleteInner_` | padding | bottom 8 | autocomplete.rs | ✅ CSS only |
| autocomplete row | 790340 `autocompleteRow_` / `base_` | box | row padding 0 8; base padding 8, radius 3; 14/16 medium --interactive-text-default | autocomplete.rs | ✅ CSS only (was 36px rows, 15px, radius 4) |
| row selected / hover | 790340 `[aria-selected=true]>.base_`, `:hover` | bg | --interactive-background-hover | autocomplete.rs | ✅ CSS only |
| row icon | 790340 `autocompleteRowIcon_`, `icon_` | size / gap | 16px icon (channels), margin-inline-end 8; emoji 20px (`emojiImage_`) | autocomplete.rs | ✅ CSS only |
| row secondary text | 790340 `autocompleteRowContentSecondary_` | layout | flex-grow 1, margin-inline-start 16, text-align end, ellipsis | autocomplete.rs | ✅ CSS only |
| section title | 790340 `contentTitle_` | text | 14px medium --interactive-text-default, padding 4px 0, no uppercase | autocomplete.rs | ✅ (was 12px uppercase). JS Autocomplete.Title = div.base_ (padding 8, no row wrapper) > contentTitle_: 8px inline, 12px above and below |
| user row avatar | JS Autocomplete.User: Avatar `SIZE_24`, `getAvatarURL(guild,24)`, `status` | size | 24px with status | autocomplete.rs | ✅ 24px + status cut-out |
| / slash commands popup | 116815/790340 container + rows; 358931 `optionPill_`, `optionPillKey_`, `optionPillValue_` | look | shared autocomplete container and rows; option pills: radius 4, 1px --background-base-lowest, --background-mod-normal, key padding 1 8 on --background-surface-high | ui/chat/slash.rs | ✅ CSS only. The app-icon rail (`rail_3dKq9h`, `categorySection_`) is ❌ missing |
| sticker suggestions | 116815 `stickerAutoComplete_` | grid | 116px cells, gap 8, padding 8 16 | — | ❌ missing |
| upload tray | 331249 `channelAttachmentArea_iH8bsl` | layout | flex, gap 24, padding 20 10 10, overflow-x auto | ui/chat/composer.rs upload_tray | ✅ (was padding 16, divider) |
| upload card | 331249 `upload_JyxBSo` | box | 200x200, padding 8, --background-surface-high, 1px --border-normal, radius --radius-sm (12), --shadow-low | composer.rs | ✅ (was 216, radius 4) |
| upload preview | 331249 `media_sBdlOA` | radius | --radius-sm | composer.rs | ✅ |
| upload filename | 331249 `filename_sBdlOA` | text | margin-top 8, one line, ellipsis | composer.rs | ✅ |
| upload spoiler | 331249 `spoiler_sBdlOA` | filter | blur(--custom-channel-attachment-upload-spoiler-blur-radius 44px) | ui/chat/spoiler.rs (image crate: downscale to 64px, fast_blur, PNG) | ✅ (blur done once, off the UI thread; dimmed until ready) |
| upload action bar | 331249 `actionBar_JyxBSo` | position / look | top-right, translate(25%, -25%); hover-bar popover (surface-high, 1px border-muted, radius 8, padding 2); 20px icons | composer.rs | ✅ |
| upload action: edit | client action bar (Spoiler, Edit, Remove) | — | rename the file | composer.rs open_upload_edit | ✅ rename. The description (alt text) and spoiler fields of the client's modal are ❌ |
| character counter | JS CharacterCount; 331249 `characterCount_A7cPID` | rule / place | shown when remaining <= max/10 (200 of 2000); absolute bottom 10, inset-inline-end 14; body-sm/semibold, --text-default / --text-feedback-critical | composer.rs | ✅ (was < 200, 12px, below the box) |
| Nitro 4000 limit | JS CharacterCount: max = 4000 with `canUseIncreasedMessageLength`, visible when remaining <= max/10 or (Nitro and length > 2000) | rule | — | composer.rs char_count (+ test) | ✅ |
| edit-in-place hints | 775417/8e8ecc `operations_h_zE4S` | text | 12px medium --text-default, padding 7px 0; links --text-link | ui/chat/message_ext.rs | ✅ (was padding 4, regular) |
| drag-and-drop overlay | 865429 `uploadArea_TLgD1o` | bg | --opacity-black-80 | ui/chat/panels.rs drop_overlay | ✅ (was 70% black) |
| drop modal | 865429 `uploadDropModal_`, `bgScale_`, `inner_` | box | 310x170; --brand-500 radius 10; inner 2px dashed --opacity-white-40 radius 6; title 22px bold; instructions 12/16, mt 4 | panels.rs | ✅ (was 20/14px, no fixed height) |
| drop modal icons | 865429 `icons_`, `icon_ one/two/three` | art | three animated file icons (Discord art) | panels.rs | ❌ one generic glyph |
| slowmode | d53e40 `cooldownWrapper_bbCgC2`, `slowModeIcon_bbCgC2` | state | right of the typing row: margin-inline-start auto, padding 3 8 / inline-end 16, gap 4; countdown text then icon (margin-inline-start 4); bypass with MANAGE_MESSAGES/MANAGE_CHANNELS | ChannelExtra.slowmode/bypass_slowmode (rate_limit_per_user, cache permissions), composer.rs cooldown(), send held while counting down | ✅ CSS only. The icon (clock) and the mm:ss format are assumed |
| no-permission composer | client notice | state | disabled text area with the notice, no buttons | ChannelExtra.send_blocked (SEND_MESSAGES / SEND_MESSAGES_IN_THREADS from the cache's overwrite computation), composer.rs | ✅ wording = locale key `IYKTTc` of the client's fr/en bundles (fr: "Tu n'as pas la permission…") |
| multiline input | 962953 `scrollableContainer_2x8-Qn`, `buttons_2x8-Qn`, `textArea_2x8-Qn` | layout | grows with the text up to --custom-channel-textarea-text-area-max-height (50vh) then scrolls; buttons are a 56px row pinned to the top; text padding 17 0, 16/22; Enter sends, Shift+Enter newline | app.rs composer (TextareaState auto_grow + submit_on_enter), composer.rs | ✅ (was a single-line input). 22px lines: `.line_height` on the Textarea overrides the Input's 1.25rem (its refinement is applied last), padding back to 17 |
| typing indicator | 56501 `base_U9Tvvz` | text / dots | see chat.md | composer.rs typing | ✅ (chat.md) |
| reply bar | 841136 `replyBar_` | see chat.md | — | composer.rs reply_bar | ✅ (chat.md) |
