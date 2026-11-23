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

## Live DOM dump pass (Nov 2026 client, CDP dumps)

Dumps: `/mnt/windata/project-phoenix/dumps/-channels-N-N+.json` (guild, member list),
`-channels-me+.json` (Friends page), CDP walk of `.outer_P1YEjQ` on a DM (profile side panel).
Node refs are the dump `i` index. Colours are the computed values (midnight theme).
The member counts widget (`vc-membercount-*`) and platform/speaker indicators (`vc-*`) are
client-mod additions, not Discord: kept as our counts row but not treated as reference.

| element | dump node | property | value | our code | status |
|---|---|---|---|---|---|
| group header | N-N #1598 `membersGroup_86es0b` | padding / height | 24px 4px 4px 20px / 46px | ui/members.rs header | ✅ (was 20/16) |
| Activity header | N-N #1503 `headerContainer_HxSvbL` | label + 24px settings button (12px gear) + spacer + 24x18 chevron, margin -3px 0 | — | ui/members.rs activity_section | ✅ |
| Activity card | N-N #1512 `container_g7esM5 usesCardRows_` | padding / margin / gap / radius | 8 / 8 0 0 8 / 8 / 12px | ui/members.rs activity_section | ✅ (was 12 / 8px radius) |
| Activity card | N-N #1512 | bg / border | #0a0a0c / 0.67px #97979f@16% | BG_TERTIARY / DIVIDER | ✅ |
| Activity user line | N-N #1515–1522 | 16px face pile, margin-right 6, name 14/18 500, margin-bottom 2 | — | ui/members.rs | ✅ |
| Activity title | N-N #1527 `heading-xs/medium` | 14/18 500 #d4d5d8 | --text-default | ui/members.rs | ✅ (was 16 semibold) |
| Activity badges | N-N #1528 `badgesContainer_CTWncy` | gap 0 8, 12px icon + 12/16 #96979e | elapsed + state | ui/members.rs | ✅ ("Nouveau joueur" needs content-inventory data) |
| Activity thumbnail | N-N #1537/1538 | 48x48, radius 4 | — | ui/members.rs | ✅ (was 72) |
| member row | N-N #1643 `member_Iy0c7A` | margin-left 8, padding 1px 0, radius 8, 44px | — | ui/members.rs member_row | ✅ (was py 4 → 50px, radius 4) |
| member row inner | N-N #1644/1645 | padding 0 8 0 10; inner padding 4 0, 42px | — | ui/members.rs member_row | ✅ |
| server tag | N-N #1657 `chipletContainerInner_k8jyN7` | margin 1 0 1 4, padding 0 4, radius 4, rgba(151,151,159,.24); 14px badge + 12/16 600 #d4d5d8 | — | ui/members.rs server_tag | ✅ (badge image ❌: tag model has no badge URL here) |
| member subtext | N-N #1668–1681 `subText_J1Adu_` | margin-top -2; gap 4: 12px voice icon, "•", 12/16 500 status | — | ui/members.rs member_row | ✅ (voice icon new) |
| Friends header | me #1066 `container_ybUK7Z` | padding 8 8 8 20, 49px; 32px icon box / 20px icon; title 16/20 500 #dcdcdf margin-right 8; 4px dot margin 0 4 | — | ui/home/friends.rs header | ✅ (was divider bar) |
| Friends tabs | me #1075–1079 `item_6zZYhF` | padding 4 12, margin 0 8, radius 8, 32px, 16/20 400; selected rgba(151,151,159,.24) #dcdcdf; idle #96979e | — | ui/home/friends.rs header | ✅ (was 24px / radius 4) |
| Add Friend pill | me #1079 `addFriend_zrUwGA` | rgb(88,101,242) / white | --background-brand | ui/home/friends.rs | ✅ (was green) |
| Blocked tab | me #1074 | not present in the 2026 tab bar | — | ui/home/friends.rs | ✅ removed from the bar |
| New group DM button | me #1081/1082 | 32px box, 20px icon, padding-left 10, right 8 | — | ui/home/friends.rs | ✅ |
| Friends search | me #1086–1093 `searchBar_wVFOQW` | margin 12 16 8 24, 40px, radius 8, 1px rgba(151,151,159,.24), 30px icon box | — | ui/home/friends.rs list | ✅ (new, filters) |
| section title | me #1098/1099 | 50px; h2 margin 16 20 16 24, 14/18 500 #d4d5d8, not uppercased | — | ui/home/friends.rs list | ✅ (was 12px uppercase) |
| row divider | me #1100 `divider_y75PxA` | 1px, margin 0 20 0 30 | --border-subtle | ui/home/friends.rs list | ✅ |
| friend row | me #1101 `peopleListItem_y75PxA` | padding 16 10, margin 0 10 0 14, radius 8, 61px | — | ui/home/friends.rs friend_row | ✅ |
| friend name | me #1111/1112/1117 | 16/20 600 #dcdcdf; tag margin-left 4; username margin-left 5, 14/16 500 (hover) | — | ui/home/friends.rs | ✅ |
| friend subtext | me #1119 | 14/17.5 500 #81828a | --text-muted | ui/home/friends.rs | ✅ |
| row actions | me #1120–1124 `actionButton_5GqXgh` | gap 10, 36px circle, 20px icon #96979e: Message, More | — | ui/home/friends.rs | ✅ (Remove moved to the More menu) |
| Active Now column | me #1539–1543 | 360px, scroller padding 6 8 16, h2 20/24 600 margin 8 0 9 | — | ui/home/friends.rs active_now | ✅ |
| Active Now card | me #1577 `itemCard_-7KzVR` | margin-top 12, radius 16, 1px #97979f/.2, shadow 0 1 4 .14 | — | ui/home/friends.rs | ✅ (shadow ❌) |
| Active Now header | me #1579 `headerFull_o3hXTo` | padding 16, gap 12, rgba(151,151,159,.08); 32px avatar, 16/20 600, 14/18 #81828a, 24px icon r4 | — | ui/home/friends.rs | ✅ |
| Active Now game | me #1590–1600 `gameSection_o3hXTo` | padding 16, gap 12; 32px icon, 14/18 600 title, 12/16 sub, 24px party avatar | — | ui/home/friends.rs | ✅ |
| DM list Nitro/Shop/Quests | me (sidebar) | "Ton Nitro", "Boutique", "Quêtes" rows under Friends; no Message Requests row | — | sidebar (other owner) | — |
| DM profile panel | CDP `aside_HbUF2J` | 320px, padding 8 | — | ui/dm_profile_panel.rs | ✅ (new) |
| DM profile card | CDP `outer_P1YEjQ` / `inner_P1YEjQ` | padding 2 radius 8 / radius 6 | theme gradient → --border-subtle | ui/dm_profile_panel.rs | ✅ |
| DM banner | CDP `banner_vNEdYY` | 300x105; banner buttons 32px circles rgba(0,0,0,.56), top/right 8, gap 8 | — | ui/dm_profile_panel.rs | ✅ |
| DM avatar | CDP `avatar_WRLIGC` | 80px at (14, 55) | — | ui/dm_profile_panel.rs | ✅ |
| DM body | CDP `body_A7qlid` / `info_A7qlid` | margin 0 16, gap 20 / gap 8; name 20/24 600 | — | ui/dm_profile_panel.rs | ✅ |
| DM mutuals | CDP `mutuals_PXYMZ1` | padding 4 0, 12/16; 16px avatars -3 overlap | — | ui/dm_profile_panel.rs | ✅ servers (mutual friends ❌ no data) |
| DM bio | CDP `section_NcvRRg`, `viewFullBio_BcddaK` | 14/18 clamp; button margin-top 8, 12/16 | — | ui/dm_profile_panel.rs | ✅ |
| DM member since | CDP `headings_NcvRRg` | 12/16 600; value 14/18; gap 8 | — | ui/dm_profile_panel.rs | ✅ |
| DM widgets ("Collection de jeux") | CDP `overlay_P1YEjQ` breadcrumb | padding 8 radius 8, 24px icons gap 4 | — | — | ❌ no widget data |
| DM footer | CDP `redesignFooter_YBm90d` / `button_F7qvhY` | padding 16; 39px, radius 8, 16/20 500 "View Full Profile" | — | ui/dm_profile_panel.rs | ✅ |

## Context menus, tooltip, modal — live CDP dumps (2026-11-23)

Captured from the running client (read-only extracts in /tmp, private, not committed):
`user-context` (DM row menu), `message` (message menu), `guild-context`, `channel-context`,
submenu of `user-context-favorite-channel`, guild icon tooltip, settings `[role=dialog]`.
The `-channels-me-N+menumenu*.json` / `+dialog*.json` dumps in /mnt/windata/project-phoenix/dumps
do NOT contain the layer (it closed before the DOM snapshot); only their .png shows the menu.

| element | dump node | property | live value | our code | status |
|---|---|---|---|---|---|
| menu | `#user-context.menu_M3qk4j.flexible_M3qk4j` | width | content, 262 (DM) / 247 (msg) / 240 (guild); min 188 max 320 | context_menu.rs panel_at | ✅ (was fixed 220) |
| menu | same | border / radius / bg | 1px --border-subtle / 8 / --background-surface-higher | panel_at | ✅ |
| menu | same | shadow | --shadow-high 0 12 24 rgb(0 0 0/.24) | panel_at | ✅ (was shadow_lg) |
| menu | same | max-height | viewport − 2×16 (604 at 700), scrolls | panel_at VIEW_H | ✅ |
| scroller | `scroller_M3qk4j scrollerWithScrollbar_` | padding | 8 8 8 8 (end 0 with scrollbar) | panel_at | ✅ |
| item | `#user-context-user-profile.item_ labelContainer_` | padding / min-h / box | 8 / 32 / 245×34 | context_menu.rs row | ✅ (was fixed h 32) |
| item | same | radius / focused radius | 2 / 4 | row | ✅ |
| label | `label_M3qk4j > container_cmkR8z > span body-sm/medium` | font | 14/18 500 --text-strong | row | ✅ |
| item w/ icon | `#user-context-pin-dm` | height | 36 (20px icon + 16) | row_h | ✅ |
| left icon | `iconContainerLeft_M3qk4j` | size / margin | 20×20, margin-inline-end 8, --interactive-text-default | Action icon | ✅ (was on the right) |
| subtext | `#user-context-note .defaultColor_ body-xs/normal` | font / margin / color | 12/16 400, mt 2, --text-muted; row 52 | Action subtext | ✅ (new) |
| caret | `svg.caret_M3qk4j` | size / color | 20, margin-start 8, --text-muted | Submenu | ✅ (was --text-header) |
| separator | `div.separator_M3qk4j` | margin / border | 8px 0 / 1px --border-subtle | panel_at | ✅ (was 8 all, divider color) |
| danger | `#user-context-block` | color | --text-feedback-critical | row | ✅ |
| checkbox | `#guild-context-hide-muted-channels .checkboxIndicator_vTUVyc` | size / radius / border / bg | 20 / 4 / 1px checkbox-border-default / checkbox-background-default | Check | ✅ (was 18, blurple border) |
| submenu | 2nd `[role=menu]` while hovering `#user-context-favorite-channel` | x / y / w | parent right − 4 / row top − 9 / 188 | panel_at sub_panel | ✅ (was menu_w − 8, −6) |
| DM menu order | `#user-context` groups | items | Mark As Read ‖ Pin, Add to Favorites > ‖ Profile, Call, Add Note (subtext), Add Friend Nickname, Close DM ‖ Apps >, Invite to Server >, Remove Friend, Ignore, Block ‖ Mute @user > | nav_menus.rs | ✅ (Pin/Favorite/Nickname/Apps/Ignore are stubs) |
| message menu | `#message` | items | reaction strip (44px buttons, radius 8, gap 4) ‖ Add Reaction > ‖ Edit, Reply, Forward ‖ Copy Text, Pin, Bookmark, Remind Me >, Apps >, Mark Unread, Copy Link, Speak ‖ Delete ‖ Copy ID — all with LEFT icons | chat/panels.rs (not mine) | ❌ order/icons to align by chat owner |
| tooltip | `.tooltip_ZDdyhE` | padding / radius / bg / shadow | 8 12 / 8 / --background-surface-high / inset 1px --border-subtle + --shadow-high | tooltip.rs | ✅ (was legacy palette) |
| tooltip text | `.tooltip_ZDdyhE` | color | --text-default | tooltip.rs | ✅ |
| modal (settings) | `.container_dvN-MP.modal__F9ieB` | box / radius / border / shadow | 1200×620 at 40,40 / 12 / 1px --border-subtle / --shadow-high | modal.rs (radius/border ✅) | — settings owned elsewhere |
| modal close | `button[aria-label=Fermer].icon-only_F7qvhY.sm_` | size / radius / pos | 32 / 8 / top 8 right 8 | modal.rs close | ❌ top is 16 (no confirm-dialog dump yet) |

## Inbox, search options, pins, threads, Friends rows — live CDP dumps (2026-11-23)

Dumps (private, not committed): `inbox-unreads`, `inbox-mentions`, `inbox-saved`, `inbox-reminders`,
`inbox-scheduled`, `search-popout`, `pins`, `pins-list-hover`, `threads`, `threads-lrer`,
`friends-all`, `friends-pending` (viewport 1280×700, midnight theme). Node refs are dump `i`.
Illustrations (inbox upsells, pins/threads empty art) are CDN assets: drawn as the matching icon
in the same box. Code: ui/inbox.rs (inbox, threads, pins), ui/search.rs, ui/home/friends.rs.

| element | dump node | property | live value | our code | status |
|---|---|---|---|---|---|
| inbox popout | unreads #5 `container_uTMZTY` | box / w / max-h | x 483..1043, y 36 (right edge = inbox button) / 560 (480–750) / 70vh | inbox.rs inbox | ✅ (was 600 at top 52) |
| inbox popout | #5 | bg / radius / shadow | --background-surface-high / 8 / --shadow-border + --shadow-high | inbox.rs popout_shadow | ✅ (was bg-tertiary, shadow_lg) |
| inbox header | #6 `header_LSviKk` | padding | --space-md --space-md 0 (20 cozy) | inbox.rs inbox | ✅ |
| inbox title | #8/#9 | glyph / text | 20px --icon-strong, mr 8 / body-lg/semibold 20/24 --text-strong | inbox.rs | ✅ (was 24px icon, 20 bold) |
| inbox controls | #10–#19 `controls_`, `button_F7qvhY sm secondary`, `friendRequestsButton_QwUPp6` | gap / button | 8 / 32px, radius 8, --control-secondary-*, 16px glyph | inbox.rs sm_icon_button | ✅ Unreads: Mark All Read; Mentions: Filter; all tabs: friend requests |
| tab bar | #21/#22 `headerTabs_` `tabBar_ top_6zZYhF` | margin / border | 0 −16, mt 16 / 1px --border-subtle | inbox.rs tab_bar | ✅ |
| tabs | #23–#37 `tab_LSviKk item_ brand_` | order | Unreads, Mentions, Saved, Reminders, Scheduled (no "For You") | inbox.rs TABS | ✅ (was Mentions/Unreads/Saved) |
| tab | #23 | flex / padding / gap / margin | 1 / 0 8 16 / 8 / start --space-xs | inbox.rs tab_bar | ✅ |
| tab label | #25/#28 | font / colour | body-sm/semibold 14px, ellipsis / --text-subtle; selected --text-brand + 2px bar radius 2 2 0 0; hover --text-strong | inbox.rs tab_bar | ✅ (was pill) |
| unreads empty | #38–#44 `container_FgrH4y` | layout | min-h 360, padding 0 16; 80px circle --background-base-low, 36 glyph --text-muted, mb 16; 24/32 600 --text-strong mb 8; 12/16 --text-default + protip 14 500 --text-feedback-positive | inbox.rs caught_up | ✅ (protip text from dump) |
| mention card | mentions #39 `box_0JZW4Z` | bg / border / radius / mb | --background-surface-higher / 1px --border-subtle / --radius-md / 16 | inbox.rs inbox_box | ✅ (was bg-primary row) |
| mention head | #41 `channelHeader_8ZcKCk` | padding / gap / border | 12 / 8 / bottom --border-subtle; 32px guild icon r8, 12px chevron, 16/20 600 name with 12px type glyph, 12/16 guild line | inbox.rs channel_head | ✅ |
| mention buttons | #53–#61 `buttonGroup_` | gap / buttons | 8 / Jump + Mark Read (sm secondary icon) | inbox.rs message_entry | ✅ (was text "Jump" + ×) |
| mention message | #63 `singleMessage_ cozy_` | padding / avatar / name / time | 2 20 2 80 / 40 at 20 / 16/22 500 / 12 500 --chat-text-muted ml 4 | inbox.rs cozy_message | ✅ |
| scroller | #38 `scroller_0JZW4Z` | padding | 16 16 0 | inbox.rs | ✅ |
| saved / reminders / scheduled empty | saved #33–#73, reminders #1655, scheduled #1655 `container_jMz1Ee`/`_XzStbz` | layout | h 480, padding 16; 140×93 image mb 24; 24/32 600 --text-default mb 8; 14/18 500 --text-subtle with `strong` 600, mb 16 | inbox.rs upsell | ✅ (image drawn as icon) |
| upsell demo | saved #41 `messages_jMz1Ee`, #57 `menu_` | box / menu | --background-base-lowest, 1px --border-normal, r12, margin 16 0, padding 8 0; menu --background-surface-highest, 1px --border-subtle, r8, padding 4, start 300 / rows 6 8 gap 8, highlighted --background-mod-normal | inbox.rs wumpus_demo | ✅ |
| scheduled demo | scheduled #1662–#1673 | menu / input | menu ml 12 mb −5; `chatInput_` padding 8 12 gap 8 r8 --background-base-lowest, 24px plus r8 --background-mod-strong | inbox.rs scheduled_demo | ✅ |
| reminders / scheduled data | — | list | reminders & scheduled messages API not wired | — | ❌ upsell only |
| search options | search-popout #2 `container_cbVzxY` | box / pad / radius / border / bg / shadow | 356×auto at right 12, top header-bottom−4 / 12 8 / 8 / 1px --border-subtle / --background-surface-high / --shadow-high | search.rs options_popout | ✅ (was a 420px side panel) |
| search groups | #3–#5 `groupWrapper_lm2_KW`, `groupHeader_` | gap / header | 4 / padding 4 8, 12/16 600 --text-subtle | search.rs group_header | ✅ (was "SEARCH OPTIONS" list) |
| filter rows | #6–#43 `itemContainer_` `item_` | box / icon / text | r8, padding 4 8, 44px / 20px --text-muted mr 8 / label 14/18 500 --text-strong; key 14/18 600 --text-subtle + value 14/18 500 --text-muted gap 4 | search.rs option_row | ✅ From user, In channel, Has, Mentions, More filters |
| divider | #44 `divider_lm2_KW` | margin / colour | 8 −8 / --border-subtle | search.rs | ✅ |
| history | #45–#82 | header / rows | "History" + 18px trash; 28px rows: search glyph + query 14/18 600 --text-strong | search.rs (SearchHistory, session only) | ✅ (filter pills in history ❌) |
| search bar | header `searchBar_` | focused | 244×32 input in place, r8, --input-border-active | search.rs options_popout | ✅ |
| pins popout | pins #0 `messagesPopoutWrap_6bgemo` | box | 420, min-h 200, max 80vh, x 552..972 (right edge = pin button), y 81; --background-surface-high, popout shadow | inbox.rs pins_popout | ✅ (was 420 at right 16, bg-tertiary) |
| pins header | #1/#2 | padding / border / title | 16 / bottom --border-subtle / 24px glyph gap 8, 20/24 600 --text-strong | inbox.rs | ✅ (was 48px, 16px title) |
| pins empty | #6–#8 `emptyPlaceholder_` | layout | margin 8 0 32; 120px image; body mt 20, 16/20 500 --text-default, 2 lines | inbox.rs | ✅ |
| pins footer | #9–#12 `footer_6bgemo` | padding / bg / text | 16 / --background-mod-subtle / protip 14/18 600 --text-feedback-positive, tip 14/16 --text-default | inbox.rs | ✅ (new) |
| pinned message | pins-list-hover #1417 `messageGroupWrapper_6bgemo` | box | --background-surface-higher, 1px --border-subtle, r8, margin 0 4 8 0; scroller padding 8 0 0 8 | inbox.rs | ✅ |
| pinned message | #1418 `messageGroupCozy_` | padding | 16 20 16 80, margin-start −4 | inbox.rs cozy_message | ✅ |
| pin actions | #1438–#1443 `actionButtons_` | hover | top/right 8, gap 8, sm secondary "Jump" (+ unpin) | inbox.rs | ✅ |
| threads popout | threads-lrer #1411 `browser_ container_c0tu5x` | box | 480×400, x 408..888 (right edge = threads button), y 81, r8, --background-surface-high | inbox.rs threads | ✅ (was 600 at right 16) |
| threads header | #1412–#1428 `header_c0tu5x` | layout | 48px, padding 0 16, border-bottom; 24px glyph --interactive-text-default mr 8; 16/20 600 title mr 16; 1×24 divider mr 16; 248×32 sm search; primary sm "Create" ml 8 | inbox.rs threads | ✅ (search field is display-only) |
| thread section | #1432 `sectionHeader_6_wQXw` | box / font | 24px, margin 8 0 / 14px 500 --text-default | inbox.rs | ✅ ("N joined" / "older" split ❌: no joined data) |
| thread row | #1433 `container_yRVHqK` | box | 72px, padding 15, r8, 1px --border-subtle, --background-surface-higher, mb 8; hover --background-surface-highest + --border-normal | inbox.rs | ✅ |
| thread row text | #1435–#1437 | name / sub | 16/20 600 --text-default / 14/18 --text-default "No recent messages • ago" | inbox.rs | ✅ (last-activity / starter ❌ no data) |
| thread facepile | #1438 | size / overlap | 24px, −6, ml 16 | inbox.rs | ✅ |
| threads empty | threads #25–#34 `container_0FCMV2` | layout | padding 32; 256×144 art mb 12; 24/32 600 --text-strong mb 8; 16/20 --text-default; md primary "Create Thread" mt 24 | inbox.rs | ✅ |
| Friends title | friends-all #41, pending #41 | text | "Tous les amis - N" (hyphen), also on Pending | friends.rs list | ✅ (was em dash, Pending label) |
| pending row | pending #46–#60 `listItemContents_nByiE3` | layout | no hover discriminator; subtext = username 14/18 500 --text-subtle; actions ml 8; outgoing: one deny X | friends.rs friend_row | ✅ (was "Outgoing request" text) |
| row actions | all #59 `actionButton_5GqXgh` | bg / colour / hover | --background-base-lower / --interactive-text-default / --interactive-text-hover; accept --icon-feedback-positive, deny --icon-feedback-critical | friends.rs round_button | ✅ (glyphs were missing: chat/more-vertical now mapped) |
| row hover | `peopleListItem_:hover` | bg | --background-mod-subtle | friends.rs | ✅ |
| "Implicit" tab | friends-all header | — | client-mod (Vencord) tab, not Discord | — | — skipped |
