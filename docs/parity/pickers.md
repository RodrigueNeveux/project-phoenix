# Expression pickers (emoji / GIF / sticker)

Source: dumps `vanilla-picker-emoji`, `vanilla-picker-gif`, `vanilla-picker-sticker` (1280x700 viewport) + CSS bundle 569603.

| element | CSS / dump source | property | value | our code | status |
|---|---|---|---|---|---|
| picker body | emojiPicker_-sZhZn (#2502) | size | 497x427 | pickers/mod.rs PICKER_W | ✅ |
| nav tabs | nav_58b0l2 (#2495) | height / padding | 30 / 0 16 | pickers/mod.rs tab_button | ✅ |
| search | container_VhFsFg (#2504) | size | 272x40 | pickers/mod.rs search_box | ✅ |
| diversity | diversitySelector_eHzdRB (#2511) | size | 38 | grid.rs tone_button | ✅ |
| emoji cell | emojiItemMedium_6OcK5D | size / img | 48 / 40 | grid.rs render_row | ✅ |
| rail | wrapper_6AvvhR (#2697) | width | 48 | grid.rs rail | ✅ |
| rail Nitro row | row_3MXiRB (#2701) | border-bottom / pb / mb | 1px --border-subtle / 8 / 8 | grid.rs render_body | ✅ (non-Nitro only) |
| rail pill | .small__o8_Xq .item.selected | height | 24px, on default categories too | grid.rs rail_pill | ✅ |
| frequently used | categorySection (#2525), 42 items | source | settings-proto/2 emoji frecency | grid.rs build + rest/frecency.rs | ✅ |
| inspector | inspector_1bDrGE (#2687) | height / graphic | 48 / 28 | grid.rs inspector | ✅ |
| GIF tiles | result_H8nOze | size / gaps | 231x110, 11 / 12 | gif.rs | ✅ |
| GIF favorites tile | img gif_H8nOze under categoryFadeBlurple | image | newest favorite GIF | gif.rs (favorite_gifs) | ✅ |
| GIF trending tile | img under categoryFade | image | first trending GIF | gif.rs | ✅ cause: `/gifs/trending` returns the preview as a raw `static.klipy.com` URL (not proxied), blocked by the image host allowlist until 24dd2a3; checked 2026-11-24 with `PHX_DEBUG_GIF` on real data |
| GIF requests | bundle 3ad442 `GIFS_TRENDING` / `GIFS_SEARCH` / `GIFS_SUGGEST` | query | `media_format` + `locale` (+ `q`, `limit`), no `provider` since Klipy | rest/search.rs gif_query | ✅ |
| GIF search placeholder | intl `T1Frnm` | text | "Search Klipy" / "Rechercher sur Klipy" | picker.search_klipy | ✅ |
| HC borders | `.high-contrast-mode .navButton_58b0l2` / `.navButtonActive_` / `.result_H8nOze` / `.wrapper_6AvvhR` / bare `.wrapper_-sZhZn` | border | 1px --border-subtle / --border-normal / --border-strong / inline-end --border-subtle / --border-normal | mod.rs tab_button, gif.rs hc_outline, grid.rs rail_shell | ✅ CDP computed (iframe) |
| Light selected emoji | `.theme-light .emojiItemSelected_6OcK5D` | bg / shadow | --background-base-low, inset 1px --border-subtle | grid.rs render_row | ✅ CDP computed |
| locked sticker | `.stickerLocked_5z8c5P` | filter | brightness(.4); Light opacity .4 | sticker.rs cell (dark: opacity .3 approximation, GPUI has no filter) | ✅ |
| sticker cell | sticker_5z8c5P | size / gap | 98 / 13 | sticker.rs | ✅ |
| sticker frequent | stickerCategoryGeneric (#2731) | rail + section | clock, first | sticker.rs | ✅ |
| soundboard | official guide + local client bundle `current-client/js/web.b63d846b329d6470.js`; UI still needs a safe visual reference | read path | default sounds: `GET /soundboard-default-sounds`; guild sounds: Gateway `REQUEST_SOUNDBOARD_SOUNDS {guild_ids}` → `SOUNDBOARD_SOUNDS {guild_id, soundboard_sounds[]}`. Observed sound fields: `name`, `sound_id`, `emoji_name`, `emoji_id`, `user_id`, `volume`, `available`; REST route registry also names `/guilds/{id}/soundboard-sounds`, top sounds, and sound metadata routes but method/use not yet proven here | no backend/model/picker | ⚠ read contract partly traced; no voice call, sound playback, upload, or account setting mutation. Picker UI/geometry and send payload remain unmeasured |
