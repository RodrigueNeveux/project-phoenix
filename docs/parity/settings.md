# Settings pages parity audit

Source: CDP outlines of each page of the live 2026 client (user settings modal, `panel_nNq2Vb` content, 2026-11-23) and the CSS bundles in `~/discord-phoenix-ref/current-client/css/`. Code: `crates/phoenix-app/src/ui/settings/` (`pages.rs` holds the page contents as blocks).

| element | CSS / dump source | property | value | our code | status |
|---|---|---|---|---|---|
| sidebar sections | vanilla-settings-open sidebar | order | (General) Account, Data & Privacy, Message Permissions, Notifications / Billing: Nitro, Server Boost, Subscriptions, Gift Inventory, Billing / Experience: Voice & Video, Appearance, Accessibility, System, Language & Time / Games & Apps: Registered Games, Activity Privacy, Connected Apps / (Other) Developer, Log Out — no Vencord/Plugins/Themes/Updater | `mod.rs` `NAV` | ✅ (Billing and Registered Games added; billing pages point to the web) |
| developer page | vanilla-settings-15 | rows | Developer Mode, Application Test Mode (switches) | `pages.rs` `DEVELOPER` | ✅ (extra "User ID" row removed) |
| registered games | vanilla-settings-12 | rows | Current Game ("No game detected", "Not seeing your game? Add it!"), Added Games + IGDB note, Related → Activity Privacy | `pages.rs` `GAMES` | ✅ (detected game list ❌: no process scan) |
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
| voice & video | vanilla-settings-07 #2702–#2777 | top block | "Micro"/"Haut-parleur" 16/500 labels (340px columns, gap 16), "Volume du micro" sliders (bar 4px, 16px round grabber #2737), "Test du micro" + meter, "Besoin d'aide ? … guide de dépannage." 14/18 | `voice.rs` `render`/`slider`/`mic_test` | ✅ top block (content mismatch 9.2% → 7.6%); ❌ rest of page (Profil d'entrée audio radios, advanced voice expander, Caméra preview/backgrounds, Streaming) still old layout |

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
| Apparence "Thèmes par défaut" | #2721 heading-sm/semibold | 600 weight | 7.8% | ✅ (❌ "Aperçu du thème" button, Nitro theme upsell, limited/colour themes, app icon section) |
| Connexions lead | #2685-ish body-sm | full FR sentence | 14.4% → 14.5% | ✅ text (❌ brand logo tiles: we show text pills) |
| Voix & Vidéo breadcrumb | #2671 | "Voix & Vidéo" | — | ✅ |
| Nitro / Facturation | vanilla-settings-05/06 | Discord marketing art + live subscription data | 49% | ❌ pointer page by design (no billing API) |
| Système | vanilla-settings-10 | hardware-acceleration warning banner, "Ajouter un raccourci clavier" button | 6.7% | ❌ |
| Accessibilité | vanilla-settings-09 | chat "Aperçu" preview card at the top | 7.0% | ❌ |
