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
| profile popout | 253781 `outer_P1YEjQ.user-profile-popout` | width / bg / radius / max-height | --custom-user-profile-popout-width 300 / --background-surface-high / --radius-sm / 100vh-20 | ui/profile.rs popout_card | ✅ (was 300 / floating / fixed height) |
| profile popout | 253781 `inner_P1YEjQ` | gap / padding-bottom | 8px / 4px | ui/profile.rs popout_card | ✅ |
| popout banner | 253781 `.user-profile-popout` + 58495 `banner_s9bWOi` | height / radius / bottom line | 105px / 8px 8px 0 0 / 1px --border-muted | ui/profile.rs banner | ✅ |
| popout header | 352421 `header_NOlkhE` | min-height | banner + 35px | ui/profile.rs popout_card | ✅ |
| popout avatar | 875762 `avatar_WRLIGC` | inset-start / top | 16px / 61px (80px avatar + 6px ring) | ui/profile.rs popout_card | ✅ (was banner-40) |
| badges | 352421 `headerTag_NOlkhE` | position / radius | bottom 8px, end 16px / --radius-xs | ui/profile.rs badges | ✅ (new; icon 22px from CDN asset, list bundle not cached) |
| custom status bubble | 82171 `container_rmCN9_`, `outer_`, `inner_` | pos / bg / border / radius / padding / max-width | start 105, top banner-7 / --background-surface-highest / 1px --border-muted / --radius-lg / 8px 12px / 181px | ui/profile.rs status_bubble | ✅ (was a grey box in the body) |
| popout body | 352421 `body_NOlkhE` | gap / padding | 12px / 4px 16px 8px | ui/profile.rs popout_card | ✅ (was 12/16) |
| display name | 866212 `displayNameRow_C2XpHA` | gap / line-height | --space-4 / 24px (lg 30px) | ui/profile.rs name_block | ✅ |
| username row | 866212 `usernameAndPronounsRow_` | column / row gap, username colour | 9.33px (6px w/ pronouns) / 2px, --text-strong | ui/profile.rs name_block | ✅ |
| dot spacer | 866212 `dotSpacer_C2XpHA` | size / colour / margin | 4px / --text-default / 0 1px | ui/profile.rs name_block | ✅ |
| server tag pill | 866212 `guildTagPill_C2XpHA` | h / radius / bg / border / padding / hover | 24 / --radius-sm / --background-mod-subtle / 1px --border-muted / 0 8 / --background-mod-strong | ui/profile.rs guild_tag_pill | ✅ (new) |
| role pill | `role_nie4FD`, `roleCircle_7RsUnm` | h / radius / bg / font / pad / margin; dot | 22 / 4 / --background-mod-normal / 12px 500 / 4 / 0 4 4 0; 12px, 0 4px | ui/profile.rs roles_block | ✅ (was 24h, --bg-tertiary) |
| activity card | 253781 `overlay_P1YEjQ`, 352421 `card_NOlkhE` | bg / radius / padding | --user-profile-overlay-background / --radius-sm / 12px | ui/profile.rs overlay_card | ✅ |
| popout footer | 352421 `footer_NOlkhE` | padding | 0 16px 12px | ui/profile.rs popout_card | ✅ |
| message input | --control-input-height-md, --input-* | h / bg / border / radius | 40 / --input-background-default / --input-border-default / 8 | ui/profile.rs popout_card | ✅ (was a bordered button) |
| profile modal | f4f30b `root_ZhNJfE`, 8e8ecc `layoutContainer_s3DB6Y` | w / h / bg / border / radius | min(962, vw-96) / min(800, vh-96) / --background-base-lower / 1px --border-normal / --radius-lg | ui/profile.rs modal_card | ✅ (was 600px single column) |
| modal padding / gutter | `profileContentInner_s3DB6Y`, `root_ZhNJfE` | padding / gutter | --space-48 (≥1025px, else 32) / --space-32 | ui/profile.rs modal_card | ✅ |
| modal profile card | 8e8ecc `profile_s3DB6Y` | width / bg / radius / outline / shadow | min(max(300, 45.5vw-43.7), 400) / --background-surface-high / 16 16 0 0 / --border-muted / --shadow-medium | ui/profile.rs modal_card | ✅ |
| modal banner / avatar | 253781 `.user-profile-modal-v2`, 875762 | banner / avatar / avatar top / start | 140 / 120 / banner-60 / card padding 32 | ui/profile.rs modal_card | ✅ (was 212 / 120) |
| modal header buttons | 315513 `headerInner_NxZlvf`, `headerButtons_` | min-h / gap / pt | 52 / 8 / 16 | ui/profile.rs modal_card | ✅ |
| modal tabs | 291103 `tabBar_OpoUmC`, a94a5b `tabPanelScroller_y4ZEM5` | gap / pad / border / item h; panel pad | 32 / 0 16 / --user-profile-border / 25; top 20 bottom 32 | ui/profile.rs modal_card | ✅ (new) |
| modal close | f4f30b `modalHeaderButtons_ZhNJfE` | top / end | --space-8 / --space-8 | ui/profile.rs modal_card | ✅ |
| Mana button | 384209 `button_F7qvhY` sm/md, primary/secondary/critical-primary | h / pad / radius / min-w / colours | 32: 4 12, 40: 8 16 / --radius-sm / 60, 100 / --control-* | ui/profile.rs mana_button | ✅ (new) |
| confirm modal | 776412 `container_dvN-MP` | bg / border / radius / width | --background-surface-high / 1px --border-subtle / --radius-md / size-md 480 | ui/modal.rs confirm | ✅ (was 440, BG_PRIMARY, radius 8) |
| confirm modal | `padding-size-sm_dvN-MP`, `section_dvN-MP` | padding | top 8 bottom 16; sections 16 24 8 | ui/modal.rs confirm | ✅ (was grey footer 16) |
| confirm footer | `actionBarTrailing_dvN-MP` | gap / buttons | --space-8 / md secondary + primary/critical | ui/modal.rs confirm | ✅ (was "Cancel" link) |
| modal backdrop | --background-scrim | bg | scrim token | ui/modal.rs frame, ui/profile.rs | ✅ (was #000000B3) |
| settings, switcher, pickers, inbox, home rows, status picker | — | — | — | — | ❌ not audited yet |
