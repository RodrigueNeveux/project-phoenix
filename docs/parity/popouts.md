# Parity audit: member list, menus, tooltips, popouts

Source: `~/discord-phoenix-ref/current-client/css` (Nov 2026 client). Density default
(`--space-xxs` 4, `--space-xs` 8, `--space-sm` 12, `--space-md` 16, `--space-lg` 20);
`--radius-xs` 4, `--radius-sm` 8, `--radius-md` 12.

| element | CSS source (file + selector prefix) | property | CSS value | our code | status |
|---|---|---|---|---|---|
| member list | 831130 `container_86es0b` | background | --background-base-lower | ui/members.rs render | ✅ |
| member list | 831130 `container_86es0b` | border-inline-start | 1px --border-subtle | ui/members.rs render | ✅ |
| member list | theme `--custom-member-list-width` | width | 264px | theme m::MEMBER_LIST_W | ✅ |
| member list | 831130 `members_86es0b` | padding-bottom | 10px | ui/members.rs render | ✅ (fixed) |
| group header | 831130 `membersGroup_86es0b` | padding-block | --space-lg --space-xxs (20/4) | ui/members.rs header | ✅ (was 24/0, h 40) |
| group header | 831130 `membersGroup_86es0b` | padding-inline | --space-md --space-xxs (16/4) | ui/members.rs header | ✅ (was 16/8) |
| group header | screenshot (Text variant, no CSS) | case / size | not uppercased, 14px/500 | ui/members.rs header | ✅ (was uppercase 12px/600) |
| member row | 40791 `member_Iy0c7A` | margin-inline-start / max-width | 8px / list-16px | ui/members.rs member_row | ✅ |
| member row | 40791 `memberInner_Iy0c7A` | padding-block | --space-xxs (4px) | ui/members.rs member_row | ✅ (was missing) |
| member row | 121046 `layout_TBNqol` | height / radius | 42px / 4px | ui/members.rs member_row | ✅ |
| member row | 121046 `avatar_TBNqol` | margin-inline-end | 12px | ui/members.rs member_row | ✅ |
| member row | 121046 `subText_TBNqol` | margin-top | -2px | ui/members.rs member_row | ✅ (was 0) |
| member row | 40791 `offline_Iy0c7A` | opacity / hover | .3 / 1 | ui/members.rs member_row | ✅ |
| owner crown | 40791 `ownerIcon_Iy0c7A`, `icon_Iy0c7A` | color / size / margin | --text-feedback-warning / 14px / 4px | ui/members.rs member_row | ✅ (was missing) |
| bot tag | 40791 `botTag_Iy0c7A` | margin-inline-start | 4px | ui/members.rs app_tag | ✅ |
| server tag | 40791 `clanTag_Iy0c7A` | margin | 1px 0 1px 4px | ui/members.rs member_row | ✅ (was missing) |
| server tag chip | `chipletContainerInner_k8jyN7` | bg / radius / padding / line-height | --background-mod-strong / 4px / 0 4px / 16px | ui/members.rs server_tag | ✅ (new) |
| counts row | not in cached CSS (screenshot) | online / members / voice | dot+count, gap 16 | ui/members.rs counts_row | ✅ (new) |
| Activity cards | 4187692 `container_DKsPd0` (closest), `thumbnailContainer` 72px | padding / gap / radius / border | --space-12 / 12 / --radius-sm / --border-subtle | ui/members.rs activity_section | ✅ (new; exact member-list bundle not cached) |
| Activity badges ("New player", "Resurrection") | content-inventory API | — | — | — | ❌ data not available from presences |
| context menu | 476662 `menu_M3qk4j` | background | --background-surface-higher | ui/context_menu.rs MENU_BG | ✅ (was surface-high) |
| context menu | 476662 `menu_M3qk4j` | border / radius | 1px --border-subtle / 8px | ui/context_menu.rs panel_at | ✅ (was 4px, hard-coded colour) |
| context menu | `scroller_M3qk4j`, `--menu-scroller-block-padding` | padding | 8px 8px | ui/context_menu.rs panel_at | ✅ (was 6px 8px) |
| menu item | `labelContainer_M3qk4j` | min-height / padding | 32px / 8px | ui/context_menu.rs | ✅ |
| menu item | `item_M3qk4j` | margin | 0 | ui/context_menu.rs | ✅ (was 2px 0) |
| menu item focused | `colorDefault_M3qk4j.focused_` | bg / color / radius | --interactive-background-hover / --text-strong / 4px | ui/context_menu.rs HOVER_BG | ✅ (was blurple + white) |
| menu danger | `colorDanger_M3qk4j` | color / focused bg | --text-feedback-critical / --background-feedback-mod-critical | ui/context_menu.rs | ✅ (was red bg) |
| menu icon / caret | `iconContainer_M3qk4j`, `caret_M3qk4j` | size | 20px | ui/context_menu.rs | ✅ (was 18) |
| menu separator | `separator_M3qk4j` | margin | --custom-menu-separator-margin 8px | ui/context_menu.rs | ✅ (was 4px) |
| tooltip | `tooltip_ZDdyhE` | bg / radius / padding | --background-surface-high / --radius-sm / 8px 12px | ui/tooltip.rs | ✅ |
| tooltip | `tooltip_ZDdyhE` | max-width | 200px | ui/tooltip.rs | ✅ (was 190) |
| profile popout | `--custom-user-profile-avatar-size`, `-banner-height` | sizes | 96/120px, 105/120/140px | ui/profile.rs | ❌ not audited yet |
| modals, settings, switcher, pickers, inbox, home rows | — | — | — | — | ❌ not audited yet |
