# Settings pages parity audit

Source: CDP outlines of each page of the live 2026 client (user settings modal, `panel_nNq2Vb` content, 2026-11-23) and the CSS bundles in `~/discord-phoenix-ref/current-client/css/`. Code: `crates/phoenix-app/src/ui/settings/` (`pages.rs` holds the page contents as blocks).

| element | CSS / dump source | property | value | our code | status |
|---|---|---|---|---|---|
| sidebar sections | vanilla-settings-open sidebar | order | (General) Account, Data & Privacy, Message Permissions, Notifications / Billing: Nitro, Server Boost, Subscriptions, Gift Inventory, Billing / Experience: Voice & Video, Appearance, Accessibility, System, Language & Time / Games & Apps: Registered Games, Activity Privacy, Connected Apps / (Other) Developer, Log Out — no Vencord/Plugins/Themes/Updater | `mod.rs` `NAV` | ✅ (Billing and Registered Games added; billing pages point to the web) |
| developer page | vanilla-settings-15 | rows | Developer Mode, Application Test Mode (switches) | `pages.rs` `DEVELOPER` | ✅ (extra "User ID" row removed) |
| registered games | vanilla-settings-12 | rows | Current Game ("No game detected", "Not seeing your game? Add it!"), Added Games + IGDB note, Related → Activity Privacy | `pages.rs` `GAMES` | ✅ #2686 `notDetected_` box (bg --background-mod-subtle, r12, p16, 16/22 600 + 14 muted + --text-brand "Add it!"), content mismatch 3.7% (game list: no process scan, empty like a fresh client) |
| sidebar sub-nav | `subnav_8qMUr2` under the active item | rows | the page's category titles | `view.rs` `subnav` / `pages::titles` | ✅ |
| sidebar search | `searchBar` input (40px, 16px text) | behaviour | filters pages whose texts match, hides empty sections, the sub-nav and Log Out | `view.rs` `nav`, `pages::matches` | ✅ |
| category title | `heading-xl/medium_RAOoio` | font | 24/32, 500, --text-strong | `controls::section_title` | ✅ |
| category lead | `body-sm/normal_RAOoio` | font | 14/18, --text-subtle, 4px under the title | `pages.rs` `B::Lead` | ✅ |
| row | `baseControlItem_FiiQ6I` | box | padding/gap --space-12, margin-inline -12, min-height 48, --radius-lg | `pages.rs` `item` | ✅ |
| clickable row hover | `.clickable_FiiQ6I` | background | --interactive-background-hover | `pages.rs` `related`, `B::Expand` | ✅ |
| row title | `label.body-md/medium_RAOoio` | font | 16/20, 500, --text-strong | `pages.rs` `title_text` | ✅ |
| switch | `switchIndicator_9AraPt` | position | top-aligned with the title, trailing | `pages.rs` `item(.., top)` | ✅ |
| legend | `legend.body-md/semibold_RAOoio` | font | 16/20, 600, 30px box | `pages.rs` `B::Legend` | ✅ |
| radio group | `group_oJuRbG` | options | 20px indicator, text at 32px, 36px stride, optional body-sm description | `pages.rs` `radio_group` | ✅ |
| select | `selectButton_vWC4bk` + `listBox_b5PEJH` | box | 40px field, list padding 4, 40px options, radius 4 | `pages.rs` `dropdown` | ✅ |
| related settings | `heading-sm/semibold` "Related Settings" + `baseControlItem` with 48px leading icon box and trailing chevron | layout | 16/20 600 --text-muted header | `pages.rs` `related` | ✅ |
| divider | `divider_ZaYWLb` | box | 1px --border-subtle, margin 50px 0 | `pages.rs` `B::Divider` | ✅ |
| keybind key cap | `key_dMUH0f` | box | --background-mod-muted, 1px --border-strong, radius 4, h 23, padding 3 6 4, 12/600, 3px apart | `pages.rs` `key_cap` | ✅ |
| keybind group legend | `legend.body-lg/medium_RAOoio` | font | 20/24, 500 | `pages.rs` `key_group` | ✅ |
| keybind rows | default keybinds list | row | 16/20 --text-default, 1px separators, 56px rows | `pages.rs` `key_group` | ✅ |
| slider markers | `markValue_OnqacX` (vanilla-settings-09) | position | proportional to value (12/14/15/16/18/20/24px) | `controls::marker_slider` (numeric labels → proportional, midpoint hit zones) | ✅ side-by-side match, content mismatch 7.0% |
| profiles page | profile editor modal (`focusLock`, styles panel + preview) | layout | full-screen editor | `view.rs` `profiles` (styles column + preview inside settings) | ❌ partial |
| voice & video | vanilla-settings-07 #2702–#2777 | top block | "Micro"/"Haut-parleur" 16/500 labels (340px columns, gap 16), "Volume du micro" sliders (bar 4px, 16px round grabber #2737), "Test du micro" + meter, "Besoin d'aide ? … guide de dépannage." 14/18 | `voice.rs` `render`/`slider`/`mic_test` | ✅ top block 4.4% (07); lower half 07b (#2816 label 180, radios 215/273/331, PTT #2842 410, advanced #2855 471, divider then Caméra #2919): 4.0% → 3.2%; 07c backgrounds 24.9% (font + the 4 animated mp4 tiles stay plain, stills load at runtime) |

### Screen-by-screen pass (2026-11-23, vanilla dumps `vanilla-settings-01..15`, 1280x700 @1.5x, onyx/midnight, FR)

Scores = `% pixels |Δ|>24` on the content panel (dump png vs `dialog_shots settings:<page>` upscaled). Residuals come from the font (gg sans vs Noto Sans wraps differently), the user's real data, and dumps scrolled a few px (e.g. 01 is scrolled 38px: `panel_nNq2Vb` at y=115 instead of 153).

| page | dump node | fix | content before → after | status |
|---|---|---|---|---|
| Compte | #2684–#2729 | no "Display Name" row (only Username/E-mail/Phone) | 4.2% → 4.1% | ✅ |
| sidebar profile tab | #2489/#2490 | "Modifier les profils" + 12px pencil, 4px gap | — | ✅ |
| sidebar search | #2498 `input_0PLWhS` | text starts right after the 30px icon box (no input padding), 16px | — | ✅ |
| sidebar sub-nav | #2510/#2518–#2527 | first row 40px under the item label; Compte adds "Centre familial"; rows wrap instead of overflowing | side 10.0% → 9.8% | ✅ |
| switch rows | #2688 `baseSettingWrapper_4Z`, #2700 `description_VhFsFg` | padding 10 0, description max-width 572 | notifications 8.0% → 7.1%, data 12.4% → 12.6% (font wrap) | ✅ |
| related settings | #2733 `icon_W6yko_` | 48px --background-mod-muted tile behind the 24px glyph | activity 7.7% → 7.5% | ✅ |
| Apparence "Thèmes par défaut" | #2721 heading-sm/semibold | 600 weight | 7.8% | ✅ (08b) 5th "Synchroniser avec l'ordinateur" swatch (current theme bg + 24px RefreshIcon), `displayContainer_yH_g_W` banner (117x69 art, 20/24 title, primary button), limited-time theme (countdown pill + Vice City 135deg swatch), app icons (23 + limited) moved under the theme category with the client's art at runtime: 7.8% → 7.0% |
| Connexions lead | #2685-ish body-sm | full FR sentence | 14.4% → 14.6% | ✅ text + 22 `accountButtonInner_yX-kzH` tiles (44px r8, 52/53px pitch, logos fetched at runtime from the URLs the client uses; 28px app icons for Riot/Battle.net); the Nitro rewards promo card (dynamic offer) is not reproduced |
| Voix & Vidéo breadcrumb | #2671 | "Voix & Vidéo" | — | ✅ |
| Nitro / Facturation | vanilla-settings-05b (re-dumped after the spinner) | `tierCard_20JRSQ` (95deg 3-stop gradient as two halves, r16, perks 32px pitch, overlay buttons) + `giftCard_` with runtime art | 49% → 4.3% (Nitro) | ✅ Nitro. The account is a Nitro Squad member, so the non-subscriber variant couldn't be observed; title is generic "NITRO" (no plan data). Facturation stays a pointer page (no billing API) ❌ |
| Système | vanilla-settings-10 | #2728 `warning_vb4RFD` banner (p8, --radius-xs, 20px icon), #2764 `info_0v8Ae1` notices + sm "Modifier dans les paramètres système", secondary "+ Ajouter" button | 6.7% → 6.4% | ✅ |
| Accessibilité | vanilla-settings-09 | #2690 `preview_TpS9Es` (--background-base-low, --border-normal, p24, 175px max), user's messages + reactions + link, 3 status avatars + sm primary button | 7.0% → 5.9% | ✅ |

## Profile editor (full screen, "Modifier les profils")

Dump `vanilla-settings-profile-editor` (+ `-scrolled`, `-switcher`), 2026-11-23, vanilla client, 1280x700 CSS px. Code: `crates/phoenix-app/src/ui/settings/profile_editor.rs`; offscreen `--example profile_editor -- out.png [dirty]`.

| element | dump node | property | CSS value | our code | status |
|---|---|---|---|---|---|
| layer / root | #3153 `layer_C_WHpX`, #3161 `layoutContainer_s3DB6Y` | layout | 1184 wide, 48 each side, full height | `render` max_w 1184 mx 48 | ✅ |
| styles panel | #3170 `panel_BfN7RR editingPanel_` | box | 242 wide, padding-top 32, --background-base-lowest, 1px --border-subtle left/right | `styles_panel` | ✅ |
| profile switcher | #3178 `header_cuaHiV`, #3185 `selectButton_VrghH6` | header | padding 4 8, border-bottom; button padding 8 4 4 8, r8, 16/20 500 | `styles_panel` header | ✅ |
| switcher popout | `-switcher` dump | rows | main profile (Discord glyph + check) then one row per server with the per-server name | `switcher` | ✅ structure, ❌ exact popout box not measured |
| groups | #3189 `content_UqVm6V`, #3192 `group_Z5MObs` | spacing | padding 16, gap 24, label h3 14/18 500 --text-strong, gap 8 | `group_label` / `content` | ✅ |
| tiles | #3195/#3231 `tile_HhFoZE` | box | bar 208x42, square 100x100, gap 8, r8, --background-mod-subtle | `tile` | ✅ |
| theme tile | #3281 `gradient_Z2TFt5`, #3284 `swatchInner_` | swatches | two 24px swatches, 2px white border, r8, on a primary→accent gradient | `theme_tile` | ✅ |
| card | #3330 `profile_s3DB6Y` | box | 400 wide, r 16 16 0 0, banner 140, avatar 120 at (32,80) | `card` | ✅ |
| card body | #3373 `profileBody_` | spacing | padding 4 24 32 32, gap 24; name 24/32 600; username 14/18 + 4px dot + pronouns | `card` body | ✅ |
| inline fields | #3383/#3405/#3443 `container_CJLDJI` / `editable_IEutSJ` | editing | display name, pronouns, bio edited in place on the card | `inline_field` + Input/Textarea | ✅ |
| tabs | #3479 `tabBarItem_y4ZEM5` | layout | Board / Activity / Wishlist, padding-bottom 16, 30 apart; "Your widgets" 14/18 600 + "Add widget" | `tabs` | ✅ structure, ❌ widget cards (user content) |
| close | #3164 `modalHeaderButtons_` | box | 31px secondary button, top 40, right 8 | `close` | ✅ |
| unsaved bar | #3327 `profileCardToastContainer_` | content | not dumpable without a real edit (forbidden) | `unsaved_bar` | ❌ unverified |
| Nitro locks | n/a (account has Nitro) | locked tiles | lock badge + disabled pickers when `has_premium()` is false | `tile(…, locked)` | ❌ unverified |

Scores (`% pixels |Δ|>24`, dump png vs offscreen mock render, 1280x700): styles panel 22.3%, card 95.4%, tabs 59.2%, overall 59.2%. The card and right column are dominated by the user's real data (Nitro gradient theme, image banner, custom profile background, widgets) versus mock data, so they are not a layout measure; the styles panel is the comparable region (residuals: avatar/banner images, gg sans vs Noto Sans).

Real-data mode (`PHX_REAL=1 --example profile_editor`, user's own account read-only, same 1280x700 regions):

| run | styles panel | card | tabs | board | right column | overall |
|---|---|---|---|---|---|---|
| mock (e1e62f9) | 22.3% | 95.4% | 59.2% | — | — | 59.2% |
| real data, before fixes | 14.3% | 79.7% | 99.5% | 94.1% | 70.2% | 55.0% |
| + themed card/backdrop, tab geometry, app widget | 14.3% | 12.2% | 8.9% | 18.0% | 12.5% | 12.7% |

Added rows:

| element | dump node | property | CSS value | our code | status |
|---|---|---|---|---|---|
| themed card | #3330 `profile_s3DB6Y` + `:before` | background | gradient primary→140px→secondary, overlay black/.6 | `card` `card_layers` | ✅ |
| themed backdrop | #3324-#3328 `outer_P1YEjQ`/`inner_`/`backgroundImage_s3DB6Y` | layers | modal colour, black .48→.12 @300, banner .3 masked 280 | `render` `backdrop` | ✅ (no 15px blur) |
| tabs | #3476-#3485 | position | x+32, y+20; item lh 14 pb 16; panel +20 | `tabs` | ✅ 8.9% |
| app widget | #3498-#3539 `widgetContainer_zEJODa` | layout | r16, header 24, content p32 0, hero 204x148 fade, 2x2 items 48px gap 16 | `profile_widgets.rs` | ✅ structure, ❌ hero image fade exact |
