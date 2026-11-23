# Navigation parity audit

Primary source: the live-client DOM dumps (`/mnt/windata/project-phoenix/dumps/-channels-N-N+.json` = "N-N", `-channels-me+.json` = "me"; node `#i` = dump index, inspected with `~/discord-phoenix-ref/tree.py`). The user's client runs `.density-cozy` + `.enlarge-target-size` + `.theme-midnight` (Onyx), so the dump values below override the default-density CSS rows further down where they disagree.

## Dump-verified (density-cozy)

| element | dump node | property | dump value | our code | status |
|---|---|---|---|---|---|
| title bar | N-N #13 `bar_3zSH5d` | height / background | 32 / none (app bg) | title_bar.rs `render` | ✅ |
| back / forward | N-N #19..#23 `backForwardButtons_nS9Yqn` | box / padding / radius / glyph | 80px leading, 26×24, `0 2 0 6`/`0 6 0 2`, 6px outer, 18px `--interactive-icon-default` | title_bar.rs `nav_arrow` | ✅ (history not wired) |
| title | N-N #15..#17 `title_LnjVAX` | gap / icon / text | 8 / 16px guild icon r4 / 14/500 `--text-strong` | title_bar.rs | ✅ |
| trailing | N-N #24..#38 | gap / button / glyph | 12 / 24 r6 / 18 `--icon-muted` | title_bar.rs `bar_button` | ✅ (Quests, window buttons: ❌ not implemented) |
| guild list | N-N #50 `nav.guilds_` | width | 80 (40 + 2×20, `--space-md` cozy) | `m::SERVER_BAR_W` | ✅ |
| guild list | N-N #55 `scroller_5dqHTG` | gap / padding | 10 / `0 0 26` (linux) | server_bar.rs `GAP`, `PAD_*` | ✅ |
| guild icon | N-N #160..#163 | size / mask | 40 / squircle | server_bar.rs `ICON` | ✅ |
| separator | N-N #66 `guildSeparator_3WHVfy` | size / color | 32×1 / `#97979f33` | server_bar.rs `separator` | ✅ |
| folder | N-N #68..#81 | header / bg / radius / list | 48 / folder colour 15% / 16 / first item margin-top 6, gap 10 | server_bar.rs `folder` | ✅ |
| channel list | N-N #585 `sidebarList_pog62O` | border / radius | 1px top+left `--border-subtle`, 12px top-left | sidebar.rs `render` | ✅ (width is user-resized 336 in the dump; we keep 268 default) |
| guild header | N-N #588 `header_XH4OpX` | height / padding / border | 49 / 10 / 1px `--border-subtle` | sidebar.rs `header` | ✅ |
| guild dropdown | N-N #590 `guildDropdown_XH4OpX` | height / padding / radius / gap | 32 / 0 10 / 8 / 2 | sidebar.rs `header` | ✅ |
| guild name | N-N #605 | font | 16/20 600 `--text-strong` + 16px chevron | sidebar.rs `header` | ✅ |
| invite button | N-N #609 `inviteButton_XH4OpX` | size / radius / glyph | 32 / 8 / 20 | sidebar.rs `header` (icon `invite`) | ✅ |
| channel row | N-N #628 `wrapper_TNyK-N` | margin-left / padding | 10 / 1 0 | sidebar.rs `channel_row` | ✅ |
| channel link | N-N #629/#638 `link_TNyK-N` | height / padding / radius / gap | 36 / 4 10 / 8 / 10 | sidebar.rs `channel_row` | ✅ |
| channel name | N-N #643 | font | 16 / 500 / line 28, `--channels-default` | sidebar.rs `channel_row` | ✅ |
| channel selected | N-N #858 | bg / color | `#97979f3d` / `--text-strong` | `INTERACTIVE_BACKGROUND_SELECTED` | ✅ |
| channel hover actions | N-N #865 `children_TNyK-N` | size | 2×24 at margin -4 | sidebar.rs `invite_hover` | ✅ |
| category | N-N #655..#663 | padding-top / height / padding / font | 16 / 24 / 0 6 0 20 / 14/18 500 | sidebar.rs `row` | ✅ |
| category add | N-N #662 `addButton forceVisible` | size / visibility | 24 (padding 4, 16 glyph) / always | sidebar.rs `row` | ✅ |
| panels card | N-N #976 `panels_pog62O` | inset / radius / border | 10 / 8 / 1px `--border-muted` | sidebar.rs `panels_card` | ✅ |
| user panel | N-N #979 `container_g6VwXp` | height / padding / gap | 56 / 14 / 10 | sidebar.rs `user_panel` | ✅ |
| account button | N-N #980 | margin / padding / gap / radius | -6 / 5 6 / 8 / 4 | sidebar.rs `user_panel` | ✅ |
| name / subtext | N-N #985 / #989 | font | 16/22 500 `--text-strong` / 12/13 500 `--interactive-text-default` | sidebar.rs `user_panel` | ✅ |
| mic / deafen | N-N #999..#1016 | button / chevron | 32 (r `8 0 0 8`) + 1 + 16 (r `0 8 8 0`), red 12% when muted | voice_panel.rs `audio_button` | ✅ |
| settings | N-N #1017 | size / radius | 32 / 8 | voice_panel.rs `panel_button` | ✅ |
| DM search | me #587/#588 `searchBar_u88IQl` | header / button | 49, padding 0 10 / 31px r8 `--control-secondary-background-default`, 1px `--border-muted`, 14/18 500 centred | sidebar.rs `header` | ✅ |
| DM nav rows | me #597..#625 `channel_zln_tS` | li / inner / link | 40 (margin-left 8) / 38 r8 / padding 8 16 8 10, 20px glyph + 12, 16/20 500 | sidebar.rs `dm_top` | ✅ |
| DM nav selected | me #598 | bg / color | `--background-mod-strong` / `--text-default` | sidebar.rs `dm_top` | ✅ |
| DM divider | me #648 `sectionDivider_u88IQl` | margin / color | 12 8 / `--border-subtle` | sidebar.rs `dm_top` | ✅ |
| DM header | me #649 `privateChannelsHeaderContainer_Bce46W` | height / padding / font | 24 / 0 6 4 20 / 14/18 500 `--channels-default` | sidebar.rs `dm_top` | ✅ |
| DM row | me #656..#665 `dm_zln_tS` | li / link / avatar | 50 (margin-left 8) / 48, pl 10 / 32 + margin 12 | sidebar.rs `channel_row` | ✅ |
| voice users, threads, voice panel | — | — | not present in the dumps yet (needs a dump of a guild with people in voice and a connected call) | voice_panel.rs | ❓ |

## CSS reference (default density)

Source: `~/discord-phoenix-ref/current-client/css/`, using the default density (`.density-default`: `--space-xs` 8, `--space-sm` 12, `--space-md` 16) and without the `refresh-fast-follow-avatars` experiment (`--guildbar-avatar-size` 40, `--guildbar-folder-size` 48). Colours are generated tokens (`theme::generated::idx`). Paths are relative to `crates/phoenix-app/src/ui/`.

| element | CSS source (file + selector prefix) | property | CSS value | our code | status |
|---|---|---|---|---|---|
| title bar | 419070 `:root` | height | `--custom-app-top-bar-height` 32px | title_bar.rs `m::TITLE_BAR_H` | ✅ |
| title bar | 300559 `bar_3zSH5d` | padding-inline-end / gap | `--space-12` / `--space-8` | title_bar.rs `.pr(12)` | ✅ |
| title bar title | 300559 `title_3zSH5d` | layout | absolute, centred | title_bar.rs `.absolute().inset_0()` | ✅ |
| title bar trailing | 300559 `trailing_3zSH5d` | gap | `--space-12` | title_bar.rs `.gap(12)` | ✅ |
| title bar button | `clickable_fErCLM` | size / radius | `--space-24` / `--custom-app-top-bar-item-radius` 6px | title_bar.rs `bar_button` | ✅ |
| title bar button | `clickable_fErCLM` | color / hover / selected | `--icon-muted` / bg `--background-mod-normal` / `--interactive-icon-active` | title_bar.rs `bar_button` | ✅ |
| guild list | 419070 `:root` | width | `--guildbar-avatar-size + 2*--custom-guild-list-padding` = 72 | `m::SERVER_BAR_W` | ✅ |
| guild list | 703298 `wrapper_5dqHTG` | background | `--background-base-lowest` | server_bar.rs `BG_TERTIARY` | ✅ |
| guild list | 703298 `scroller_5dqHTG` | padding-top / bottom | 4px (win/osx) / `--space-xs + 16px` = 24 | server_bar.rs `PAD_TOP`, `PAD_BOTTOM` | ✅ |
| guild list | 703298 `scroller_5dqHTG>[hidden]` | item spacing | `--space-xs` 8 | server_bar.rs `GAP` | ✅ |
| guild item | 607986 `listItem_rwEpIL` | width / justify | list width, centred | server_bar.rs `row` | ✅ |
| guild icon | 343178 `wrapper_JJ-hgR`, `icon_JJ-hgR` | size | `--guildbar-avatar-size` 40 | server_bar.rs `ICON` | ✅ |
| guild icon | `--radius-md` (squircle mask) | radius | 12px | server_bar.rs `ICON_RADIUS` | ✅ |
| guild icon (acronym) | 343178 `childWrapper_JJ-hgR` | bg / color | `--background-mod-subtle` / `--text-default` | server_bar.rs `guild` | ✅ |
| guild icon hover/selected | 343178 `wrapper:hover .childWrapper` | bg / color | `--background-brand` / `--white` | server_bar.rs `guild` | ✅ |
| acronym | 343178 `acronym_JJ-hgR` | font-weight | `--font-weight-medium` | server_bar.rs `guild_inner` | ✅ |
| pill | 441674 `wrapper__o8_Xq` / `item__o8_Xq` | width / visible part | 8px, margin-inline-start -4 → 4px | server_bar.rs `pill` | ✅ |
| pill | 441674 `item__o8_Xq` | radius / color | `0 4px 4px 0` / `--text-strong` | server_bar.rs `pill` | ✅ |
| pill | 441674 `.visible/.hovered/.selected` | height | 8 / 20 / 40 | server_bar.rs `pill` | ✅ |
| pill | 441674 `.full-motion .item` | transition | height .2s ease-out | server_bar.rs `PILL_MS` 200 | ✅ |
| separator | 878504 `guildSeparator_3WHVfy` | size / radius / color | 32x1 / 1px / `--app-frame-border` | server_bar.rs `separator` | ✅ |
| mention badge | 239502 `lowerBadge_hqZPsU` | position | bottom 0, right 0 (cut-out) | server_bar.rs `mention_badge` | ✅ |
| badge | 5257 `base_lMvXrQ` / `baseShapeRound` | height / min-width / radius | 16 / 16 / `--radius-round` | mod.rs `badge` | ✅ |
| add / discover | 607986 `circleIconButton_g8YUxD` | size / bg / color | 40 / `--background-mod-subtle` / `--text-default` | server_bar.rs `action` | ✅ |
| add / discover hover | 607986 `circleIconButton:hover` | bg | `--background-brand` | server_bar.rs `action` | ✅ |
| add / discover icon | 607986 `circleIcon_g8YUxD` | size | 20px | server_bar.rs `action` | ✅ |
| folder header | 878504 `folderHeader_RTMSuZ`, `folderButton` | size | `--guildbar-folder-size` 48 | server_bar.rs `folder` `.h(FOLDER)` | ✅ |
| closed folder | 878504 `folderPreviewWrapper_RTMSuZ` | size / radius / padding / bg | 48 / `--radius-lg` / 4px / folder colour, 60% transparent | server_bar.rs `folder` | ✅ |
| folder preview | 878504 `folderPreview`, `folderPreviewGuildIcon` | gap / cell / radii | 2px / 19px / 13px outer, 4px inner | server_bar.rs `folder` | ✅ |
| open folder icon | 878504 `folderIconWrapper`, `folderIcon` | padding / size / radius / hover | 4 / 40 / `--radius-md` / `--background-mod-subtle` | server_bar.rs `folder` | ✅ |
| open folder bg | 878504 `folderGroupBackground_RTMSuZ` | width / radius / border / bg | 48 / `--radius-lg` / 1px `--border-muted` / colour, 85% transparent | server_bar.rs `folder` | ✅ |
| folder list | 878504 `folderGuildsList>:first-child` | margin-top | `--space-xs - --custom-folder-padding` = 4 | server_bar.rs `folder` | ✅ |
| guild tooltip | `tooltip_3_v8qp`, `guildTooltipWrapper__-eZlF` | radius / border / padding / max-width | 8 / 1px `--border-subtle` / 8 12 / 196 | server_bar.rs `right_tooltip` | ✅ |
| channel list | 795186 `sidebarList_pog62O` | width | `--custom-guild-sidebar-width` 268 (incl. 1px border) | `m::SIDEBAR_W` | ✅ |
| channel list | 795186 `sidebarList_pog62O` | border / radius | 1px `--app-frame-border` start+top / `--radius-md` | sidebar.rs `render` | ✅ |
| channel list | 795186 `sidebar_pog62O` | background | `--background-base-lower` | sidebar.rs `BG_SECONDARY` | ✅ |
| guild header | 65a `header__xoAVS` | height / border | `--custom-channel-header-height` 49 / 1px `--border-subtle` | sidebar.rs `header` | ✅ |
| guild header | 65a `header` + `title` padding | name inset | `--space-xs` + `--space-xs` = 16 | sidebar.rs `header .px(16)` | ✅ |
| category | 65a `containerDefault_yOdhm_` | padding-top | `--custom-category-channel-space-before-category` 16 | sidebar.rs `Category .pt(16)` | ✅ |
| category | 65a `wrapperCommon_yOdhm_` | height / padding-inline | 24 / `--space-md` `--space-md - --space-xs` (16/8) | sidebar.rs `Category` | ✅ |
| category | 65a `wrapperCommon` | color / hover | `--channels-default` / `--interactive-text-hover` | sidebar.rs `Category` | ✅ |
| category | 65a `mainContent` / `icon_yOdhm_` | gap / chevron | 4px / 12px after the name, -90deg collapsed | sidebar.rs `Category` | ✅ |
| category add | 65a `addButton_yOdhm_` | size / visibility | 16px / hover only | sidebar.rs `Category` | ✅ |
| channel row | f4b2 `wrapper_TNyK-N` | margin-start / padding | `--space-xs` / 1px 0 | sidebar.rs `channel_row` | ✅ |
| channel row | f4b2 `link_TNyK-N` | radius / padding | `--radius-sm` / `--space-xxs` `--space-xs` | sidebar.rs `ROW_RADIUS`, `.px(8)` | ✅ |
| channel row | f4b2 `basicChannelRowLink` | min-height / gap | 24 (+8 → 32) / `--space-xs` | sidebar.rs `.h(32)`, `.gap(8)` | ✅ |
| channel name | f4b2 `name_TNyK-N` | line-height / color | `--channels-name-line-height` 24 / `--channels-default` | sidebar.rs `channel_row` | ✅ |
| channel icon | f4b2 `icon_TNyK-N` | size / color | 20 / `--channel-icon` | sidebar.rs `decorated_icon` | ✅ |
| hover | f4b2 `wrapper:hover .link` / `.name` | bg / color | `--interactive-background-hover` / `--interactive-text-hover` | sidebar.rs `channel_row` | ✅ |
| selected | f4b2 `modeSelected .link` / `.name` | bg / color / weight | `--interactive-background-selected` / `--interactive-text-active` / 500 | sidebar.rs `channel_row` | ✅ |
| unread | f4b2 `modeUnreadImportant` | name / icon | `--interactive-text-active` 500 / `--icon-subtle` | sidebar.rs `channel_row` | ✅ |
| muted | f4b2 `modeMuted` | name / icon | `--text-muted` / `--icon-muted`, opacity .5 | sidebar.rs `channel_row` | ✅ |
| unread pill | f4b2 `unread_TNyK-N` / `unreadImportant` | size / inset / radius / color | 4x8 / `-(--space-xs)+1px` / `0 --radius-xs --radius-xs 0` / `--interactive-text-active` | sidebar.rs `channel_row` | ✅ |
| hover action icon | f4b2 `iconItem_Wx6SnQ`, `actionIcon` | margin / size / color | 4px / 16 / `--interactive-text-default` | sidebar.rs `invite_hover` | ✅ |
| thread row | f4b2 `typeThread.wrapper` | margin-start | `28px + --space-xs` = 36 | sidebar.rs `thread_row` | ✅ |
| thread row | f4b2 `typeThread .link` | padding | 4px `--space-xs` | sidebar.rs `thread_row` | ✅ |
| thread spine | 3cf8 `spine_reDhww` | offset / color | `--channels-spine-offset-left` 24 / `--spine-default` | sidebar.rs `thread_row` | ✅ |
| voice user | 880186 `content_YUoOxm` | radius / padding / gap | `--radius-sm` / 4px `--space-xs` / 8 | voice_panel.rs `voice_user_row` | ✅ |
| voice user | 880186 `userSmall` / `avatarSmall` | height / avatar | 32 / 24 | voice_panel.rs `voice_user_row` | ✅ |
| voice user | 880186 `username` / `usernameSpeaking` | color | `--channels-default` / `--interactive-text-active` | voice_panel.rs `voice_user_row` | ✅ |
| voice user hover | 880186 `.clickable:hover .content` | bg | `--interactive-background-hover` | voice_panel.rs `voice_user_row` | ✅ |
| voice icons | 880186 `icons`, `icon_YUoOxm` | gap / size / color | 4 / 16 / `--channels-default` | voice_panel.rs `voice_user_row` | ✅ |
| server mute | 880186 `iconServer_YUoOxm` | color | `--icon-feedback-critical` | voice_panel.rs `voice_user_row` | ✅ |
| LIVE | 880186 `liveIcon_YUoOxm` | bg / color | `--background-feedback-notification` / white | voice_panel.rs `voice_user_row` | ✅ |
| server tag | 234017 `chipletContainerInner_k8jyN7` | bg / radius / padding / line-height | `--background-mod-strong` / 4 / 0 4 / 16 | voice_panel.rs `server_tag` | ✅ |
| server tag | `chipletParent_kQORUk` | gap to name | padding-start 4 | voice_panel.rs `server_tag` | ✅ |
| panels card | 795186 `panels_pog62O` | inset / radius / border / bg | `--space-xs` / `--radius-sm` / 1px `--border-muted` / `--background-base-low` | sidebar.rs `panels_card` | ✅ |
| user panel | 718878 `container_g6VwXp` | height / padding / gap | 56 / 12 (`--custom-guild-list-padding - --custom-panels-spacing + 4`) / `--space-xs` | sidebar.rs `user_panel` | ✅ |
| account button | 718878 `accountPopoutButtonWrapper` | margin / padding / radius / gap / min-width | -4 / 5 4 / `--radius-xs` / 8 / 112 | sidebar.rs `user_panel` | ✅ |
| account button | 718878 `accountPopoutButtonWrapper:hover` | bg | `--background-mod-normal` | sidebar.rs `user_panel` | ✅ |
| name | 718878 `panelTitleContainer` | margin | -1 top, 2 bottom | sidebar.rs `user_panel` | ✅ |
| buttons | 718878 `buttons_g6VwXp` | gap | `--space-8` | sidebar.rs `user_panel` | ✅ |
| panel button | 718878 `button_Y-8oZW` | size / radius / color / hover | 32 / `--radius-sm` / `--icon-subtle` / `--control-secondary-background-hover` | voice_panel.rs `panel_button` | ✅ |
| muted / deafened | 718878 `redGlow_Y-8oZW`, `redIcon__WEKcz` | bg / hover / icon | `--opacity-red-12` / `-24` / `--icon-feedback-critical` | voice_panel.rs `audio_button` | ✅ |
| mic / deafen chevron | 718878 `buttonChevron_gKpIZP` | size / gap / radii / icon | 16x32 / 1px / square inner corners / 16px | voice_panel.rs `audio_button` | ✅ |
| chevron hover | 718878 `audioButtonParent:hover` | bg | `--background-mod-muted` for both, hovered part `--control-secondary-background-hover` | voice_panel.rs `audio_button` | ✅ |
| voice panel | 593586 `container_9U6LUN` | padding / gap / border | 12 / 12 / 1px `--border-muted` bottom | voice_panel.rs `voice_panel` | ✅ |
| status line | 593586 `connection_9U6LUN`, 587669 `rtcConnectionStatus` | height | 32 | voice_panel.rs `voice_panel` | ✅ |
| ping glyph | 587669 `ping.largePing` | size / padding / radius / margin / bg | 18 + 7 = 32 / 8 / 8 / `--background-feedback-mod-*` | voice_panel.rs `large_ping` | ✅ |
| status label | 587669 `rtcConnectionStatusConnected/Connecting/Error` | color / height | `--text-feedback-positive/-warning/-critical` / 20 | voice_panel.rs `voice_panel` | ✅ |
| channel text | 593586 `channel_9U6LUN:hover` | decoration | underline | voice_panel.rs `voice_panel` | ✅ |
| noise / disconnect | 593586 `voiceButtonsContainer_9U6LUN` | gap | `--space-4` | voice_panel.rs `voice_panel` | ✅ |
| action buttons | 593586 `actionButtons_9U6LUN` | layout / gap | equal columns / `--space-xs` | voice_panel.rs `voice_panel` | ✅ |
| action button | 593586 `button_9U6LUN.buttonColor` | height / radius / border / bg / hover / color | `--space-32` / `--radius-sm` / `--border-muted` / `--control-secondary-background-default` / `-hover` / `--control-secondary-text-default` | voice_panel.rs `action_button` | ✅ |

Notes:
- Font sizes are set from JS text variants and don't appear in the CSS: channel 16/medium, category 14/medium sentence case, status 16/semibold. They were checked against the current client by eye.
- Not done yet: the title bar back/forward buttons (`backForwardButtons_nS9Yqn`, they need a navigation history), and the account popout that replaces the old status picker.
