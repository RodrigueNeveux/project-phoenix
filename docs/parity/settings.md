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
| slider markers | `markValue_OnqacX` | position | proportional to value | `controls::marker_slider` (evenly spaced) | ❌ |
| profiles page | profile editor modal (`focusLock`, styles panel + preview) | layout | full-screen editor | `view.rs` `profiles` (styles column + preview inside settings) | ❌ partial |
| voice & video | `heading-xl` "Voix" / "Caméra" categories, mana controls | style | tokens above | `voice.rs` helpers | ✅ restyled, layout kept |
