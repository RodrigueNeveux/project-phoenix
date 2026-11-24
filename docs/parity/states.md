# Interactive states parity (hover / active / selected / tooltip)

Source: live vanilla client (CDP, CSSOM rules matched against the on-screen elements with
`:hover`/`:active` stripped, 2026-11-24), the current-client CSS bundles, and the
`vanilla-tooltip-guild` / `vanilla-tooltip-header` dumps. Values only, no CSS copied.

Legend: ✅ implemented and traced to the CSS rule, ❌ missing or wrong, ➖ the state does not exist in the client.

## Guild rail

| component | state | client (CSS source) | ours | status |
|---|---|---|---|---|
| pill `item__o8_Xq` | rest / unread | 8px, opacity 0 → 1 with `.visible`, translateX(-4px → 0) | server_bar.rs `pill` | ✅ |
| pill | hover | height 20px | `pill` | ✅ |
| pill | selected | height 40px | `pill` | ✅ |
| pill | transition | height/opacity/transform .2s ease-out | `PILL_MS` 200 | ✅ |
| guild icon | squircle | fixed squircle mask path (`svg_hqZPsU`), no radius morph on hover in the refresh | `ICON_RADIUS` 12 | ✅ |
| home / add / discover tile `childWrapper_JJ-hgR` | hover, selected | `--background-brand` bg, `--white` glyph, scale 1.1 (`childWrapperHoverScale`), .15s ease-out | brand bg + white glyph, `states::fade_to` 150ms; live DOM: no rail item carries `childWrapperHoverScale`, so nothing scales | ✅ |
| folder `folderIcon_RTMSuZ` | hover | `--background-mod-subtle` bg | `--background-mod-subtle` | ✅ |
| rail tooltip `tooltip_ZDdyhE` | position | card starts 12px right of the tile (x 72 for a 20..60 tile), vertically centred, caret left | anchor at tile right + 12 − 5 | ✅ (fixed) |
| rail tooltip | style | `--background-surface-high`, 8px radius, inset 1px `--border-subtle` + `--shadow-high`, padding 8 12, body-md/semibold 16/20 | tokens | ✅ (fixed: was old palette) |
| rail tooltip | appear | no delay (~60ms), opacity 1, slides 4px leftwards over ~100ms (ref-cache tooltip-rail-appear.json) | 100ms 4px slide + `zoom_card_sized` 0.95 → 1, no fade | ✅ |

## Channel list

| component | state | client | ours | status |
|---|---|---|---|---|
| channel row `wrapper_TNyK-N .link` | hover | `--interactive-background-hover` bg, name `--interactive-text-hover` | sidebar.rs channel row | ✅ |
| channel row | active (pressed) | `--background-modifier-active` → `--interactive-background-active` | `.active()` | ✅ (added) |
| channel row | selected | `--interactive-background-selected` | ✅ | ✅ |
| channel row muted `modeMuted_` | hover | name `--interactive-text-hover` | opacity 1 + hover colour | ✅ |
| thread row | hover / active | same as channel row | `.active()` | ✅ (added) |
| DM home nav rows (Friends, Nitro, Shop, Quests) | hover / active | same link rules | `.active()` | ✅ (added) |
| row action icons `actionIcon_Wx6SnQ` (invite, settings) | reveal on hover | `iconVisibility:hover .iconItem` display block, `.channelInfo` hidden | sidebar.rs `invite_hover` group_hover | ✅ |
| row action icons | hover / active | `--icon-muted` → `--interactive-text-hover`; active `--interactive-text-active` + translateY(.5px) | group_hover colour; group_active colour + 0.5px offset | ✅ |
| category `wrapper_yOdhm_` | hover | name + chevron `--interactive-text-hover` | sidebar.rs:408 | ✅ |

## Member list

| component | state | client | ours | status |
|---|---|---|---|---|
| member row `clickable_J1Adu_ .childContainer` | hover | `--interactive-background-hover`, avatar opacity 1 | members.rs | ✅ (now token) |
| member row | active | `--interactive-background-active`, text `--interactive-text-active` | `.active()` | ✅ (added bg) |
| offline member | hover | opacity back to 1 | ✅ | ✅ |

## Messages

| component | state | client | ours | status |
|---|---|---|---|---|
| avatar `avatar_2i8Bfm.clickable` | hover | drop-shadow `--elevation-medium` (0 4px 4px black 16%) | chat.rs msg-avatar `.hover(shadow)` | ✅ |
| avatar | active | translateY(1px) (decoration too) | `.active(mt 1px)` on the avatar; decoration stays put | ✅ (decoration offset not reproduced) |
| username `.clickable` | hover | underline | chat.rs author name `.hover(underline)` | ✅ |
| reaction `reaction_MmKct8` | hover / active | `--reaction-background-hover/-active`, border, count `--reaction-text-hover/-active` | message_ext.rs `reactions` | ✅ (added active + count colours) |
| add-reaction button `reactionBtn_` | reveal / hover / active | `forceShow` variant (padding 5 7, opacity 1), hover/active reaction tokens | message_ext.rs | ✅ |
| reply preview `repliedTextPreview_` | hover | `--text-strong` | chat.rs reply line `.hover` | ✅ (added) |
| reply spine | hover | border `--interactive-text-default` | chat.rs spine `.hover` | ✅ (added) |
| hover toolbar `buttonsInner_` | reveal | right 14px, top −16px, 34px tall, padding 2, radius 8, `--background-surface-high` (ref-cache message-hover-toolbar.json) | chat/message_ext.rs `toolbar` | ✅ |
| toolbar button `button_Z0MHyc` | hover | `--interactive-background-hover`, glyph `--interactive-text-hover`, icon scale 1.1 (.4s cubic-bezier(.16,1,.3,1)) | group_hover, 22px glyph (no easing) | ✅ |
| toolbar button | active | `--interactive-background-active`, glyph `--interactive-text-active`, icon scale 1 | `.active()` + group_active | ✅ (added) |

## Header, user panel, composer

| component | state | client | ours | status |
|---|---|---|---|---|
| header icon `clickable_ybUK7Z .icon` | hover / selected | no background; glyph `--icon-muted` → `--icon-subtle`, selected `--icon-strong` | states.rs `header_button` | ✅ (removed the old hover tile) |
| header tooltip | position | shows ~50ms after hover (no delay), centred, card 11px below the button, caret up, body-sm/medium 14/18, scale-in ~100ms (ref-cache tooltip-header-appear.json) | states.rs `below_tip`: centred, card top 11px below, no delay (offscreen: card y 84 = live 84) | ✅ (enter zoom 0.95 → 1 in 100ms via `states::zoom_card`) |
| user panel button `button_Y-8oZW` | hover | `--control-secondary-background-hover`, glyph `--interactive-text-hover`, background .2s ease (`states::fade_layer`) | bg + glyph via `panel-btn` group | ✅ (added) |
| user panel red (muted/deafened) | rest / hover | `--opacity-red-12` → `--opacity-red-24` | voice_panel.rs | ✅ |
| user panel | active | none in CSS | | ➖ |
| audio button pair `audioButtonParent_` | hover | pair `--background-mod-muted`, hovered half `--control-secondary-background-hover` | voice_panel.rs | ✅ |
| composer buttons `button_hfM7yP` (attach, gift, GIF, sticker, emoji) | hover | glyph `--interactive-text-hover`, 8px tile `--interactive-background-selected`, .2s | states.rs `composer_button` | ✅ (added) |
| composer buttons | active | glyph `--interactive-text-active` | `group_active` | ✅ (added) |
| app launcher button | hover / active | same rules | composer.rs `apps` | ✅ (added) |
| shared tooltip (`tooltip.rs`) | style | `tooltip_ZDdyhE` tokens, 14/18 medium | ✅ | ✅ |

## Context menus (`context_menu.rs`, `menu_M3qk4j`)

Source: CSS `_M3qk4j` rules + dumps vanilla-menu-{channel,guild,dm,member,message,…}.

| component | state | client | ours | status |
|---|---|---|---|---|
| menu item | hover (mouse) | `item_:hover` `--background-mod-subtle`, radius 2px | `row` `.hover` | ✅ |
| menu item | focused (keyboard / highlighted) | `colorDefault_.focused_` `--interactive-background-hover`, radius 4px, `--text-strong`; caret/subtext recoloured | `row(lit)` | ✅ |
| menu item | active (pressed) | `colorDefault_:active` `--background-mod-subtle` | `.active()` | ✅ (added) |
| danger item | rest / focused / active | `--text-feedback-critical`; `--background-feedback-mod-critical` | `row(danger)` + `.active()` | ✅ |
| disabled item | rest | `disabled_M3qk4j` opacity .5, no pointer events (live: "Mark As Read" with nothing unread, 34px) | `MenuItem::Disabled`; channel + guild "Mark As Read" | ✅ (added) |
| submenu row | hover opens, caret 20px `--text-muted` → white when focused | `Submenu` | ✅ |
| checkbox / radio | checked colours `--control-brand-foreground-new` | `Check` | ✅ |

## Header labels (DM)

| component | state | client (dump aria-label) | ours | status |
|---|---|---|---|---|
| DM toolbar tooltips | label | vanilla-dm: Démarrer un appel vocal / vidéo, Messages épinglés, Ajouter au MP, Masquer le profil d'utilisateur | fr.yml `header.*` | ✅ (relabelled from the dump) |
| group DM toolbar tooltips | label | vanilla-group-dm: … Inviter dans le groupe privé, Masquer la liste des membres | `gdm-add` / `gdm-members` | ✅ (added) |

## Popouts and settings

| component | state | client | ours | status |
|---|---|---|---|---|
| emoji picker cell `emojiItem_6OcK5D` | hovered / selected | `--interactive-background-selected` | pickers/grid.rs | ✅ |
| picker category rail `categoryItemDefaultCategory_` | hover / selected | hover `--interactive-background-hover`; selected `--background-base-low` | grid.rs `rail_item` | ✅ |
| picker tabs (GIF / Stickers / Emoji) | hover / active | `--control-secondary-background-hover`; active `--background-mod-normal` (+ strong on hover) | pickers/mod.rs | ✅ |
| settings sidebar `item_xyJHva` | hover | `--background-mod-muted`, label + glyph `--text-strong` | settings/view.rs `nav_item` | ✅ (glyph + label tint added) |
| settings sidebar | selected | `--background-mod-normal`, background-color .3s ease on selection | `states::fade_to` 300ms | ✅ (added) |
| settings sidebar destructive (Log Out) | hover | `--background-feedback-mod-critical` | ✅ | ✅ |

## Transitions

`states.rs` has a small colour-fade helper: `track` + `fade_layer` (a bg layer animated
with gpui `with_animation`, keyed on the last hover change, so first paint and remounts
don't flash) and `fade_to` for elements that already track hover.
Per CSS: channel rows, member rows, header icons and toolbar buttons have NO transition
(instant is correct). Faded: user panel buttons .2s, composer buttons .2s, rail tiles .15s,
pill .2s, reactions .1s (with the pressed colour over the fading layer).

Offscreen check: `cargo run -p phoenix-app --example states_shots -- <dir> x,y[,down] ...`
renders the mock app with the mouse at each point (pressed with `,down`).

## Coverage

61 rows: 60 ✅, 1 ➖, 0 ❌.
Transitions are instant in our app apart from the pill and the rail tooltip; the client uses
.1s–.2s ease-out colour fades.
